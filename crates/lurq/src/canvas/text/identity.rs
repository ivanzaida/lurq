//! Engine-local identity retention, independent of bitmap ownership.
use std::{cmp::Ordering, collections::BTreeMap, mem::size_of};

use super::{CanvasFont, Color};

pub(super) const MAX_KEYS: usize = 4096;
pub(super) const MAX_BYTES: usize = 1024 * 1024;
// Conservative policy charge for tree slots/links and the small root node.
// This is retained-cache accounting, not allocator/RSS or GPU memory.
const ROOT_CHARGE: usize = 16 * size_of::<(RenderedKey, Entry)>();
const ENTRY_CHARGE: usize = 3 * size_of::<(RenderedKey, Entry)>() + 128;

struct RenderedKey {
  text: Box<str>,
  font: CanvasFont,
  scale: f32,
  color: Color,
}

impl RenderedKey {
  fn new(text: &str, font: &CanvasFont, scale: f32, color: Color) -> Self {
    let mut font = font.clone();
    // Keep the old float equality semantics for signed zero. Nonfinite keys
    // never enter this table, so equality remains reflexive.
    font.size = zero(font.size);
    font.letter_spacing = zero(font.letter_spacing);
    Self {
      text: text.into(),
      font,
      scale: zero(scale),
      color,
    }
  }
}

fn zero(value: f32) -> f32 {
  if value == 0.0 { 0.0 } else { value }
}

impl Ord for RenderedKey {
  fn cmp(&self, other: &Self) -> Ordering {
    self
      .text
      .cmp(&other.text)
      .then_with(|| self.font.family.cmp(&other.font.family))
      .then_with(|| self.font.size.total_cmp(&other.font.size))
      .then_with(|| self.font.weight.value().cmp(&other.font.weight.value()))
      .then_with(|| (self.font.style as u8).cmp(&(other.font.style as u8)))
      .then_with(|| self.font.letter_spacing.total_cmp(&other.font.letter_spacing))
      .then_with(|| {
        self
          .font
          .font_features
          .iter()
          .map(|f| (f.tag(), f.value()))
          .cmp(other.font.font_features.iter().map(|f| (f.tag(), f.value())))
      })
      .then_with(|| self.scale.total_cmp(&other.scale))
      .then_with(|| {
        (self.color.r(), self.color.g(), self.color.b(), self.color.a()).cmp(&(
          other.color.r(),
          other.color.g(),
          other.color.b(),
          other.color.a(),
        ))
      })
  }
}
impl PartialOrd for RenderedKey {
  fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
    Some(self.cmp(other))
  }
}
impl PartialEq for RenderedKey {
  fn eq(&self, other: &Self) -> bool {
    self.cmp(other) == Ordering::Equal
  }
}
impl Eq for RenderedKey {}

struct Entry {
  id: u64,
  last: u64,
  charge: usize,
}

/// The font/alias database is immutable for this engine. No bitmap or GPU
/// resource is held here; identical complete inputs deterministically produce
/// the same raster. A retired key receives a fresh ID when encountered again.
pub(super) struct IdentityCache {
  entries: BTreeMap<RenderedKey, Entry>,
  bytes: usize,
  tick: u64,
}

impl IdentityCache {
  pub(super) fn new() -> Self {
    Self {
      entries: BTreeMap::new(),
      bytes: ROOT_CHARGE,
      tick: 0,
    }
  }

  pub(super) fn resolve(&mut self, text: &str, font: &CanvasFont, scale: f32, color: Color) -> u64 {
    if self.tick == u64::MAX {
      let mut entries: Vec<_> = self.entries.values_mut().collect();
      entries.sort_by_key(|entry| entry.last);
      for (index, entry) in entries.into_iter().enumerate() {
        entry.last = index as u64;
      }
      self.tick = self.entries.len() as u64;
    }
    self.tick += 1;
    let charge = ENTRY_CHARGE.saturating_add(super::key_payload_bytes(text, font));
    if !font.size.is_finite()
      || !font.letter_spacing.is_finite()
      || !scale.is_finite()
      || charge > MAX_BYTES - ROOT_CHARGE
    {
      #[cfg(feature = "perf_profile")]
      super::profile::identity_hit(false);
      return fresh_id();
    }
    let key = RenderedKey::new(text, font, scale, color);
    if let Some(entry) = self.entries.get_mut(&key) {
      entry.last = self.tick;
      #[cfg(feature = "perf_profile")]
      super::profile::identity_hit(true);
      return entry.id;
    }
    #[cfg(feature = "perf_profile")]
    super::profile::identity_hit(false);
    while !self.entries.is_empty() && (self.entries.len() >= MAX_KEYS || self.bytes + charge > MAX_BYTES) {
      let oldest = self.entries.iter().min_by_key(|(_, entry)| entry.last).unwrap().0;
      let oldest = RenderedKey::new(&oldest.text, &oldest.font, oldest.scale, oldest.color);
      self.bytes -= self.entries.remove(&oldest).unwrap().charge;
      #[cfg(feature = "perf_profile")]
      super::profile::identity_evicted();
    }
    let id = fresh_id();
    self.bytes += charge;
    self.entries.insert(
      key,
      Entry {
        id,
        last: self.tick,
        charge,
      },
    );
    id
  }

  #[cfg(any(test, feature = "perf_profile"))]
  pub(super) fn len(&self) -> usize {
    self.entries.len()
  }
  #[cfg(any(test, feature = "perf_profile"))]
  pub(super) fn charged_bytes(&self) -> usize {
    self.bytes
  }
}

fn fresh_id() -> u64 {
  // Globally allocated identities are never recycled on cache/engine eviction.
  // Refuse exhaustion instead of allowing the reserved high bit to wrap.
  let id = super::super::NEXT_CANVAS_ID
    .fetch_update(
      std::sync::atomic::Ordering::Relaxed,
      std::sync::atomic::Ordering::Relaxed,
      |id| id.checked_add(1).filter(|next| *next < (1u64 << 63)),
    )
    .expect("Canvas asset identity space exhausted");
  id | (1u64 << 63)
}

#[cfg(test)]
mod tests;
