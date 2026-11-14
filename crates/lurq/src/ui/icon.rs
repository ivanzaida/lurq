//! An icon: a glyph of the theme's set, by name.

use std::{sync::Arc, time::Duration};

use super::{part_style::Length, recipe::Name, theme::current_theme};
use crate::{
  animation::{Animation, AnimationIterationCount, Easing, KeyframesId},
  app::theme::{IconSize, Theme},
  components::{Rect, Text},
  layout::text_style::{TextAlign, TextStyle},
  node::{Element, TextColor, dimension::Dimension},
};

/// The keyframes of a full turn, which every window has: what a spinning
/// icon plays. Every run of them is in phase, so spinners that appear at
/// different moments turn together. Identifiers from here up are lurq's own;
/// an app numbers its keyframes below.
pub const SPIN: KeyframesId = KeyframesId::new(0xFF00);

/// How long a spinning icon takes to turn once.
pub const SPIN_PERIOD: Duration = Duration::from_millis(1000);

/// An icon.
///
/// ```
/// use lurq::{
///   app::theme::{IconSize, PaletteColor},
///   components::Row,
///   ui::{Icon, icons},
/// };
///
/// let row = Row::new()
///   .foreground(PaletteColor::MutedForeground)
///   .child(Icon::new(icons::SEARCH))
///   .child(Icon::new(icons::SETTINGS).size(IconSize::Lg))
///   .child(Icon::new("check").color(PaletteColor::Success));
/// ```
///
/// It is drawn with the glyph the theme has for its name
/// ([`ThemeIcons`](crate::app::theme::ThemeIcons)): one of lurq's own
/// ([`icons`](super::icons)) unless the theme gives the name another, and any
/// name the theme adds. A name the theme has no glyph for takes its room and
/// draws nothing.
///
/// An icon names no color: it is drawn in the foreground of what it is in, as
/// text is, so a button's icon is the color of its label. It is a square the
/// size of a step of the theme's icon sizes, 16 by default.
pub struct Icon {
  name: Name,
  size: Length,
  color: Option<TextColor>,
  spin: bool,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl Icon {
  /// The icon the theme has under `name`: a constant of [`icons`](super::icons), or a name the theme added.
  pub fn new(name: impl Into<Name>) -> Self {
    Self {
      name: name.into(),
      size: Length::Icon(IconSize::Md),
      color: None,
      spin: false,
      id: None,
      key: None,
    }
  }

  /// The side of the icon: an [`IconSize`], or a number of logical pixels.
  pub fn size(mut self, size: impl Into<Length>) -> Self {
    self.size = size.into();
    self
  }

  /// A color for the icon in place of the foreground of what it is in.
  pub fn color(mut self, color: impl Into<TextColor>) -> Self {
    self.color = Some(color.into());
    self
  }

  /// Whether the icon turns, once in [`SPIN_PERIOD`]: a spinner.
  pub fn spin(mut self, spin: bool) -> Self {
    self.spin = spin;
    self
  }

  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// The side of the icon in `theme`, in logical pixels.
  fn side(&self, theme: &Theme) -> f32 {
    match self.size.resolve(theme) {
      Some(Dimension::Px(side)) => side,
      // A size that is no number of pixels, or a step the theme does not have: the default one.
      _ => theme.icon_size_value(IconSize::Md),
    }
  }

  /// Builds the icon in `theme`. Converting an icon into an element does
  /// this in the theme of what is being rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let side = self.side(theme);
    let Some(glyph) = theme.icon(self.name) else {
      let mut blank = Rect::new(side, side);
      if let Some(id) = self.id {
        blank = blank.id(id);
      }
      if let Some(key) = self.key {
        blank = blank.key(key);
      }
      return blank.into();
    };
    let mut buffer = [0; 4];
    let mut icon = Text::styled(
      glyph.glyph.encode_utf8(&mut buffer),
      TextStyle {
        font_family: glyph.family,
        font_size: side,
        line_height: 1.0,
        text_align: TextAlign::Center,
        ..TextStyle::default()
      },
    )
    .nowrap()
    .width(side)
    .height(side);
    if let Some(color) = self.color {
      icon = icon.color(color);
    }
    if self.spin {
      icon = icon.animation(
        Animation::new(SPIN)
          .duration(SPIN_PERIOD)
          .easing(Easing::Linear)
          .iteration_count(AnimationIterationCount::Infinite),
      );
    }
    if let Some(id) = self.id {
      icon = icon.id(id);
    }
    if let Some(key) = self.key {
      icon = icon.key(key);
    }
    icon.into()
  }
}

impl From<Icon> for Element {
  fn from(icon: Icon) -> Self {
    icon.build(&current_theme())
  }
}
