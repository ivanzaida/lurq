use crate::node::dimension::Dimension;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SpacingSize {
  Xs,
  Sm,
  Md,
  Lg,
  Xl,
  Section,
}

impl SpacingSize {
  pub const fn as_str(self) -> &'static str {
    match self {
      Self::Xs => "xs",
      Self::Sm => "sm",
      Self::Md => "md",
      Self::Lg => "lg",
      Self::Xl => "xl",
      Self::Section => "section",
    }
  }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ThemeSpacing {
  pub xs: Dimension,
  pub sm: Dimension,
  pub md: Dimension,
  pub lg: Dimension,
  pub xl: Dimension,
  pub section: Dimension,
}

impl ThemeSpacing {
  pub fn new() -> Self {
    Self::default()
  }

  pub fn get(&self, size: impl Into<SpacingSize>) -> Dimension {
    match size.into() {
      SpacingSize::Xs => self.xs,
      SpacingSize::Sm => self.sm,
      SpacingSize::Md => self.md,
      SpacingSize::Lg => self.lg,
      SpacingSize::Xl => self.xl,
      SpacingSize::Section => self.section,
    }
  }

  pub fn set(&mut self, size: impl Into<SpacingSize>, value: impl Into<Dimension>) {
    match size.into() {
      SpacingSize::Xs => self.xs = value.into(),
      SpacingSize::Sm => self.sm = value.into(),
      SpacingSize::Md => self.md = value.into(),
      SpacingSize::Lg => self.lg = value.into(),
      SpacingSize::Xl => self.xl = value.into(),
      SpacingSize::Section => self.section = value.into(),
    }
  }
}

impl Default for ThemeSpacing {
  fn default() -> Self {
    Self {
      xs: Dimension::Px(4.0),
      sm: Dimension::Px(8.0),
      md: Dimension::Px(12.0),
      lg: Dimension::Px(16.0),
      xl: Dimension::Px(24.0),
      section: Dimension::Px(32.0),
    }
  }
}
