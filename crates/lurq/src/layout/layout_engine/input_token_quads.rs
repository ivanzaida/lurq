//! Paint exactly the token runs and caret geometry measured by input_token_layout.
use super::{LayoutEngine, intersect_clip, text_input_vertical_offset, transformed_quad_frame};
use crate::{
  layout::{
    layout_result::LayoutResult,
    quad::{ClipRect, Quad, QuadContent},
    text_style::{TextStyle, VerticalAlign},
  },
  node::{
    TextTransformMode,
    border::BorderRadius,
    node::Node,
    node_kind::{TextInputOverflow, TextInputState},
    transform::Transform2D,
  },
};

impl LayoutEngine {
  pub(super) fn input_content(
    &self,
    state: &TextInputState,
    style: &TextStyle,
    placeholder: Option<&TextStyle>,
  ) -> QuadContent {
    let display = super::text_input_display_style(state, style, placeholder).clone();
    // Keep the input's normal background/selection paint path. Decorated glyphs
    // are emitted below from their measured runs instead of this empty text leaf.
    if state.token_layout().is_some() {
      return QuadContent::Text {
        text: String::new(),
        style: display,
        wrap: false,
        vertical_align: VerticalAlign::LineBox,
        transform_mode: TextTransformMode::Bitmap,
      };
    }
    let vertical_align = if state.overflow() == TextInputOverflow::Multiline {
      VerticalAlign::LineBox
    } else {
      display.vertical_align
    };
    QuadContent::Text {
      text: state.rendered_text_for_layout(),
      style: display,
      wrap: state.overflow() == TextInputOverflow::Multiline,
      vertical_align,
      transform_mode: TextTransformMode::Bitmap,
    }
  }

  pub(super) fn push_input_tokens(
    &self,
    node: &Node,
    state: &TextInputState,
    result: &LayoutResult,
    origin: (f32, f32),
    transform: Transform2D,
    clip: ClipRect,
    opacity: f32,
    backgrounds: bool,
    quads: &mut Vec<Quad>,
  ) {
    let Some(layout) = state.token_layout() else {
      return;
    };
    let padding = self.resolved_padding_for_size(node, result.size);
    let height = (result.size.height - padding.top - padding.bottom).max(0.0);
    let width = (result.size.width - padding.left - padding.right).max(0.0);
    let clip = intersect_clip(
      clip,
      ClipRect {
        x: origin.0 + padding.left,
        y: origin.1 + padding.top,
        width,
        height,
        active: true,
        border_radius: None,
      }
      .transformed(transform),
    );
    let start = (
      origin.0 + padding.left - state.scroll_x(),
      origin.1 + padding.top + text_input_vertical_offset(state, height) - state.scroll_y(),
    );
    for piece in &layout.pieces {
      if backgrounds {
        let Some((color, radius)) = piece.background else {
          continue;
        };
        let radius = if radius.is_finite() { radius.max(0.0) } else { 0.0 };
        for (x, y, row_width) in &piece.backgrounds {
          let (x, y, xf, center) =
            transformed_quad_frame(start.0 + piece.x + x - piece.padding, start.1 + piece.y + y, transform);
          quads.push(Quad {
            x,
            y,
            width: *row_width,
            height: state.caret_height(),
            opacity,
            transform: xf,
            transform_origin: center,
            content: QuadContent::Rect { color, gradient: None },
            border_radius: Some(BorderRadius::all(radius).clamped_to_rect(*row_width, state.caret_height())),
            border: None,
            clip,
          });
        }
      } else {
        let (x, y, xf, center) = transformed_quad_frame(start.0 + piece.x, start.1 + piece.y, transform);
        quads.push(Quad {
          x,
          y,
          width: piece.width,
          height: piece.height,
          opacity,
          transform: xf,
          transform_origin: center,
          content: QuadContent::Text {
            text: piece.text.clone(),
            style: piece.style.clone(),
            wrap: state.overflow() == TextInputOverflow::Multiline,
            vertical_align: VerticalAlign::LineBox,
            transform_mode: TextTransformMode::Bitmap,
          },
          border_radius: None,
          border: None,
          clip,
        });
      }
    }
  }
}
