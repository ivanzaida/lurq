use super::EventControl;
use crate::core::NodeId;

#[derive(Debug, Clone)]
pub struct ScrollEvent {
  pub x: f32,
  pub y: f32,
  pub delta_x: f32,
  pub delta_y: f32,
  pub phase: ScrollPhase,
  /// The keys held as it came. A touchpad's pinch on Windows is the wheel with Ctrl held, as a browser tells
  /// it: what zooms by the wheel zooms with Ctrl and the wheel, notches or a touchpad's travel.
  pub shift: bool,
  pub ctrl: bool,
  pub alt: bool,
  pub meta: bool,
  /// It is a mouse's notches ([`Tree::wheel`](crate::app::Tree::wheel)), not the travel of a touchpad, of a
  /// precise wheel or of a finger ([`Tree::scroll`](crate::app::Tree::scroll)): what zooms by it goes a step
  /// at a notch, and with the travel as far as the fingers went.
  pub notched: bool,
  pub target_id: NodeId,
  pub(crate) control: EventControl,
}

impl ScrollEvent {
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

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ScrollPhase {
  Start,
  #[default]
  Scroll,
  End,
}
