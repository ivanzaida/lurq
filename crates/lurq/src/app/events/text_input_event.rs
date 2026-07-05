use super::{EventControl, KeyboardEvent};
use crate::core::Signal;

#[derive(Clone)]
pub struct TextInputEvent {
  /// The input's bound value. The event fires *before* the edit applies, so
  /// reading this signal yields the text prior to the change.
  pub value: Signal<String>,
  /// The text the input will hold once this edit applies (unless a handler
  /// calls [`TextInputEvent::prevent_default`]).
  pub new_value: String,
  pub keyboard: KeyboardEvent,
  pub(crate) control: EventControl,
}

impl TextInputEvent {
  pub(crate) fn new(value: Signal<String>, new_value: String, keyboard: KeyboardEvent) -> Self {
    Self {
      value,
      new_value,
      keyboard,
      control: EventControl::new(),
    }
  }

  /// The text prior to this edit.
  pub fn old_value(&self) -> String {
    self.value.get()
  }

  /// The text after this edit applies.
  pub fn new_value(&self) -> &str {
    &self.new_value
  }

  pub fn prevent_default(&self) {
    self.control.prevent_default();
  }

  pub fn default_prevented(&self) -> bool {
    self.control.default_prevented()
  }
}
