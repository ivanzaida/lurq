use std::{
  any::{Any, TypeId},
  collections::HashMap,
  sync::Arc,
};

use parking_lot::RwLock;

#[derive(Clone, Default)]
pub struct ContextMap {
  values: HashMap<TypeId, Arc<dyn Any + Send + Sync>>,
}

impl ContextMap {
  pub fn provide<T: Clone + Send + Sync + 'static>(&mut self, value: T) {
    self.values.insert(TypeId::of::<T>(), Arc::new(value));
  }

  pub fn get<T: Clone + Send + Sync + 'static>(&self) -> Option<T> {
    self
      .values
      .get(&TypeId::of::<T>())
      .and_then(|v| v.downcast_ref::<T>())
      .cloned()
  }
}

#[derive(Clone)]
pub struct ReactiveContext<T: Send + Sync + 'static> {
  inner: Arc<RwLock<ReactiveContextInner<T>>>,
}

struct ReactiveContextInner<T> {
  value: T,
  hash: u64,
  subscribers: Vec<Arc<dyn Fn() + Send + Sync>>,
}

impl<T: Send + Sync + 'static> ReactiveContext<T> {
  pub fn new(value: T) -> Self
  where
    T: std::hash::Hash,
  {
    let hash = compute_hash(&value);
    Self {
      inner: Arc::new(RwLock::new(ReactiveContextInner {
        value,
        hash,
        subscribers: Vec::new(),
      })),
    }
  }

  pub fn get(&self) -> T
  where
    T: Clone,
  {
    self.inner.read().value.clone()
  }

  pub fn set(&self, value: T)
  where
    T: std::hash::Hash,
  {
    let new_hash = compute_hash(&value);
    let mut inner = self.inner.write();
    if inner.hash != new_hash {
      inner.value = value;
      inner.hash = new_hash;
      let subs = inner.subscribers.clone();
      drop(inner);
      for sub in subs {
        sub();
      }
    } else {
      inner.value = value;
    }
  }

  pub(crate) fn subscribe(&self, f: impl Fn() + Send + Sync + 'static) {
    self.inner.write().subscribers.push(Arc::new(f));
  }
}

fn compute_hash<T: std::hash::Hash>(value: &T) -> u64 {
  use std::hash::Hasher;
  let mut hasher = std::collections::hash_map::DefaultHasher::new();
  value.hash(&mut hasher);
  hasher.finish()
}
