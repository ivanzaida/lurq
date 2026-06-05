use crate::node::{dimension::Dimension, spacing_value::SpacingValue};

#[derive(Default, Clone, Copy, Debug, PartialEq)]
pub struct Padding {
  pub left: SpacingValue,
  pub top: SpacingValue,
  pub right: SpacingValue,
  pub bottom: SpacingValue,
}

impl Padding {
  pub fn new() -> Self {
    Self::default()
  }

  pub fn all(value: impl Into<SpacingValue>) -> Self {
    let value = value.into();
    Self {
      left: value,
      top: value,
      right: value,
      bottom: value,
    }
  }

  pub fn symmetric(horizontal: impl Into<SpacingValue>, vertical: impl Into<SpacingValue>) -> Self {
    let horizontal = horizontal.into();
    let vertical = vertical.into();
    Self {
      left: horizontal,
      right: horizontal,
      top: vertical,
      bottom: vertical,
    }
  }

  pub fn horizontal(value: impl Into<SpacingValue>) -> Self {
    let value = value.into();
    Self {
      left: value,
      right: value,
      ..Self::default()
    }
  }

  pub fn vertical(value: impl Into<SpacingValue>) -> Self {
    let value = value.into();
    Self {
      top: value,
      bottom: value,
      ..Self::default()
    }
  }

  pub fn left(mut self, value: impl Into<SpacingValue>) -> Self {
    self.left = value.into();
    self
  }

  pub fn top(mut self, value: impl Into<SpacingValue>) -> Self {
    self.top = value.into();
    self
  }

  pub fn right(mut self, value: impl Into<SpacingValue>) -> Self {
    self.right = value.into();
    self
  }

  pub fn bottom(mut self, value: impl Into<SpacingValue>) -> Self {
    self.bottom = value.into();
    self
  }

  pub fn merge_from(&mut self, other: &Padding) {
    if other.left != SpacingValue::default() {
      self.left = other.left;
    }
    if other.top != SpacingValue::default() {
      self.top = other.top;
    }
    if other.right != SpacingValue::default() {
      self.right = other.right;
    }
    if other.bottom != SpacingValue::default() {
      self.bottom = other.bottom;
    }
  }

  pub fn get_left(&self) -> &SpacingValue {
    &self.left
  }

  pub fn get_top(&self) -> &SpacingValue {
    &self.top
  }

  pub fn get_right(&self) -> &SpacingValue {
    &self.right
  }

  pub fn get_bottom(&self) -> &SpacingValue {
    &self.bottom
  }
}

impl From<f32> for Padding {
  fn from(value: f32) -> Self {
    Self::all(value)
  }
}

impl From<Dimension> for Padding {
  fn from(value: Dimension) -> Self {
    Self::all(value)
  }
}

impl From<SpacingValue> for Padding {
  fn from(value: SpacingValue) -> Self {
    Self::all(value)
  }
}

impl From<crate::app::theme::SpacingSize> for Padding {
  fn from(value: crate::app::theme::SpacingSize) -> Self {
    Self::all(value)
  }
}
