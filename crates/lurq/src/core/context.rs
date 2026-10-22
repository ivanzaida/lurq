use std::{
  any::{Any, TypeId},
  collections::HashMap,
  sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
  },
};

use parking_lot::RwLock;

static NEXT_CONTEXT_REVISION: AtomicU64 = AtomicU64::new(1);

fn next_revision() -> u64 {
  NEXT_CONTEXT_REVISION.fetch_add(1, Ordering::Relaxed)
}

/// A provided value with the revision of its providing. A component that read
/// the value's type renders again when the revision of the entry it would
/// read now differs from the one it last inherited.
#[derive(Clone)]
pub(crate) struct ContextEntry {
  value: Arc<dyn Any + Send + Sync>,
  revision: u64,
}

impl ContextEntry {
  pub(crate) fn value<T: 'static>(&self) -> Option<&T> {
    self.value.downcast_ref::<T>()
  }
}

/// The values provided to a component, by type.
///
/// The map's revision changes whenever its entries do: every `provide` gives
/// the map and the entry a fresh one, and a map layered over another takes a
/// fresh one. A component whose render provides the entries its previous
/// render did (`Ctx::provide_if_changed`) gives the map its previous revision
/// back, so its children find nothing to inherit.
#[derive(Clone, Default)]
pub struct ContextMap {
  values: HashMap<TypeId, ContextEntry>,
  revision: u64,
}

impl ContextMap {
  pub fn provide<T: Clone + Send + Sync + 'static>(&mut self, value: T) {
    self.insert(
      TypeId::of::<T>(),
      ContextEntry {
        value: Arc::new(value),
        revision: next_revision(),
      },
    );
  }

  /// Puts `entry` in as the value of `id`, keeping its revision: a value
  /// provided again, unchanged, is the entry of its earlier providing.
  pub(crate) fn insert(&mut self, id: TypeId, entry: ContextEntry) {
    self.values.insert(id, entry);
    self.revision = next_revision();
  }

  pub fn get<T: Clone + Send + Sync + 'static>(&self) -> Option<T> {
    self
      .entry(TypeId::of::<T>())
      .and_then(ContextEntry::value::<T>)
      .cloned()
  }

  pub(crate) fn entry(&self, id: TypeId) -> Option<&ContextEntry> {
    self.values.get(&id)
  }

  /// The revision of the entry of `id`, `None` when there is none.
  pub(crate) fn entry_revision(&self, id: TypeId) -> Option<u64> {
    self.values.get(&id).map(|entry| entry.revision)
  }

  pub(crate) fn revision(&self) -> u64 {
    self.revision
  }

  pub(crate) fn set_revision(&mut self, revision: u64) {
    self.revision = revision;
  }

  pub(crate) fn is_empty(&self) -> bool {
    self.values.is_empty()
  }

  /// Whether `other` holds the entries this map holds: the same types, each
  /// from the same providing.
  pub(crate) fn has_same_entries(&self, other: &ContextMap) -> bool {
    self.values.len() == other.values.len()
      && self
        .values
        .iter()
        .all(|(id, entry)| other.entry_revision(*id) == Some(entry.revision))
  }

  /// `inherited` with `provided` layered on top: a component's own values
  /// shadow those of its ancestors. The result gets a fresh revision when
  /// `provided` is non-empty so descendants notice the inherited change.
  pub(crate) fn layered(inherited: &ContextMap, provided: &ContextMap) -> ContextMap {
    let mut map = inherited.clone();
    if provided.values.is_empty() {
      return map;
    }
    map
      .values
      .extend(provided.values.iter().map(|(id, entry)| (*id, entry.clone())));
    map.revision = next_revision();
    map
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
