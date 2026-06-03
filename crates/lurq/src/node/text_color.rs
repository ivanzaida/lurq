use crate::{app::theme::PaletteId, node::color::Color};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextColor {
  Color(Color),
  Palette(PaletteId),
}

impl TextColor {
  pub fn as_color(self) -> Option<Color> {
    match self {
      Self::Color(color) => Some(color),
      Self::Palette(_) => None,
    }
  }

  pub(crate) fn resolve(self, palette: &crate::app::theme::ThemePalette) -> Option<Color> {
    match self {
      Self::Color(color) => Some(color),
      Self::Palette(id) => palette.get(id).copied(),
    }
  }
}

impl From<Color> for TextColor {
  fn from(color: Color) -> Self {
    Self::Color(color)
  }
}

impl From<&str> for TextColor {
  fn from(color: &str) -> Self {
    Self::Color(Color::from(color))
  }
}

impl From<PaletteId> for TextColor {
  fn from(id: PaletteId) -> Self {
    Self::Palette(id)
  }
}
