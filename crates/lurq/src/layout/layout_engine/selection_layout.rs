//! The caret geometry of a selectable text leaf, from the layout the paint
//! draws it with.
//!
//! The paint shapes a text in device pixels: its style scaled by the display
//! scale, wrapped at its box's width rounded up in device pixels,
//! `ceil(width * scale)`, and placed in its whole box (a `Text`'s padding
//! does not move its glyphs) where `GlyphEngine::painted_text_offset` puts
//! its first line. At a fractional width or scale that is not the
//! logical layout: a word ending between the two wraps in one and not the
//! other, and glyph advances differ by rounding. A selection's hit-testing,
//! highlight and copied text index the carets of the painted layout, kept in
//! its device pixels with the scale ([`PaintedCarets`]), so they meet the
//! glyphs on screen; the shaped layout is the one the paint finds in the
//! glyph engine's caches.

use std::sync::Arc;

use super::{LayoutEngine, TextLayoutOutput};
use crate::{
  app::glyph_engine::GlyphEngine,
  layout::{
    Size,
    layout_result::LayoutResult,
    text_style::{TextAlign, TextStyle},
  },
  node::{
    Node,
    node_kind::{NodeKind, PaintedCarets},
  },
};

/// Where a text's glyphs lie in its box: its carets, and its first line's
/// offset from the box's top, in logical pixels.
struct PaintedText {
  carets: PaintedCarets,
  top: f32,
}

/// The width the paint wraps a text of `width` at, in device pixels, or
/// none when it neither wraps nor aligns.
///
/// Layout shapes a text in logical pixels and the paint shapes it again at
/// the font size of the display's scale, so the box is rounded out to a
/// device pixel: the paint never needs a line more than layout measured.
/// Where the box is a whole number of device pixels already, rounding out
/// gives no room, and the paint's sum of advances may come out a few
/// millionths over it: ten letters of 10.8 are 108.00002 in a box of 108 (a
/// monospace font of 7.2 a letter at a scale of 1.5), and the last letter
/// went to a second line. So the width has a hundredth of a device pixel
/// more, and more than that for a long line, whose sum drifts further.
pub(crate) fn painted_max_width(scale: f32, wrap: bool, align: TextAlign, width: f32) -> f32 {
  if (wrap || align != TextAlign::Left) && width > 0.0 {
    let device = (width * scale).ceil();
    device + (device * 1e-5).max(0.01)
  } else {
    f32::MAX
  }
}

impl LayoutEngine {
  /// Writes the carets of a selectable or highlighted `Text` or `RichText`
  /// leaf into its state and its result, once `result` holds its box.
  pub(super) fn lay_out_text_selection(&self, glyph_engine: &mut GlyphEngine, node: &Node, result: &mut LayoutResult) {
    let Some(TextLayoutOutput::Text(output)) = result.text_layout.as_deref() else {
      return;
    };
    let painted = match node.node_kind() {
      NodeKind::Text { state, style, .. } if state.measures_carets() => {
        let style = style.resolve(&self.typography.borrow(), &self.palette.borrow());
        let content = node.text_content().unwrap_or_default();
        let text = output.display_text.as_deref().unwrap_or(content);
        self.painted_text(glyph_engine, text, &style, output.render_wrap, result.size)
      }
      NodeKind::RichText { state, spans, .. } if state.measures_carets() => {
        let wrap = node.text_wrap && node.text_overflow == crate::node::node_kind::TextOverflow::Clip;
        // A paragraph's objects are as wide in its carets as in its paint: a selection and a highlight past one
        // meet the words they cover.
        let objects = self.object_spans(glyph_engine, node, spans, result.size.width);
        let laid_out = objects
          .as_ref()
          .map_or(spans.as_slice(), |(widened, _)| widened.as_slice());
        self.painted_rich_text(glyph_engine, laid_out, wrap, result.size)
      }
      _ => return,
    };
    let output = output.with_selection(painted.carets, painted.top);
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
    let positions = glyph_engine.caret_positions(text, &scaled, max_width, wrap);
    let top = glyph_engine.painted_text_offset(
      text,
      style,
      &scaled,
      size.width,
      size.height,
      max_width,
      wrap,
      style.vertical_align,
      scale,
    ) / scale;
    PaintedText {
      carets: PaintedCarets { positions, scale },
      top,
    }
  }

  /// `spans` laid out as the paint lays them out in a box of `size`:
  /// centered in it, unless the paragraph paints links or underlines and
  /// writes its own frame's top afterwards.
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
    let positions = glyph_engine.rich_text_caret_positions(spans, scale, max_width, wrap);
    let top = glyph_engine.rich_text_vertical_align_offset(
      spans,
      scale,
      max_width,
      wrap,
      crate::layout::text_style::VerticalAlign::Center,
      size.height * scale,
    ) / scale;
    PaintedText {
      carets: PaintedCarets { positions, scale },
      top,
    }
  }
}
