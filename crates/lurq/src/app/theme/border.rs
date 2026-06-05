#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BorderSize {
  Sm,
  Md,
  Lg,
}

impl BorderSize {
  pub const fn as_str(self) -> &'static str {
    match self {
      Self::Sm => "sm",
      Self::Md => "md",
      Self::Lg => "lg",
    }
  }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ThemeBorderSizes {
  pub sm: f32,
  pub md: f32,
  pub lg: f32,
}

impl ThemeBorderSizes {
  pub fn new() -> Self {
    Self::default()
  }

  pub fn get(&self, size: impl Into<BorderSize>) -> f32 {
    match size.into() {
      BorderSize::Sm => self.sm,
      BorderSize::Md => self.md,
      BorderSize::Lg => self.lg,
    }
  }

  pub fn set(&mut self, size: impl Into<BorderSize>, value: f32) {
    match size.into() {
      BorderSize::Sm => self.sm = value,
      BorderSize::Md => self.md = value,
      BorderSize::Lg => self.lg = value,
    }
  }
}

impl Default for ThemeBorderSizes {
  fn default() -> Self {
    Self {
      sm: 1.0,
      md: 2.0,
      lg: 3.0,
    }
  }
}
