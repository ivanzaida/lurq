//! Scrolling an element into view, like the web's `scrollIntoView({ block: "nearest", inline: "nearest" })`: every
//! scroll container around the element moves by the smallest amount that shows it, innermost first, on each axis the
//! container scrolls. An element larger than a viewport is aligned to the viewport start.
//!
//! Tab and Shift+Tab reveal the newly focused element at once, from the last layout. Requests (`Ctx::scroll_into_view`,
//! `Ctx::focus`, [`ElementHandle::scroll_into_view`](super::ElementHandle::scroll_into_view),
//! [`ElementHandle::focus`](super::ElementHandle::focus)) are queued and resolved by the next pass against its fresh
//! layout; when one moves a container, the pass lays out again so the frame it draws already shows the element.
//!
//! What sticks to an edge of a container (`.sticky(..)`: the header of a table, its first column) covers what
//! scrolls under it, so the element is shown clear of it, as the web's `scroll-padding` would have it: the
//! viewport an element is brought into leaves out the room of what sticks to its edges, unless the element is in
//! what sticks.

use super::Tree;
use crate::{
  core::{ElementRef as OwnedElementRef, NodeId},
  layout::{
    layout_kind::{LayoutKind, ScrollDirection, ScrollState},
    layout_result::LayoutResult,
    sticky_reach::{StickyReach, sticky_reach},
  },
  node::Node,
};

impl Tree {
  /// Queue scrolling the node into view in the next pass.
  pub(super) fn request_scroll_into_view(&mut self, node_id: NodeId) {
    self.pending_scroll_into_view.push(node_id);
  }

  /// Resolve the queued requests against the last layout, in the order they were made. A `Ctx` request for an
  /// element ref the tree does not hold is dropped, like a focus request. Returns whether any container moved.
  pub(super) fn resolve_pending_scroll_into_view(&mut self) -> bool {
    let mut targets = std::mem::take(&mut self.pending_scroll_into_view);
    let requested = self
      .root_ctx
      .as_ref()
      .and_then(|ctx| ctx.take_scroll_into_view_request());
    let range = self
      .root_ctx
      .as_ref()
      .and_then(|ctx| ctx.take_scroll_range_into_view_request());
    let (Some(root), Some(layout)) = (&self.root, &self.last_layout) else {
      return false;
    };
    targets.extend(requested.and_then(|element| node_with_ref(root, &element)));
    let mut scrolled = false;
    for node_id in targets {
      scrolled |= scroll_into_view(root, layout, node_id);
    }
    // A range of a text: the lines it covers, where the text is laid out now.
    if let Some((element, range)) = range
      && let Some(node_id) = node_with_ref(root, &element)
    {
      let engine = &self.layout_engine;
      scrolled |= scroll_part_into_view(root, layout, node_id, |node, result| {
        engine
          .text_range_rect(node, result, range.clone())
          .map(|(x, y, width, height)| Rect { x, y, width, height })
      });
    }
    scrolled
  }
}

/// The node an element ref is attached to.
fn node_with_ref(node: &Node, element: &OwnedElementRef) -> Option<NodeId> {
  if node
    .element_ref
    .as_ref()
    .is_some_and(|current| current.same_handle(element))
  {
    return Some(node.node_id());
  }
  node.children().iter().find_map(|child| node_with_ref(child, element))
}

/// Scrolls every scroll container around `node_id` just enough to show the
/// node, innermost first. Returns whether any container moved.
pub(super) fn scroll_into_view(root: &Node, layout: &LayoutResult, node_id: NodeId) -> bool {
  scroll_part_into_view(root, layout, node_id, |_, _| None)
}

/// Scrolls every scroll container around `node_id` just enough to show the
/// part of the node `part` gives, in the node's box (the lines of a range of
/// a text); the whole node where it gives none. Returns whether any
/// container moved; nothing moves where `part` finds nothing in a node that
/// has it.
fn scroll_part_into_view(
  root: &Node,
  layout: &LayoutResult,
  node_id: NodeId,
  part: impl Fn(&Node, &LayoutResult) -> Option<Rect>,
) -> bool {
  let mut containers = Vec::new();
  let mut found = None;
  let Some(whole) = find_rect(root, layout, (0.0, 0.0), node_id, &mut containers, &mut found) else {
    return false;
  };
  let mut target = match found.and_then(|(node, result)| part(node, result)) {
    Some(inner) => Rect {
      x: whole.x + inner.x,
      y: whole.y + inner.y,
      width: inner.width,
      height: inner.height,
    },
    None => whole,
  };
  let mut scrolled = false;
  for container in containers.iter().rev() {
    let (scrolls_x, scrolls_y) = match container.direction {
      ScrollDirection::Horizontal => (true, false),
      ScrollDirection::Vertical => (false, true),
      ScrollDirection::Both => (true, true),
    };
    let viewport = clear_of_sticky(container, node_id);
    let dx = if scrolls_x {
      axis_delta(target.x, target.width, viewport.x, viewport.width)
    } else {
      0.0
    };
    let dy = if scrolls_y {
      axis_delta(target.y, target.height, viewport.y, viewport.height)
    } else {
      0.0
    };
    if dx == 0.0 && dy == 0.0 {
      continue;
    }
    let state = container.state;
    let (old_x, old_y) = (state.scroll_x(), state.scroll_y());
    state.set_scroll(old_x + dx, old_y + dy);
    let (moved_x, moved_y) = (state.scroll_x() - old_x, state.scroll_y() - old_y);
    if moved_x != 0.0 || moved_y != 0.0 {
      scrolled = true;
      target.x -= moved_x;
      target.y -= moved_y;
    }
  }
  scrolled
}

#[derive(Clone, Copy)]
struct Rect {
  x: f32,
  y: f32,
  width: f32,
  height: f32,
}

struct ScrollContainer<'n> {
  state: &'n ScrollState,
  direction: ScrollDirection,
  viewport: Rect,
  node: &'n Node,
  layout: &'n LayoutResult,
}

/// The viewport of `container` less the room what sticks to its edges takes, where the node `target` is not in
/// what sticks: how far in from each edge a sticky element reaches, its inset and its size.
fn clear_of_sticky(container: &ScrollContainer<'_>, target: NodeId) -> Rect {
  let mut reach = StickyReach::default();
  if container.layout.holds.sticky
    && let (Some(content), Some(content_layout)) =
      (container.node.children().first(), container.layout.children.first())
  {
    reach = sticky_reach(content, &content_layout.result, Some(target));
  }
  let viewport = container.viewport;
  let width = (viewport.width - reach.left - reach.right).max(0.0);
  let height = (viewport.height - reach.top - reach.bottom).max(0.0);
  if width <= 0.0 || height <= 0.0 {
    return viewport;
  }
  Rect {
    x: viewport.x + reach.left,
    y: viewport.y + reach.top,
    width,
    height,
  }
}

fn find_rect<'n>(
  node: &'n Node,
  layout: &'n LayoutResult,
  origin: (f32, f32),
  node_id: NodeId,
  containers: &mut Vec<ScrollContainer<'n>>,
  found: &mut Option<(&'n Node, &'n LayoutResult)>,
) -> Option<Rect> {
  // A dropped subtree is not drawn: there is nothing to scroll to.
  if layout.dropped {
    return None;
  }
  let rect = Rect {
    x: origin.0,
    y: origin.1,
    width: layout.size.width,
    height: layout.size.height,
  };
  if node.node_id() == node_id {
    *found = Some((node, layout));
    return Some(rect);
  }
  let scroll = match node.layout_kind() {
    LayoutKind::ScrollModifier { state, direction, .. } => {
      containers.push(ScrollContainer {
        state,
        direction: *direction,
        viewport: rect,
        node,
        layout,
      });
      true
    }
    _ => false,
  };
  for (child, child_layout) in node.children().iter().zip(&layout.children) {
    let child_origin = (origin.0 + child_layout.offset.x, origin.1 + child_layout.offset.y);
    if let Some(rect) = find_rect(child, &child_layout.result, child_origin, node_id, containers, found) {
      return Some(rect);
    }
  }
  if scroll {
    containers.pop();
  }
  None
}

/// Smallest scroll along one axis that brings `[start, start + size]` into
/// `[view, view + view_size]`; aligns the start when the node is larger.
fn axis_delta(start: f32, size: f32, view: f32, view_size: f32) -> f32 {
  if start < view || size > view_size {
    start - view
  } else if start + size > view + view_size {
    start + size - (view + view_size)
  } else {
    0.0
  }
}
