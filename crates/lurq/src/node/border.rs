use crate::node::color::Color;

#[derive(Clone, Copy, Default)]
pub struct BorderRadius {
  pub top_left: f32,
  pub top_right: f32,
  pub bottom_right: f32,
  pub bottom_left: f32,
}

impl BorderRadius {
  pub fn all(r: f32) -> Self {
    Self {
      top_left: r,
      top_right: r,
      bottom_right: r,
      bottom_left: r,
    }
  }

  pub fn to_array(&self) -> [f32; 4] {
    [self.top_left, self.top_right, self.bottom_right, self.bottom_left]
  }
}

#[derive(Clone, Copy)]
pub struct Border {
  pub width: BorderWidth,
  pub color: Color,
  pub placement: BorderPlacement,
}

#[derive(Clone, Copy, Default)]
pub struct BorderWidth {
  pub top: f32,
  pub right: f32,
  pub bottom: f32,
  pub left: f32,
}

impl BorderWidth {
  pub fn all(w: f32) -> Self {
    Self {
      top: w,
      right: w,
      bottom: w,
      left: w,
    }
  }

  pub fn to_array(&self) -> [f32; 4] {
    [self.top, self.right, self.bottom, self.left]
  }
}

#[derive(Clone, Copy, Default)]
pub enum BorderPlacement {
  #[default]
  Inside,
  Outside,
  Center,
}
