use std::{collections::HashMap, sync::Arc};

use super::role_name::RoleName;
use crate::node::{box_shadow::BoxShadow, color::Color};

/// A box shadow (elevation) role. Each role resolves to a list of
/// [`BoxShadow`]s in [`ThemeShadows`], so one role can stack a sharp contact
/// shadow and a soft ambient one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ShadowStyle {
  Sm,
  Md,
  Lg,
  /// An application-defined shadow stored in [`ThemeShadows::extra`].
  Extra(RoleName),
}

impl ShadowStyle {
  /// An application-defined shadow. The name is interned (see [`RoleName`]).
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

impl From<&str> for ShadowStyle {
  fn from(name: &str) -> Self {
    Self::extra(name)
  }
}

impl From<Arc<str>> for ShadowStyle {
  fn from(name: Arc<str>) -> Self {
    Self::extra(name)
  }
}

/// The theme's shadow table. Lists are shared (`Arc`), so reading a role and
/// cloning the table do not copy shadows.
#[derive(Clone, Debug, PartialEq)]
pub struct ThemeShadows {
  pub sm: Arc<[BoxShadow]>,
  pub md: Arc<[BoxShadow]>,
  pub lg: Arc<[BoxShadow]>,
  pub extra: HashMap<Arc<str>, Arc<[BoxShadow]>>,
}

impl ThemeShadows {
  pub fn new() -> Self {
    Self::default()
  }

  /// Panics when a [`ShadowStyle::Extra`] name is not in [`Self::extra`]; see [`Self::try_get`].
  pub fn get(&self, style: impl Into<ShadowStyle>) -> Arc<[BoxShadow]> {
    let style = style.into();
    self
      .try_get(style)
      .unwrap_or_else(|| panic!("shadow style not found: {}", style.as_str()))
  }

  pub fn try_get(&self, style: impl Into<ShadowStyle>) -> Option<Arc<[BoxShadow]>> {
    self.entry(style.into()).cloned()
  }

  pub(crate) fn try_get_ref(&self, style: ShadowStyle) -> Option<&[BoxShadow]> {
    self.entry(style).map(|shadows| &**shadows)
  }

  fn entry(&self, style: ShadowStyle) -> Option<&Arc<[BoxShadow]>> {
    match style {
      ShadowStyle::Sm => Some(&self.sm),
      ShadowStyle::Md => Some(&self.md),
      ShadowStyle::Lg => Some(&self.lg),
      ShadowStyle::Extra(name) => self.extra.get(name.as_str()),
    }
  }

  pub fn set(&mut self, style: impl Into<ShadowStyle>, shadows: impl Into<Arc<[BoxShadow]>>) {
    let shadows = shadows.into();
    match style.into() {
      ShadowStyle::Sm => self.sm = shadows,
      ShadowStyle::Md => self.md = shadows,
      ShadowStyle::Lg => self.lg = shadows,
      ShadowStyle::Extra(name) => {
        self.extra.insert(Arc::from(name.as_str()), shadows);
      }
    }
  }

  pub fn resolve(&self, style: impl Into<ShadowStyle>) -> Arc<[BoxShadow]> {
    self.get(style)
  }

  pub fn try_resolve(&self, style: impl Into<ShadowStyle>) -> Option<Arc<[BoxShadow]>> {
    self.try_get(style)
  }
}

/// The design's elevations (`pencil/lurq-design-system.pen`, Foundations): `sm` for controls, `md` for cards
/// and menus, `lg` for popovers and dialogs.
impl Default for ThemeShadows {
  fn default() -> Self {
    let black = |alpha: u8| Color::new(0, 0, 0, alpha);
    Self {
      sm: Arc::from([BoxShadow::new(0.0, 1.0, 2.0, black(0x0D))]),
      md: Arc::from([
        BoxShadow::new(0.0, 4.0, 8.0, black(0x1A)),
        BoxShadow::new(0.0, 1.0, 2.0, black(0x0F)),
      ]),
      lg: Arc::from([
        BoxShadow::new(0.0, 12.0, 24.0, black(0x24)),
        BoxShadow::new(0.0, 3.0, 6.0, black(0x14)),
      ]),
      extra: HashMap::new(),
    }
  }
}

impl ThemeShadows {
  /// The elevations of lurq 0.61.
  pub fn legacy() -> Self {
    let black = |alpha: u8| Color::new(0, 0, 0, alpha);
    Self {
      sm: Arc::from([BoxShadow::new(0.0, 1.0, 2.0, black(13))]),
      md: Arc::from([
        BoxShadow::new(0.0, 4.0, 6.0, black(26)).spread(-1.0),
        BoxShadow::new(0.0, 2.0, 4.0, black(26)).spread(-2.0),
      ]),
      lg: Arc::from([
        BoxShadow::new(0.0, 10.0, 15.0, black(26)).spread(-3.0),
        BoxShadow::new(0.0, 4.0, 6.0, black(26)).spread(-4.0),
      ]),
      extra: HashMap::new(),
    }
  }
}
