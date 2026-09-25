//! OpenType feature settings for shaping, like CSS `font-feature-settings`.

use std::{fmt, sync::Arc};

/// One OpenType feature setting: a four-byte feature tag and its value. A value
/// of `0` turns the feature off and `1` turns it on; features that choose among
/// alternates (`salt`, `cvNN`) take the alternate's index.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct FontFeature {
  tag: [u8; 4],
  value: u32,
}

impl FontFeature {
  pub const fn new(tag: [u8; 4], value: u32) -> Self {
    Self { tag, value }
  }

  /// Turns the feature on (value `1`).
  pub const fn enable(tag: [u8; 4]) -> Self {
    Self::new(tag, 1)
  }

  /// Turns the feature off (value `0`), e.g. `FontFeature::disable(*b"liga")`.
  pub const fn disable(tag: [u8; 4]) -> Self {
    Self::new(tag, 0)
  }

  pub const fn tag(&self) -> [u8; 4] {
    self.tag
  }

  pub const fn value(&self) -> u32 {
    self.value
  }
}

/// OpenType feature settings applied to the whole text run, on top of the
/// features the shaper enables by default (`liga`, `calt`, `kern`, ...). The
/// default is empty and shapes exactly as without settings.
///
/// Settings are kept sorted by tag with one setting per tag; when a tag is given
/// more than once, the last setting wins. Equal settings therefore compare,
/// hash and cache equally regardless of the order they were written in.
#[derive(Clone, Default, PartialEq, Eq, Hash)]
pub struct FontFeatures(Option<Arc<[FontFeature]>>);

impl FontFeatures {
  pub fn new(features: impl IntoIterator<Item = FontFeature>) -> Self {
    let mut settings: Vec<FontFeature> = Vec::new();
    for feature in features {
      match settings.binary_search_by_key(&feature.tag, |setting| setting.tag) {
        Ok(index) => settings[index] = feature,
        Err(index) => settings.insert(index, feature),
      }
    }
    if settings.is_empty() {
      Self(None)
    } else {
      Self(Some(settings.into()))
    }
  }

  pub fn is_empty(&self) -> bool {
    self.0.is_none()
  }

  /// The settings, sorted by tag.
  pub fn iter(&self) -> impl Iterator<Item = FontFeature> + '_ {
    self.0.iter().flat_map(|settings| settings.iter().copied())
  }

  pub(crate) fn to_cosmic(&self) -> cosmic_text::FontFeatures {
    let mut features = cosmic_text::FontFeatures::new();
    for feature in self.iter() {
      features.set(cosmic_text::FeatureTag::new(&feature.tag), feature.value);
    }
    features
  }
}

/// Formats as `liga=0`.
impl fmt::Debug for FontFeature {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{}={}", String::from_utf8_lossy(&self.tag), self.value)
  }
}

impl fmt::Debug for FontFeatures {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_list().entries(self.iter()).finish()
  }
}

impl FromIterator<FontFeature> for FontFeatures {
  fn from_iter<I: IntoIterator<Item = FontFeature>>(features: I) -> Self {
    Self::new(features)
  }
}

impl<const N: usize> From<[FontFeature; N]> for FontFeatures {
  fn from(features: [FontFeature; N]) -> Self {
    Self::new(features)
  }
}
