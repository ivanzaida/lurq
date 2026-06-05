use crate::{app::theme::PaletteColor, node::color::Color};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BackgroundColor {
  Color(Color),
  Palette(PaletteColor),
}

impl BackgroundColor {
  pub fn as_color(self) -> Option<Color> {
    match self {
      Self::Color(color) => Some(color),
      Self::Palette(_) => None,
    }
  }

  pub(crate) fn resolve(&self, palette: &crate::app::theme::ThemePalette) -> Option<Color> {
    match self {
      Self::Color(color) => Some(*color),
      Self::Palette(color) => palette.try_get(color),
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
