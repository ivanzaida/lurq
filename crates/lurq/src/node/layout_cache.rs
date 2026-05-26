use std::cell::RefCell;

use crate::layout::{Constraints, layout_result::LayoutResult};

pub struct LayoutCache {
  inner: RefCell<Option<CachedLayout>>,
}

struct CachedLayout {
  constraints: Constraints,
  result: LayoutResult,
}

impl LayoutCache {
  pub fn new() -> Self {
    Self {
      inner: RefCell::new(None),
    }
  }

  pub fn get(&self, constraints: Constraints) -> Option<LayoutResult> {
    let borrow = self.inner.borrow();
    if let Some(cached) = borrow.as_ref() {
      if cached.constraints == constraints {
        return Some(cached.result.clone());
      }
    }
    None
  }

  pub fn store(&self, constraints: Constraints, result: LayoutResult) {
    *self.inner.borrow_mut() = Some(CachedLayout { constraints, result });
  }

  pub fn invalidate(&self) {
    *self.inner.borrow_mut() = None;
  }

  pub(crate) fn estimated_memory_bytes(&self) -> usize {
    let borrow = self.inner.borrow();
    let cached_bytes = borrow
      .as_ref()
      .map(|cached| std::mem::size_of::<CachedLayout>() + cached.result.estimated_memory_bytes())
      .unwrap_or(0);
    std::mem::size_of::<Self>() + cached_bytes
  }
}

impl Default for LayoutCache {
  fn default() -> Self {
    Self::new()
  }
}
