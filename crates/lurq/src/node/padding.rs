use crate::node::dimension::Dimension;

#[derive(Default, Clone)]
pub struct Padding {
  left: Dimension,
  top: Dimension,
  right: Dimension,
  bottom: Dimension,
}

impl Padding {
  pub fn new() -> Self {
    Self::default()
  }

  pub fn all(value: Dimension) -> Self {
    Self {
      left: value.clone(),
      top: value.clone(),
      right: value.clone(),
      bottom: value,
    }
  }

  pub fn symmetric(horizontal: Dimension, vertical: Dimension) -> Self {
    Self {
      left: horizontal.clone(),
      right: horizontal,
      top: vertical.clone(),
      bottom: vertical,
    }
  }

  pub fn horizontal(value: Dimension) -> Self {
    Self {
      left: value.clone(),
      right: value,
      ..Self::default()
    }
  }

  pub fn vertical(value: Dimension) -> Self {
    Self {
      top: value.clone(),
      bottom: value,
      ..Self::default()
    }
  }

  pub fn left(mut self, value: Dimension) -> Self {
    self.left = value;
    self
  }

  pub fn top(mut self, value: Dimension) -> Self {
    self.top = value;
    self
  }

  pub fn right(mut self, value: Dimension) -> Self {
    self.right = value;
    self
  }

  pub fn bottom(mut self, value: Dimension) -> Self {
    self.bottom = value;
    self
  }

  pub fn get_left(&self) -> &Dimension {
    &self.left
  }

  pub fn get_top(&self) -> &Dimension {
    &self.top
  }

  pub fn get_right(&self) -> &Dimension {
    &self.right
  }

  pub fn get_bottom(&self) -> &Dimension {
    &self.bottom
  }
}
