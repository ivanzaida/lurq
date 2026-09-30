use std::{collections::HashMap, sync::Arc};

use super::role_name::RoleName;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BorderSize {
  Sm,
  Md,
  Lg,
  /// An application-defined border width stored in [`ThemeBorderSizes::extra`].
  Extra(RoleName),
}

impl BorderSize {
  /// An application-defined border width. The name is interned (see [`BorderSize::Extra`]).
  pub fn extra(name: impl AsRef<str>) -> Self {
    Self::Extra(RoleName::new(name))
  }

  pub fn as_str(self) -> &'static str {
    match self {
      Self::Sm => "sm",
      Self::Md => "md",
      Self::Lg => "lg",
      Self::Extra(name) => name.as_str(),
    }
  }
}

impl From<&str> for BorderSize {
  fn from(name: &str) -> Self {
    Self::extra(name)
  }
}

impl From<Arc<str>> for BorderSize {
  fn from(name: Arc<str>) -> Self {
    Self::extra(name)
  }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ThemeBorderSizes {
  pub sm: f32,
  pub md: f32,
  pub lg: f32,
  pub extra: HashMap<Arc<str>, f32>,
}

impl ThemeBorderSizes {
  pub fn new() -> Self {
    Self::default()
  }

  /// Panics when a [`BorderSize::Extra`] name is not in [`Self::extra`]; see [`Self::try_get`].
  pub fn get(&self, size: impl Into<BorderSize>) -> f32 {
    let size = size.into();
    self
      .try_get(size)
      .unwrap_or_else(|| panic!("border size not found: {}", size.as_str()))
  }

  pub fn try_get(&self, size: impl Into<BorderSize>) -> Option<f32> {
    match size.into() {
      BorderSize::Sm => Some(self.sm),
      BorderSize::Md => Some(self.md),
      BorderSize::Lg => Some(self.lg),
      BorderSize::Extra(name) => self.extra.get(name.as_str()).copied(),
    }
  }

  pub fn set(&mut self, size: impl Into<BorderSize>, value: f32) {
    match size.into() {
      BorderSize::Sm => self.sm = value,
      BorderSize::Md => self.md = value,
      BorderSize::Lg => self.lg = value,
      BorderSize::Extra(name) => {
        self.extra.insert(Arc::from(name.as_str()), value);
      }
    }
  }

  pub fn resolve(&self, size: impl Into<BorderSize>) -> f32 {
    self.get(size)
  }

  pub fn try_resolve(&self, size: impl Into<BorderSize>) -> Option<f32> {
    self.try_get(size)
  }
}

impl Default for ThemeBorderSizes {
  fn default() -> Self {
    Self {
      sm: 1.0,
      md: 2.0,
      lg: 3.0,
      extra: HashMap::new(),
    }
  }
}
