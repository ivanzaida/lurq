use std::{collections::HashMap, sync::Arc};

use super::role_name::RoleName;
use crate::node::dimension::Dimension;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SpacingSize {
  Xs,
  Sm,
  Md,
  Lg,
  Xl,
  Section,
  /// An application-defined spacing stored in [`ThemeSpacing::extra`].
  Extra(RoleName),
}

impl SpacingSize {
  /// An application-defined spacing. The name is interned (see [`SpacingSize::Extra`]).
  pub fn extra(name: impl AsRef<str>) -> Self {
    Self::Extra(RoleName::new(name))
  }

  pub fn as_str(self) -> &'static str {
    match self {
      Self::Xs => "xs",
      Self::Sm => "sm",
      Self::Md => "md",
      Self::Lg => "lg",
      Self::Xl => "xl",
      Self::Section => "section",
      Self::Extra(name) => name.as_str(),
    }
  }
}

impl From<&str> for SpacingSize {
  fn from(name: &str) -> Self {
    Self::extra(name)
  }
}

impl From<Arc<str>> for SpacingSize {
  fn from(name: Arc<str>) -> Self {
    Self::extra(name)
  }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ThemeSpacing {
  pub xs: Dimension,
  pub sm: Dimension,
  pub md: Dimension,
  pub lg: Dimension,
  pub xl: Dimension,
  pub section: Dimension,
  pub extra: HashMap<Arc<str>, Dimension>,
}

impl ThemeSpacing {
  pub fn new() -> Self {
    Self::default()
  }

  /// Panics when a [`SpacingSize::Extra`] name is not in [`Self::extra`]; see [`Self::try_get`].
  pub fn get(&self, size: impl Into<SpacingSize>) -> Dimension {
    let size = size.into();
    self
      .try_get(size)
      .unwrap_or_else(|| panic!("spacing size not found: {}", size.as_str()))
  }

  pub fn try_get(&self, size: impl Into<SpacingSize>) -> Option<Dimension> {
    match size.into() {
      SpacingSize::Xs => Some(self.xs),
      SpacingSize::Sm => Some(self.sm),
      SpacingSize::Md => Some(self.md),
      SpacingSize::Lg => Some(self.lg),
      SpacingSize::Xl => Some(self.xl),
      SpacingSize::Section => Some(self.section),
      SpacingSize::Extra(name) => self.extra.get(name.as_str()).copied(),
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
      SpacingSize::Extra(name) => {
        self.extra.insert(Arc::from(name.as_str()), value.into());
      }
    }
  }

  pub fn resolve(&self, size: impl Into<SpacingSize>) -> Dimension {
    self.get(size)
  }

  pub fn try_resolve(&self, size: impl Into<SpacingSize>) -> Option<Dimension> {
    self.try_get(size)
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
      extra: HashMap::new(),
    }
  }
}
