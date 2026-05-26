use std::{cell::RefCell, sync::Arc};

pub type SubscribeFn = Box<dyn FnOnce(Arc<dyn Fn() + Send + Sync>) -> Box<dyn Send + Sync>>;

pub struct TrackingEntry {
  pub signal_id: usize,
  pub subscribe_fn: SubscribeFn,
}

thread_local! {
  static TRACKING: RefCell<Option<Vec<TrackingEntry>>> = const { RefCell::new(None) };
}

pub fn start_tracking() {
  TRACKING.with(|t| {
    *t.borrow_mut() = Some(Vec::new());
  });
}

pub fn stop_tracking() -> Vec<TrackingEntry> {
  TRACKING.with(|t| t.borrow_mut().take().unwrap_or_default())
}

pub fn is_tracking() -> bool {
  TRACKING.with(|t| t.borrow().is_some())
}

pub fn track(signal_id: usize, subscribe_fn: SubscribeFn) {
  TRACKING.with(|t| {
    let mut borrow = t.borrow_mut();
    if let Some(ref mut entries) = *borrow {
      if !entries.iter().any(|e| e.signal_id == signal_id) {
        entries.push(TrackingEntry {
          signal_id,
          subscribe_fn,
        });
      }
    }
  });
}
