use std::cell::{Cell, RefCell};

use crate::layout::{Constraints, layout_result::LayoutResult};

const MAX_CACHED_LAYOUTS: usize = 2;

pub struct LayoutCache {
  inner: RefCell<Vec<CachedLayout>>,
  local_dirty: Cell<bool>,
  descendant_dirty: Cell<bool>,
}

#[derive(Clone)]
struct CachedLayout {
  constraints: Constraints,
  result: LayoutResult,
}

impl LayoutCache {
  pub fn new() -> Self {
    Self {
      inner: RefCell::new(Vec::new()),
      local_dirty: Cell::new(false),
      descendant_dirty: Cell::new(false),
    }
  }

  pub fn get(&self, constraints: Constraints) -> Option<LayoutResult> {
    if self.is_dirty() {
      return None;
    }
    self.get_cached(constraints)
  }

  pub(crate) fn contains(&self, constraints: Constraints) -> bool {
    if self.is_dirty() {
      return false;
    }
    self
      .inner
      .borrow()
      .iter()
      .any(|cached| cached.constraints == constraints)
  }

  pub(crate) fn get_dirty(&self, constraints: Constraints) -> Option<LayoutResult> {
    self.get_cached(constraints)
  }

  fn get_cached(&self, constraints: Constraints) -> Option<LayoutResult> {
    let borrow = self.inner.borrow();
    for cached in borrow.iter() {
      if cached.constraints == constraints {
        return Some(cached.result.clone());
      }
    }
    None
  }

  pub(crate) fn constraints(&self) -> Option<Constraints> {
    self.inner.borrow().first().map(|cached| cached.constraints)
  }

  pub(crate) fn has_cached_result(&self) -> bool {
    !self.inner.borrow().is_empty()
  }

  /// Constraints and size of the most recently stored result, if any.
  pub(crate) fn cached_entry(&self) -> Option<(Constraints, crate::layout::Size)> {
    self
      .inner
      .borrow()
      .first()
      .map(|cached| (cached.constraints, cached.result.size))
  }

  pub(crate) fn preserve_from(&self, old: &Self) {
    *self.inner.borrow_mut() = old.inner.borrow().clone();
    // Carry the old cache's unresolved dirtiness instead of clearing it: the
    // old tree may hold marks no layout pass has consumed yet (re-render
    // chains between paints). Dropping them here laundered staleness — a
    // duplicate re-render inherited the stale results with clean flags and
    // the engine served them wholesale (stale spacer heights / stale text
    // measurements on screen).
    self.local_dirty.set(self.local_dirty.get() || old.local_dirty.get());
    self
      .descendant_dirty
      .set(self.descendant_dirty.get() || old.descendant_dirty.get());
  }

  pub fn store(&self, constraints: Constraints, result: LayoutResult) {
    let mut borrow = self.inner.borrow_mut();
    if self.is_dirty() {
      // The dirty flags are cache-wide, but a store only replaces the entry
      // for the constraints just laid out. Any other cached entry predates
      // the invalidation and is equally stale — clearing the flags below
      // while keeping it would launder it into a servable result (observed
      // in production: a two-entry cache under oscillating constraints — a
      // scrollbar gutter toggling a column's width — served a pre-change
      // sibling layout with clean flags, freezing a text row at its old
      // child offsets).
      borrow.clear();
    } else if let Some(index) = borrow.iter().position(|cached| cached.constraints == constraints) {
      borrow.remove(index);
    }
    borrow.insert(0, CachedLayout { constraints, result });
    borrow.truncate(MAX_CACHED_LAYOUTS);
    self.clear_dirty();
  }

  pub fn invalidate(&self) {
    self.inner.borrow_mut().clear();
    self.clear_dirty();
  }

  pub(crate) fn mark_local_dirty(&self) {
    self.local_dirty.set(true);
  }

  pub(crate) fn mark_descendant_dirty(&self) {
    self.descendant_dirty.set(true);
  }

  pub(crate) fn is_local_dirty(&self) -> bool {
    self.local_dirty.get()
  }

  pub(crate) fn is_descendant_dirty(&self) -> bool {
    self.descendant_dirty.get()
  }

  pub(crate) fn is_dirty(&self) -> bool {
    self.is_local_dirty() || self.is_descendant_dirty()
  }

  fn clear_dirty(&self) {
    self.local_dirty.set(false);
    self.descendant_dirty.set(false);
  }

  pub(crate) fn estimated_memory_bytes(&self) -> usize {
    let borrow = self.inner.borrow();
    let cached_bytes = borrow
      .iter()
      .map(|cached| std::mem::size_of::<CachedLayout>() + cached.result.estimated_memory_bytes())
      .sum::<usize>();
    std::mem::size_of::<Self>() + borrow.capacity() * std::mem::size_of::<CachedLayout>() + cached_bytes
  }
}

impl Default for LayoutCache {
  fn default() -> Self {
    Self::new()
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::layout::Size;

  fn constraints(max_width: f32) -> Constraints {
    Constraints {
      min_width: 0.0,
      min_height: 0.0,
      max_width,
      max_height: f32::MAX,
    }
  }

  fn result(width: f32) -> LayoutResult {
    LayoutResult {
      size: Size::new(width, 10.0),
      children: vec![],
    }
  }

  /// A dirty cache must not keep sibling entries across a store: they were
  /// computed before the invalidation, and clearing the cache-wide dirty
  /// flags while keeping them would serve them as fresh once the caller's
  /// constraints oscillate back (the studio skills-viewer bug: a scrollbar
  /// gutter toggled a column's width per pass, and a text row whose content
  /// grew kept serving its pre-change layout from the second cache slot).
  #[test]
  fn store_on_a_dirty_cache_drops_stale_sibling_entries() {
    let cache = LayoutCache::new();
    cache.store(constraints(100.0), result(40.0));
    cache.store(constraints(90.0), result(38.0));
    assert!(cache.get(constraints(100.0)).is_some());
    assert!(cache.get(constraints(90.0)).is_some());

    // Content changed: both entries are stale. Repair relayouts under one
    // constraint set only.
    cache.mark_descendant_dirty();
    cache.store(constraints(90.0), result(45.0));

    assert_eq!(
      cache.get(constraints(90.0)).map(|cached| cached.size.width),
      Some(45.0),
      "the freshly stored entry is served"
    );
    assert!(
      cache.get(constraints(100.0)).is_none(),
      "the pre-invalidation sibling entry must not be served as fresh"
    );
  }

  /// A clean cache keeps memoizing both constraint sets (the two-slot memo
  /// exists for constraint oscillation with unchanged content).
  #[test]
  fn clean_stores_keep_both_entries() {
    let cache = LayoutCache::new();
    cache.store(constraints(100.0), result(40.0));
    cache.store(constraints(90.0), result(38.0));
    assert_eq!(
      cache.get(constraints(100.0)).map(|cached| cached.size.width),
      Some(40.0)
    );
    assert_eq!(cache.get(constraints(90.0)).map(|cached| cached.size.width), Some(38.0));
  }
}
