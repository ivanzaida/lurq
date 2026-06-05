use std::{
  ops::Deref,
  sync::{Arc, RwLock, RwLockReadGuard},
};

use crate::{
  core::Signal,
  layout::text_style::TextStyle,
  node::{color::Color, dimension::Dimension},
};

#[cfg(feature = "form")]
mod form;
mod palette;
mod radius;
mod spacing;
mod typography;

#[cfg(feature = "form")]
pub use form::{FormButtonStyle, FormCheckboxStyle, FormFieldStyle, FormInputStyle, FormSliderStyle, FormTheme};
pub use palette::{PaletteId, ThemePalette};
pub use radius::{RadiusId, ThemeRadii};
pub use spacing::{SpacingId, ThemeSpacing};
pub use typography::{ThemeFonts, ThemeTypography, TypographyId};

#[derive(Clone)]
pub struct Theme {
  inner: Arc<RwLock<ThemeInner>>,
  version_signal: Signal<u64>,
}

struct ThemeInner {
  palette: ThemePalette,
  spacing: ThemeSpacing,
  radii: ThemeRadii,
  typography: ThemeTypography,
  #[cfg(feature = "form")]
  form: FormTheme,
  version: u64,
}

pub struct ThemeRef<'a, T> {
  inner: RwLockReadGuard<'a, ThemeInner>,
  value: fn(&ThemeInner) -> &T,
}

impl<T> Deref for ThemeRef<'_, T> {
  type Target = T;

  fn deref(&self) -> &Self::Target {
    (self.value)(&self.inner)
  }
}

impl Default for Theme {
  fn default() -> Self {
    Self {
      inner: Arc::new(RwLock::new(ThemeInner {
        palette: ThemePalette::default(),
        spacing: ThemeSpacing::default(),
        radii: ThemeRadii::default(),
        typography: ThemeTypography::default(),
        #[cfg(feature = "form")]
        form: FormTheme::default(),
        version: 0,
      })),
      version_signal: Signal::new(0),
    }
  }
}

impl Theme {
  pub fn new() -> Self {
    Self::default()
  }

  pub fn lens<T: Clone + Send + Sync + 'static>(
    &self,
    getter: impl Fn(&Theme) -> T + Send + Sync + 'static,
    setter: impl Fn(&Theme, T) + Send + Sync + 'static,
  ) -> ThemeLens<T> {
    ThemeLens {
      theme: self.clone(),
      getter: Arc::new(getter),
      setter: Arc::new(setter),
    }
  }

  pub fn palette(&self) -> ThemeRef<'_, ThemePalette> {
    ThemeRef {
      inner: self.inner.read().unwrap(),
      value: |inner| &inner.palette,
    }
  }

  pub fn set_palette(&self, palette: ThemePalette) {
    let mut inner = self.inner.write().unwrap();
    inner.palette = palette;
    self.bump_version(&mut inner);
  }

  pub fn set_palette_color(&self, id: impl Into<PaletteId>, color: Color) {
    let mut inner = self.inner.write().unwrap();
    inner.palette.set(id, color);
    self.bump_version(&mut inner);
  }

  pub fn register_palette_color(&self, color: Color) -> PaletteId {
    let mut inner = self.inner.write().unwrap();
    let id = inner.palette.register(color);
    self.bump_version(&mut inner);
    id
  }

  pub fn palette_color(&self, id: impl Into<PaletteId>) -> Option<Color> {
    self.inner.read().unwrap().palette.get(id).copied()
  }

  pub fn spacing(&self) -> ThemeRef<'_, ThemeSpacing> {
    ThemeRef {
      inner: self.inner.read().unwrap(),
      value: |inner| &inner.spacing,
    }
  }

  pub fn set_spacing(&self, spacing: ThemeSpacing) {
    let mut inner = self.inner.write().unwrap();
    inner.spacing = spacing;
    self.bump_version(&mut inner);
  }

  pub fn set_spacing_value(&self, id: impl Into<SpacingId>, value: impl Into<Dimension>) {
    let mut inner = self.inner.write().unwrap();
    inner.spacing.set(id, value);
    self.bump_version(&mut inner);
  }

  pub fn register_spacing(&self, value: impl Into<Dimension>) -> SpacingId {
    let mut inner = self.inner.write().unwrap();
    let id = inner.spacing.register(value);
    self.bump_version(&mut inner);
    id
  }

  pub fn spacing_value(&self, id: impl Into<SpacingId>) -> Option<Dimension> {
    self.inner.read().unwrap().spacing.get(id)
  }

  pub fn radii(&self) -> ThemeRef<'_, ThemeRadii> {
    ThemeRef {
      inner: self.inner.read().unwrap(),
      value: |inner| &inner.radii,
    }
  }

  pub fn set_radii(&self, radii: ThemeRadii) {
    let mut inner = self.inner.write().unwrap();
    inner.radii = radii;
    self.bump_version(&mut inner);
  }

  pub fn set_radius_value(&self, id: impl Into<RadiusId>, value: f32) {
    let mut inner = self.inner.write().unwrap();
    inner.radii.set(id, value);
    self.bump_version(&mut inner);
  }

  pub fn register_radius(&self, value: f32) -> RadiusId {
    let mut inner = self.inner.write().unwrap();
    let id = inner.radii.register(value);
    self.bump_version(&mut inner);
    id
  }

  pub fn radius_value(&self, id: impl Into<RadiusId>) -> Option<f32> {
    self.inner.read().unwrap().radii.get(id)
  }

  pub fn typography(&self) -> ThemeRef<'_, ThemeTypography> {
    ThemeRef {
      inner: self.inner.read().unwrap(),
      value: |inner| &inner.typography,
    }
  }

  pub fn set_typography(&self, typography: ThemeTypography) {
    let mut inner = self.inner.write().unwrap();
    inner.typography = typography;
    self.bump_version(&mut inner);
  }

  pub fn set_typography_style(&self, id: impl Into<TypographyId>, style: TextStyle) {
    let mut inner = self.inner.write().unwrap();
    inner.typography.set(id, style);
    self.bump_version(&mut inner);
  }

  pub fn default_text_style(&self) -> TextStyle {
    self.inner.read().unwrap().typography.default_style().clone()
  }

  pub fn set_default_text_style(&self, style: TextStyle) {
    let mut inner = self.inner.write().unwrap();
    inner.typography.set_default_style(style);
    self.bump_version(&mut inner);
  }

  pub fn register_typography_style(&self, style: TextStyle) -> TypographyId {
    let mut inner = self.inner.write().unwrap();
    let id = inner.typography.register(style);
    self.bump_version(&mut inner);
    id
  }

  pub fn typography_style(&self, id: impl Into<TypographyId>) -> Option<TextStyle> {
    self.inner.read().unwrap().typography.get(id).cloned()
  }

  pub fn fonts(&self) -> ThemeFonts {
    self.inner.read().unwrap().typography.clone().into()
  }

  pub fn set_fonts(&self, fonts: ThemeFonts) {
    self.set_typography(fonts.into());
  }

  #[cfg(feature = "form")]
  pub fn form(&self) -> ThemeRef<'_, FormTheme> {
    ThemeRef {
      inner: self.inner.read().unwrap(),
      value: |inner| &inner.form,
    }
  }

  #[cfg(feature = "form")]
  pub fn set_form(&self, form: FormTheme) {
    let mut inner = self.inner.write().unwrap();
    inner.form = form;
    self.bump_version(&mut inner);
  }

  pub(crate) fn version(&self) -> u64 {
    self.inner.read().unwrap().version
  }

  pub(crate) fn track_access(&self) {
    let _ = self.version_signal.get();
  }

  fn bump_version(&self, inner: &mut ThemeInner) {
    inner.version = inner.version.wrapping_add(1);
    self.version_signal.set(inner.version);
  }
}

pub struct ThemeLens<T> {
  theme: Theme,
  getter: Arc<dyn Fn(&Theme) -> T + Send + Sync>,
  setter: Arc<dyn Fn(&Theme, T) + Send + Sync>,
}

impl<T: Clone + Send + Sync + 'static> ThemeLens<T> {
  pub fn get(&self) -> T {
    (self.getter)(&self.theme)
  }

  pub fn set(&self, value: T) {
    (self.setter)(&self.theme, value);
  }

  pub fn update(&self, f: impl FnOnce(&mut T)) {
    let mut value = self.get();
    f(&mut value);
    self.set(value);
  }
}

impl<T> Clone for ThemeLens<T> {
  fn clone(&self) -> Self {
    Self {
      theme: self.theme.clone(),
      getter: self.getter.clone(),
      setter: self.setter.clone(),
    }
  }
}
