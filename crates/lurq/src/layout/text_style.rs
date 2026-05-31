use std::sync::Arc;

use crate::node::color::Color;

#[derive(Clone, PartialEq)]
pub struct TextStyle {
  pub font_family: Arc<str>,
  pub font_size: f32,
  pub line_height: f32,
  pub weight: FontWeight,
  pub style: FontStyle,
  pub color: Color,
}

impl Default for TextStyle {
  fn default() -> Self {
    Self {
      font_family: default_font_family(),
      font_size: 16.0,
      line_height: 1.2,
      weight: FontWeight::Normal,
      style: FontStyle::Normal,
      color: Color::new(0, 0, 0, 255),
    }
  }
}

fn default_font_family() -> Arc<str> {
  #[cfg(target_os = "windows")]
  {
    Arc::from("Segoe UI")
  }
  #[cfg(not(target_os = "windows"))]
  {
    Arc::from("")
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
