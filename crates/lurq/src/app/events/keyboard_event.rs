use super::EventControl;
use crate::core::NodeId;

pub struct KeyboardEvent {
  pub key: String,
  pub code: String,
  pub shift: bool,
  pub ctrl: bool,
  pub alt: bool,
  pub meta: bool,
  pub target_id: NodeId,
  pub(crate) control: EventControl,
}

impl KeyboardEvent {
  pub fn new(
    key: impl Into<String>,
    code: impl Into<String>,
    shift: bool,
    ctrl: bool,
    alt: bool,
    meta: bool,
    target_id: NodeId,
  ) -> Self {
    Self {
      key: key.into(),
      code: code.into(),
      shift,
      ctrl,
      alt,
      meta,
      target_id,
      control: EventControl::new(),
    }
  }

  pub fn prevent_default(&self) {
    self.control.prevent_default();
  }

  pub fn default_prevented(&self) -> bool {
    self.control.default_prevented()
  }

  pub fn stop_propagation(&self) {
    self.control.stop_propagation();
  }

  pub fn propagation_stopped(&self) -> bool {
    self.control.propagation_stopped()
  }

  pub fn stop_immediate_propagation(&self) {
    self.control.stop_immediate_propagation();
  }

  pub fn immediate_propagation_stopped(&self) -> bool {
    self.control.immediate_propagation_stopped()
  }
}
