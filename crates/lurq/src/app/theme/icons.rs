//! The theme's icons: what each name of an icon is drawn with, a glyph of a font or an SVG.

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

/// How an [`IconSvg`] takes the icon's color.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum IconTint {
  /// Its lines are in the icon's color: an icon drawn in strokes, as lucide's are.
  Stroke,
  /// Its shapes are in the icon's color: an icon drawn in fills.
  Fill,
  /// It keeps its own colors: a brand's mark.
  None,
}

/// What an icon is drawn with in place of a glyph: an SVG, at the icon's size, in the icon's color or in its own.
///
/// ```
/// use lurq::app::theme::{IconSvg, IconTint, ThemeIcons};
///
/// const GITHUB: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none"
///   stroke="currentColor" stroke-width="2"><circle cx="12" cy="12" r="9"/></svg>"#;
///
/// let mut set = ThemeIcons::lurq();
/// set.set_svg("github", IconSvg::stroked(GITHUB));
/// assert_eq!(set.svg("github").map(IconSvg::tint), Some(IconTint::Stroke));
/// // A name has one drawing: a glyph given to it takes the SVG's place.
/// assert!(set.get("github").is_none());
/// ```
///
/// SVG is drawn with the feature `canvas`. Without it, or where the source is no SVG, the icon takes its room and
/// draws nothing, as a name the theme has no icon for.
#[derive(Clone)]
pub struct IconSvg {
  source: Arc<str>,
  tint: IconTint,
  #[cfg(feature = "canvas")]
  data: Option<crate::svg::SvgData>,
}

impl IconSvg {
  /// An SVG drawn in `tint`.
  pub fn new(source: impl Into<Arc<str>>, tint: IconTint) -> Self {
    let source = source.into();
    Self {
      #[cfg(feature = "canvas")]
      data: crate::svg::SvgData::parse(source.as_bytes()),
      source,
      tint,
    }
  }

  /// An SVG whose lines are in the icon's color.
  pub fn stroked(source: impl Into<Arc<str>>) -> Self {
    Self::new(source, IconTint::Stroke)
  }

  /// An SVG whose shapes are in the icon's color.
  pub fn filled(source: impl Into<Arc<str>>) -> Self {
    Self::new(source, IconTint::Fill)
  }

  /// An SVG in its own colors.
  pub fn colored(source: impl Into<Arc<str>>) -> Self {
    Self::new(source, IconTint::None)
  }

  /// The SVG's text, as it was given.
  pub fn source(&self) -> &str {
    &self.source
  }

  pub fn tint(&self) -> IconTint {
    self.tint
  }

  /// The SVG read, ready to be painted in the foreground its node inherits where its tint says so: `None` where the
  /// source is no SVG.
  #[cfg(feature = "canvas")]
  pub fn data(&self) -> Option<crate::svg::SvgData> {
    let data = self.data.clone()?;
    Some(match self.tint {
      IconTint::Stroke => data.stroke_current_color(),
      IconTint::Fill => data.fill_current_color(),
      IconTint::None => data,
    })
  }
}

impl std::fmt::Debug for IconSvg {
  fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    formatter
      .debug_struct("IconSvg")
      .field("tint", &self.tint)
      .field("bytes", &self.source.len())
      .finish()
  }
}

impl PartialEq for IconSvg {
  fn eq(&self, other: &Self) -> bool {
    self.tint == other.tint && self.source == other.source
  }
}

/// What a name of the theme's set is drawn with.
#[derive(Clone, Debug, PartialEq)]
enum Drawing {
  Glyph(IconGlyph),
  Svg(IconSvg),
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
///
/// A name may be drawn with an SVG in place of a glyph ([`set_svg`](Self::set_svg), [`IconSvg`]): an app's marks.
#[derive(Clone, Debug, PartialEq)]
pub struct ThemeIcons {
  glyphs: HashMap<RoleName, Drawing>,
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

  /// The glyph `name` is drawn with, if the theme has it drawn with a glyph: `None` for a name drawn with an SVG.
  pub fn get(&self, name: impl Into<RoleName>) -> Option<&IconGlyph> {
    match self.glyphs.get(&name.into())? {
      Drawing::Glyph(glyph) => Some(glyph),
      Drawing::Svg(_) => None,
    }
  }

  /// The SVG `name` is drawn with, if the theme has it drawn with one.
  pub fn svg(&self, name: impl Into<RoleName>) -> Option<&IconSvg> {
    match self.glyphs.get(&name.into())? {
      Drawing::Svg(svg) => Some(svg),
      Drawing::Glyph(_) => None,
    }
  }

  /// Gives `name` a glyph: a new icon, or another drawing of one there is, an SVG too.
  pub fn set(&mut self, name: impl Into<RoleName>, glyph: IconGlyph) {
    self.glyphs.insert(name.into(), Drawing::Glyph(glyph));
  }

  /// Gives `name` an SVG: a new icon, or another drawing of one there is, a glyph too.
  pub fn set_svg(&mut self, name: impl Into<RoleName>, svg: IconSvg) {
    self.glyphs.insert(name.into(), Drawing::Svg(svg));
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

  /// Takes `name` out: an [`Icon`](crate::ui::Icon) of that name draws nothing. The glyph it was drawn with, if it
  /// was drawn with one; an SVG is taken out too, and is not given back.
  pub fn remove(&mut self, name: impl Into<RoleName>) -> Option<IconGlyph> {
    match self.glyphs.remove(&name.into())? {
      Drawing::Glyph(glyph) => Some(glyph),
      Drawing::Svg(_) => None,
    }
  }

  /// The names the theme has an icon for, a glyph or an SVG, in the order of the alphabet.
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
