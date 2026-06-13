use super::{EventControl, KeyboardEvent};
use crate::core::Signal;

pub struct TextInputEvent {
  pub value: Signal<String>,
  pub keyboard: KeyboardEvent,
  pub(crate) control: EventControl,
}

impl TextInputEvent {
  pub(crate) fn new(value: Signal<String>, keyboard: KeyboardEvent) -> Self {
    Self {
      value,
      keyboard,
      control: EventControl::new(),
    }
  }

  pub fn prevent_default(&self) {
    self.control.prevent_default();
  }

  pub fn default_prevented(&self) -> bool {
    self.control.default_prevented()
  }
}
