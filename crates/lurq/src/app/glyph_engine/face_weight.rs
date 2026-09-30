//! Nearest-face weight resolution.
//!
//! cosmic-text (checked through 0.19) only takes a face from the requested
//! family when its weight equals the requested weight exactly, or when it is a
//! variable face whose `wght` axis covers it; any other request falls through
//! to the script and common fallback families, which on Windows can land on a
//! symbol font. Resolving the weight first with fontdb's CSS font-matching query and
//! shaping with the matched face's own weight keeps text in its family and
//! selects the nearest loaded face — Medium and SemiBold faces included.

use std::collections::HashMap;

use cosmic_text::{
  Family, Weight,
  fontdb::{Database, Query, Stretch},
};

use crate::layout::text_style::{FontStyle, FontWeight};

/// Weight of the face CSS font matching selects for `family`. A family with no
/// loaded faces is matched against the generic sans-serif family instead, since
/// that is where its text falls back to; without either, the requested weight is
/// returned unchanged.
pub(crate) fn match_face_weight(db: &Database, family: &str, weight: FontWeight, style: FontStyle) -> Weight {
  let requested = weight.to_cosmic();
  let family = if family.is_empty() {
    Family::SansSerif
  } else {
    Family::Name(family)
  };
  [family, Family::SansSerif]
    .iter()
    .find_map(|family| {
      let id = db.query(&Query {
        families: std::slice::from_ref(family),
        weight: requested,
        stretch: Stretch::Normal,
        style: style.to_cosmic(),
      })?;
      db.face(id).map(|face| face.weight)
    })
    .unwrap_or(requested)
}

/// Memoized [`match_face_weight`]. Must be cleared whenever fonts are loaded.
#[derive(Default)]
pub(crate) struct FaceWeights {
  resolved: HashMap<Box<str>, HashMap<(u16, FontStyle), Weight>>,
}

impl FaceWeights {
  pub(crate) fn resolve(&mut self, db: &Database, family: &str, weight: FontWeight, style: FontStyle) -> Weight {
    let key = (weight.value(), style);
    if let Some(resolved) = self.resolved.get(family).and_then(|weights| weights.get(&key)) {
      return *resolved;
    }
    let resolved = match_face_weight(db, family, weight, style);
    self.resolved.entry(family.into()).or_default().insert(key, resolved);
    resolved
  }

  pub(crate) fn clear(&mut self) {
    self.resolved.clear();
  }
}
