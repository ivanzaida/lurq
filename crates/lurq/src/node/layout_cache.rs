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

  pub(crate) fn preserve_from(&self, old: &Self) {
    *self.inner.borrow_mut() = old.inner.borrow().clone();
    self.clear_dirty();
  }

  pub fn store(&self, constraints: Constraints, result: LayoutResult) {
    let mut borrow = self.inner.borrow_mut();
    if let Some(index) = borrow.iter().position(|cached| cached.constraints == constraints) {
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
