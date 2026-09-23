//! Cap height lookup for optical text centering.

use cosmic_text::{
  Font, LayoutGlyph, Weight,
  fontdb::Database,
  skrifa::{FontRef, MetadataProvider, instance::Size, raw::types::Tag},
};

/// The face and size a laid-out glyph was shaped with.
#[derive(Clone, Copy)]
pub(super) struct GlyphFace {
  pub(super) font_id: cosmic_text::fontdb::ID,
  pub(super) weight: Weight,
  pub(super) font_size: f32,
}

impl GlyphFace {
  pub(super) fn of(glyph: &LayoutGlyph) -> Self {
    Self {
      font_id: glyph.font_id,
      weight: glyph.font_weight,
      font_size: glyph.font_size,
    }
  }
}

/// Cap height in pixels at `weight` (the `wght` axis position cosmic-text
/// shapes with), using the Latin H outline when older text fonts omit the
/// metric. Fonts without either still use the ink fallback.
pub(super) fn cap_height_px(db: &Database, font: &Font, weight: Weight, font_size: f32) -> Option<f32> {
  let metrics = font.metrics();
  let upem = f32::from(metrics.units_per_em);
  if upem <= 0.0 {
    return None;
  }
  if let Some(cap_height) = metrics.cap_height.filter(|&cap| cap > 0.0) {
    return Some(cap_height * font_size / upem);
  }
  // A fixed reference glyph keeps the baseline independent of the current
  // value (e.g. "as" -> "asd" in DejaVu Sans with an older OS/2 table).
  let index = db.face(font.id())?.index;
  let face = FontRef::from_index(font.data(), index).ok()?;
  let glyph = face.charmap().map('H')?;
  if glyph.to_u32() == 0 {
    return None;
  }
  let location = face.axes().location([(Tag::new(b"wght"), f32::from(weight.0))]);
  let bounds = face.glyph_metrics(Size::unscaled(), &location).bounds(glyph)?;
  Some(bounds.y_max * font_size / upem)
}
