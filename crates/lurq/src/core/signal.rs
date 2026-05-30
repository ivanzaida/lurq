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

pub type SignalSubscriber<T> = dyn Fn(&T) + Send + Sync + 'static;

type Watcher = Arc<dyn Fn() + Send + Sync>;

struct SignalInner<T> {
  id: usize,
  value: RwLock<T>,
  next_subscriber_id: AtomicUsize,
  subscribers: Mutex<Vec<(usize, Arc<SignalSubscriber<T>>)>>,
  watchers: Mutex<Vec<(usize, Watcher)>>,
}

pub struct Signal<T> {
  inner: Arc<SignalInner<T>>,
}

impl<T> fmt::Debug for Signal<T> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_tuple("Signal").field(&self.inner.id).finish()
  }
}

#[must_use = "dropping the subscription immediately unsubscribes it"]
pub struct Subscription<T> {
  id: usize,
  inner: Weak<SignalInner<T>>,
}

#[must_use = "dropping the handle immediately unsubscribes it"]
pub struct WatchHandle<T> {
  id: usize,
  inner: Weak<SignalInner<T>>,
}

impl<T> Clone for Signal<T> {
  fn clone(&self) -> Self {
    Self {
      inner: Arc::clone(&self.inner),
    }
  }
}

impl<T> Signal<T> {
  pub fn new(value: T) -> Self {
    Self {
      inner: Arc::new(SignalInner {
        id: NEXT_SIGNAL_ID.fetch_add(1, Ordering::Relaxed),
        value: RwLock::new(value),
        next_subscriber_id: AtomicUsize::new(0),
        subscribers: Mutex::new(Vec::new()),
        watchers: Mutex::new(Vec::new()),
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
    let id = self.inner.next_subscriber_id.fetch_add(1, Ordering::Relaxed);
    self.inner.subscribers.lock().push((id, Arc::new(sub)));
    Subscription {
      id,
      inner: Arc::downgrade(&self.inner),
    }
  }

  pub(crate) fn watch(&self, f: impl Fn() + Send + Sync + 'static) -> WatchHandle<T> {
    let id = self.inner.next_subscriber_id.fetch_add(1, Ordering::Relaxed);
    self.inner.watchers.lock().push((id, Arc::new(f)));
    WatchHandle {
      id,
      inner: Arc::downgrade(&self.inner),
    }
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
            inner.watchers.lock().push((id, watcher));
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
      for (_, sub) in subs.iter() {
        sub(&value);
      }
    }
    let watchers = self
      .inner
      .watchers
      .lock()
      .iter()
      .map(|(_, watcher)| Arc::clone(watcher))
      .collect::<Vec<_>>();
    for watcher in watchers {
      watcher();
    }
  }
}

struct DropGuard<T> {
  id: usize,
  inner: Weak<SignalInner<T>>,
}

impl<T> Drop for DropGuard<T> {
  fn drop(&mut self) {
    if let Some(inner) = self.inner.upgrade() {
      inner.watchers.lock().retain(|(id, _)| *id != self.id);
    }
  }
}

impl<T> Drop for Subscription<T> {
  fn drop(&mut self) {
    if let Some(inner) = self.inner.upgrade() {
      inner.subscribers.lock().retain(|(id, _)| *id != self.id);
    }
  }
}

impl<T> Drop for WatchHandle<T> {
  fn drop(&mut self) {
    if let Some(inner) = self.inner.upgrade() {
      inner.watchers.lock().retain(|(id, _)| *id != self.id);
    }
  }
}

impl<T> From<T> for Signal<T> {
  fn from(value: T) -> Self {
    Self::new(value)
  }
}
