use std::collections::HashMap;

use crate::layout::text_style::TextStyle;

#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TypographyId(u8);

impl TypographyId {
  pub const fn new(id: u8) -> Self {
    Self(id)
  }

  pub const fn get(self) -> u8 {
    self.0
  }
}

impl From<u8> for TypographyId {
  fn from(id: u8) -> Self {
    Self::new(id)
  }
}

impl From<&TypographyId> for TypographyId {
  fn from(id: &TypographyId) -> Self {
    *id
  }
}

#[derive(Clone)]
pub struct ThemeTypography {
  styles: HashMap<TypographyId, TextStyle>,
  default_style: TextStyle,
}

impl ThemeTypography {
  pub fn new() -> Self {
    Self::default()
  }

  pub fn from_styles<I, K>(styles: I) -> Self
  where
    I: IntoIterator<Item = (K, TextStyle)>,
    K: Into<TypographyId>,
  {
    Self {
      styles: styles.into_iter().map(|(id, style)| (id.into(), style)).collect(),
      default_style: TextStyle::default(),
    }
  }

  pub fn styles(&self) -> &HashMap<TypographyId, TextStyle> {
    &self.styles
  }

  pub fn default_style(&self) -> &TextStyle {
    &self.default_style
  }

  pub fn set_default_style(&mut self, style: TextStyle) {
    self.default_style = style;
  }

  pub fn set(&mut self, id: impl Into<TypographyId>, style: TextStyle) {
    self.styles.insert(id.into(), style);
  }

  pub fn register(&mut self, style: TextStyle) -> TypographyId {
    let id = self.next_available_id();
    self.styles.insert(id, style);
    id
  }

  pub fn get(&self, id: impl Into<TypographyId>) -> Option<&TextStyle> {
    self.styles.get(&id.into())
  }

  pub fn resolve(&self, id: impl Into<TypographyId>) -> TextStyle {
    self
      .styles
      .get(&id.into())
      .cloned()
      .unwrap_or_else(|| self.default_style.clone())
  }

  fn next_available_id(&self) -> TypographyId {
    for raw in 0..=u8::MAX {
      let id = TypographyId::new(raw);
      if !self.styles.contains_key(&id) {
        return id;
      }
    }
    panic!("no typography ids available");
  }
}

impl Default for ThemeTypography {
  fn default() -> Self {
    Self {
      styles: HashMap::new(),
      default_style: TextStyle::default(),
    }
  }
}

#[derive(Clone)]
pub struct ThemeFonts {
  pub body: TextStyle,
  pub heading: TextStyle,
  pub mono: TextStyle,
}

impl From<ThemeTypography> for ThemeFonts {
  fn from(typography: ThemeTypography) -> Self {
    Self {
      body: typography.default_style,
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

impl From<ThemeFonts> for ThemeTypography {
  fn from(fonts: ThemeFonts) -> Self {
    let mut typography = Self::default();
    typography.set_default_style(fonts.body);
    typography
  }
}

impl Default for ThemeFonts {
  fn default() -> Self {
    ThemeTypography::default().into()
  }
}
