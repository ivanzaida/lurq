//! Input-only paint dispatch and geometry offsets.
use super::{
  ElementLayoutRecord, LayoutEngine, OverlayLayoutIndex, ResolvedPadding, TEXT_INPUT_CARET_WIDTH,
  inset_clip_for_border, intersect_clip, text_input_vertical_offset, text_layout_output::TextLayoutOutput,
  transformed_quad_frame,
};
use crate::{
  core::ElementRect,
  layout::{
    layout_result::LayoutResult,
    quad::{ClipRect, Quad, QuadContent},
    text_style::{TextStyle, VerticalAlign},
  },
  node::{
    TextTransformMode,
    border::{BorderPlacement, BorderRadius, ResolvedBorder, ResolvedBorders},
    node::Node,
    node_kind::{NodeKind, TextInputOverflow, TextInputState},
    transform::Transform2D,
  },
};

impl LayoutEngine {
  /// Publishes the element a text input puts at a byte of its value ([`TextInput::anchor_at`]): where the caret
  /// before that byte stands in the layout just made, inside the input's padding, past what it scrolled, one
  /// wide and as high as the text's band. An overlay that hangs from the element is placed by it in this pass.
  ///
  /// [`TextInput::anchor_at`]: crate::components::TextInput::anchor_at
  pub(super) fn index_input_anchor(
    node: &Node,
    layout: &LayoutResult,
    rect: ElementRect,
    visible: bool,
    index: &mut OverlayLayoutIndex,
  ) {
    let NodeKind::TextInput { state, .. } = node.node_kind() else {
      return;
    };
    let Some((anchor, byte)) = state.byte_anchor() else {
      return;
    };
    let Some(TextLayoutOutput::TextInput(output)) = layout.text_layout.as_deref() else {
      return;
    };
    let padding = output.padding;
    let content_height = (layout.size.height - padding.top - padding.bottom).max(0.0);
    let (x, y, height) = state.controller_point(byte);
    let x = padding.left + x;
    let y = padding.top + text_input_vertical_offset(state, content_height) + y;
    let placed = ElementRect {
      x: rect.x + x,
      y: rect.y + y,
      relative_x: x,
      relative_y: y,
      width: TEXT_INPUT_CARET_WIDTH,
      height: height.max(1.0),
    };
    anchor.update(placed.x, placed.y, x, y, placed.width, placed.height);
    index.elements.push(ElementLayoutRecord {
      element_ref: anchor,
      rect: placed,
      visible,
    });
  }

  pub(super) fn input_content(
    &self,
    state: &TextInputState,
    style: &TextStyle,
    placeholder: Option<&TextStyle>,
  ) -> QuadContent {
    if let Some(layout) = state.painted_layout() {
      return QuadContent::DecoratedInput { layout };
    }
    let mut display = super::text_input_display_style(state, style, placeholder).clone();
    display.color = self.palette.borrow().text_color(&display);
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

  /// The box a text input shows its text, selection and caret in, before the
  /// node's transform. A single-line input shows them inside its padding. A
  /// multiline input scrolls its padding with its text, so it shows them from
  /// its top border to its bottom border, as a scroll container shows its
  /// content; across, the text stays inside the padding, which holds a
  /// reserved scrollbar's lane.
  pub(super) fn text_input_viewport(
    &self,
    node: &Node,
    state: &TextInputState,
    result: &LayoutResult,
    origin: (f32, f32),
    padding: ResolvedPadding,
  ) -> ClipRect {
    let content = ClipRect::rect(
      origin.0 + padding.left,
      origin.1 + padding.top,
      (result.size.width - padding.left - padding.right).max(0.0),
      (result.size.height - padding.top - padding.bottom).max(0.0),
    );
    if state.overflow() != TextInputOverflow::Multiline {
      return content;
    }
    let border = node.get_resolved_border(&self.palette.borrow(), &self.border_sizes.borrow());
    let inside_border = inset_clip_for_border(
      ClipRect::rect(origin.0, origin.1, result.size.width, result.size.height),
      border,
    );
    ClipRect {
      y: inside_border.y,
      height: inside_border.height,
      ..content
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
    quads: &mut Vec<Quad>,
  ) {
    let Some(layout) = state.painted_layout() else {
      return;
    };
    let padding = self.resolved_padding_for_size(node, result.size);
    let height = (result.size.height - padding.top - padding.bottom).max(0.0);
    let clip = intersect_clip(
      clip,
      self
        .text_input_viewport(node, state, result, origin, padding)
        .transformed(transform),
    );
    let start = (
      origin.0 + padding.left - state.scroll_x(),
      origin.1 + padding.top + text_input_vertical_offset(state, height) - state.scroll_y(),
    );
    for (at, part) in layout.fragments.iter().enumerate() {
      let (x, y, xf, center) = transformed_quad_frame(start.0 + part.x, start.1 + part.y, transform);
      let edge = part.token.and_then(|(_, border)| border).map(|(width, color)| {
        let side = Some(ResolvedBorder {
          width,
          color,
          placement: BorderPlacement::Inside,
          dash: [0.0, 0.0],
        });
        ResolvedBorders {
          top: side,
          right: side,
          bottom: side,
          left: side,
        }
      });
      quads.push(Quad {
        x,
        y,
        width: part.width,
        height: part.height,
        opacity,
        transform: xf,
        transform_origin: center,
        content: QuadContent::Rect {
          color: part.color,
          gradient: None,
        },
        border_radius: Some(BorderRadius::all(part.radius).clamped_to_rect(part.width, part.height)),
        border: edge,
        clip,
      });
      // A token's icon is at the end of its last fragment, in the room its padding keeps for it.
      let Some((token, _)) = part.token else {
        continue;
      };
      let Some(icon) = &layout.config.styles[token].chip.icon else {
        continue;
      };
      if layout.fragments[at + 1..]
        .iter()
        .any(|later| later.token.is_some_and(|(later, _)| later == token))
      {
        continue;
      }
      let (x, y, xf, center) = transformed_quad_frame(
        start.0 + part.x + part.width - icon.inset - icon.size,
        start.1 + part.y + (part.height - icon.size) * 0.5,
        transform,
      );
      let mut text = [0; 4];
      quads.push(Quad {
        x,
        y,
        width: icon.size,
        height: icon.size,
        opacity,
        transform: xf,
        transform_origin: center,
        content: QuadContent::Text {
          text: icon.glyph.encode_utf8(&mut text).to_owned(),
          style: icon.style.clone(),
          wrap: false,
          vertical_align: VerticalAlign::Center,
          transform_mode: TextTransformMode::Bitmap,
        },
        border_radius: None,
        border: None,
        clip,
      });
    }
  }
}
