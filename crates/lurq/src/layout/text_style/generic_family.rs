//! CSS generic font families (`serif`, `sans-serif`, `monospace`, `cursive`,
//! `fantasy`) and the font family text is shaped with.

use cosmic_text::Family;

/// A CSS generic font family. A [`TextStyle::font_family`] that names one, in
/// any ASCII case (`"monospace"`, `"Monospace"`), asks for the font the
/// platform uses for it rather than for a font of that name; see
/// [`TextStyle::font_family`] for the fonts each platform resolves them to.
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
  /// The generic family a font family name names, as CSS reads its keywords:
  /// `"serif"`, `"sans-serif"`, `"monospace"`, `"cursive"` or `"fantasy"` in
  /// any ASCII case. Any other name, the empty one included, is `None`.
  pub fn from_name(name: &str) -> Option<Self> {
    [
      ("serif", Self::Serif),
      ("sans-serif", Self::SansSerif),
      ("monospace", Self::Monospace),
      ("cursive", Self::Cursive),
      ("fantasy", Self::Fantasy),
    ]
    .into_iter()
    .find_map(|(keyword, generic)| name.eq_ignore_ascii_case(keyword).then_some(generic))
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
