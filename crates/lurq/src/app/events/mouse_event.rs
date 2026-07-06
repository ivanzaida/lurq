#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum MouseButton {
  #[default]
  Left,
  Right,
  Middle,
  Other(u8),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, crate::DevtoolsInspectable)]
pub struct MouseButtonMask(u8);

impl MouseButtonMask {
  pub const NONE: Self = Self(0);
  pub const LEFT: Self = Self(1 << 0);
  pub const RIGHT: Self = Self(1 << 1);
  pub const MIDDLE: Self = Self(1 << 2);
  pub const OTHER: Self = Self(1 << 3);
  pub const ANY: Self = Self(Self::LEFT.0 | Self::RIGHT.0 | Self::MIDDLE.0 | Self::OTHER.0);

  pub const LMB: Self = Self::LEFT;
  pub const RMB: Self = Self::RIGHT;
  pub const MMB: Self = Self::MIDDLE;

  pub fn contains_button(self, button: MouseButton) -> bool {
    let bit = match button {
      MouseButton::Left => Self::LEFT.0,
      MouseButton::Right => Self::RIGHT.0,
      MouseButton::Middle => Self::MIDDLE.0,
      MouseButton::Other(_) => Self::OTHER.0,
    };
    self.0 & bit != 0
  }
}

impl Default for MouseButtonMask {
  fn default() -> Self {
    Self::ANY
  }
}

impl std::ops::BitOr for MouseButtonMask {
  type Output = Self;

  fn bitor(self, rhs: Self) -> Self::Output {
    Self(self.0 | rhs.0)
  }
}

impl std::ops::BitOrAssign for MouseButtonMask {
  fn bitor_assign(&mut self, rhs: Self) {
    self.0 |= rhs.0;
  }
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
