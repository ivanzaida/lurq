use std::sync::Arc;

use crate::core::signal::Signal;

pub struct Store<T: Send + Sync + 'static> {
  signal: Signal<T>,
}

impl<T: Clone + Send + Sync + 'static> Store<T> {
  pub fn new(value: T) -> Self {
    Self {
      signal: Signal::new(value),
    }
  }

  pub fn get(&self) -> T {
    self.signal.get()
  }

  pub fn set(&self, value: T) {
    self.signal.set(value);
  }

  pub fn update(&self, f: impl FnOnce(&mut T)) {
    self.signal.update(f);
  }

  pub fn with<R>(&self, f: impl FnOnce(&T) -> R) -> R {
    self.signal.with(f)
  }

  pub fn lens<R: Clone + PartialEq + Send + Sync + 'static>(
    &self,
    getter: impl Fn(&T) -> R + Send + Sync + 'static,
    setter: impl Fn(&mut T, R) + Send + Sync + 'static,
  ) -> Lens<T, R> {
    Lens {
      signal: self.signal.clone(),
      getter: Arc::new(getter),
      setter: Arc::new(setter),
    }
  }

  pub(crate) fn signal(&self) -> &Signal<T> {
    &self.signal
  }
}

impl<T: Clone + Send + Sync + 'static> Clone for Store<T> {
  fn clone(&self) -> Self {
    Self {
      signal: self.signal.clone(),
    }
  }
}

pub struct Lens<T: Send + Sync + 'static, R> {
  signal: Signal<T>,
  getter: Arc<dyn Fn(&T) -> R + Send + Sync>,
  setter: Arc<dyn Fn(&mut T, R) + Send + Sync>,
}

impl<T: Clone + Send + Sync + 'static, R: Clone + PartialEq + Send + Sync + 'static> Lens<T, R> {
  pub fn get(&self) -> R {
    self.signal.with(|v| (self.getter)(v))
  }

  pub fn set(&self, value: R) {
    let setter = self.setter.clone();
    self.signal.update(move |v| setter(v, value));
  }

  pub fn update(&self, f: impl FnOnce(&mut R)) {
    let getter = self.getter.clone();
    let setter = self.setter.clone();
    self.signal.update(move |v| {
      let mut field = getter(v);
      f(&mut field);
      setter(v, field);
    });
  }
}

impl<T: Send + Sync + 'static, R> Clone for Lens<T, R> {
  fn clone(&self) -> Self {
    Self {
      signal: self.signal.clone(),
      getter: self.getter.clone(),
      setter: self.setter.clone(),
    }
  }
}
