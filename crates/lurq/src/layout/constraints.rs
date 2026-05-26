use crate::layout::size::Size;

#[derive(Clone, Copy, PartialEq)]
pub struct Constraints {
  pub min_width: f32,
  pub max_width: f32,
  pub min_height: f32,
  pub max_height: f32,
}

impl Constraints {
  pub fn tight(size: Size) -> Self {
    Self {
      min_width: size.width,
      max_width: size.width,
      min_height: size.height,
      max_height: size.height,
    }
  }

  pub fn loose(max: Size) -> Self {
    Self {
      min_width: 0.0,
      max_width: max.width,
      min_height: 0.0,
      max_height: max.height,
    }
  }

  pub fn unbounded() -> Self {
    Self {
      min_width: 0.0,
      max_width: f32::INFINITY,
      min_height: 0.0,
      max_height: f32::INFINITY,
    }
  }

  pub fn loosen_height(self) -> Self {
    Self {
      min_height: 0.0,
      ..self
    }
  }

  pub fn loosen_width(self) -> Self {
    Self { min_width: 0.0, ..self }
  }

  pub fn constrain(&self, size: Size) -> Size {
    Size {
      width: size.width.clamp(self.min_width, self.max_width),
      height: size.height.clamp(self.min_height, self.max_height),
    }
  }
}
