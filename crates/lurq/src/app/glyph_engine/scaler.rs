//! Swash scaler setup shared by the plain and transformed glyph rasterizers.

use cosmic_text::{CacheKey, Font};
use swash::{
  Tag,
  scale::{ScaleContext, ScalerBuilder},
};

const WGHT: Tag = Tag::from_be_bytes(*b"wght");

/// An unhinted scaler at the cache key's size. A variable font is placed at the
/// `wght` axis position cosmic-text shaped it with, so outlines match advances.
///
/// Coordinates are always replaced: a `ScaleContext` keeps the previous
/// scaler's coordinates, which would otherwise leak between fonts.
pub(super) fn unhinted_scaler<'a>(context: &'a mut ScaleContext, font: &'a Font, key: CacheKey) -> ScalerBuilder<'a> {
  let swash = font.as_swash();
  let variations = swash.variations();
  let coords = variations.find_by_tag(WGHT).map(|axis| {
    let weight = f32::from(key.font_weight.0).clamp(axis.min_value(), axis.max_value());
    variations.normalized_coords([(WGHT, weight)])
  });
  context
    .builder(swash)
    .size(f32::from_bits(key.font_size_bits))
    .hint(false)
    .normalized_coords(coords.into_iter().flatten())
}
