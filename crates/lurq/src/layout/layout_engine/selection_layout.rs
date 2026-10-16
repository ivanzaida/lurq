//! The caret geometry of a selectable text leaf, from the layout the paint
//! draws it with.
//!
//! The paint shapes a text in device pixels: its style scaled by the display
//! scale, wrapped at its box's width rounded up in device pixels,
//! `ceil(width * scale)`. At a fractional width or scale that is not the
//! logical layout: a word ending between the two wraps in one and not the
//! other, and glyph advances differ by rounding. A selection's hit-testing,
//! highlight and copied text index the carets of the painted layout, in
//! logical pixels, so they meet the glyphs on screen; the shaped layout is
//! the one the paint finds in the glyph engine's caches.

use std::sync::Arc;

use super::{LayoutEngine, TextLayoutOutput};
use crate::{
  app::glyph_engine::GlyphEngine,
  layout::{
    Size,
    layout_result::LayoutResult,
    text_style::{TextAlign, TextStyle},
  },
  node::{Node, node_kind::NodeKind, text_selection::CaretPositions},
};

/// Where a text's glyphs lie in its box: its carets, in logical pixels, and
/// its first line's offset from the box's top-left.
struct PaintedText {
  carets: CaretPositions,
  left: f32,
  top: f32,
}

/// The width the paint wraps a text of `width` at, in device pixels, or
/// none when it neither wraps nor aligns.
fn painted_max_width(scale: f32, wrap: bool, align: TextAlign, width: f32) -> f32 {
  if (wrap || align != TextAlign::Left) && width > 0.0 {
    (width * scale).ceil()
  } else {
    f32::MAX
  }
}

impl LayoutEngine {
  /// Writes the carets of a selectable `Text` or `RichText` leaf into its
  /// state and its result, once `result` holds its box.
  pub(super) fn lay_out_text_selection(&self, glyph_engine: &mut GlyphEngine, node: &Node, result: &mut LayoutResult) {
    let Some(TextLayoutOutput::Text(output)) = result.text_layout.as_deref() else {
      return;
    };
    let painted = match node.node_kind() {
      NodeKind::Text { state, style, .. } if state.selectable() => {
        let style = style.resolve(&self.typography.borrow(), &self.palette.borrow());
        let content = node.text_content().unwrap_or_default();
        let text = output.display_text.as_deref().unwrap_or(content);
        // The paint draws the text in the box inside the padding.
        let padding = self.resolved_padding_for_size(node, result.size);
        let inner = Size::new(
          (result.size.width - padding.left - padding.right).max(0.0),
          (result.size.height - padding.top - padding.bottom).max(0.0),
        );
        let mut painted = self.painted_text(glyph_engine, text, &style, output.render_wrap, inner);
        painted.left += padding.left;
        painted.top += padding.top;
        painted
      }
      #[cfg(feature = "markdown")]
      NodeKind::RichText { state, spans, .. } if state.selectable() => {
        let wrap = node.text_wrap && node.text_overflow == crate::node::node_kind::TextOverflow::Clip;
        self.painted_rich_text(glyph_engine, spans, wrap, result.size)
      }
      _ => return,
    };
    let output = output.with_selection(painted.carets, painted.left, painted.top);
    result.text_layout = Some(Arc::new(TextLayoutOutput::Text(output)));
    Self::restore_text_layout_output(node, result);
  }

  /// `text` laid out as the paint lays it out in a box of `size`.
  fn painted_text(
    &self,
    glyph_engine: &mut GlyphEngine,
    text: &str,
    style: &TextStyle,
    wrap: bool,
    size: Size,
  ) -> PaintedText {
    let scale = self.paint_scale.get();
    let mut scaled = style.clone();
    scaled.scale_pixels(scale);
    let max_width = painted_max_width(scale, wrap, style.text_align, size.width);
    let carets = glyph_engine
      .caret_positions(text, &scaled, max_width, wrap)
      .scaled(1.0 / scale);
    let top = glyph_engine.text_vertical_align_offset(
      text,
      &scaled,
      max_width,
      wrap,
      style.vertical_align,
      size.height * scale,
    ) / scale;
    PaintedText { carets, left: 0.0, top }
  }

  /// `spans` laid out as the paint lays them out in a box of `size`:
  /// centered in it, unless the paragraph paints links or underlines and
  /// writes its own frame's top afterwards.
  #[cfg(feature = "markdown")]
  fn painted_rich_text(
    &self,
    glyph_engine: &mut GlyphEngine,
    spans: &[crate::layout::quad::RichTextSpan],
    wrap: bool,
    size: Size,
  ) -> PaintedText {
    let scale = self.paint_scale.get();
    let align = spans.first().map_or(TextAlign::Left, |span| span.style.text_align);
    let max_width = painted_max_width(scale, wrap, align, size.width);
    let mut scaled = spans.to_vec();
    for span in &mut scaled {
      span.style.scale_pixels(scale);
    }
    let carets = glyph_engine
      .rich_text_caret_positions(&scaled, max_width, wrap)
      .scaled(1.0 / scale);
    let top = glyph_engine.rich_text_vertical_align_offset(
      &scaled,
      max_width,
      wrap,
      crate::layout::text_style::VerticalAlign::Center,
      size.height * scale,
    ) / scale;
    PaintedText { carets, left: 0.0, top }
  }
}
