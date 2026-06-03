use crate::{
  app::theme::{SpacingId, ThemeSpacing},
  node::dimension::Dimension,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SpacingValue {
  Dimension(Dimension),
  Theme(SpacingId),
}

impl SpacingValue {
  pub fn resolve(&self, spacing: &ThemeSpacing, parent_size: f32) -> f32 {
    match self {
      Self::Dimension(value) => value.resolve(parent_size),
      Self::Theme(id) => spacing.get(*id).map(|value| value.resolve(parent_size)).unwrap_or(0.0),
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

impl From<SpacingId> for SpacingValue {
  fn from(value: SpacingId) -> Self {
    Self::Theme(value)
  }
}
