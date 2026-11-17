//! A rich text's links without the `markdown` feature, which is where links
//! come from: a paragraph of runs (`Node::rich_text`, the words of a
//! highlight's mark in another weight) has none, so it has no spans drawn as
//! presentation, no frame of its own and no state to paint its spans in.

use super::LayoutEngine;
use crate::{
  layout::{layout_result::LayoutResult, quad::RichTextSpan},
  node::Node,
};

/// Where a span drawn as presentation lies: none here.
pub(crate) struct InlineRect;

impl LayoutEngine {
  /// Where the spans a rich text draws as presentation lie: none.
  pub(super) fn generated_span_rects(_: &Node, _: &LayoutResult) -> Vec<InlineRect> {
    Vec::new()
  }

  /// The text's own frame, which only a paragraph with links or underlines has.
  pub(super) fn inline_text_frame(_: &LayoutResult) -> Option<(f32, f32)> {
    None
  }

  /// The paragraph's spans as painted: as they are, with no link to restyle them.
  pub(super) fn painted_rich_text_spans(
    &self,
    _: &Node,
    spans: &[RichTextSpan],
    _: &LayoutResult,
  ) -> Vec<RichTextSpan> {
    spans.to_vec()
  }
}

/// The parts of the band `x..x + width` that no span drawn as presentation covers: all of it.
pub(super) fn without_rects(x: f32, _: f32, width: f32, _: f32, _: &[InlineRect]) -> Vec<(f32, f32)> {
  vec![(x, width)]
}
