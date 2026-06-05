use crate::app::theme::{RadiusSize, ThemeRadii};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RadiusValue {
  Px(f32),
  Theme(RadiusSize),
}

impl RadiusValue {
  pub fn resolve(&self, radii: &ThemeRadii) -> f32 {
    match self {
      Self::Px(value) => *value,
      Self::Theme(size) => radii.get(*size),
    }
  }

  pub fn as_px(&self) -> Option<f32> {
    match self {
      Self::Px(value) => Some(*value),
      Self::Theme(_) => None,
    }
  }
}

impl Default for RadiusValue {
  fn default() -> Self {
    Self::Px(0.0)
  }
}

impl From<f32> for RadiusValue {
  fn from(value: f32) -> Self {
    Self::Px(value)
  }
}

impl From<RadiusSize> for RadiusValue {
  fn from(value: RadiusSize) -> Self {
    Self::Theme(value)
  }
}
