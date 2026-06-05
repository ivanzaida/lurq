use crate::{
  app::theme::{SpacingSize, ThemeSpacing},
  node::dimension::Dimension,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SpacingValue {
  Dimension(Dimension),
  Theme(SpacingSize),
}

impl SpacingValue {
  pub fn resolve(&self, spacing: &ThemeSpacing, parent_size: f32) -> f32 {
    match self {
      Self::Dimension(value) => value.resolve(parent_size),
      Self::Theme(size) => spacing.get(*size).resolve(parent_size),
    }
  }

  pub fn as_dimension(&self) -> Option<Dimension> {
    match self {
      Self::Dimension(value) => Some(*value),
      Self::Theme(_) => None,
    }
  }
}

impl Default for SpacingValue {
  fn default() -> Self {
    Self::Dimension(Dimension::Auto)
  }
}

impl From<f32> for SpacingValue {
  fn from(value: f32) -> Self {
    Self::Dimension(Dimension::Px(value))
  }
}

impl From<Dimension> for SpacingValue {
  fn from(value: Dimension) -> Self {
    Self::Dimension(value)
  }
}

impl From<SpacingSize> for SpacingValue {
  fn from(value: SpacingSize) -> Self {
    Self::Theme(value)
  }
}
