use std::sync::{
  Arc,
  atomic::{AtomicBool, Ordering},
};

#[derive(Debug, Default)]
struct EventControlState {
  default_prevented: AtomicBool,
  propagation_stopped: AtomicBool,
  immediate_propagation_stopped: AtomicBool,
}

#[derive(Clone, Debug, Default)]
pub struct EventControl {
  state: Arc<EventControlState>,
}

impl EventControl {
  pub fn new() -> Self {
    Self::default()
  }

  pub fn prevent_default(&self) {
    self.state.default_prevented.store(true, Ordering::Relaxed);
  }

  pub fn default_prevented(&self) -> bool {
    self.state.default_prevented.load(Ordering::Relaxed)
  }

  pub fn stop_propagation(&self) {
    self.state.propagation_stopped.store(true, Ordering::Relaxed);
  }

  pub fn propagation_stopped(&self) -> bool {
    self.state.propagation_stopped.load(Ordering::Relaxed)
  }

  pub fn stop_immediate_propagation(&self) {
    self.stop_propagation();
    self.state.immediate_propagation_stopped.store(true, Ordering::Relaxed);
  }

  pub fn immediate_propagation_stopped(&self) -> bool {
    self.state.immediate_propagation_stopped.load(Ordering::Relaxed)
  }
}
