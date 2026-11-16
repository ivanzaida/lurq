//! The objects of a styled text input: child nodes placed on the boxes its text keeps for them.
//!
//! An object is one character of the input's text and a child node of the input, the element the app gave for
//! it ([`crate::components::TextInput::objects`]). Before the text is laid out each child is measured in the room
//! the text has, and its size is the box its character takes in its row. Once the input's own box is known the
//! children are laid out at their boxes and placed on them, inside the input's padding and past what it has
//! scrolled: the input lays out again when it scrolls, so they move with its text, and they are cut by its box as
//! anything in it is. As a paragraph places its links ([`super::inline_links`]), so an input places its objects.

use std::sync::Arc;

use super::{DEFAULT_TEXT_SELECTION_COLOR, LayoutEngine, intersect_clip, text_input_vertical_offset};
use crate::{
  app::{glyph_engine::GlyphEngine, theme::PaletteColor},
  layout::{
    Constraints, Offset, Size,
    layout_result::{ChildLayout, LayoutResult, SubtreeHolds},
    quad::{ClipRect, Quad, QuadContent},
  },
  node::{
    border::BorderRadius,
    node::Node,
    node_kind::{NodeKind, TextInputState},
    transform::Transform2D,
  },
};

/// How round the veil over a selected object is, and how much of what is under it it hides.
const OBJECT_VEIL_RADIUS: f32 = 2.0;
const OBJECT_VEIL_OPACITY: f32 = 0.4;

impl LayoutEngine {
  /// The size of each object the input shows, in the order of its text: what its child node takes in `room`,
  /// the width the text is laid out in. `None` for an object no child was built for yet. Empty for an input
  /// without objects.
  ///
  /// A child is first asked how large it is by itself. One no wider than the room is that large. One that is
  /// wider is asked again in the room: where it makes itself fit (a picture keeps its shape), it is as large as
  /// it then says, and where it does not, it is as wide as the room and as high as keeps the shape it had.
  pub(super) fn measure_input_objects(
    &self,
    glyph_engine: &mut GlyphEngine,
    node: &Node,
    state: &TextInputState,
    room: f32,
  ) -> Vec<Option<Size>> {
    let children = node.children();
    let room = room.max(0.0);
    let finite = |size: Size| size.width.is_finite() && size.height.is_finite();
    state
      .object_children()
      .into_iter()
      .map(|child| {
        let child = children.get(child?)?;
        let own = self.layout_node(glyph_engine, child, Constraints::unbounded()).size;
        if finite(own) && own.width <= room {
          return Some(own);
        }
        let fitted = self
          .layout_node(glyph_engine, child, Constraints::loose(Size::new(room, f32::INFINITY)))
          .size;
        if !finite(fitted) {
          return Some(Size::default());
        }
        let kept_its_height = finite(own) && own.width > 0.0 && (fitted.height - own.height).abs() < 0.01;
        Some(if kept_its_height {
          Size::new(fitted.width.min(room), own.height * (room / own.width))
        } else {
          fitted
        })
      })
      .collect()
  }

  /// Places the input's child nodes on the boxes of their objects, once `result` holds the input's box. A child
  /// whose object is gone from the text, or is not laid out, takes no room and is not painted.
  pub(super) fn lay_out_input_objects(&self, glyph_engine: &mut GlyphEngine, node: &Node, result: &mut LayoutResult) {
    let NodeKind::TextInput { state, .. } = node.node_kind() else {
      return;
    };
    let children = node.children();
    if children.is_empty() {
      return;
    }
    let mut placed: Vec<Option<ChildLayout>> = children.iter().map(|_| None).collect();
    if let (Some(layout), Some(objects)) = (state.painted_layout(), state.shown_objects()) {
      let padding = self.resolved_padding_for_size(node, result.size);
      let content_height = (result.size.height - padding.top - padding.bottom).max(0.0);
      let origin = (
        padding.left - state.scroll_x(),
        padding.top + text_input_vertical_offset(state, content_height) - state.scroll_y(),
      );
      for (object, child) in objects.iter().zip(state.object_children()) {
        let Some(index) = child.filter(|index| *index < children.len()) else {
          continue;
        };
        let Some(room) = layout.objects.iter().find(|room| room.byte == object.at) else {
          continue;
        };
        let size = Size::new(room.width, room.height);
        placed[index] = Some(ChildLayout {
          offset: Offset {
            x: origin.0 + room.x,
            y: origin.1 + room.y,
          },
          result: self.layout_node(glyph_engine, &children[index], Constraints::tight(size)),
        });
      }
    }
    result.children = children
      .iter()
      .zip(placed)
      .map(|(child, placed)| {
        placed.unwrap_or_else(|| {
          // Nothing lays it out: the marks that asked for its layout are the input's to consume.
          child.layout_cache.mark_placed_by_parent();
          ChildLayout {
            offset: Offset::default(),
            result: Arc::new(LayoutResult {
              size: Size::default(),
              children: Vec::new(),
              text_layout: None,
              dropped: true,
              holds: SubtreeHolds::UNKNOWN,
              generation: 0,
              sticky_shifts: None,
              reach: crate::layout::layout_result::Reach::UNKNOWN,
            }),
          }
        })
      })
      .collect();
  }

  /// The veil over each object the selection covers, in the selection's color: drawn over the input's children,
  /// which a highlight behind the text would lie under. It is the object's whole sign of the selection (the
  /// highlight of the text leaves its box out), shown as the highlight is, while the input has the focus, and cut
  /// where the text is.
  pub(super) fn push_input_object_veils(
    &self,
    node: &Node,
    result: &LayoutResult,
    origin: (f32, f32),
    transform: Transform2D,
    clip: ClipRect,
    quads: &mut Vec<Quad>,
  ) {
    let NodeKind::TextInput { state, .. } = node.node_kind() else {
      return;
    };
    if !state.is_focused() || state.is_composing() {
      return;
    }
    let (Some((start, end)), Some(layout)) = (state.selection_bounds(), state.painted_layout()) else {
      return;
    };
    let character = crate::components::OBJECT_CHARACTER.len_utf8();
    let mut selected = layout
      .objects
      .iter()
      .filter(|room| start <= room.byte && room.byte + character <= end)
      .peekable();
    if selected.peek().is_none() {
      return;
    }
    let color = {
      let palette = self.palette.borrow();
      node
        .selection_color_value()
        .and_then(|color| color.resolve(&palette))
        .or_else(|| palette.try_get(&PaletteColor::Selection))
        .unwrap_or(DEFAULT_TEXT_SELECTION_COLOR)
        .with_opacity(OBJECT_VEIL_OPACITY)
    };
    let padding = self.resolved_padding_for_size(node, result.size);
    let clip = intersect_clip(
      clip,
      self
        .text_input_viewport(node, state, result, origin, padding)
        .transformed(transform),
    );
    let content_height = (result.size.height - padding.top - padding.bottom).max(0.0);
    let text = (
      origin.0 + padding.left - state.scroll_x(),
      origin.1 + padding.top + text_input_vertical_offset(state, content_height) - state.scroll_y(),
    );
    for room in selected {
      let (width, height) = (room.width, room.height);
      let (x, y, transform, transform_origin) =
        super::transformed_quad_frame(text.0 + room.x, text.1 + room.y, transform);
      quads.push(Quad {
        x,
        y,
        width,
        height,
        opacity: 1.0,
        transform,
        transform_origin,
        content: QuadContent::Rect { color, gradient: None },
        border_radius: Some(BorderRadius::all(OBJECT_VEIL_RADIUS).clamped_to_rect(width, height)),
        border: None,
        clip,
      });
    }
  }
}
