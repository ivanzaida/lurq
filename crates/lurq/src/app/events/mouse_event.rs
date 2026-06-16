#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum MouseButton {
  #[default]
  Left,
  Right,
  Middle,
  Other(u8),
}

use super::EventControl;
use crate::core::NodeId;

#[derive(Debug, Clone)]
pub struct MouseEvent {
  pub x: f32,
  pub y: f32,
  pub button: MouseButton,
  pub kind: MouseEventKind,
  pub shift: bool,
  pub ctrl: bool,
  pub alt: bool,
  pub target_id: NodeId,
  pub(crate) control: EventControl,
}

impl MouseEvent {
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

#[derive(Debug, Clone, Copy, Default)]
pub enum MouseEventKind {
  #[default]
  Click,
  Move,
  Up,
  Down,
  DoubleClick,
}
