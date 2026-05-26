use std::sync::Arc;

use parking_lot::Mutex;

use crate::core::{signal::Signal, tracking};

pub struct Memo<T: Clone + PartialEq + Send + Sync + 'static> {
  output: Signal<T>,
  _subscriptions: Arc<Mutex<Vec<Box<dyn Send + Sync>>>>,
}

impl<T: Clone + PartialEq + Send + Sync + 'static> Memo<T> {
  pub fn new(f: impl Fn() -> T + Send + Sync + 'static) -> Self {
    tracking::start_tracking();
    let initial = f();
    let deps = tracking::stop_tracking();

    let output = Signal::new(initial);
    let subscriptions: Arc<Mutex<Vec<Box<dyn Send + Sync>>>> = Arc::new(Mutex::new(Vec::new()));

    let output_clone = output.clone();
    let compute = Arc::new(f);
    let recompute: Arc<dyn Fn() + Send + Sync> = Arc::new(move || {
      let new_val = compute();
      output_clone.with_untracked(|current| {
        if *current != new_val {
          output_clone.set(new_val);
        }
      });
    });

    let mut subs = subscriptions.lock();
    for entry in deps {
      let guard = (entry.subscribe_fn)(recompute.clone());
      subs.push(guard);
    }
    drop(subs);

    Self {
      output,
      _subscriptions: subscriptions,
    }
  }

  pub fn get(&self) -> T {
    self.output.get()
  }

  pub fn with<R>(&self, f: impl FnOnce(&T) -> R) -> R {
    self.output.with(f)
  }
}

impl<T: Clone + PartialEq + Send + Sync + 'static> Clone for Memo<T> {
  fn clone(&self) -> Self {
    Self {
      output: self.output.clone(),
      _subscriptions: self._subscriptions.clone(),
    }
  }
}
