use std::sync::Arc;

use crate::layout::text_style::{FontWeight, TextStyle};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TypographyStyle {
  Heading,
  Title,
  Body,
  Description,
  Caption,
  Label,
  FieldLabel,
  Button,
  Link,
  Mono,
}

impl TypographyStyle {
  pub const fn as_str(self) -> &'static str {
    match self {
      Self::Heading => "heading",
      Self::Title => "title",
      Self::Body => "body",
      Self::Description => "description",
      Self::Caption => "caption",
      Self::Label => "label",
      Self::FieldLabel => "field_label",
      Self::Button => "button",
      Self::Link => "link",
      Self::Mono => "mono",
    }
  }
}

#[derive(Clone, PartialEq)]
pub struct ThemeTypography {
  pub heading: TextStyle,
  pub title: TextStyle,
  pub body: TextStyle,
  pub description: TextStyle,
  pub caption: TextStyle,
  pub label: TextStyle,
  pub field_label: TextStyle,
  pub button: TextStyle,
  pub link: TextStyle,
  pub mono: TextStyle,
}

impl ThemeTypography {
  pub fn new() -> Self {
    Self::default()
  }

  pub fn get(&self, style: impl Into<TypographyStyle>) -> TextStyle {
    match style.into() {
      TypographyStyle::Heading => self.heading.clone(),
      TypographyStyle::Title => self.title.clone(),
      TypographyStyle::Body => self.body.clone(),
      TypographyStyle::Description => self.description.clone(),
      TypographyStyle::Caption => self.caption.clone(),
      TypographyStyle::Label => self.label.clone(),
      TypographyStyle::FieldLabel => self.field_label.clone(),
      TypographyStyle::Button => self.button.clone(),
      TypographyStyle::Link => self.link.clone(),
      TypographyStyle::Mono => self.mono.clone(),
    }
  }

  pub fn set(&mut self, style: impl Into<TypographyStyle>, value: TextStyle) {
    match style.into() {
      TypographyStyle::Heading => self.heading = value,
      TypographyStyle::Title => self.title = value,
      TypographyStyle::Body => self.body = value,
      TypographyStyle::Description => self.description = value,
      TypographyStyle::Caption => self.caption = value,
      TypographyStyle::Label => self.label = value,
      TypographyStyle::FieldLabel => self.field_label = value,
      TypographyStyle::Button => self.button = value,
      TypographyStyle::Link => self.link = value,
      TypographyStyle::Mono => self.mono = value,
    }
  }

  pub fn default_style(&self) -> &TextStyle {
    &self.body
  }

  pub fn set_default_style(&mut self, style: TextStyle) {
    self.body = style;
  }

  pub fn resolve(&self, style: impl Into<TypographyStyle>) -> TextStyle {
    self.get(style)
  }
}

impl Default for ThemeTypography {
  fn default() -> Self {
    let body = TextStyle::default();
    Self {
      heading: TextStyle {
        font_size: 24.0,
        weight: FontWeight::Bold,
        ..body.clone()
      },
      title: TextStyle {
        font_size: 20.0,
        weight: FontWeight::Bold,
        ..body.clone()
      },
      body: body.clone(),
      description: TextStyle {
        font_size: 14.0,
        ..body.clone()
      },
      caption: TextStyle {
        font_size: 12.0,
        ..body.clone()
      },
      label: TextStyle {
        font_size: 13.0,
        weight: FontWeight::Medium,
        ..body.clone()
      },
      field_label: TextStyle {
        font_size: 13.0,
        weight: FontWeight::Medium,
        ..body.clone()
      },
      button: TextStyle {
        font_size: 13.0,
        weight: FontWeight::Medium,
        ..body.clone()
      },
      link: body.clone(),
      mono: TextStyle {
        font_family: Arc::from("monospace"),
        ..body
      },
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
      body: typography.body,
      heading: typography.heading,
      mono: typography.mono,
    }
  }
}

impl From<ThemeFonts> for ThemeTypography {
  fn from(fonts: ThemeFonts) -> Self {
    let mut typography = Self::default();
    typography.body = fonts.body;
    typography.heading = fonts.heading;
    typography.mono = fonts.mono;
    typography
  }
}

impl Default for ThemeFonts {
  fn default() -> Self {
    ThemeTypography::default().into()
  }
}
