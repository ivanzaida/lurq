use crate::app::theme::{BorderSize, ThemeBorderSizes};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BorderSizeValue {
  Px(f32),
  Theme(BorderSize),
}

impl BorderSizeValue {
  pub fn resolve(&self, border_sizes: &ThemeBorderSizes) -> f32 {
    match self {
      Self::Px(value) => *value,
      Self::Theme(size) => border_sizes.get(*size),
    }
  }

  pub fn as_px(&self) -> Option<f32> {
    match self {
      Self::Px(value) => Some(*value),
      Self::Theme(_) => None,
    }
  }
}

impl From<f32> for BorderSizeValue {
  fn from(value: f32) -> Self {
    Self::Px(value)
  }
}

impl From<BorderSize> for BorderSizeValue {
  fn from(value: BorderSize) -> Self {
    Self::Theme(value)
  }
}
