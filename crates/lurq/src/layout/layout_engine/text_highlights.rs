//! The fills behind highlighted ranges of a `Text` or `RichText` leaf
//! (`Text::highlights`): one per laid-out line a range covers, from the
//! carets of the painted layout, as a selection is drawn. They go under the
//! glyphs and under a selection.

use super::{LayoutEngine, transformed_quad_frame};
use crate::{
  layout::{
    layout_result::LayoutResult,
    quad::{ClipRect, Quad, QuadContent},
  },
  node::{
    border::{BorderPlacement, BorderRadius, ResolvedBorder, ResolvedBorders},
    node::Node,
    node_kind::{NodeKind, TextState},
    transform::Transform2D,
  },
};

impl LayoutEngine {
  /// The text a highlight of `node` is in: its state, its value, and the
  /// height of one of its lines. `None` for a node that is not text.
  fn highlighted_text<'n>(&self, node: &'n Node) -> Option<(&'n TextState, String, f32)> {
    match node.node_kind() {
      NodeKind::Text { state, style, .. } => {
        let style = style.resolve(&self.typography.borrow(), &self.palette.borrow());
        Some((
          state,
          node.text_content().unwrap_or_default().to_owned(),
          style.font_size * style.line_height,
        ))
      }
      NodeKind::RichText { state, spans, .. } => Some((
        state,
        node.rich_text_value().unwrap_or_default(),
        spans
          .first()
          .map_or(16.0, |span| span.style.font_size * span.style.line_height),
      )),
      _ => None,
    }
  }

  /// Where bytes `range` of the text `node` are drawn, in its box: from the
  /// start of the first line they cover to the end of the last, as one
  /// rectangle. `None` where the node is not text or the range is empty.
  pub(crate) fn text_range_rect(
    &self,
    node: &Node,
    result: &LayoutResult,
    range: std::ops::Range<usize>,
  ) -> Option<(f32, f32, f32, f32)> {
    let (state, value, line_height) = self.highlighted_text(node)?;
    let lines = state.range_lines(&value, range);
    let height = line_height.min(result.size.height).max(1.0);
    let left = lines.iter().map(|line| line.x).reduce(f32::min)?;
    let right = lines.iter().map(|line| line.x + line.width).reduce(f32::max)?;
    let top = lines.iter().map(|line| line.y).reduce(f32::min)?;
    let bottom = lines.iter().map(|line| line.y + height).reduce(f32::max)?;
    Some((left, top, right - left, bottom - top))
  }

  #[allow(clippy::too_many_arguments)]
  pub(super) fn push_text_highlights(
    &self,
    node: &Node,
    result: &LayoutResult,
    origin: (f32, f32),
    transform: Transform2D,
    clip: ClipRect,
    opacity: f32,
    quads: &mut Vec<Quad>,
  ) {
    let Some((state, value, line_height)) = self.highlighted_text(node) else {
      return;
    };
    let highlights = state.highlight_ranges(&value);
    if highlights.is_empty() {
      return;
    }
    let height = line_height.min(result.size.height).max(1.0);
    #[cfg(feature = "markdown")]
    let generated = Self::generated_span_rects(node, result);
    let palette = self.palette.borrow();
    for (highlight, lines) in highlights {
      let Some(color) = highlight.color.resolve(&palette) else {
        continue;
      };
      let radius = highlight.drawn_radius();
      let (pad_x, pad_y) = highlight.drawn_padding();
      // The line around the fill, inside it, on every side.
      let border = highlight.border.and_then(|(width, color)| {
        let color = color.resolve(&palette)?;
        let side = ResolvedBorder {
          width: if width.is_finite() { width.max(0.0) } else { 0.0 },
          color,
          placement: BorderPlacement::Inside,
          dash: [0.0, 0.0],
        };
        (side.width > 0.0).then_some(ResolvedBorders {
          top: Some(side),
          right: Some(side),
          bottom: Some(side),
          left: Some(side),
        })
      });
      // A rich text's presentation spans (a link's isolate marks) are not
      // its text: a highlight leaves them out, as a selection does.
      #[cfg(feature = "markdown")]
      let pieces = lines.into_iter().flat_map(|line| {
        super::inline_links::without_rects(line.x, line.y, line.width, height, &generated)
          .into_iter()
          .map(move |(x, width)| (x, line.y, width))
      });
      #[cfg(not(feature = "markdown"))]
      let pieces = lines.into_iter().map(|line| (line.x, line.y, line.width));
      for (x, y, width) in pieces {
        // A padding less than none draws inside the line, never less than a pixel.
        let (fill_width, fill_height) = ((width + 2.0 * pad_x).max(1.0), (height + 2.0 * pad_y).max(1.0));
        let (x, y) = (x + (width - fill_width) / 2.0, y + (height - fill_height) / 2.0);
        let (width, height) = (fill_width, fill_height);
        let (x, y, transform, transform_origin) = transformed_quad_frame(origin.0 + x, origin.1 + y, transform);
        quads.push(Quad {
          x,
          y,
          width,
          height,
          opacity,
          transform,
          transform_origin,
          content: QuadContent::Rect { color, gradient: None },
          border_radius: (radius > 0.0).then(|| BorderRadius::all(radius).clamped_to_rect(width, height)),
          border,
          clip,
        });
      }
    }
  }
}
