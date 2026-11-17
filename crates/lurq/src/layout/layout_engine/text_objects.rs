//! The objects of a rich-text paragraph: elements that stand in its lines (`Text::object`).
//!
//! An object is a span of the paragraph that is one [`OBJECT_CHARACTER`], and a child node of the paragraph, the
//! element the app gave for it: the paragraph's objects and its children that are no links are paired in order.
//! Before the text is laid out each child is measured in the room the paragraph has, and its span is made as wide
//! as the child by spacing its one glyph, which draws nothing: the line breaks around it as around a word, and
//! keeps the height of its text. Once the paragraph's box is known the children are laid out at their size and
//! placed on their spans, each on the middle of its line ([`super::inline_links`], which places a paragraph's
//! links, places its objects too).

use super::LayoutEngine;
use crate::{
  app::glyph_engine::GlyphEngine,
  components::OBJECT_CHARACTER,
  layout::{Constraints, Size, quad::RichTextSpan},
  node::{color::Color, node::Node, node_kind::NodeKind},
};

/// An object of a paragraph: its span, the child drawn for it, and the child's size.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct TextObject {
  pub(crate) span: usize,
  pub(crate) child: usize,
  pub(crate) size: Size,
}

/// The spans of `spans` that are objects, each with the index of the child drawn for it: the spans that are one
/// [`OBJECT_CHARACTER`] and the children that are no links, in order. A span left without a child is text.
pub(crate) fn object_slots(node: &Node, spans: &[RichTextSpan]) -> Vec<(usize, usize)> {
  let mut children = node
    .children()
    .iter()
    .enumerate()
    .filter(|(_, child)| !matches!(child.node_kind(), NodeKind::InlineLink { .. }))
    .map(|(index, _)| index);
  let mut object = [0u8; 4];
  let object: &str = OBJECT_CHARACTER.encode_utf8(&mut object);
  spans
    .iter()
    .enumerate()
    .filter(|(_, span)| span.text == object)
    .map_while(|(span, _)| Some((span, children.next()?)))
    .collect()
}

impl LayoutEngine {
  /// The objects of `node`, measured in `room`, the width its text is laid out in. Empty for a paragraph with none.
  ///
  /// A child no wider than the room is as large as it is by itself; one that is wider is asked again in the room,
  /// and is as large as it then says, no wider than the room.
  pub(super) fn measure_text_objects(
    &self,
    glyph_engine: &mut GlyphEngine,
    node: &Node,
    spans: &[RichTextSpan],
    room: f32,
  ) -> Vec<TextObject> {
    let slots = object_slots(node, spans);
    if slots.is_empty() {
      return Vec::new();
    }
    let children = node.children();
    let finite = |size: Size| size.width.is_finite() && size.height.is_finite();
    slots
      .into_iter()
      .map(|(span, child)| {
        let node = &children[child];
        let own = self.layout_node(glyph_engine, node, Constraints::unbounded()).size;
        let size = if finite(own) && (own.width <= room || !room.is_finite()) {
          own
        } else {
          let fitted = self
            .layout_node(
              glyph_engine,
              node,
              Constraints::loose(Size::new(room.max(0.0), f32::INFINITY)),
            )
            .size;
          if finite(fitted) {
            Size::new(fitted.width.min(room.max(0.0)), fitted.height)
          } else {
            Size::default()
          }
        };
        TextObject { span, child, size }
      })
      .collect()
  }

  /// `spans` with the span of each of `objects` as wide as its child: its one glyph spaced by what it lacks of the
  /// child's width, and drawn in no color, with no line under or through it.
  pub(super) fn spans_with_objects(
    glyph_engine: &mut GlyphEngine,
    spans: &[RichTextSpan],
    objects: &[TextObject],
  ) -> Vec<RichTextSpan> {
    let mut widened = spans.to_vec();
    for object in objects {
      let Some(span) = widened.get_mut(object.span) else {
        continue;
      };
      span.style.letter_spacing = 0.0;
      span.style.underline = None;
      span.style.strikethrough = None;
      span.style.shadow = None;
      span.style.color = Color::new(0, 0, 0, 0);
      let advance = glyph_engine
        .measure_rich_text(std::slice::from_ref(span), 1.0, f32::MAX)
        .width;
      span.style.letter_spacing = object.size.width - advance;
    }
    widened
  }

  /// `spans` with the objects of `node` measured in `room` given their width: `None` for a paragraph with none.
  pub(super) fn object_spans(
    &self,
    glyph_engine: &mut GlyphEngine,
    node: &Node,
    spans: &[RichTextSpan],
    room: f32,
  ) -> Option<(Vec<RichTextSpan>, Vec<TextObject>)> {
    let objects = self.measure_text_objects(glyph_engine, node, spans, room);
    if objects.is_empty() {
      return None;
    }
    Some((Self::spans_with_objects(glyph_engine, spans, &objects), objects))
  }
}
