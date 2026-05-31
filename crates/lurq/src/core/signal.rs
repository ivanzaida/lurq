use std::{
  fmt,
  sync::{
    Arc, Weak,
    atomic::{AtomicUsize, Ordering},
  },
};

use parking_lot::{Mutex, RwLock};

use crate::core::tracking;

static NEXT_SIGNAL_ID: AtomicUsize = AtomicUsize::new(0);

#[cfg(feature = "devtools")]
pub trait SignalValue: crate::app::component::DevtoolsInspectable {}

#[cfg(feature = "devtools")]
impl<T: crate::app::component::DevtoolsInspectable> SignalValue for T {}

#[cfg(not(feature = "devtools"))]
pub trait SignalValue {}

#[cfg(not(feature = "devtools"))]
impl<T> SignalValue for T {}

pub type SignalSubscriber<T> = dyn Fn(&T) + Send + Sync + 'static;

type Watcher = Arc<dyn Fn() + Send + Sync>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SignalObserverKind {
  External,
  Runtime,
  Reactive,
  #[cfg(feature = "devtools")]
  Debug,
}

struct SignalInner<T: SignalValue> {
  id: usize,
  value: RwLock<T>,
  next_subscriber_id: AtomicUsize,
  subscribers: Mutex<Vec<(usize, SignalObserverKind, Arc<SignalSubscriber<T>>)>>,
  watchers: Mutex<Vec<(usize, SignalObserverKind, Watcher)>>,
  #[cfg(feature = "devtools")]
  devtools_subscriber_count: Arc<AtomicUsize>,
}

pub struct Signal<T: SignalValue> {
  inner: Arc<SignalInner<T>>,
}

impl<T: SignalValue> fmt::Debug for Signal<T> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_tuple("Signal").field(&self.inner.id).finish()
  }
}

#[must_use = "dropping the subscription immediately unsubscribes it"]
pub struct Subscription<T: SignalValue> {
  id: usize,
  inner: Weak<SignalInner<T>>,
}

#[must_use = "dropping the handle immediately unsubscribes it"]
pub struct WatchHandle<T: SignalValue> {
  id: usize,
  inner: Weak<SignalInner<T>>,
}

impl<T: SignalValue> Clone for Signal<T> {
  fn clone(&self) -> Self {
    Self {
      inner: Arc::clone(&self.inner),
    }
  }
}

impl<T: SignalValue> Signal<T> {
  pub fn new(value: T) -> Self {
    Self {
      inner: Arc::new(SignalInner {
        id: NEXT_SIGNAL_ID.fetch_add(1, Ordering::Relaxed),
        value: RwLock::new(value),
        next_subscriber_id: AtomicUsize::new(0),
        subscribers: Mutex::new(Vec::new()),
        watchers: Mutex::new(Vec::new()),
        #[cfg(feature = "devtools")]
        devtools_subscriber_count: Arc::new(AtomicUsize::new(0)),
      }),
    }
  }

  pub fn id(&self) -> usize {
    self.inner.id
  }

  pub fn get(&self) -> T
  where
    T: Clone + Send + Sync + 'static,
  {
    self.track_access();
    self.inner.value.read().clone()
  }

  pub fn get_untracked(&self) -> T
  where
    T: Clone + Send + Sync + 'static,
  {
    self.inner.value.read().clone()
  }

  pub fn with<R>(&self, f: impl FnOnce(&T) -> R) -> R
  where
    T: Send + Sync + 'static,
  {
    self.track_access();
    let value = self.inner.value.read();
    f(&value)
  }

  pub fn with_untracked<R>(&self, f: impl FnOnce(&T) -> R) -> R
  where
    T: Send + Sync + 'static,
  {
    let value = self.inner.value.read();
    f(&value)
  }

  pub fn set(&self, value: T) {
    *self.inner.value.write() = value;
    self.notify();
  }

  pub fn update(&self, f: impl FnOnce(&mut T)) {
    f(&mut self.inner.value.write());
    self.notify();
  }

  pub(crate) fn subscribe(&self, sub: impl Fn(&T) + Send + Sync + 'static) -> Subscription<T> {
    self.subscribe_with_kind(sub, SignalObserverKind::External)
  }

  #[cfg(feature = "devtools")]
  pub(crate) fn subscribe_debug(&self, sub: impl Fn(&T) + Send + Sync + 'static) -> Subscription<T> {
    self.subscribe_with_kind(sub, SignalObserverKind::Debug)
  }

  fn subscribe_with_kind(&self, sub: impl Fn(&T) + Send + Sync + 'static, kind: SignalObserverKind) -> Subscription<T> {
    let id = self.inner.next_subscriber_id.fetch_add(1, Ordering::Relaxed);
    self.inner.subscribers.lock().push((id, kind, Arc::new(sub)));
    self.inner.refresh_devtools_subscriber_count();
    Subscription {
      id,
      inner: Arc::downgrade(&self.inner),
    }
  }

  pub(crate) fn watch(&self, f: impl Fn() + Send + Sync + 'static) -> WatchHandle<T> {
    let id = self.inner.next_subscriber_id.fetch_add(1, Ordering::Relaxed);
    self
      .inner
      .watchers
      .lock()
      .push((id, SignalObserverKind::Runtime, Arc::new(f)));
    self.inner.refresh_devtools_subscriber_count();
    WatchHandle {
      id,
      inner: Arc::downgrade(&self.inner),
    }
  }

  #[cfg(feature = "devtools")]
  pub(crate) fn devtools_subscriber_count(&self) -> Arc<AtomicUsize> {
    self.inner.devtools_subscriber_count.clone()
  }

  fn track_access(&self)
  where
    T: Send + Sync + 'static,
  {
    if tracking::is_tracking() {
      let weak = Arc::downgrade(&self.inner);
      let signal_id = self.inner.id;
      tracking::track(
        signal_id,
        Box::new(move |watcher| {
          if let Some(inner) = weak.upgrade() {
            let id = inner.next_subscriber_id.fetch_add(1, Ordering::Relaxed);
            inner.watchers.lock().push((id, SignalObserverKind::Reactive, watcher));
            inner.refresh_devtools_subscriber_count();
            let weak2 = Weak::clone(&Weak::clone(&weak));
            Box::new(DropGuard { id, inner: weak2 })
          } else {
            Box::new(())
          }
        }),
      );
    }
  }

  fn notify(&self) {
    {
      let subs = self.inner.subscribers.lock();
      let value = self.inner.value.read();
      for (_, _, sub) in subs.iter() {
        sub(&value);
      }
    }
    let watchers = self
      .inner
      .watchers
      .lock()
      .iter()
      .map(|(_, _, watcher)| Arc::clone(watcher))
      .collect::<Vec<_>>();
    for watcher in watchers {
      watcher();
    }
  }
}

impl<T: SignalValue> SignalInner<T> {
  #[cfg(feature = "devtools")]
  fn refresh_devtools_subscriber_count(&self) {
    let external_subscribers = self
      .subscribers
      .lock()
      .iter()
      .filter(|(_, kind, _)| *kind == SignalObserverKind::External)
      .count();
    let reactive_watchers = self
      .watchers
      .lock()
      .iter()
      .filter(|(_, kind, _)| *kind == SignalObserverKind::Reactive)
      .count();
    self
      .devtools_subscriber_count
      .store(external_subscribers + reactive_watchers, Ordering::Relaxed);
  }

  #[cfg(not(feature = "devtools"))]
  fn refresh_devtools_subscriber_count(&self) {}
}

struct DropGuard<T: SignalValue> {
  id: usize,
  inner: Weak<SignalInner<T>>,
}

impl<T: SignalValue> Drop for DropGuard<T> {
  fn drop(&mut self) {
    if let Some(inner) = self.inner.upgrade() {
      inner.watchers.lock().retain(|(id, ..)| *id != self.id);
      inner.refresh_devtools_subscriber_count();
    }
  }
}

impl<T: SignalValue> Drop for Subscription<T> {
  fn drop(&mut self) {
    if let Some(inner) = self.inner.upgrade() {
      inner.subscribers.lock().retain(|(id, ..)| *id != self.id);
      inner.refresh_devtools_subscriber_count();
    }
  }
}

impl<T: SignalValue> Drop for WatchHandle<T> {
  fn drop(&mut self) {
    if let Some(inner) = self.inner.upgrade() {
      inner.watchers.lock().retain(|(id, ..)| *id != self.id);
      inner.refresh_devtools_subscriber_count();
    }
  }
}

impl<T: SignalValue> From<T> for Signal<T> {
  fn from(value: T) -> Self {
    Self::new(value)
  }
}
