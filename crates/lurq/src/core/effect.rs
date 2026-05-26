use std::sync::Arc;

use parking_lot::Mutex;

use crate::core::tracking;

pub struct Effect {
  _subscriptions: Arc<Mutex<Vec<Box<dyn Send + Sync>>>>,
}

impl Effect {
  pub fn new(f: impl Fn() + Send + Sync + 'static) -> Self {
    let subscriptions: Arc<Mutex<Vec<Box<dyn Send + Sync>>>> = Arc::new(Mutex::new(Vec::new()));

    let compute = Arc::new(f);
    let subs_clone = subscriptions.clone();
    let rerun: Arc<dyn Fn() + Send + Sync> = Arc::new(move || {
      let mut subs = subs_clone.lock();
      subs.clear();

      tracking::start_tracking();
      compute();
      let deps = tracking::stop_tracking();

      let rerun_ref = get_current_effect();
      for entry in deps {
        if let Some(ref rc) = rerun_ref {
          let guard = (entry.subscribe_fn)(rc.clone());
          subs.push(guard);
        }
      }
    });

    set_current_effect(rerun.clone());
    rerun();
    clear_current_effect();

    Self {
      _subscriptions: subscriptions,
    }
  }
}

thread_local! {
  static CURRENT_EFFECT: std::cell::RefCell<Option<Arc<dyn Fn() + Send + Sync>>> = const { std::cell::RefCell::new(None) };
}

fn set_current_effect(f: Arc<dyn Fn() + Send + Sync>) {
  CURRENT_EFFECT.with(|r| *r.borrow_mut() = Some(f));
}

fn get_current_effect() -> Option<Arc<dyn Fn() + Send + Sync>> {
  CURRENT_EFFECT.with(|r| r.borrow().clone())
}

fn clear_current_effect() {
  CURRENT_EFFECT.with(|r| *r.borrow_mut() = None);
}
