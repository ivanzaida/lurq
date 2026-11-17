//! The colors of code: a style for each name of a token, in the theme.

use std::{collections::HashMap, sync::Arc};

use super::{PaletteColor, RoleName, Theme};
use crate::{app::window::Appearance, layout::text_style::FontWeight, node::TextColor};

/// How a token of code is drawn: its color, its weight, whether it leans. What it does not say is the code's own.
///
/// ```
/// use lurq::{app::theme::CodeStyle, node::color::Color};
///
/// let todo = CodeStyle::new().foreground(Color::from_hex("#d97706")).bold().italic();
/// ```
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CodeStyle {
  pub color: Option<TextColor>,
  pub weight: Option<FontWeight>,
  pub italic: Option<bool>,
}

impl CodeStyle {
  /// A style that says nothing: the code's own.
  pub fn new() -> Self {
    Self::default()
  }

  /// Its color: a role of the palette, which follows the light and the dark appearance, or a color of its own.
  pub fn foreground(mut self, color: impl Into<TextColor>) -> Self {
    self.color = Some(color.into());
    self
  }

  pub fn weight(mut self, weight: FontWeight) -> Self {
    self.weight = Some(weight);
    self
  }

  pub fn bold(self) -> Self {
    self.weight(FontWeight::Bold)
  }

  pub fn italic(mut self) -> Self {
    self.italic = Some(true);
    self
  }

  /// Upright, where a less particular name leans.
  pub fn upright(mut self) -> Self {
    self.italic = Some(false);
    self
  }

  /// Whether it says nothing.
  pub fn is_empty(&self) -> bool {
    *self == Self::default()
  }

  /// This style, and what it does not say taken from `under`.
  pub fn or(mut self, under: &CodeStyle) -> Self {
    if self.color.is_none() {
      self.color.clone_from(&under.color);
    }
    if self.weight.is_none() {
      self.weight = under.weight;
    }
    if self.italic.is_none() {
      self.italic = under.italic;
    }
    self
  }

  /// Whether it says each of color, weight and leaning.
  pub fn is_whole(&self) -> bool {
    self.color.is_some() && self.weight.is_some() && self.italic.is_some()
  }
}

impl<T: Into<TextColor>> From<T> for CodeStyle {
  fn from(color: T) -> Self {
    Self::new().foreground(color)
  }
}

/// The colors of code: a [`CodeStyle`] for each name of a token (`keyword`, `string.escape`), looked up dropping the
/// tail of a name it has none for (`keyword.control.rust`, `keyword.control`, `keyword`, then the code's own color).
///
/// A theme has one for its light appearance and one for its dark ([`Theme::set_code_color`],
/// [`Theme::set_code_scheme`]); [`CodeScheme::lurq`] is the theme's unless told, in the palette's roles.
///
/// ```
/// use lurq::app::theme::{CodeScheme, CodeStyle, PaletteColor};
/// use lurq::node::color::Color;
///
/// let scheme = CodeScheme::lurq()
///   .color("keyword.control", Color::from_hex("#c026d3"))
///   .style("comment", CodeStyle::new().italic());
/// assert!(scheme.resolve("comment.doc").italic == Some(true));
/// ```
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CodeScheme {
  styles: HashMap<RoleName, CodeStyle>,
}

impl CodeScheme {
  /// A scheme with no style: every token in the code's own color.
  pub fn new() -> Self {
    Self::default()
  }

  /// lurq's colors of code, in the palette's roles: the words of a language, types, namespaces, tags, keys, `self`
  /// and the language's constants in `Info`; strings and regular expressions in `Success`; numbers, constants and
  /// attributes in `Warning`; comments, operators and punctuation in `MutedForeground`. Of Markdown: headings in
  /// `Info` and bold, code in `Success`, links in `Info`, the markers of lists in `Warning`, quotes muted and
  /// leaning, strong text bold and emphasis leaning. Functions, variables and labels are in the code's own color.
  pub fn lurq() -> Self {
    use PaletteColor::{Info, MutedForeground, Success, Warning};
    let mut scheme = Self::new();
    for (name, role) in [
      ("keyword", Info),
      ("type", Info),
      ("namespace", Info),
      ("tag", Info),
      ("property", Info),
      ("variable.builtin", Info),
      ("constant.language", Info),
      ("string", Success),
      ("regex", Success),
      ("number", Warning),
      ("constant", Warning),
      ("attribute", Warning),
      ("comment", MutedForeground),
      ("punctuation", MutedForeground),
      ("operator", MutedForeground),
      ("markup.code", Success),
      ("markup.link", Info),
      ("markup.list", Warning),
    ] {
      scheme.set(name, CodeStyle::new().foreground(role));
    }
    scheme.set("markup.heading", CodeStyle::new().foreground(Info).bold());
    scheme.set("markup.quote", CodeStyle::new().foreground(MutedForeground).italic());
    scheme.set("markup.bold", CodeStyle::new().bold());
    scheme.set("markup.italic", CodeStyle::new().italic());
    scheme
  }

  /// This scheme, with `name`'s color `color`.
  pub fn color(mut self, name: impl Into<RoleName>, color: impl Into<TextColor>) -> Self {
    self.set_color(name, color);
    self
  }

  /// This scheme, with `name`'s style `style`.
  pub fn style(mut self, name: impl Into<RoleName>, style: impl Into<CodeStyle>) -> Self {
    self.set(name, style);
    self
  }

  /// Gives `name` the style `style`, in place of what it had.
  pub fn set(&mut self, name: impl Into<RoleName>, style: impl Into<CodeStyle>) {
    self.styles.insert(name.into(), style.into());
  }

  /// Gives `name` the color `color`, keeping its weight and its leaning.
  pub fn set_color(&mut self, name: impl Into<RoleName>, color: impl Into<TextColor>) {
    self.styles.entry(name.into()).or_default().color = Some(color.into());
  }

  /// Takes `name`'s style away: it is a less particular name's.
  pub fn remove(&mut self, name: impl Into<RoleName>) {
    self.styles.remove(&name.into());
  }

  /// The style `name` has of its own, not looked up.
  pub fn get(&self, name: impl Into<RoleName>) -> Option<&CodeStyle> {
    self.styles.get(&name.into())
  }

  /// The names it has a style for.
  pub fn names(&self) -> impl Iterator<Item = RoleName> + '_ {
    self.styles.keys().copied()
  }

  /// The style of a token named `name`: each of its color, weight and leaning from the most particular name that
  /// says it, dropping the tail (`keyword.control.rust`, `keyword.control`, `keyword`).
  pub fn resolve(&self, name: &str) -> CodeStyle {
    let mut style = CodeStyle::new();
    for name in crate::code::lookup_names(name) {
      if let Some(own) = self.styles.get(&RoleName::new(name)) {
        style = style.or(own);
        if style.is_whole() {
          break;
        }
      }
    }
    style
  }
}

/// The schemes of a theme: one for its light appearance, and one for its dark where it has one.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct ThemeCode {
  pub(super) light: Arc<CodeScheme>,
  pub(super) dark: Option<Arc<CodeScheme>>,
}

impl ThemeCode {
  pub(super) fn of(&self, appearance: Appearance) -> &Arc<CodeScheme> {
    match (appearance, &self.dark) {
      (Appearance::Dark, Some(dark)) => dark,
      _ => &self.light,
    }
  }

  /// Changes the scheme of `appearance`, or both where `None`.
  fn update(&mut self, appearance: Option<Appearance>, change: impl Fn(&mut CodeScheme)) {
    match appearance {
      None => {
        change(Arc::make_mut(&mut self.light));
        if let Some(dark) = &mut self.dark {
          change(Arc::make_mut(dark));
        }
      }
      Some(Appearance::Light) => {
        if self.dark.is_none() {
          self.dark = Some(self.light.clone());
        }
        change(Arc::make_mut(&mut self.light));
      }
      Some(Appearance::Dark) => {
        let dark = self.dark.get_or_insert_with(|| self.light.clone());
        change(Arc::make_mut(dark));
      }
    }
  }
}

impl Theme {
  /// The colors of code in the appearance the theme is in. Read in a render, it renders that again when they change.
  pub fn code_scheme(&self) -> Arc<CodeScheme> {
    self.track_access();
    let inner = self.inner.read().unwrap();
    inner.code.of(inner.appearance).clone()
  }

  /// The colors of code in `appearance`, whatever appearance the theme is in now.
  pub fn code_scheme_for(&self, appearance: Appearance) -> Arc<CodeScheme> {
    self.inner.read().unwrap().code.of(appearance).clone()
  }

  /// The style a token named `name` is drawn in, dropping the tail of a name the scheme has nothing for.
  pub fn code_style(&self, name: &str) -> CodeStyle {
    self.code_scheme().resolve(name)
  }

  /// Gives code the colors of `scheme`, in the light appearance and the dark.
  pub fn set_code_scheme(&self, scheme: CodeScheme) {
    self.mutate_inner(|inner| {
      inner.code = ThemeCode {
        light: Arc::new(scheme),
        dark: None,
      };
    });
  }

  /// Gives code the colors of `light` in the light appearance and of `dark` in the dark.
  pub fn set_code_schemes(&self, light: CodeScheme, dark: CodeScheme) {
    self.mutate_inner(|inner| {
      inner.code = ThemeCode {
        light: Arc::new(light),
        dark: Some(Arc::new(dark)),
      };
    });
  }

  /// Gives tokens named `name` the color `color`, in every appearance: a role of the palette follows the appearance
  /// by itself. [`Theme::set_code_color_for`] sets it in one.
  ///
  /// ```
  /// use lurq::app::theme::{PaletteColor, Theme};
  ///
  /// let theme = Theme::new();
  /// theme.set_code_color("keyword.control", PaletteColor::Destructive);
  /// assert_eq!(theme.code_style("keyword.control.rust").color, Some(PaletteColor::Destructive.into()));
  /// ```
  pub fn set_code_color(&self, name: impl Into<RoleName>, color: impl Into<TextColor>) {
    let (name, color) = (name.into(), color.into());
    self.mutate_inner(|inner| inner.code.update(None, |scheme| scheme.set_color(name, color.clone())));
  }

  /// Gives tokens named `name` the color `color` in `appearance` alone.
  pub fn set_code_color_for(&self, appearance: Appearance, name: impl Into<RoleName>, color: impl Into<TextColor>) {
    let (name, color) = (name.into(), color.into());
    self.mutate_inner(|inner| {
      inner
        .code
        .update(Some(appearance), |scheme| scheme.set_color(name, color.clone()))
    });
  }

  /// Gives tokens named `name` the style `style`, in every appearance.
  pub fn set_code_style(&self, name: impl Into<RoleName>, style: impl Into<CodeStyle>) {
    let (name, style) = (name.into(), style.into());
    self.mutate_inner(|inner| inner.code.update(None, |scheme| scheme.set(name, style.clone())));
  }
}
