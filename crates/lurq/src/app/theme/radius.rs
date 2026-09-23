use std::{collections::HashMap, sync::Arc};

use super::role_name::intern;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RadiusSize {
  Sm,
  Md,
  Lg,
  /// An application-defined radius stored in [`ThemeRadii::extra`].
  Extra(&'static str),
}

impl RadiusSize {
  /// An application-defined radius. The name is interned (see [`RadiusSize::Extra`]).
  pub fn extra(name: impl AsRef<str>) -> Self {
    Self::Extra(intern(name.as_ref()))
  }

  pub const fn as_str(self) -> &'static str {
    match self {
      Self::Sm => "sm",
      Self::Md => "md",
      Self::Lg => "lg",
      Self::Extra(name) => name,
    }
  }
}

impl From<&str> for RadiusSize {
  fn from(name: &str) -> Self {
    Self::extra(name)
  }
}

impl From<Arc<str>> for RadiusSize {
  fn from(name: Arc<str>) -> Self {
    Self::extra(name)
  }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ThemeRadii {
  pub sm: f32,
  pub md: f32,
  pub lg: f32,
  pub extra: HashMap<Arc<str>, f32>,
}

impl ThemeRadii {
  pub fn new() -> Self {
    Self::default()
  }

  /// Panics when a [`RadiusSize::Extra`] name is not in [`Self::extra`]; see [`Self::try_get`].
  pub fn get(&self, size: impl Into<RadiusSize>) -> f32 {
    let size = size.into();
    self
      .try_get(size)
      .unwrap_or_else(|| panic!("radius size not found: {}", size.as_str()))
  }

  pub fn try_get(&self, size: impl Into<RadiusSize>) -> Option<f32> {
    match size.into() {
      RadiusSize::Sm => Some(self.sm),
      RadiusSize::Md => Some(self.md),
      RadiusSize::Lg => Some(self.lg),
      RadiusSize::Extra(name) => self.extra.get(name).copied(),
    }
  }

  pub fn set(&mut self, size: impl Into<RadiusSize>, value: f32) {
    match size.into() {
      RadiusSize::Sm => self.sm = value,
      RadiusSize::Md => self.md = value,
      RadiusSize::Lg => self.lg = value,
      RadiusSize::Extra(name) => {
        self.extra.insert(Arc::from(name), value);
      }
    }
  }

  pub fn resolve(&self, size: impl Into<RadiusSize>) -> f32 {
    self.get(size)
  }

  pub fn try_resolve(&self, size: impl Into<RadiusSize>) -> Option<f32> {
    self.try_get(size)
  }
}

impl Default for ThemeRadii {
  fn default() -> Self {
    Self {
      sm: 3.0,
      md: 5.0,
      lg: 6.0,
      extra: HashMap::new(),
    }
  }
}
