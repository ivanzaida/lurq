#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RadiusSize {
  Sm,
  Md,
  Lg,
}

impl RadiusSize {
  pub const fn as_str(self) -> &'static str {
    match self {
      Self::Sm => "sm",
      Self::Md => "md",
      Self::Lg => "lg",
    }
  }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ThemeRadii {
  pub sm: f32,
  pub md: f32,
  pub lg: f32,
}

impl ThemeRadii {
  pub fn new() -> Self {
    Self::default()
  }

  pub fn get(&self, size: impl Into<RadiusSize>) -> f32 {
    match size.into() {
      RadiusSize::Sm => self.sm,
      RadiusSize::Md => self.md,
      RadiusSize::Lg => self.lg,
    }
  }

  pub fn set(&mut self, size: impl Into<RadiusSize>, value: f32) {
    match size.into() {
      RadiusSize::Sm => self.sm = value,
      RadiusSize::Md => self.md = value,
      RadiusSize::Lg => self.lg = value,
    }
  }
}

impl Default for ThemeRadii {
  fn default() -> Self {
    Self {
      sm: 3.0,
      md: 5.0,
      lg: 6.0,
    }
  }
}
