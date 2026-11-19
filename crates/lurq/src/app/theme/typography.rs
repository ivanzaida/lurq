use std::{collections::HashMap, sync::Arc};

use super::role_name::RoleName;
use crate::layout::text_style::{FontWeight, TextCase, TextStyle};

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
  /// The headings of a document, as Markdown's `#` to `######` and the design's Prose draw them: 30 bold, 24, 20,
  /// 16 and 14 semibold, and 12 semibold in capitals. Apart from `Heading` and `Title`, which head a page or a
  /// panel of the interface.
  Heading1,
  Heading2,
  Heading3,
  Heading4,
  Heading5,
  Heading6,
  /// Text read at length in the interface, as the design's Prose has it: 14 on lines of 1.6.
  Prose,
  /// The text of a document, as Markdown's paragraphs and lists are: 15 on lines of 1.65.
  Article,
  /// An application-defined style stored in [`ThemeTypography::extra`].
  Extra(RoleName),
}

impl TypographyStyle {
  /// An application-defined style. The name is interned (see [`TypographyStyle::Extra`]).
  pub fn extra(name: impl AsRef<str>) -> Self {
    Self::Extra(RoleName::new(name))
  }

  pub fn as_str(self) -> &'static str {
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
      Self::Heading1 => "heading1",
      Self::Heading2 => "heading2",
      Self::Heading3 => "heading3",
      Self::Heading4 => "heading4",
      Self::Heading5 => "heading5",
      Self::Heading6 => "heading6",
      Self::Prose => "prose",
      Self::Article => "article",
      Self::Extra(name) => name.as_str(),
    }
  }
}

impl From<&str> for TypographyStyle {
  fn from(name: &str) -> Self {
    Self::extra(name)
  }
}

impl From<Arc<str>> for TypographyStyle {
  fn from(name: Arc<str>) -> Self {
    Self::extra(name)
  }
}

#[derive(Clone, PartialEq)]
pub struct ThemeTypography {
  /// The family of interface text: every style but `mono` is in it. The system's own font unless the theme
  /// names one; [`Self::set_sans`] changes it in every style at once.
  pub sans: Arc<str>,
  /// The family of code: the `mono` style is in it. The generic `monospace` unless the theme names one;
  /// [`Self::set_mono`] changes it in every style that has it.
  pub mono_family: Arc<str>,
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
  /// A document's headings, from `#` to `######`.
  pub heading1: TextStyle,
  pub heading2: TextStyle,
  pub heading3: TextStyle,
  pub heading4: TextStyle,
  pub heading5: TextStyle,
  pub heading6: TextStyle,
  /// Text read at length in the interface.
  pub prose: TextStyle,
  /// The text of a document.
  pub article: TextStyle,
  pub extra: HashMap<Arc<str>, TextStyle>,
}

impl ThemeTypography {
  pub fn new() -> Self {
    Self::default()
  }

  /// Panics when a [`TypographyStyle::Extra`] name is not in [`Self::extra`]; see [`Self::try_get`].
  pub fn get(&self, style: impl Into<TypographyStyle>) -> TextStyle {
    let style = style.into();
    self
      .try_get(style)
      .unwrap_or_else(|| panic!("typography style not found: {}", style.as_str()))
  }

  pub fn try_get(&self, style: impl Into<TypographyStyle>) -> Option<TextStyle> {
    let style = match style.into() {
      TypographyStyle::Heading => &self.heading,
      TypographyStyle::Title => &self.title,
      TypographyStyle::Body => &self.body,
      TypographyStyle::Description => &self.description,
      TypographyStyle::Caption => &self.caption,
      TypographyStyle::Label => &self.label,
      TypographyStyle::FieldLabel => &self.field_label,
      TypographyStyle::Button => &self.button,
      TypographyStyle::Link => &self.link,
      TypographyStyle::Mono => &self.mono,
      TypographyStyle::Heading1 => &self.heading1,
      TypographyStyle::Heading2 => &self.heading2,
      TypographyStyle::Heading3 => &self.heading3,
      TypographyStyle::Heading4 => &self.heading4,
      TypographyStyle::Heading5 => &self.heading5,
      TypographyStyle::Heading6 => &self.heading6,
      TypographyStyle::Prose => &self.prose,
      TypographyStyle::Article => &self.article,
      TypographyStyle::Extra(name) => self.extra.get(name.as_str())?,
    };
    Some(style.clone())
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
      TypographyStyle::Heading1 => self.heading1 = value,
      TypographyStyle::Heading2 => self.heading2 = value,
      TypographyStyle::Heading3 => self.heading3 = value,
      TypographyStyle::Heading4 => self.heading4 = value,
      TypographyStyle::Heading5 => self.heading5 = value,
      TypographyStyle::Heading6 => self.heading6 = value,
      TypographyStyle::Prose => self.prose = value,
      TypographyStyle::Article => self.article = value,
      TypographyStyle::Extra(name) => {
        self.extra.insert(Arc::from(name.as_str()), value);
      }
    }
  }

  pub fn default_style(&self) -> &TextStyle {
    &self.body
  }

  /// Sets the family of interface text: every style in the family [`Self::sans`] was, an app's own included,
  /// is in `family` now. One setting changes the font of the whole interface.
  pub fn set_sans(&mut self, family: impl Into<Arc<str>>) {
    let (old, new) = (self.sans.clone(), family.into());
    self.replace_family(&old, &new);
    self.sans = new;
  }

  /// Sets the family of code: every style in the family [`Self::mono_family`] was is in `family` now.
  pub fn set_mono(&mut self, family: impl Into<Arc<str>>) {
    let (old, new) = (self.mono_family.clone(), family.into());
    self.replace_family(&old, &new);
    self.mono_family = new;
  }

  fn replace_family(&mut self, old: &Arc<str>, new: &Arc<str>) {
    let styles = [
      &mut self.heading,
      &mut self.title,
      &mut self.body,
      &mut self.description,
      &mut self.caption,
      &mut self.label,
      &mut self.field_label,
      &mut self.button,
      &mut self.link,
      &mut self.mono,
      &mut self.heading1,
      &mut self.heading2,
      &mut self.heading3,
      &mut self.heading4,
      &mut self.heading5,
      &mut self.heading6,
      &mut self.prose,
      &mut self.article,
    ];
    for style in styles.into_iter().chain(self.extra.values_mut()) {
      if style.font_family == *old {
        style.font_family = new.clone();
      }
    }
  }

  pub fn set_default_style(&mut self, style: TextStyle) {
    self.body = style;
  }

  pub fn resolve(&self, style: impl Into<TypographyStyle>) -> TextStyle {
    self.get(style)
  }

  pub fn try_resolve(&self, style: impl Into<TypographyStyle>) -> Option<TextStyle> {
    self.try_get(style)
  }

  /// Node resolution: a missing extra style falls back to the default style, as
  /// an unresolved palette color leaves a node's own color in place.
  pub(crate) fn resolve_or_default(&self, style: TypographyStyle) -> TextStyle {
    self.try_get(style).unwrap_or_else(|| self.default_style().clone())
  }
}

impl ThemeTypography {
  /// These styles as a person who asked the system for larger text has them: every size multiplied by
  /// `scale`.
  pub fn scaled(&self, scale: f32) -> Self {
    let adjust = |style: &TextStyle| {
      let mut style = style.clone();
      style.font_size *= scale;
      style.letter_spacing *= scale;
      style
    };
    Self {
      sans: self.sans.clone(),
      mono_family: self.mono_family.clone(),
      heading: adjust(&self.heading),
      title: adjust(&self.title),
      body: adjust(&self.body),
      description: adjust(&self.description),
      caption: adjust(&self.caption),
      label: adjust(&self.label),
      field_label: adjust(&self.field_label),
      button: adjust(&self.button),
      link: adjust(&self.link),
      mono: adjust(&self.mono),
      heading1: adjust(&self.heading1),
      heading2: adjust(&self.heading2),
      heading3: adjust(&self.heading3),
      heading4: adjust(&self.heading4),
      heading5: adjust(&self.heading5),
      heading6: adjust(&self.heading6),
      prose: adjust(&self.prose),
      article: adjust(&self.article),
      extra: self
        .extra
        .iter()
        .map(|(name, style)| (name.clone(), adjust(style)))
        .collect(),
    }
  }
}

/// The design's type scale (`pencil/lurq-design-system.pen`, Foundations), in the system's own font.
impl Default for ThemeTypography {
  fn default() -> Self {
    let body = TextStyle {
      font_size: 14.0,
      ..TextStyle::default()
    };
    let mono_family: Arc<str> = Arc::from("monospace");
    Self {
      sans: body.font_family.clone(),
      mono_family: mono_family.clone(),
      heading: TextStyle {
        font_size: 24.0,
        weight: FontWeight::SemiBold,
        letter_spacing: -0.5,
        ..body.clone()
      },
      title: TextStyle {
        font_size: 20.0,
        weight: FontWeight::SemiBold,
        letter_spacing: -0.3,
        ..body.clone()
      },
      body: body.clone(),
      description: body.clone(),
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
      // JetBrains Mono's own line, 17 at 13, as the design draws code.
      mono: TextStyle {
        font_family: mono_family,
        font_size: 13.0,
        line_height: 1.3,
        ..body.clone()
      },
      ..Self::document_roles(&body)
    }
  }
}

impl ThemeTypography {
  /// The roles of a document, on `body`'s family: the headings of the design's Markdown and Prose (`hMuPL`,
  /// `cS7gJ`), each on the line its family draws (1.2), and the text read at length on its own.
  fn document_roles(body: &TextStyle) -> Self {
    let heading = |size: f32, weight: FontWeight, letter_spacing: f32| TextStyle {
      font_size: size,
      weight,
      letter_spacing,
      line_height: 1.2,
      ..body.clone()
    };
    Self {
      sans: body.font_family.clone(),
      mono_family: Arc::from("monospace"),
      heading: body.clone(),
      title: body.clone(),
      body: body.clone(),
      description: body.clone(),
      caption: body.clone(),
      label: body.clone(),
      field_label: body.clone(),
      button: body.clone(),
      link: body.clone(),
      mono: body.clone(),
      heading1: heading(30.0, FontWeight::Bold, -0.6),
      heading2: heading(24.0, FontWeight::SemiBold, -0.4),
      heading3: heading(20.0, FontWeight::SemiBold, 0.0),
      heading4: heading(16.0, FontWeight::SemiBold, 0.0),
      heading5: heading(14.0, FontWeight::SemiBold, 0.0),
      heading6: TextStyle {
        case: TextCase::Upper,
        ..heading(12.0, FontWeight::SemiBold, 0.8)
      },
      prose: TextStyle {
        font_size: 14.0,
        line_height: 1.6,
        ..body.clone()
      },
      article: TextStyle {
        font_size: 15.0,
        line_height: 1.65,
        ..body.clone()
      },
      extra: HashMap::new(),
    }
  }
}

impl ThemeTypography {
  /// The typography of lurq 0.61: body text of 16, bold headings, and code as large as body text.
  pub fn legacy() -> Self {
    let body = TextStyle::default();
    let mono_family: Arc<str> = Arc::from("monospace");
    Self {
      sans: body.font_family.clone(),
      mono_family: mono_family.clone(),
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
        font_family: mono_family,
        ..body.clone()
      },
      // 0.61 had no roles of a document: they are the design's, on its body text.
      ..Self::document_roles(&body)
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
    Self {
      body: fonts.body,
      heading: fonts.heading,
      mono: fonts.mono,
      ..Self::default()
    }
  }
}

impl Default for ThemeFonts {
  fn default() -> Self {
    ThemeTypography::default().into()
  }
}
