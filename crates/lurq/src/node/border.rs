use crate::node::color::Color;

#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub struct BorderRadius {
  pub top_left: f32,
  pub top_right: f32,
  pub bottom_right: f32,
  pub bottom_left: f32,
}

impl BorderRadius {
  pub fn new(top_left: f32, top_right: f32, bottom_right: f32, bottom_left: f32) -> Self {
    Self {
      top_left,
      top_right,
      bottom_right,
      bottom_left,
    }
  }

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

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Border {
  pub width: f32,
  pub color: Color,
  pub placement: BorderPlacement,
}

impl Border {
  pub fn new(width: f32, color: Color, placement: BorderPlacement) -> Self {
    Self {
      width,
      color,
      placement,
    }
  }

  pub fn inside(width: f32, color: Color) -> Self {
    Self::new(width, color, BorderPlacement::Inside)
  }

  pub fn outside(width: f32, color: Color) -> Self {
    Self::new(width, color, BorderPlacement::Outside)
  }

  pub fn center(width: f32, color: Color) -> Self {
    Self::new(width, color, BorderPlacement::Center)
  }
}

#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub struct Borders {
  pub top: Option<Border>,
  pub right: Option<Border>,
  pub bottom: Option<Border>,
  pub left: Option<Border>,
}

impl Borders {
  pub fn all(border: Border) -> Self {
    Self {
      top: Some(border),
      right: Some(border),
      bottom: Some(border),
      left: Some(border),
    }
  }

  pub fn any(&self) -> bool {
    self.top.is_some() || self.right.is_some() || self.bottom.is_some() || self.left.is_some()
  }

  pub fn color(&self) -> Option<Color> {
    self
      .top
      .or(self.right)
      .or(self.bottom)
      .or(self.left)
      .map(|border| border.color)
  }

  pub fn set_color(&mut self, color: Color) {
    if let Some(border) = &mut self.top {
      border.color = color;
    }
    if let Some(border) = &mut self.right {
      border.color = color;
    }
    if let Some(border) = &mut self.bottom {
      border.color = color;
    }
    if let Some(border) = &mut self.left {
      border.color = color;
    }
  }

  pub fn top_width(&self) -> Option<f32> {
    self.top.map(|border| border.width)
  }

  pub fn right_width(&self) -> Option<f32> {
    self.right.map(|border| border.width)
  }

  pub fn bottom_width(&self) -> Option<f32> {
    self.bottom.map(|border| border.width)
  }

  pub fn left_width(&self) -> Option<f32> {
    self.left.map(|border| border.width)
  }
}

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub enum BorderPlacement {
  #[default]
  Inside,
  Outside,
  Center,
}
