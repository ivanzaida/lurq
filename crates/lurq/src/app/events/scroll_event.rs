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
  /// What moves the content.
  pub source: ScrollSource,
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

/// What moves a scroll container's content, as its [`ScrollEvent`] tells it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ScrollSource {
  /// A finger on the content, which follows it.
  Finger,
  /// A mouse's wheel, a touchpad or a precise wheel.
  #[default]
  Wheel,
  /// What a finger left the content doing when it lifted: it goes on, slowing down.
  Momentum,
  /// A list that scrolls by pages on its way to one, after the finger lifted
  /// ([`ScrollState::set_paging`](crate::layout::layout_kind::ScrollState::set_paging)).
  Snap,
  /// The bar: a drag of its thumb, or a press on its track.
  Scrollbar,
  /// The keys of a container that has the focus.
  Keyboard,
  /// The pointer held away from where the middle button went down, or a drag held at an edge.
  Autoscroll,
  /// The app: an offset it set, an element it brought into view, a scroll to the top it asked for. And the
  /// layout, where the content changed under the offset.
  Code,
}

impl ScrollSource {
  /// The source's name in a log and to a tool: `finger`, `wheel`, `momentum`, `snap`, `scrollbar`, `keyboard`,
  /// `autoscroll`, `code`.
  pub fn as_str(self) -> &'static str {
    match self {
      Self::Finger => "finger",
      Self::Wheel => "wheel",
      Self::Momentum => "momentum",
      Self::Snap => "snap",
      Self::Scrollbar => "scrollbar",
      Self::Keyboard => "keyboard",
      Self::Autoscroll => "autoscroll",
      Self::Code => "code",
    }
  }
}
