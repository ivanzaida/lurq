use cosmic_text::{CacheKey, FontSystem, SwashCache, SwashImage};

const MAX_ENTRIES: usize = 2048;
const MAX_BYTES: usize = 16 * 1024 * 1024;

/// Owns every image-cache mutation, including cached missing glyphs. The byte
/// total counts precisely the same image data as the former per-glyph scan.
pub(super) struct GlyphCache {
  swash: SwashCache,
  bytes: usize,
}

impl GlyphCache {
  pub(super) fn new() -> Self {
    Self {
      swash: SwashCache::new(),
      bytes: 0,
    }
  }

  fn clear(&mut self) {
    self.swash.image_cache.clear();
    self.bytes = 0;
  }

  pub(super) fn image(&mut self, fonts: &mut FontSystem, key: CacheKey) -> &Option<SwashImage> {
    // Preserve pre-lookup eviction, even on a hit: an insertion can exceed the
    // byte cap, and the following glyph retires that cohort. None counts as an
    // entry, but contributes no image bytes. Equality at the byte cap is valid.
    if self.swash.image_cache.len() >= MAX_ENTRIES || self.bytes > MAX_BYTES {
      self.clear();
    }
    let missing = !self.swash.image_cache.contains_key(&key);
    let image = self.swash.get_image(fonts, key);
    if missing {
      self.bytes += image.as_ref().map_or(0, |image| image.data.len());
    }
    image
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn fonts() -> FontSystem {
    let mut fonts = FontSystem::new_with_locale_and_db("en-US".into(), Default::default());
    fonts
      .db_mut()
      .load_font_data(include_bytes!("../../../tests/assets/weight_probe/LurqWeightProbe-Regular.ttf").to_vec());
    fonts
  }

  fn key(fonts: &FontSystem, size: f32) -> CacheKey {
    CacheKey::new(
      fonts.db().faces().next().unwrap().id,
      2, // The authored box-shaped "a" in the existing real font.
      size,
      (0., 0.),
      cosmic_text::fontdb::Weight::NORMAL,
      cosmic_text::CacheKeyFlags::empty(),
    )
    .0
  }

  fn actual_bytes(cache: &GlyphCache) -> usize {
    cache
      .swash
      .image_cache
      .values()
      .flatten()
      .map(|image| image.data.len())
      .sum()
  }

  #[test]
  fn real_insertions_hits_and_clear_keep_exact_image_bytes() {
    let mut fonts = fonts();
    let mut cache = GlyphCache::new();
    let first = key(&fonts, 20.);
    let data = cache.image(&mut fonts, first).as_ref().unwrap().data.clone();
    assert!(!data.is_empty());
    assert_eq!(cache.bytes, data.len());
    assert_eq!(cache.image(&mut fonts, first).as_ref().unwrap().data, data);
    assert_eq!(cache.bytes, data.len());
    let second = key(&fonts, 30.);
    cache.image(&mut fonts, second);
    assert_eq!(cache.bytes, actual_bytes(&cache));
    assert!(cache.bytes > data.len());
    cache.clear();
    assert_eq!((cache.bytes, cache.swash.image_cache.len()), (0, 0));
    cache.image(&mut fonts, first);
    assert_eq!(cache.bytes, data.len());
  }

  #[test]
  fn missing_images_count_toward_pre_lookup_entry_eviction() {
    let mut fonts = fonts();
    let mut cache = GlyphCache::new();
    let mut missing = key(&fonts, 20.);
    missing.font_id = cosmic_text::fontdb::ID::dummy();
    for glyph_id in 0..MAX_ENTRIES {
      missing.glyph_id = glyph_id as u16;
      assert!(cache.image(&mut fonts, missing).is_none());
    }
    assert_eq!((cache.bytes, cache.swash.image_cache.len()), (0, MAX_ENTRIES));
    // A hit must also clear at exactly 2048 entries, as the original rule did.
    assert!(cache.image(&mut fonts, missing).is_none());
    assert_eq!((cache.bytes, cache.swash.image_cache.len()), (0, 1));
  }

  #[test]
  fn byte_limit_equality_is_kept_and_excess_retires_before_lookup() {
    let mut fonts = fonts();
    let mut cache = GlyphCache::new();
    let cached = key(&fonts, 20.);
    // An independently specified cache cohort at the byte boundary. No decode
    // or large glyph rasterization is needed to exercise the retirement rule.
    cache.swash.image_cache.insert(
      cached,
      Some(SwashImage {
        data: vec![0; MAX_BYTES],
        ..Default::default()
      }),
    );
    cache.bytes = MAX_BYTES;
    cache.image(&mut fonts, cached);
    assert_eq!(cache.bytes, MAX_BYTES);
    let added = key(&fonts, 30.);
    assert!(cache.image(&mut fonts, added).is_some());
    assert!(cache.bytes > MAX_BYTES);
    assert_eq!(cache.bytes, actual_bytes(&cache));
    cache.image(&mut fonts, added);
    assert_eq!(cache.swash.image_cache.len(), 1);
    assert_eq!(cache.bytes, actual_bytes(&cache));
    assert!(cache.bytes < MAX_BYTES);
  }
}
