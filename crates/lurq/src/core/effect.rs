use std::sync::{
  Arc, Weak,
  atomic::{AtomicBool, Ordering},
};

use parking_lot::Mutex;

use crate::core::tracking;

pub struct Effect {
  _subscriptions: Arc<Mutex<Vec<Box<dyn Send + Sync>>>>,
  alive: Arc<AtomicBool>,
}

impl Effect {
  pub fn new(f: impl Fn() + Send + Sync + 'static) -> Self {
    let subscriptions: Arc<Mutex<Vec<Box<dyn Send + Sync>>>> = Arc::new(Mutex::new(Vec::new()));
    let alive = Arc::new(AtomicBool::new(true));
    let compute = Arc::new(f);
    let subs_clone = subscriptions.clone();
    let alive_clone = alive.clone();

    let self_ref: Arc<Mutex<Weak<dyn Fn() + Send + Sync>>> = Arc::new(Mutex::new(Weak::<fn()>::new()));

    let self_ref_clone = self_ref.clone();
    let rerun: Arc<dyn Fn() + Send + Sync> = Arc::new(move || {
      if !alive_clone.load(Ordering::Relaxed) {
        return;
      }
      let mut subs = subs_clone.lock();
      subs.clear();

      tracking::start_tracking();
      compute();
      let deps = tracking::stop_tracking();

      let strong = self_ref_clone.lock().upgrade();
      if let Some(rerun_arc) = strong {
        for entry in deps {
          let guard = (entry.subscribe_fn)(rerun_arc.clone());
          subs.push(guard);
        }
      }
    });

    *self_ref.lock() = Arc::downgrade(&rerun);
    rerun();

    Self {
      _subscriptions: subscriptions,
      alive,
    }
  }
}

impl Drop for Effect {
  fn drop(&mut self) {
    self.alive.store(false, Ordering::Relaxed);
  }
}
