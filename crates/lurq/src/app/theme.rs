use std::{
  collections::HashMap,
  sync::{Arc, RwLock},
};

use crate::{layout::text_style::TextStyle, node::color::Color};

#[derive(Clone, Debug)]
pub enum ThemeValue {
  String(String),
  U32(u32),
  F32(f32),
}

impl ThemeValue {
  pub fn as_str(&self) -> Option<&str> {
    match self {
      Self::String(s) => Some(s),
      _ => None,
    }
  }

  pub fn as_u32(&self) -> Option<u32> {
    match self {
      Self::U32(v) => Some(*v),
      _ => None,
    }
  }

  pub fn as_f32(&self) -> Option<f32> {
    match self {
      Self::F32(v) => Some(*v),
      _ => None,
    }
  }
}

impl From<&str> for ThemeValue {
  fn from(s: &str) -> Self {
    Self::String(s.to_owned())
  }
}

impl From<String> for ThemeValue {
  fn from(s: String) -> Self {
    Self::String(s)
  }
}

impl From<u32> for ThemeValue {
  fn from(v: u32) -> Self {
    Self::U32(v)
  }
}

impl From<f32> for ThemeValue {
  fn from(v: f32) -> Self {
    Self::F32(v)
  }
}

#[derive(Clone)]
pub struct Theme {
  inner: Arc<RwLock<ThemeInner>>,
}

struct ThemeInner {
  colors: ThemeColors,
  fonts: ThemeFonts,
  custom: HashMap<String, ThemeValue>,
}

#[derive(Clone)]
pub struct ThemeColors {
  pub background: Color,
  pub surface: Color,
  pub primary: Color,
  pub secondary: Color,
  pub accent: Color,
  pub error: Color,
  pub text: Color,
  pub text_secondary: Color,
  pub border: Color,
}

impl Default for ThemeColors {
  fn default() -> Self {
    Self {
      background: Color::from_hex("#ffffff"),
      surface: Color::from_hex("#f8fafc"),
      primary: Color::from_hex("#3b82f6"),
      secondary: Color::from_hex("#64748b"),
      accent: Color::from_hex("#8b5cf6"),
      error: Color::from_hex("#ef4444"),
      text: Color::from_hex("#1e293b"),
      text_secondary: Color::from_hex("#64748b"),
      border: Color::from_hex("#e2e8f0"),
    }
  }
}

#[derive(Clone)]
pub struct ThemeFonts {
  pub body: TextStyle,
  pub heading: TextStyle,
  pub mono: TextStyle,
}

impl Default for ThemeFonts {
  fn default() -> Self {
    Self {
      body: TextStyle::default(),
      heading: TextStyle {
        font_size: 24.0,
        weight: crate::layout::text_style::FontWeight::Bold,
        ..TextStyle::default()
      },
      mono: TextStyle {
        font_family: std::sync::Arc::from("monospace"),
        ..TextStyle::default()
      },
    }
  }
}

impl Default for Theme {
  fn default() -> Self {
    Self {
      inner: Arc::new(RwLock::new(ThemeInner {
        colors: ThemeColors::default(),
        fonts: ThemeFonts::default(),
        custom: HashMap::new(),
      })),
    }
  }
}

impl Theme {
  pub fn new() -> Self {
    Self::default()
  }

  pub fn colors(&self) -> ThemeColors {
    self.inner.read().unwrap().colors.clone()
  }

  pub fn set_colors(&self, colors: ThemeColors) {
    self.inner.write().unwrap().colors = colors;
  }

  pub fn fonts(&self) -> ThemeFonts {
    self.inner.read().unwrap().fonts.clone()
  }

  pub fn set_fonts(&self, fonts: ThemeFonts) {
    self.inner.write().unwrap().fonts = fonts;
  }

  pub fn set(&self, key: &str, value: impl Into<ThemeValue>) {
    self.inner.write().unwrap().custom.insert(key.to_owned(), value.into());
  }

  pub fn get(&self, key: &str) -> Option<ThemeValue> {
    self.inner.read().unwrap().custom.get(key).cloned()
  }
}
