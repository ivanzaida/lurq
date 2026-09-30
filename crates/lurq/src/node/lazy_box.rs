use std::ops::{Deref, DerefMut};

/// A rarely-set part of a [`Node`](super::Node), kept on the heap and allocated
/// on first write.
///
/// Nodes are built by value in component render functions and moved through
/// `Ctx::mount`, and an unoptimized build gives every such temporary its own
/// stack slot, so a large inline field grows every frame on the render path
/// (lurq#25). Until it is written, a `LazyBox` is one null pointer and reads
/// through [`EmptyDefault::empty`].
pub(crate) struct LazyBox<T>(Option<Box<T>>);

/// A shared, immutable empty value read through an unset [`LazyBox`].
pub(crate) trait EmptyDefault: Default + 'static {
  fn empty() -> &'static Self;
}

impl<T> LazyBox<T> {
  pub(crate) const fn new() -> Self {
    Self(None)
  }
}

impl<T> Default for LazyBox<T> {
  fn default() -> Self {
    Self::new()
  }
}

impl<T: Clone> Clone for LazyBox<T> {
  fn clone(&self) -> Self {
    Self(self.0.clone())
  }
}

impl<T: EmptyDefault> Deref for LazyBox<T> {
  type Target = T;

  fn deref(&self) -> &T {
    match &self.0 {
      Some(value) => value,
      None => T::empty(),
    }
  }
}

impl<T: EmptyDefault> DerefMut for LazyBox<T> {
  fn deref_mut(&mut self) -> &mut T {
    self.0.get_or_insert_with(Default::default)
  }
}

/// Implements [`EmptyDefault`] with a lazily built process-wide default.
macro_rules! empty_default {
  ($ty:ty) => {
    impl $crate::node::lazy_box::EmptyDefault for $ty {
      fn empty() -> &'static Self {
        static EMPTY: std::sync::LazyLock<$ty> = std::sync::LazyLock::new(<$ty>::default);
        &EMPTY
      }
    }
  };
}
pub(crate) use empty_default;
