use std::sync::Arc;

use super::handle::RouterHandle;
use crate::core::signal::Signal;

#[derive(Clone)]
pub struct Navigator {
  pub(crate) handle: RouterHandle,
}

impl Navigator {
  pub fn push(&self, path: impl Into<String>) {
    self.handle.push(path);
  }

  pub fn replace(&self, path: impl Into<String>) {
    self.handle.replace(path);
  }

  pub fn back(&self) -> bool {
    self.handle.back()
  }

  pub fn forward(&self) -> bool {
    self.handle.forward()
  }

  pub fn path(&self) -> Signal<String> {
    self.handle.path()
  }
}

impl std::hash::Hash for Navigator {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    (Arc::as_ptr(&self.handle.inner) as usize).hash(state);
  }
}
