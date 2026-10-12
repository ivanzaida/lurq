use super::*;
use crate::layout::render_list::GlyphCmd;

impl GlyphEngine {
  pub(crate) fn rasterize_input_glyph(
    &mut self,
    input: &InputGlyph,
    origin: (f32, f32),
    scale: f32,
    out: &mut Vec<GlyphCmd>,
  ) {
    let physical = input
      .glyph
      .physical((origin.0 * scale, (origin.1 + input.baseline) * scale), scale);
    let Some(packed) = self.get_or_pack_glyph(physical.cache_key) else {
      return;
    };
    self.push_glyph_cmd(
      out,
      physical.x as f32 + packed.left as f32,
      physical.y as f32 - packed.top as f32,
      packed,
      input.color.to_linear_f32_array(),
      false,
    );
  }
}
