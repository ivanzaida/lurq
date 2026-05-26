use std::ops::{Deref, DerefMut};

pub struct Guard<T> {
  value: T,
  changed: bool,
}

impl<T> Guard<T> {
  pub fn new(value: T) -> Self {
    Self { value, changed: true }
  }

  pub fn is_changed(&self) -> bool {
    self.changed
  }

  pub fn clear_changed(&mut self) {
    self.changed = false;
  }

  pub fn set(&mut self, value: T) {
    self.value = value;
    self.changed = true;
  }

  pub fn into_inner(self) -> T {
    self.value
  }
}

impl<T> Deref for Guard<T> {
  type Target = T;

  fn deref(&self) -> &T {
    &self.value
  }
}

impl<T> DerefMut for Guard<T> {
  fn deref_mut(&mut self) -> &mut T {
    self.changed = true;
    &mut self.value
  }
}

impl<T: Clone> Clone for Guard<T> {
  fn clone(&self) -> Self {
    Self {
      value: self.value.clone(),
      changed: self.changed,
    }
  }
}

impl<T: Default> Default for Guard<T> {
  fn default() -> Self {
    Self::new(T::default())
  }
}
