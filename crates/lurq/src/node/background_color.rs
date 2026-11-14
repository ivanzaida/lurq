use crate::{app::theme::PaletteColor, node::color::Color};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BackgroundColor {
  Color(Color),
  Palette(PaletteColor),
  /// The foreground the element is in: its own `.foreground(..)` or the nearest one above it, which is the
  /// color a text there with no color of its own is drawn in, and the theme's `Foreground` where none is
  /// set. For a mark that is a fill and goes with the text around it, as the dots of a spinner in a button
  /// do. It is known where the element is painted, so it does not run a transition of the background.
  CurrentForeground,
}

impl BackgroundColor {
  pub fn as_color(self) -> Option<Color> {
    match self {
      Self::Color(color) => Some(color),
      Self::Palette(_) | Self::CurrentForeground => None,
    }
  }

  /// The color by the palette alone: none for the inherited foreground, which is known where the element
  /// is painted ([`Self::resolve_in`]).
  pub(crate) fn resolve(&self, palette: &crate::app::theme::ThemePalette) -> Option<Color> {
    match self {
      Self::Color(color) => Some(*color),
      Self::Palette(color) => palette.try_get(color),
      Self::CurrentForeground => None,
    }
  }

  /// The color where the element is painted, in the `foreground` it inherits there.
  pub(crate) fn resolve_in(
    &self,
    palette: &crate::app::theme::ThemePalette,
    foreground: Option<Color>,
  ) -> Option<Color> {
    match self {
      Self::CurrentForeground => foreground.or_else(|| palette.try_get(&PaletteColor::Foreground)),
      other => other.resolve(palette),
    }
  }
}

impl From<Color> for BackgroundColor {
  fn from(color: Color) -> Self {
    Self::Color(color)
  }
}

impl From<&str> for BackgroundColor {
  fn from(color: &str) -> Self {
    Self::Color(Color::from(color))
  }
}

impl From<PaletteColor> for BackgroundColor {
  fn from(color: PaletteColor) -> Self {
    Self::Palette(color)
  }
}

impl From<&PaletteColor> for BackgroundColor {
  fn from(color: &PaletteColor) -> Self {
    Self::Palette(color.clone())
  }
}
