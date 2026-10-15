//! Rich text that paints more than its glyphs: underlined spans, and links
//! that are child nodes of their paragraph ([`crate::node::inline_link`]).
//!
//! Once the paragraph's box is known, its spans are laid out at the width the
//! paint lays them out at, which gives one box per span and laid-out line.
//! The links are placed on the boxes of their spans, and the paragraph paints
//! underlines under the boxes of underlined spans and the links' fills and
//! focus rings around the boxes of their words, so all of them follow
//! wrapping. Such a paragraph's line boxes are centered in its box, as
//! multi-span rich text already was, and painted from the top of the laid-out
//! text, so glyphs and the geometry painted around them agree.

use std::{ops::Range, sync::Arc};

use super::{LayoutEngine, TextLayoutOutput, transformed_quad_frame};
use crate::{
  app::{
    glyph_engine::{GlyphEngine, SpanBox},
    theme::MarkdownLinkState,
  },
  layout::{
    Offset, Size,
    layout_result::{ChildLayout, LayoutResult},
    quad::{ClipRect, Quad, QuadContent, RichTextSpan},
    text_style::TextAlign,
  },
  node::{
    border::{BorderPlacement, BorderRadius, ResolvedBorder, ResolvedBorders},
    color::Color,
    inline_link::InlineLink,
    node::Node,
    node_kind::{NodeKind, TextOverflow},
    transform::Transform2D,
  },
};

/// Where a paragraph's laid-out text lies in its box, for painting around it.
pub(crate) struct InlineTextLayout {
  /// The text's offset from the top of the paragraph's box.
  pub(crate) top: f32,
  pub(crate) text_height: f32,
  pub(crate) span_boxes: Arc<[SpanBox]>,
}

impl InlineTextLayout {
  pub(super) fn estimated_memory_bytes(&self) -> usize {
    std::mem::size_of::<Self>() + self.span_boxes.len() * std::mem::size_of::<SpanBox>()
  }
}

/// A rectangle in the paragraph's coordinates.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct InlineRect {
  pub(crate) x: f32,
  pub(crate) y: f32,
  pub(crate) width: f32,
  pub(crate) height: f32,
}

impl InlineRect {
  fn contains(&self, x: f32, y: f32) -> bool {
    x >= self.x && x <= self.x + self.width && y >= self.y && y <= self.y + self.height
  }

  fn outset(&self, by: f32) -> Self {
    Self {
      x: self.x - by,
      y: self.y - by,
      width: self.width + 2.0 * by,
      height: self.height + 2.0 * by,
    }
  }
}

/// Where a link lies in its paragraph: one rectangle per laid-out line of its
/// words and of its target (its words and what follows them).
pub(crate) struct InlineLinkOutput {
  /// The link node's offset in its paragraph: the top-left of its first line.
  pub(crate) origin: Offset,
  pub(crate) label: Box<[InlineRect]>,
  pub(crate) target: Box<[InlineRect]>,
}

impl InlineLinkOutput {
  /// Whether `(x, y)`, relative to the link node's origin, is on the link.
  pub(crate) fn hits(&self, x: f32, y: f32) -> bool {
    let (x, y) = (x + self.origin.x, y + self.origin.y);
    self.target.iter().any(|rect| rect.contains(x, y))
  }

  pub(super) fn estimated_memory_bytes(&self) -> usize {
    (self.label.len() + self.target.len()) * std::mem::size_of::<InlineRect>()
  }
}

impl LayoutEngine {
  /// Places a rich-text paragraph's links and lays out the geometry its
  /// underlines and links paint with, once `result` holds its box.
  pub(super) fn lay_out_inline_text(&self, glyph_engine: &mut GlyphEngine, node: &Node, result: &mut LayoutResult) {
    let NodeKind::RichText { spans, .. } = node.node_kind() else {
      return;
    };
    if node.children().is_empty() && !spans.iter().any(|span| span.style.underline.is_some()) {
      return;
    }
    let wrap = node.text_wrap && node.text_overflow == TextOverflow::Clip;
    let align = spans.first().map_or(TextAlign::Left, |span| span.style.text_align);
    // The width the paint lays the spans out at (see the runtime's rich-text
    // paint), so the boxes fall where the glyphs are drawn.
    let max_width = if (wrap || align != TextAlign::Left) && result.size.width > 0.0 {
      result.size.width.ceil()
    } else {
      f32::MAX
    };
    let (text_size, span_boxes) = glyph_engine.rich_text_span_boxes(spans, max_width, wrap);
    let top = (result.size.height - text_size.height) * 0.5;
    result.children = node
      .children()
      .iter()
      .map(|child| link_child_layout(child, &span_boxes, top))
      .collect();
    let inline = Arc::new(InlineTextLayout {
      top,
      text_height: text_size.height,
      span_boxes,
    });
    if let Some(TextLayoutOutput::Text(output)) = result.text_layout.as_deref() {
      result.text_layout = Some(Arc::new(TextLayoutOutput::Text(output.with_inline(inline))));
    }
  }

  /// The text's offset from the top of its box and its height, for a
  /// paragraph that paints underlines or links.
  pub(super) fn inline_text_frame(result: &LayoutResult) -> Option<(f32, f32)> {
    inline_layout(result).map(|inline| (inline.top, inline.text_height))
  }

  /// The paragraph's spans as painted: each link's words and trailing runs
  /// take the colors of its hover and focus states.
  pub(super) fn painted_rich_text_spans(node: &Node, spans: &[RichTextSpan]) -> Vec<RichTextSpan> {
    let mut painted = spans.to_vec();
    for (link, state) in link_states(node) {
      for span in painted.get_mut(link.label.clone()).unwrap_or_default() {
        if let Some(color) = state.color {
          span.style.color = color;
        }
        if let (Some(color), Some(underline)) = (state.underline_color, span.style.underline.as_mut()) {
          underline.color = Some(color);
        }
      }
      if let Some(color) = state.trailing_color {
        for span in painted.get_mut(link.trailing()).unwrap_or_default() {
          span.style.color = color;
        }
      }
    }
    painted
  }

  /// The fills behind the words of hovered and focused links.
  pub(super) fn push_inline_link_fills(
    &self,
    node: &Node,
    result: &LayoutResult,
    paint: InlinePaint,
    quads: &mut Vec<Quad>,
  ) {
    for ((link, state), child_layout) in link_states_with_layouts(node, result) {
      let (Some(color), Some(output)) = (state.background, link_output(child_layout)) else {
        continue;
      };
      for rect in &output.label {
        quads.push(paint.rect(*rect, color, Some(link.look.radius), None));
      }
    }
  }

  /// The underlines of underlined spans, drawn in `spans`' painted colors.
  pub(super) fn inline_underlines(result: &LayoutResult, spans: &[RichTextSpan], paint: InlinePaint) -> Vec<Quad> {
    let Some(inline) = inline_layout(result) else {
      return Vec::new();
    };
    inline
      .span_boxes
      .iter()
      .filter_map(|span_box| {
        let style = &spans.get(usize::try_from(span_box.span).ok()?)?.style;
        let underline = style.underline?;
        let thickness = underline
          .thickness
          .filter(|thickness| thickness.is_finite() && *thickness >= 0.0)
          .unwrap_or(span_box.underline_thickness.max(1.0));
        let offset = underline
          .offset
          .filter(|offset| offset.is_finite())
          .unwrap_or(span_box.underline_offset - thickness * 0.5);
        let rect = InlineRect {
          x: span_box.x,
          y: (inline.top + span_box.baseline + offset).round(),
          width: span_box.width,
          height: thickness,
        };
        Some(paint.rect(rect, underline.color.unwrap_or(style.color), None, None))
      })
      .collect()
  }

  /// The focus rings of links with keyboard focus, around each line of their
  /// words. They take the paragraph's clip, not its box, so they show in full.
  pub(super) fn push_inline_link_rings(
    &self,
    node: &Node,
    result: &LayoutResult,
    paint: InlinePaint,
    quads: &mut Vec<Quad>,
  ) {
    for (child, child_layout) in node.children().iter().zip(&result.children) {
      let (NodeKind::InlineLink { link }, Some(output)) = (child.node_kind(), link_output(child_layout)) else {
        continue;
      };
      let Some(ring) = link.look.focus_ring.filter(|_| child.is_style_focus_visible()) else {
        continue;
      };
      let border = ResolvedBorder {
        width: ring.width,
        color: ring.color,
        placement: BorderPlacement::Inside,
      };
      let borders = ResolvedBorders {
        top: Some(border),
        right: Some(border),
        bottom: Some(border),
        left: Some(border),
      };
      for rect in &output.label {
        let outer = rect.outset(ring.offset + ring.width);
        quads.push(paint.rect(
          outer,
          Color::new(0, 0, 0, 0),
          Some(BorderRadius::all(ring.radius)),
          Some(borders),
        ));
      }
    }
  }
}

/// Where and how a paragraph paints around its text.
#[derive(Clone, Copy)]
pub(super) struct InlinePaint {
  /// The paragraph's top-left.
  pub(super) x: f32,
  pub(super) y: f32,
  pub(super) transform: Transform2D,
  pub(super) clip: ClipRect,
  pub(super) opacity: f32,
}

impl InlinePaint {
  fn rect(self, rect: InlineRect, color: Color, radius: Option<BorderRadius>, border: Option<ResolvedBorders>) -> Quad {
    let (x, y, transform, transform_origin) = transformed_quad_frame(self.x + rect.x, self.y + rect.y, self.transform);
    Quad {
      x,
      y,
      width: rect.width,
      height: rect.height,
      opacity: self.opacity,
      transform,
      transform_origin,
      content: QuadContent::Rect { color, gradient: None },
      border_radius: radius,
      border,
      clip: self.clip,
    }
  }
}

fn inline_layout(result: &LayoutResult) -> Option<&InlineTextLayout> {
  match result.text_layout.as_deref()? {
    TextLayoutOutput::Text(output) => output.inline.as_deref(),
    _ => None,
  }
}

fn link_output(child_layout: &ChildLayout) -> Option<&InlineLinkOutput> {
  match child_layout.result.text_layout.as_deref()? {
    TextLayoutOutput::InlineLink(output) => Some(output),
    _ => None,
  }
}

/// The paragraph's links that are hovered or have keyboard focus, with the
/// look of that state.
fn link_states(node: &Node) -> impl Iterator<Item = (&InlineLink, MarkdownLinkState)> {
  node.children().iter().filter_map(|child| {
    let NodeKind::InlineLink { link } = child.node_kind() else {
      return None;
    };
    Some((link, link_states_of(child, link)?))
  })
}

fn link_states_with_layouts<'a>(
  node: &'a Node,
  result: &'a LayoutResult,
) -> impl Iterator<Item = ((&'a InlineLink, MarkdownLinkState), &'a ChildLayout)> {
  node
    .children()
    .iter()
    .zip(&result.children)
    .filter_map(|(child, layout)| {
      let NodeKind::InlineLink { link } = child.node_kind() else {
        return None;
      };
      let state = link_states_of(child, link)?;
      Some(((link, state), layout))
    })
}

/// The look of `child`'s state, focus over hover where both apply; `None` at
/// rest.
fn link_states_of(child: &Node, link: &InlineLink) -> Option<MarkdownLinkState> {
  let hovered = child.is_style_hovered().then_some(link.look.hovered);
  let focused = child.is_style_focus_visible().then_some(link.look.focused);
  match (hovered, focused) {
    (None, None) => None,
    (Some(state), None) | (None, Some(state)) => Some(state),
    (Some(hovered), Some(focused)) => Some(MarkdownLinkState {
      color: focused.color.or(hovered.color),
      underline_color: focused.underline_color.or(hovered.underline_color),
      background: focused.background.or(hovered.background),
      trailing_color: focused.trailing_color.or(hovered.trailing_color),
    }),
  }
}

fn link_child_layout(child: &Node, span_boxes: &[SpanBox], top: f32) -> ChildLayout {
  let output = match child.node_kind() {
    NodeKind::InlineLink { link } => InlineLinkOutput {
      origin: Offset::default(),
      label: line_rects(span_boxes, &link.label, top),
      target: line_rects(span_boxes, &link.target, top),
    },
    _ => InlineLinkOutput {
      origin: Offset::default(),
      label: Box::default(),
      target: Box::default(),
    },
  };
  let first = output
    .label
    .first()
    .or(output.target.first())
    .copied()
    .unwrap_or_default();
  let origin = Offset { x: first.x, y: first.y };
  ChildLayout {
    offset: origin,
    result: Arc::new(LayoutResult {
      size: Size::new(first.width, first.height),
      children: Vec::new(),
      text_layout: Some(Arc::new(TextLayoutOutput::InlineLink(InlineLinkOutput {
        origin,
        ..output
      }))),
      dropped: false,
    }),
  }
}

/// One rectangle per laid-out line covering the boxes of `spans`, shifted
/// down by `top`.
fn line_rects(span_boxes: &[SpanBox], spans: &Range<usize>, top: f32) -> Box<[InlineRect]> {
  let mut rects: Vec<InlineRect> = Vec::new();
  let mut line_top = f32::NAN;
  for span_box in span_boxes
    .iter()
    .filter(|span_box| usize::try_from(span_box.span).is_ok_and(|span| spans.contains(&span)))
  {
    let rect = InlineRect {
      x: span_box.x,
      y: top + span_box.top,
      width: span_box.width,
      height: span_box.height,
    };
    match rects.last_mut() {
      Some(last) if span_box.top == line_top => {
        let right = (last.x + last.width).max(rect.x + rect.width);
        last.x = last.x.min(rect.x);
        last.width = right - last.x;
      }
      _ => rects.push(rect),
    }
    line_top = span_box.top;
  }
  rects.into_boxed_slice()
}
