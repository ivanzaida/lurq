use crate::app::theme::{RadiusId, ThemeRadii};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RadiusValue {
  Px(f32),
  Theme(RadiusId),
}

impl RadiusValue {
  pub fn resolve(&self, radii: &ThemeRadii) -> f32 {
    match self {
      Self::Px(value) => *value,
      Self::Theme(id) => radii.get(*id).unwrap_or(0.0),
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

impl From<RadiusId> for RadiusValue {
  fn from(value: RadiusId) -> Self {
    Self::Theme(value)
  }
}
