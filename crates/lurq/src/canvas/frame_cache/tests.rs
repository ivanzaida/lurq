use std::collections::HashMap;

use super::*;

const LIMITS: Limits = Limits {
  budget: 100,
  ceiling: 200,
};

/// Values are their own keys, so an eviction names what it evicted.
#[derive(Default)]
struct Frames {
  misses: Vec<Vec<u64>>,
  uncached: Vec<Vec<u64>>,
  evicted: Vec<u64>,
}

impl Frames {
  /// One frame looking `keys` up in order and caching each miss.
  fn draw(&mut self, cache: &mut FrameCache<u64>, keys: impl IntoIterator<Item = u64>, bytes: usize) {
    let (mut misses, mut uncached) = (Vec::new(), Vec::new());
    for key in keys {
      if cache.get(key).is_some() {
        continue;
      }
      misses.push(key);
      if cache.insert(key, key, bytes, |value| self.evicted.push(value)).is_err() {
        uncached.push(key);
      }
    }
    cache.close_frame(|value| self.evicted.push(value));
    self.misses.push(misses);
    self.uncached.push(uncached);
  }
}

#[test]
fn a_frame_larger_than_the_budget_is_kept_whole_from_the_second_frame() {
  let mut cache = FrameCache::framed(LIMITS);
  let mut frames = Frames::default();
  for _ in 0..5 {
    // 15 entries of 10 bytes: over the budget of 100, within the ceiling.
    frames.draw(&mut cache, 0..15, 10);
  }
  assert_eq!(frames.misses[0], (0..15).collect::<Vec<_>>());
  assert!(frames.misses[1..].iter().all(Vec::is_empty), "{:?}", frames.misses);
  assert!(frames.evicted.is_empty());
  assert_eq!((cache.len(), cache.bytes()), (15, 150));
}

#[test]
fn below_the_budget_nothing_is_evicted() {
  let mut cache = FrameCache::framed(LIMITS);
  let mut frames = Frames::default();
  for frame in 0..6u64 {
    // A rotating subset of a set that fits the budget.
    frames.draw(&mut cache, (0..10).filter(|key| (key + frame) % 3 != 0), 10);
  }
  assert!(frames.evicted.is_empty());
  assert_eq!(cache.bytes(), 100);
  assert!(frames.misses[3..].iter().all(Vec::is_empty));
}

#[test]
fn beyond_the_ceiling_the_overflow_is_not_cached_and_the_rest_stays() {
  let mut cache = FrameCache::framed(LIMITS);
  let mut frames = Frames::default();
  for _ in 0..5 {
    frames.draw(&mut cache, 0..30, 10);
  }
  let overflow: Vec<_> = (20..30).collect();
  assert_eq!(frames.uncached, vec![overflow.clone(); 5]);
  assert!(
    frames.misses[1..].iter().all(|misses| *misses == overflow),
    "the 20 cached entries stay cached: {:?}",
    frames.misses
  );
  assert!(frames.evicted.is_empty(), "nothing thrashes: {:?}", frames.evicted);
  assert_eq!((cache.len(), cache.bytes()), (20, 200));
}

#[test]
fn the_cache_returns_to_its_budget_once_frames_need_less() {
  let mut cache = FrameCache::framed(LIMITS);
  let mut frames = Frames::default();
  frames.draw(&mut cache, 0..15, 10);
  assert_eq!(cache.bytes(), 150);
  frames.draw(&mut cache, 0..2, 10);
  assert_eq!(cache.bytes(), 150, "entries of the previous frame stay");
  frames.draw(&mut cache, 0..2, 10);
  assert_eq!(cache.bytes(), 100);
  assert_eq!(frames.evicted, (2..7).collect::<Vec<_>>(), "oldest first");
  for _ in 0..3 {
    // Passes that use nothing do not age what the last frames used.
    cache.close_frame(|value| frames.evicted.push(value));
  }
  assert_eq!(frames.evicted.len(), 5);
  frames.draw(&mut cache, 0..2, 10);
  assert!(frames.misses[3].is_empty());
}

#[test]
fn eviction_never_takes_an_entry_of_the_current_or_previous_frame_and_stays_bounded() {
  let mut cache = FrameCache::framed(LIMITS);
  // The test's own record of the frame each key was last used in.
  let mut used: HashMap<u64, usize> = HashMap::new();
  let mut charged: HashMap<u64, usize> = HashMap::new();
  let mut random = 0x2545_f491_4f6c_dd1du64;
  let mut next = |bound: u64| {
    random ^= random << 13;
    random ^= random >> 7;
    random ^= random << 17;
    random % bound
  };
  for frame in 2..400usize {
    let start = next(60);
    let count = 1 + next(25);
    for key in start..start + count {
      used.insert(key, frame);
      if cache.get(key).is_some() {
        continue;
      }
      let bytes = 1 + (key as usize * 7) % 13;
      let mut evicted = Vec::new();
      let kept = cache.insert(key, key, bytes, |value| evicted.push(value)).is_ok();
      for value in evicted {
        assert!(
          used[&value] + 1 < frame,
          "frame {frame} evicted {value}, used in {}",
          used[&value]
        );
        charged.remove(&value);
      }
      if kept {
        charged.insert(key, bytes);
      }
      assert!(cache.bytes() <= LIMITS.ceiling);
    }
    let mut evicted = Vec::new();
    cache.close_frame(|value| evicted.push(value));
    for value in evicted {
      assert!(
        used[&value] + 1 < frame,
        "closing {frame} evicted {value}, used in {}",
        used[&value]
      );
      charged.remove(&value);
    }
    assert_eq!(cache.bytes(), charged.values().sum::<usize>());
    let recent: usize = charged
      .iter()
      .filter(|(key, _)| used[*key] + 1 >= frame)
      .map(|(_, b)| b)
      .sum();
    assert!(
      cache.bytes() <= LIMITS.budget.max(recent),
      "frame {frame}: {} bytes",
      cache.bytes()
    );
  }
}

#[test]
fn before_its_first_frame_closes_the_cache_is_lru_within_the_budget() {
  let mut cache = FrameCache::new(LIMITS);
  let mut evicted = Vec::new();
  for key in 0..10 {
    cache.insert(key, key, 10, |value| evicted.push(value)).unwrap();
  }
  assert!(cache.get(0).is_some());
  cache.insert(10, 10, 10, |value| evicted.push(value)).unwrap();
  cache.insert(11, 11, 10, |value| evicted.push(value)).unwrap();
  assert_eq!(evicted, [1, 2], "least recently used first, never above the budget");
  assert!(cache.insert(12, 12, 101, |value| evicted.push(value)).is_err());
  assert_eq!(cache.bytes(), 100);
}

#[test]
fn a_replaced_key_hands_its_old_value_to_evict() {
  let mut cache = FrameCache::framed(LIMITS);
  let mut evicted = Vec::new();
  cache.insert(1, 10, 10, |value| evicted.push(value)).unwrap();
  cache.insert(1, 11, 20, |value| evicted.push(value)).unwrap();
  assert_eq!(evicted, [10]);
  assert_eq!((cache.peek(1), cache.bytes()), (Some(&11), 20));
}

#[test]
fn the_access_queue_stays_proportional_to_the_entries() {
  let mut cache = FrameCache::framed(LIMITS);
  for frame in 0..1000 {
    for key in 0..8 {
      if cache.get(key).is_none() {
        cache.insert(key, frame, 10, drop).unwrap();
      }
    }
    cache.close_frame(drop);
  }
  assert!(cache.order.len() <= 2 * cache.len() + 64);
}
