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
    Self { inner: RefCell::new(None) }
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

  pub fn patch_scroll_offset(&self, x: f32, y: f32) {
    let mut borrow = self.inner.borrow_mut();
    if let Some(cached) = borrow.as_mut() {
      if let Some(child) = cached.result.children.first_mut() {
        child.offset.x = x;
        child.offset.y = y;
      }
    }
  }
}

impl Default for LayoutCache {
  fn default() -> Self {
    Self::new()
  }
}
