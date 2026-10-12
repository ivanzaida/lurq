//! Exact input glyph paint; no string shaping or wrapping after layout.
use super::*;
use crate::{app::glyph_engine::GlyphEngine, layout::quad::DecoratedInputLayout};

#[cfg(test)]
#[path = "../../../tests/input/text_input/token_paint.rs"]
mod tests;

fn paint_input_glyphs(
  engine: &mut GlyphEngine,
  layout: &DecoratedInputLayout,
  quad: &Quad,
  scale: f32,
  order: usize,
  clip: ClipRect,
  out: &mut Vec<GlyphCmd>,
) {
  let raster = if quad.transform.is_identity() {
    1.0
  } else {
    transformed_text_raster_scale(quad.transform)
  };
  let origin = (quad.x, quad.y);
  let xf_origin = quad
    .transform_origin
    .map(|[x, y]| [(quad.x + x) * scale, (quad.y + y) * scale])
    .unwrap_or([
      (quad.x + quad.width * 0.5) * scale,
      (quad.y + quad.height * 0.5) * scale,
    ]);
  for glyph in &layout.glyphs {
    let start = out.len();
    engine.rasterize_input_glyph(glyph, origin, scale * raster, out);
    for command in &mut out[start..] {
      command.x /= raster;
      command.y /= raster;
      command.width /= raster;
      command.height /= raster;
      command.order = order;
      command.clip = clip;
      if !quad.transform.is_identity() {
        command.transform = quad.transform.matrix_2x2();
        command.transform_origin = [xf_origin[0] - command.x, xf_origin[1] - command.y];
      }
    }
    if let Some(shadow) = glyph.shadow {
      append_text_shadow_glyphs(out, start, &shadow, scale);
    }
    if quad.opacity < 1.0 {
      for command in &mut out[start..] {
        command.color[3] *= quad.opacity.clamp(0.0, 1.0);
      }
    }
  }
}

pub(super) fn resolve_input_quad(
  engine: &mut GlyphEngine,
  layout: &DecoratedInputLayout,
  quad: &Quad,
  scale: f32,
  order: usize,
  clip: ClipRect,
  out: &mut Vec<GlyphCmd>,
) -> std::ops::Range<usize> {
  let start = out.len();
  paint_input_glyphs(engine, layout, quad, scale, order, clip, out);
  start..out.len()
}
