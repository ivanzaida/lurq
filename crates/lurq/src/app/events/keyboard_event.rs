use super::EventControl;
use crate::core::NodeId;

#[derive(Clone)]
pub struct KeyboardEvent {
  pub key: String,
  pub code: String,
  pub shift: bool,
  pub ctrl: bool,
  pub alt: bool,
  pub meta: bool,
  pub target_id: NodeId,
  /// Whether a text input holds keyboard focus when this event fires.
  /// `on_key_down` handlers run before the focused input consumes the key,
  /// so global shortcuts (e.g. an app's ctrl+z) should check this and step
  /// aside while the user is typing — the input's own editing (including
  /// its undo/redo) takes the key otherwise.
  pub text_input_focused: bool,
  /// Whether this key belongs to an input method (IME) composition, like the
  /// DOM's `isComposing`.
  ///
  /// The keys the input method takes never arrive as presses: Windows
  /// reports them as `Process` (the `Enter` that confirms a composition
  /// included) and macOS not at all, so `on_key_down` handlers and lurq's own
  /// key handling do not see them. A press that does arrive while a
  /// composition is shown, a key the input method passed on, has this set,
  /// and lurq handles it as usual; a handler that sends on `Enter` can skip
  /// it. Releases of the input method's keys reach `on_key_up` handlers with
  /// this set. It is also set on the event a committed composition passes to
  /// `on_input` handlers (`TextInputEvent::keyboard`), whose `key` is the
  /// committed text.
  pub composing: bool,
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
      text_input_focused: false,
      composing: false,
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
