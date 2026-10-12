//! Input-only whitespace advances over the public, already ordered LayoutLine.
use cosmic_text::LayoutLine;

use super::{InputConfig, rows::visual_bounds};
use crate::layout::text_style::TextAlign;

pub(super) fn apply(
  line: &mut LayoutLine,
  blank_words: &[Option<usize>],
  rtl: bool,
  final_row: bool,
  blank_count: usize,
  viewport: f32,
  config: &InputConfig,
) {
  if config.base.text_align != TextAlign::Justified
    || final_row
    || !viewport.is_finite()
    || viewport <= 0.0
    || viewport >= f32::MAX
  {
    return;
  }
  if blank_count == 0 {
    return;
  }
  let bounds = visual_bounds(&line.glyphs);
  let right = bounds.values().map(|bound| bound.1).fold(0.0_f32, f32::max);
  let left = bounds.values().map(|bound| bound.0).fold(right, f32::min);
  let occupied = right - left;
  // Overwide rows keep their advances and clip. Expansion is never a fit retry;
  // contracting spaces would erase valid negative-tracking source geometry.
  let expansion = (viewport - occupied).max(0.0) / blank_count as f32;
  let mut displacement = 0.0;
  for glyph in &mut line.glyphs {
    let added = if blank_words[glyph.metadata].is_some() {
      expansion
    } else {
      0.0
    };
    if rtl {
      displacement -= added;
      glyph.x += displacement;
    } else {
      glyph.x += displacement;
    }
    glyph.w += added;
    if !rtl {
      displacement += added;
    }
  }
  line.w = occupied + displacement.abs();
}
