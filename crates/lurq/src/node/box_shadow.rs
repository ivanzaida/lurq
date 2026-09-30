use std::sync::Arc;

use crate::{
  app::theme::{ShadowStyle, ThemePalette, ThemeShadows},
  node::{background_color::BackgroundColor, color::Color},
};

/// One CSS-like box shadow: `box-shadow: [inset] offset_x offset_y blur spread color`.
///
/// Lengths are logical pixels and scale with the display like every other
/// length. `blur` is the CSS blur radius, a Gaussian with a standard deviation
/// of `blur / 2`; `0` gives a hard edge. `spread` grows (or, negative, shrinks)
/// the shadow shape before it is blurred; its corners follow the element's
/// radii the way CSS adjusts them. `color` accepts a [`Color`], a hex string or
/// a [`PaletteColor`](crate::app::theme::PaletteColor) role.
///
/// An outer shadow paints beneath the element and only outside its border box,
/// so it never shows through a translucent background. An [`inset`](Self::inset)
/// shadow paints inside the padding box, above the background and below the
/// border and the content. Shadows never take part in layout or hit testing.
#[derive(Clone, Debug, PartialEq)]
pub struct BoxShadow {
  pub offset_x: f32,
  pub offset_y: f32,
  pub blur: f32,
  pub spread: f32,
  pub color: BackgroundColor,
  pub inset: bool,
}

impl BoxShadow {
  /// An outer shadow without spread.
  pub fn new(offset_x: f32, offset_y: f32, blur: f32, color: impl Into<BackgroundColor>) -> Self {
    Self {
      offset_x,
      offset_y,
      blur,
      spread: 0.0,
      color: color.into(),
      inset: false,
    }
  }

  pub fn spread(mut self, spread: f32) -> Self {
    self.spread = spread;
    self
  }

  /// Paints the shadow inside the element instead of beneath it.
  pub fn inset(mut self) -> Self {
    self.inset = true;
    self
  }

  pub(crate) fn resolve(&self, palette: &ThemePalette) -> Option<ResolvedBoxShadow> {
    let color = self.color.resolve(palette)?;
    let blur = sanitize(self.blur).max(0.0);
    (color.a() > 0).then_some(ResolvedBoxShadow {
      offset_x: sanitize(self.offset_x),
      offset_y: sanitize(self.offset_y),
      blur,
      spread: sanitize(self.spread),
      color,
      inset: self.inset,
    })
  }
}

fn sanitize(value: f32) -> f32 {
  if value.is_finite() { value } else { 0.0 }
}

/// The box shadow of an element: a [`ShadowStyle`] theme role or an explicit
/// list painted front to back (the first shadow on top, as in CSS).
#[derive(Clone, Debug, PartialEq)]
pub enum BoxShadowValue {
  Theme(ShadowStyle),
  Shadows(Arc<[BoxShadow]>),
}

impl BoxShadowValue {
  /// No shadow. Useful in a hover or focus [`Style`](crate::node::Style) to
  /// remove the element's own shadow.
  pub fn none() -> Self {
    Self::Shadows(Arc::from([]))
  }

  pub(crate) fn shadows<'a>(&'a self, theme: &'a ThemeShadows) -> &'a [BoxShadow] {
    match self {
      // A missing `Extra` role paints no shadow, as an unresolved palette
      // colour paints nothing.
      Self::Theme(style) => theme.try_get_ref(*style).unwrap_or(&[]),
      Self::Shadows(shadows) => shadows,
    }
  }
}

impl From<ShadowStyle> for BoxShadowValue {
  fn from(style: ShadowStyle) -> Self {
    Self::Theme(style)
  }
}

impl From<BoxShadow> for BoxShadowValue {
  fn from(shadow: BoxShadow) -> Self {
    Self::Shadows(Arc::from([shadow]))
  }
}

impl From<Vec<BoxShadow>> for BoxShadowValue {
  fn from(shadows: Vec<BoxShadow>) -> Self {
    Self::Shadows(shadows.into())
  }
}

impl<const N: usize> From<[BoxShadow; N]> for BoxShadowValue {
  fn from(shadows: [BoxShadow; N]) -> Self {
    Self::Shadows(Arc::from(shadows))
  }
}

impl From<Arc<[BoxShadow]>> for BoxShadowValue {
  fn from(shadows: Arc<[BoxShadow]>) -> Self {
    Self::Shadows(shadows)
  }
}

/// A shadow with its colour resolved against the palette; what layout hands
/// to painting. Lengths are still logical pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ResolvedBoxShadow {
  pub offset_x: f32,
  pub offset_y: f32,
  pub blur: f32,
  pub spread: f32,
  pub color: Color,
  pub inset: bool,
}

impl ResolvedBoxShadow {
  /// How far this shadow can paint outside the element's border box on any
  /// side, in the shadow's own units. Inset shadows stay inside.
  pub fn outset(&self) -> f32 {
    if self.inset {
      return 0.0;
    }
    let reach = self.spread + crate::layout::box_shadow::BLUR_EXTENT_SIGMAS * self.blur * 0.5;
    (reach + self.offset_x.abs().max(self.offset_y.abs())).max(0.0)
  }
}
