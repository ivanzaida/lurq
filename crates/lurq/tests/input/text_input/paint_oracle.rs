use super::{super::GLYPH_ATLAS_PADDING, GlyphEngine, InputGlyph};

#[derive(Debug)]
pub(crate) struct RasterSnapshot {
  pub(crate) expected_corner: [f32; 2],
  pub(crate) expected_size: [f32; 2],
}

pub(crate) fn snapshot(
  engine: &mut GlyphEngine,
  input: &InputGlyph,
  origin: (f32, f32),
  scale: f32,
) -> Option<RasterSnapshot> {
  let glyph = &input.glyph;
  let physical = glyph.physical((origin.0 * scale, (origin.1 + input.baseline) * scale), scale);
  assert_eq!(physical.cache_key.font_id, glyph.font_id);
  assert_eq!(physical.cache_key.glyph_id, glyph.glyph_id);
  assert_eq!(physical.cache_key.font_weight, glyph.font_weight);
  assert_eq!(
    f32::from_bits(physical.cache_key.font_size_bits),
    glyph.font_size * scale
  );
  let font = engine
    .font_system
    .get_font(physical.cache_key.font_id, physical.cache_key.font_weight)
    .expect("a shaped glyph retains its loaded font");
  let mut context = swash::scale::ScaleContext::new();
  let image = super::super::render_glyph_image(&mut context, &font, physical.cache_key)?;
  if image.placement.width == 0 || image.placement.height == 0 {
    return None;
  }
  let packed = engine
    .atlas_entries
    .get(&physical.cache_key)
    .expect("the observed paint call cached its nonempty glyph");
  let padding = GLYPH_ATLAS_PADDING as i32;
  // The unchanged unhinted image primitive is independent of input command
  // construction. Bearings include a fixed physical margin, not a logical one.
  assert_eq!(packed.left, image.placement.left - padding);
  assert_eq!(packed.top, image.placement.top + padding);
  let width = image.placement.width + GLYPH_ATLAS_PADDING * 2;
  let height = image.placement.height + GLYPH_ATLAS_PADDING * 2;
  assert_eq!((packed.width, packed.height), (width, height));
  Some(RasterSnapshot {
    expected_corner: [
      (physical.x + image.placement.left - padding) as f32,
      (physical.y - image.placement.top - padding) as f32,
    ],
    expected_size: [width as f32, height as f32],
  })
}
