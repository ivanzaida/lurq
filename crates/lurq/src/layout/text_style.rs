use std::sync::Arc;

use crate::{
  layout::Alignment,
  node::{TextColor, color::Color},
};

const DEFAULT_FONT_SIZE: f32 = 16.0;
const DEFAULT_LINE_HEIGHT: f32 = 1.2;
const DEFAULT_TEXT_COLOR: Color = Color::new(0, 0, 0, 255);
#[cfg(target_os = "windows")]
const DEFAULT_FONT_FAMILY_WINDOWS: &str = "Segoe UI";
#[cfg(not(target_os = "windows"))]
const DEFAULT_FONT_FAMILY_FALLBACK: &str = "";

#[derive(Clone, PartialEq)]
pub struct TextStyle {
  pub font_family: Arc<str>,
  pub font_size: f32,
  pub line_height: f32,
  pub weight: FontWeight,
  pub style: FontStyle,
  pub text_align: TextAlign,
  pub color: Color,
  pub caret_color: Option<TextColor>,
  pub shadow: Option<TextShadow>,
}

impl Default for TextStyle {
  fn default() -> Self {
    Self {
      font_family: default_font_family(),
      font_size: DEFAULT_FONT_SIZE,
      line_height: DEFAULT_LINE_HEIGHT,
      weight: FontWeight::Normal,
      style: FontStyle::Normal,
      text_align: TextAlign::Left,
      color: DEFAULT_TEXT_COLOR,
      caret_color: None,
      shadow: None,
    }
  }
}

/// CSS-like text shadow: offsets and blur radius are in logical pixels, the
/// blur radius maps to a Gaussian with `sigma = blur_radius / 2`.
#[derive(Clone, Copy, PartialEq)]
pub struct TextShadow {
  pub offset_x: f32,
  pub offset_y: f32,
  pub blur_radius: f32,
  pub color: Color,
}

impl TextShadow {
  pub fn new(offset_x: f32, offset_y: f32, blur_radius: f32, color: Color) -> Self {
    Self {
      offset_x,
      offset_y,
      blur_radius: blur_radius.max(0.0),
      color,
    }
  }

  pub(crate) fn is_visible(&self) -> bool {
    self.color.a() > 0 && (self.offset_x != 0.0 || self.offset_y != 0.0 || self.blur_radius > 0.0)
  }
}

fn default_font_family() -> Arc<str> {
  #[cfg(target_os = "windows")]
  {
    Arc::from(DEFAULT_FONT_FAMILY_WINDOWS)
  }
  #[cfg(not(target_os = "windows"))]
  {
    Arc::from(DEFAULT_FONT_FAMILY_FALLBACK)
  }
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum FontWeight {
  Thin,
  Light,
  #[default]
  Normal,
  Medium,
  Bold,
  Black,
}

impl FontWeight {
  pub fn to_cosmic(&self) -> cosmic_text::Weight {
    match self {
      Self::Thin => cosmic_text::Weight(100),
      Self::Light => cosmic_text::Weight(300),
      Self::Normal => cosmic_text::Weight(400),
      Self::Medium => cosmic_text::Weight(400),
      Self::Bold => cosmic_text::Weight(700),
      Self::Black => cosmic_text::Weight(900),
    }
  }
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum FontStyle {
  #[default]
  Normal,
  Italic,
}

impl FontStyle {
  pub fn to_cosmic(&self) -> cosmic_text::Style {
    match self {
      Self::Normal => cosmic_text::Style::Normal,
      Self::Italic => cosmic_text::Style::Italic,
    }
  }
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum TextAlign {
  #[default]
  Left,
  Center,
  Right,
  Justified,
  End,
}

impl TextAlign {
  pub(crate) fn to_cosmic(self) -> cosmic_text::Align {
    match self {
      Self::Left => cosmic_text::Align::Left,
      Self::Center => cosmic_text::Align::Center,
      Self::Right => cosmic_text::Align::Right,
      Self::Justified => cosmic_text::Align::Justified,
      Self::End => cosmic_text::Align::End,
    }
  }
}

impl From<Alignment> for TextAlign {
  fn from(alignment: Alignment) -> Self {
    match alignment {
      Alignment::Start | Alignment::Stretch => Self::Left,
      Alignment::Center => Self::Center,
      Alignment::End => Self::Right,
    }
  }
}
