//! The theme's icons: what each name of an icon is drawn with.

use std::{collections::HashMap, sync::Arc};

use super::RoleName;

/// What an icon is drawn with: a character of a font.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IconGlyph {
  /// The family of the font, as an app loaded it.
  pub family: Arc<str>,
  pub glyph: char,
}

impl IconGlyph {
  pub fn new(family: impl Into<Arc<str>>, glyph: char) -> Self {
    Self {
      family: family.into(),
      glyph,
    }
  }
}

/// The icons of a theme by name. [`Icon`](crate::ui::Icon) draws the glyph
/// its name has here, so a theme that gives a name another glyph changes that
/// icon wherever it is drawn: in an app's own interface and in lurq's
/// components, which name the icons they draw.
///
/// ```
/// use lurq::{app::theme::{IconGlyph, ThemeIcons}, ui::icons};
///
/// let mut set = ThemeIcons::lurq();
/// // One icon from another font, which the app loads.
/// set.set(icons::SEARCH, IconGlyph::new("brand icons", '\u{F002}'));
/// // An icon of the app's own, under a name of its own.
/// set.set("rocket", IconGlyph::new("brand icons", '\u{F135}'));
/// assert_eq!(set.get("rocket").map(|glyph| glyph.glyph), Some('\u{F135}'));
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct ThemeIcons {
  glyphs: HashMap<RoleName, IconGlyph>,
}

impl ThemeIcons {
  /// The icons of lurq's font ([`ui::icons`](crate::ui::icons)): what a
  /// theme has unless it says otherwise.
  pub fn lurq() -> Self {
    let mut icons = Self::empty();
    icons.set_font(crate::ui::icons::FAMILY, crate::ui::icons::ALL.iter().copied());
    icons
  }

  /// No icon at all: for a theme that brings every one it draws.
  pub fn empty() -> Self {
    Self { glyphs: HashMap::new() }
  }

  /// What `name` is drawn with, if the theme has it.
  pub fn get(&self, name: impl Into<RoleName>) -> Option<&IconGlyph> {
    self.glyphs.get(&name.into())
  }

  /// Gives `name` a glyph: a new icon, or another drawing of one there is.
  pub fn set(&mut self, name: impl Into<RoleName>, glyph: IconGlyph) {
    self.glyphs.insert(name.into(), glyph);
  }

  /// Gives each of `glyphs` its character of the font `family`: a whole set
  /// at once.
  pub fn set_font(
    &mut self,
    family: impl Into<Arc<str>>,
    glyphs: impl IntoIterator<Item = (impl Into<RoleName>, char)>,
  ) {
    let family = family.into();
    for (name, glyph) in glyphs {
      self.set(name, IconGlyph::new(family.clone(), glyph));
    }
  }

  /// Takes `name` out: an [`Icon`](crate::ui::Icon) of that name draws nothing.
  pub fn remove(&mut self, name: impl Into<RoleName>) -> Option<IconGlyph> {
    self.glyphs.remove(&name.into())
  }

  /// The names the theme has an icon for, in the order of the alphabet.
  pub fn names(&self) -> Vec<RoleName> {
    let mut names: Vec<RoleName> = self.glyphs.keys().copied().collect();
    names.sort_by_key(|name| name.as_str());
    names
  }

  pub fn len(&self) -> usize {
    self.glyphs.len()
  }

  pub fn is_empty(&self) -> bool {
    self.glyphs.is_empty()
  }
}

impl Default for ThemeIcons {
  fn default() -> Self {
    Self::lurq()
  }
}
