//! Scrolling an element into view, like the web's `scrollIntoView({ block: "nearest", inline: "nearest" })`: every
//! scroll container around the element moves by the smallest amount that shows it, innermost first, on each axis the
//! container scrolls. An element larger than a viewport is aligned to the viewport start.
//!
//! Tab and Shift+Tab reveal the newly focused element at once, from the last layout. Requests (`Ctx::scroll_into_view`,
//! `Ctx::focus`, [`ElementHandle::scroll_into_view`](super::ElementHandle::scroll_into_view),
//! [`ElementHandle::focus`](super::ElementHandle::focus)) are queued and resolved by the next pass against its fresh
//! layout; when one moves a container, the pass lays out again so the frame it draws already shows the element.

use super::Tree;
use crate::{
  core::{ElementRef as OwnedElementRef, NodeId},
  layout::{
    layout_kind::{LayoutKind, ScrollDirection, ScrollState},
    layout_result::LayoutResult,
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
    let (Some(root), Some(layout)) = (&self.root, &self.last_layout) else {
      return false;
    };
    targets.extend(requested.and_then(|element| node_with_ref(root, &element)));
    let mut scrolled = false;
    for node_id in targets {
      scrolled |= scroll_into_view(root, layout, node_id);
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
  let mut containers = Vec::new();
  let Some(mut target) = find_rect(root, layout, (0.0, 0.0), node_id, &mut containers) else {
    return false;
  };
  let mut scrolled = false;
  for container in containers.iter().rev() {
    let (scrolls_x, scrolls_y) = match container.direction {
      ScrollDirection::Horizontal => (true, false),
      ScrollDirection::Vertical => (false, true),
      ScrollDirection::Both => (true, true),
    };
    let dx = if scrolls_x {
      axis_delta(target.x, target.width, container.viewport.x, container.viewport.width)
    } else {
      0.0
    };
    let dy = if scrolls_y {
      axis_delta(target.y, target.height, container.viewport.y, container.viewport.height)
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
}

fn find_rect<'n>(
  node: &'n Node,
  layout: &LayoutResult,
  origin: (f32, f32),
  node_id: NodeId,
  containers: &mut Vec<ScrollContainer<'n>>,
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
    return Some(rect);
  }
  let scroll = match node.layout_kind() {
    LayoutKind::ScrollModifier { state, direction, .. } => {
      containers.push(ScrollContainer {
        state,
        direction: *direction,
        viewport: rect,
      });
      true
    }
    _ => false,
  };
  for (child, child_layout) in node.children().iter().zip(&layout.children) {
    let child_origin = (origin.0 + child_layout.offset.x, origin.1 + child_layout.offset.y);
    if let Some(found) = find_rect(child, &child_layout.result, child_origin, node_id, containers) {
      return Some(found);
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
