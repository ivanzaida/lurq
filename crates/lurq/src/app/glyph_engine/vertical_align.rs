//! Where a text's lines sit in a box taller than them: the offset its
//! `VerticalAlign` puts its first line at. Painting draws the glyphs there,
//! and layout writes the same offset into the text's state, so a selection's
//! hit-testing and highlight meet the glyphs on screen.

use super::{GlyphEngine, TextVerticalExtents};
use crate::layout::{
  quad::RichTextSpan,
  text_style::{TextStyle, VerticalAlign},
};

/// The offset of the text's first line in a box `box_height` tall, from its
/// measured `extents`. `Top` and `Bottom` use the visible ink; `Center` the
/// descender-agnostic optical box (the font's cap-height box), so text with
/// descenders and the icons beside it stay aligned instead of being dragged
/// upward by descenders.
pub(crate) fn offset_for_extents(align: VerticalAlign, extents: TextVerticalExtents, box_height: f32) -> f32 {
  match align {
    VerticalAlign::Top => -extents.ink_top,
    VerticalAlign::Center => (box_height - (extents.optical_bottom - extents.optical_top)) * 0.5 - extents.optical_top,
    VerticalAlign::Bottom => box_height - extents.ink_bottom,
    VerticalAlign::LineBox => 0.0,
  }
}

impl GlyphEngine {
  /// The offset of `text`'s first line in a box `box_height` tall, laid out
  /// at `max_width`, as `align` places it.
  pub(crate) fn text_vertical_align_offset(
    &mut self,
    text: &str,
    style: &TextStyle,
    max_width: f32,
    wrap: bool,
    align: VerticalAlign,
    box_height: f32,
  ) -> f32 {
    if box_height <= 0.0 || align == VerticalAlign::LineBox {
      return 0.0;
    }
    if align == VerticalAlign::Center {
      let Some((top, bottom)) = self.text_optical_extents(text, style, max_width, wrap) else {
        return 0.0;
      };
      return (box_height - (bottom - top)) * 0.5 - top;
    }
    let Some(extents) = self.text_vertical_extents(text, style, max_width, wrap) else {
      return 0.0;
    };
    offset_for_extents(align, extents, box_height)
  }

  /// The offset of a rich text's first line in a box `box_height` tall,
  /// its spans' pixel sizes scaled by `scale`. One span is placed as plain
  /// text; spans of several sizes by their measured height, which serves
  /// the Markdown that uses them.
  pub(crate) fn rich_text_vertical_align_offset(
    &mut self,
    spans: &[RichTextSpan],
    scale: f32,
    max_width: f32,
    wrap: bool,
    align: VerticalAlign,
    box_height: f32,
  ) -> f32 {
    if box_height <= 0.0 {
      return 0.0;
    }
    if let [span] = spans {
      let mut style = span.style.clone();
      style.scale_pixels(scale);
      return self.text_vertical_align_offset(&span.text, &style, max_width, wrap, align, box_height);
    }
    let measured = self.measure_rich_text(spans, scale, max_width).height;
    offset_for_extents(
      align,
      TextVerticalExtents {
        ink_top: 0.0,
        ink_bottom: measured,
        optical_top: 0.0,
        optical_bottom: measured,
      },
      box_height,
    )
  }
}
