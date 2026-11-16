//! What a `TextInput` draws beside its text, its selection and its caret: the wavy lines under ranges an app
//! marks ([`crate::components::InputSquiggles`]), and the suggestion after the end of what is typed
//! ([`crate::components::InputGhost`]).

use super::{LayoutEngine, intersect_clip, text_input_vertical_offset, transformed_quad_frame};
use crate::{
  layout::{
    layout_result::LayoutResult,
    quad::{ClipRect, Quad, QuadContent},
    text_style::{TextStyle, VerticalAlign},
  },
  node::{
    TextTransformMode,
    node::Node,
    node_kind::{TextInputOverflow, TextInputState},
    transform::Transform2D,
  },
};

/// How far down each column of a wavy line stands, a pixel to a column: a wave four across and one and a half
/// high.
const WAVE: [f32; 4] = [0.0, 0.75, 1.5, 0.75];
/// How thick a wavy line is.
const WAVE_WIDTH: f32 = 1.25;

impl LayoutEngine {
  /// Pushes the wavy lines under the ranges an input marks, under the band of each row's text, and the
  /// suggestion after the end of its text while the caret is there.
  #[allow(clippy::too_many_arguments)]
  pub(super) fn push_text_input_marks(
    &self,
    node: &Node,
    state: &TextInputState,
    style: &TextStyle,
    placeholder_style: Option<&TextStyle>,
    result: &LayoutResult,
    (abs_x, abs_y): (f32, f32),
    transform: Transform2D,
    clip: ClipRect,
    opacity: f32,
    quads: &mut Vec<Quad>,
  ) {
    let squiggles = state.squiggles();
    let ghost = state.shown_ghost();
    // The placeholder of an empty input whose kinds of paragraphs are drawn: the input's layout is its one empty
    // paragraph, with what stands before it, and the placeholder is drawn in that paragraph's type and place.
    let kind = state
      .painted_layout()
      .filter(|_| state.is_showing_placeholder())
      .and_then(|layout| layout.config.paragraphs.first().cloned());
    if squiggles.is_none() && ghost.is_none() && kind.is_none() {
      return;
    }
    let padding = self.resolved_padding_for_size(node, result.size);
    let content_height = (result.size.height - padding.top - padding.bottom).max(0.0);
    let viewport = self.text_input_viewport(node, state, result, (abs_x, abs_y), padding);
    let marks_clip = intersect_clip(clip, viewport.transformed(transform));
    let palette = self.palette.borrow();
    if let Some(squiggles) = squiggles
      && let Some(color) = squiggles.color.resolve(&palette)
    {
      let (band_top, band_height) = state.text_band();
      let top = abs_y + padding.top + text_input_vertical_offset(state, content_height);
      for range in squiggles.ranges.iter() {
        for rect in state.range_rects(range.start, range.end) {
          let (row_top, row_height) = state.selection_band_for_row(rect.y, (band_top, band_height));
          // Under the text's band, where a descender ends.
          let line = top + rect.y + row_top + row_height - WAVE_WIDTH;
          let left = abs_x + padding.left + rect.x;
          for column in 0..rect.width.max(0.0).floor() as usize {
            let (x, y, transform, transform_origin) =
              transformed_quad_frame(left + column as f32, line + WAVE[column % WAVE.len()], transform);
            quads.push(Quad {
              x,
              y,
              width: 1.0,
              height: WAVE_WIDTH,
              opacity,
              transform,
              transform_origin,
              content: QuadContent::Rect { color, gradient: None },
              border_radius: None,
              border: None,
              clip: marks_clip,
            });
          }
        }
      }
    }
    if let Some(format) = kind
      && let Some(placeholder) = state.placeholder()
    {
      let mut text = format.base.clone();
      text.color = palette.text_color(placeholder_style.unwrap_or(style));
      let x = abs_x + padding.left + format.indent;
      let width = (viewport.x + viewport.width - x).max(0.0);
      if width > 0.0 {
        let (x, y, transform, transform_origin) = transformed_quad_frame(x, abs_y + padding.top, transform);
        quads.push(Quad {
          x,
          y,
          width,
          height: content_height,
          opacity,
          transform,
          transform_origin,
          content: QuadContent::Text {
            text: placeholder,
            style: text,
            wrap: true,
            vertical_align: VerticalAlign::LineBox,
            transform_mode: TextTransformMode::Bitmap,
          },
          border_radius: None,
          border: None,
          clip: marks_clip,
        });
      }
    }
    if let Some(ghost) = ghost {
      let mut text = style.clone();
      text.color = ghost
        .color
        .as_ref()
        .and_then(|color| color.resolve(&palette))
        .or_else(|| placeholder_style.map(|style| palette.text_color(style)))
        .unwrap_or_else(|| palette.text_color(style));
      // The suggestion stands on the line the caret is on, in the box the text itself is aligned in.
      let multiline = state.overflow() == TextInputOverflow::Multiline;
      let (y, height, vertical_align) = if multiline {
        (
          abs_y + padding.top + state.caret_y(),
          state.caret_height().max(1.0),
          VerticalAlign::LineBox,
        )
      } else {
        (abs_y + padding.top, content_height, text.vertical_align)
      };
      let x = abs_x + padding.left + state.caret_x();
      let width = (viewport.x + viewport.width - x).max(0.0);
      if width > 0.0 {
        let (x, y, transform, transform_origin) = transformed_quad_frame(x, y, transform);
        quads.push(Quad {
          x,
          y,
          width,
          height,
          opacity: opacity * ghost.opacity.clamp(0.0, 1.0),
          transform,
          transform_origin,
          content: QuadContent::Text {
            text: ghost.text.to_string(),
            style: text,
            wrap: false,
            vertical_align,
            transform_mode: TextTransformMode::Bitmap,
          },
          border_radius: None,
          border: None,
          clip: marks_clip,
        });
      }
    }
  }
}
