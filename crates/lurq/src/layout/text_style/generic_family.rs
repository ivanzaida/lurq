//! CSS generic font families (`serif`, `sans-serif`, `monospace`, `cursive`,
//! `fantasy`) and the font family text is shaped with.

use cosmic_text::Family;

/// A CSS generic font family. A [`TextStyle::font_family`] (or a canvas
/// font's family) that names one, in any ASCII case (`"monospace"`,
/// `"Monospace"`), asks for the platform's font for it rather than for a
/// font of that name.
///
/// - `monospace`: Consolas, Cascadia Mono or Courier New on Windows; Menlo, Monaco or Courier New on macOS; DejaVu Sans
///   Mono, Noto Sans Mono or Liberation Mono on Linux and other systems.
/// - `serif`: Times New Roman on Windows; Times or Times New Roman on macOS; DejaVu Serif, Noto Serif or Liberation
///   Serif on Linux and other systems.
///
/// The first of these installed is used. With none of them installed,
/// `monospace` draws with the first monospaced font installed and `serif`
/// with the fallback fonts. `sans-serif` draws as the empty family does: in
/// the platform's fallback fonts. `cursive` and `fantasy` ask for the fonts
/// fontdb names (Comic Sans MS; Impact, Papyrus on macOS; fontconfig's choice
/// on Linux) and draw with the fallback fonts where those are not installed.
///
/// The fonts are chosen when the `App` is created (its text engine starts),
/// from the system fonts installed then; fonts the app loads later do not
/// change them. To
/// draw a generic family in a font of the app's own (a bundled monospace
/// font), load it and register an alias of the keyword:
/// `app.register_font("monospace", "JetBrains Mono")`. An alias is resolved
/// before the keyword is read, and the alias of a keyword matches it in any
/// ASCII case.
///
/// [`TextStyle::font_family`]: super::TextStyle::font_family
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GenericFamily {
  Serif,
  SansSerif,
  Monospace,
  Cursive,
  Fantasy,
}

impl GenericFamily {
  const ALL: [Self; 5] = [
    Self::Serif,
    Self::SansSerif,
    Self::Monospace,
    Self::Cursive,
    Self::Fantasy,
  ];

  /// The generic family a font family name names, as CSS reads its keywords:
  /// `"serif"`, `"sans-serif"`, `"monospace"`, `"cursive"` or `"fantasy"` in
  /// any ASCII case. Any other name, the empty one included, is `None`.
  pub fn from_name(name: &str) -> Option<Self> {
    Self::ALL
      .into_iter()
      .find(|generic| name.eq_ignore_ascii_case(generic.keyword()))
  }

  /// The CSS keyword of the family, in lower case: `"sans-serif"` for
  /// [`SansSerif`](Self::SansSerif).
  pub const fn keyword(self) -> &'static str {
    match self {
      Self::Serif => "serif",
      Self::SansSerif => "sans-serif",
      Self::Monospace => "monospace",
      Self::Cursive => "cursive",
      Self::Fantasy => "fantasy",
    }
  }

  pub(crate) const fn to_cosmic(self) -> Family<'static> {
    match self {
      Self::Serif => Family::Serif,
      Self::SansSerif => Family::SansSerif,
      Self::Monospace => Family::Monospace,
      Self::Cursive => Family::Cursive,
      Self::Fantasy => Family::Fantasy,
    }
  }
}

/// The name a font alias is registered and looked up under: a generic
/// family's keyword in lower case, so `"Monospace"` and `"monospace"` share
/// one alias as they share one font, and any other name as it is.
pub(crate) fn alias_name(name: &str) -> &str {
  match GenericFamily::from_name(name) {
    Some(generic) => generic.keyword(),
    None => name,
  }
}

/// The family text in `family` (a font family name after its alias, if any,
/// is resolved) is shaped with: the generic family a CSS keyword names, the
/// sans-serif family for the empty name, and otherwise the font of that name.
/// Every path that builds shaping attributes or queries faces for a family
/// goes through this, so a name always means the same font.
pub(crate) fn cosmic_family(family: &str) -> Family<'_> {
  if family.is_empty() {
    return Family::SansSerif;
  }
  match GenericFamily::from_name(family) {
    Some(generic) => generic.to_cosmic(),
    None => Family::Name(family),
  }
}
