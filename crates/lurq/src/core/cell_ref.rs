use std::sync::Arc;

use parking_lot::RwLock;

pub struct Ref<T> {
  inner: Arc<RwLock<T>>,
}

impl<T> Ref<T> {
  pub fn new(value: T) -> Self {
    Self {
      inner: Arc::new(RwLock::new(value)),
    }
  }

  pub fn get(&self) -> T
  where
    T: Clone,
  {
    self.inner.read().clone()
  }

  pub fn set(&self, value: T) {
    *self.inner.write() = value;
  }

  pub fn with<R>(&self, f: impl FnOnce(&T) -> R) -> R {
    f(&self.inner.read())
  }

  pub fn update(&self, f: impl FnOnce(&mut T)) {
    f(&mut self.inner.write());
  }
}

impl<T> Clone for Ref<T> {
  fn clone(&self) -> Self {
    Self {
      inner: Arc::clone(&self.inner),
    }
  }
}
