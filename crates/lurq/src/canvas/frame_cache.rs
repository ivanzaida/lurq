//! Frame-stamped residency for caches that an application refills in the same
//! order every frame: the GPU asset textures and the shaped text.
//!
//! Least-recently-used eviction fails such a cache as soon as one frame needs
//! more than its budget: the entries it evicts are the ones the next frame asks
//! for first, so every lookup misses. Here an entry used in the current or the
//! previous frame is never evicted. A frame whose own entries need more than
//! the budget stretches the cache up to its ceiling, and what would exceed the
//! ceiling is not cached at all, so the entries that are cached stay put.
use std::collections::{HashMap, VecDeque};

#[cfg(test)]
mod tests;

/// Charged bytes a cache settles at between frames, and the most that the
/// current and previous frames' own entries may stretch it to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Limits {
  pub budget: usize,
  pub ceiling: usize,
}

struct Resident<V> {
  value: V,
  bytes: usize,
  /// The frame (or, before frames are known, the access) that last used it.
  last: u64,
}

/// Keys are 64-bit identities. A frame ends with [`close_frame`](Self::close_frame)
/// and the next one begins at the following lookup, so closes without any use
/// in between, such as UI passes that draw no canvas, do not age entries.
///
/// Until the owner closes its first frame the cache does not know where frames
/// are, and is a plain least-recently-used cache within its budget.
pub(crate) struct FrameCache<V> {
  entries: HashMap<u64, Resident<V>>,
  /// `(last, key)` in nondecreasing `last` order, oldest first. An item is stale
  /// once its key has been used again or removed; stale items are skipped and
  /// periodically compacted away, so each use and eviction is amortized O(1).
  order: VecDeque<(u64, u64)>,
  limits: Limits,
  bytes: usize,
  tick: u64,
  open: bool,
  framed: bool,
}

impl<V> FrameCache<V> {
  pub(crate) fn new(limits: Limits) -> Self {
    debug_assert!(limits.budget <= limits.ceiling);
    Self {
      entries: HashMap::new(),
      order: VecDeque::new(),
      limits,
      bytes: 0,
      tick: 0,
      open: false,
      framed: false,
    }
  }
  /// For an owner that closes every frame from the first one.
  #[cfg_attr(
    not(any(test, feature = "wgpu", all(feature = "dx12", target_os = "windows"))),
    allow(dead_code)
  )]
  pub(crate) fn framed(limits: Limits) -> Self {
    Self {
      framed: true,
      ..Self::new(limits)
    }
  }

  pub(crate) fn limits(&self) -> Limits {
    self.limits
  }
  /// Charged bytes of every entry the cache holds.
  pub(crate) fn bytes(&self) -> usize {
    self.bytes
  }
  #[cfg(any(test, feature = "perf_profile"))]
  pub(crate) fn len(&self) -> usize {
    self.entries.len()
  }

  #[cfg(test)]
  pub(crate) fn values(&self) -> impl Iterator<Item = &V> {
    self.entries.values().map(|entry| &entry.value)
  }

  /// Looks `key` up without counting it as used.
  #[cfg(any(test, feature = "wgpu"))]
  pub(crate) fn peek(&self, key: u64) -> Option<&V> {
    self.entries.get(&key).map(|entry| &entry.value)
  }

  /// Looks `key` up and counts it as used in the current frame.
  pub(crate) fn get(&mut self, key: u64) -> Option<&V> {
    let stamp = self.stamp();
    let entry = self.entries.get_mut(&key)?;
    if entry.last != stamp {
      entry.last = stamp;
      self.order.push_back((stamp, key));
    }
    self.compact();
    self.entries.get(&key).map(|entry| &entry.value)
  }

  /// Caches `value`, replacing what `key` held. Entries that are no longer
  /// protected make room first, oldest first. `Err` hands the value back when
  /// it cannot be cached without evicting an entry of the current or previous
  /// frame or going past the ceiling (the budget, before frames are known).
  pub(crate) fn insert(&mut self, key: u64, value: V, bytes: usize, mut evict: impl FnMut(V)) -> Result<(), V> {
    let stamp = self.stamp();
    if let Some(old) = self.entries.remove(&key) {
      self.bytes -= old.bytes;
      evict(old.value);
    }
    let limit = if self.framed {
      self.limits.ceiling
    } else {
      self.limits.budget
    };
    if bytes > limit {
      return Err(value);
    }
    self.evict_older_than(
      self.protected_from(),
      self.limits.budget.saturating_sub(bytes),
      &mut evict,
    );
    if self.bytes + bytes > limit {
      return Err(value);
    }
    self.bytes += bytes;
    self.entries.insert(
      key,
      Resident {
        value,
        bytes,
        last: stamp,
      },
    );
    self.order.push_back((stamp, key));
    self.compact();
    Ok(())
  }

  /// Ends the current frame and shrinks the cache back toward its budget with
  /// entries that neither this frame nor the previous one used. From now on
  /// the cache knows where frames are. Does nothing more if nothing was used
  /// since the last close.
  pub(crate) fn close_frame(&mut self, mut evict: impl FnMut(V)) {
    self.framed = true;
    if self.open {
      self.open = false;
      self.evict_older_than(self.protected_from(), self.limits.budget, &mut evict);
    }
  }

  /// The oldest stamp that may not be evicted: that of the previous frame, or
  /// none before frames are known.
  fn protected_from(&self) -> u64 {
    if self.framed {
      self.tick.saturating_sub(1)
    } else {
      u64::MAX
    }
  }

  fn stamp(&mut self) -> u64 {
    if !self.framed || !self.open {
      self.tick += 1;
      self.open = true;
    }
    self.tick
  }

  /// Evicts entries last used before `keep`, oldest first, until the cache
  /// is charged at most `target` bytes or no such entry is left.
  fn evict_older_than(&mut self, keep: u64, target: usize, evict: &mut impl FnMut(V)) {
    while self.bytes > target {
      let Some(&(last, key)) = self.order.front() else {
        return;
      };
      if self.entries.get(&key).is_some_and(|entry| entry.last == last) {
        if last >= keep {
          return;
        }
        let entry = self.entries.remove(&key).unwrap();
        self.bytes -= entry.bytes;
        evict(entry.value);
      }
      self.order.pop_front();
    }
  }

  fn compact(&mut self) {
    if self.order.len() <= 2 * self.entries.len() + 64 {
      return;
    }
    let mut order: Vec<_> = self.entries.iter().map(|(key, entry)| (entry.last, *key)).collect();
    order.sort_unstable();
    self.order = order.into();
  }
}
