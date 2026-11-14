use std::{collections::HashMap, sync::Arc};

use super::role_name::RoleName;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RadiusSize {
  Sm,
  Md,
  Lg,
  Xl,
  /// As round as the element allows: a pill, a circle.
  Full,
  /// An application-defined radius stored in [`ThemeRadii::extra`].
  Extra(RoleName),
}

impl RadiusSize {
  /// An application-defined radius. The name is interned (see [`RadiusSize::Extra`]).
  pub fn extra(name: impl AsRef<str>) -> Self {
    Self::Extra(RoleName::new(name))
  }

  pub fn as_str(self) -> &'static str {
    match self {
      Self::Sm => "sm",
      Self::Md => "md",
      Self::Lg => "lg",
      Self::Xl => "xl",
      Self::Full => "full",
      Self::Extra(name) => name.as_str(),
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
  pub xl: f32,
  /// A radius larger than any element, which rounds one as far as its box allows.
  pub full: f32,
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
      RadiusSize::Xl => Some(self.xl),
      RadiusSize::Full => Some(self.full),
      RadiusSize::Extra(name) => self.extra.get(name.as_str()).copied(),
    }
  }

  pub fn set(&mut self, size: impl Into<RadiusSize>, value: f32) {
    match size.into() {
      RadiusSize::Sm => self.sm = value,
      RadiusSize::Md => self.md = value,
      RadiusSize::Lg => self.lg = value,
      RadiusSize::Xl => self.xl = value,
      RadiusSize::Full => self.full = value,
      RadiusSize::Extra(name) => {
        self.extra.insert(Arc::from(name.as_str()), value);
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

impl ThemeRadii {
  /// The radii of lurq 0.61: 3, 5 and 6. It had no `xl` and no `full`, which are the design's here.
  pub fn legacy() -> Self {
    Self {
      sm: 3.0,
      md: 5.0,
      lg: 6.0,
      ..Self::default()
    }
  }
}

/// The design's radii.
impl Default for ThemeRadii {
  fn default() -> Self {
    Self {
      sm: 4.0,
      md: 6.0,
      lg: 8.0,
      xl: 12.0,
      full: 999.0,
      extra: HashMap::new(),
    }
  }
}
