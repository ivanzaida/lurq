//! Tab / Shift+Tab traversal and scrolling the focused node into view.
//!
//! The Tab scope is the topmost open modal, or the whole window when no modal is open, and Tab wraps around at its
//! ends. Only nodes with an explicit `tab_index >= 0` are stops, except inside forms, where controls are stops without
//! one. A form is part of its scope's order like in a browser: Tab from its last control moves on to the next stop.
//!
//! An open modal traps Tab: focus behind it is never a stop, and Tab stays put when the modal has no stops.
//!
//! A child its Row/Column dropped to make room (`ShrinkLimit::Drop`) is not drawn, so nothing inside it is a stop.

use super::{FocusTarget, find_node_by_path};
use crate::{
  core::NodeId,
  layout::{layout_kind::LayoutKind, layout_result::LayoutResult},
  node::{Node, SyntheticNodeRole},
};

pub(super) enum TabMove {
  Focus(FocusTarget),
  /// An open modal has no stops: Tab is consumed and focus stays.
  Stay,
  /// Nothing to traverse; the key falls through to other handlers.
  Unhandled,
}

/// Sort key of a stop: positive tab indices first in ascending order, then
/// `0` (and unset form controls) in tree order.
type StopKey = (u8, i32, usize);

struct Stop {
  target: FocusTarget,
  key: StopKey,
}

struct Walk {
  focused: Option<NodeId>,
  focused_order: Option<usize>,
  visited: usize,
  stops: Vec<Stop>,
}

/// `layout` is the last layout of `root`, if any; it tells which subtrees
/// were dropped.
pub(super) fn tab_move(
  root: &Node,
  layout: Option<&LayoutResult>,
  focused: Option<(NodeId, &[usize])>,
  reverse: bool,
) -> TabMove {
  let modal = top_modal_path(root);
  let focused = focused.filter(|(_, path)| modal.as_deref().is_none_or(|modal| path.starts_with(modal)));

  let (scope, scope_layout) = match &modal {
    Some(path) => (
      find_node_by_path(root, path),
      layout.and_then(|layout| layout_at_path(layout, path)),
    ),
    None => (Some(root), layout),
  };
  match scope.and_then(|scope| next_stop(scope, scope_layout, focused.map(|(id, _)| id), reverse)) {
    Some(target) => TabMove::Focus(target),
    None if modal.is_some() => TabMove::Stay,
    None => TabMove::Unhandled,
  }
}

/// Path of the topmost open modal. Modals are direct children of the overlay
/// host, after the base tree, in stacking order. `WindowChrome`'s layer is
/// built like a modal but has no modal role, so it is never the scope.
fn top_modal_path(root: &Node) -> Option<Vec<usize>> {
  if !root.has_synthetic_role(SyntheticNodeRole::OverlayHost) {
    return None;
  }
  root
    .children()
    .iter()
    .rposition(|child| child.has_synthetic_role(SyntheticNodeRole::Modal))
    .map(|index| vec![index])
}

/// The layout of the node at `path` (child indices from the root), if the
/// layout reaches that deep.
fn layout_at_path<'l>(layout: &'l LayoutResult, path: &[usize]) -> Option<&'l LayoutResult> {
  path.iter().try_fold(layout, |layout, &index| {
    layout.children.get(index).map(|child| child.result.as_ref())
  })
}

/// Whether the node at `path` lies in a subtree its Row/Column dropped.
pub(super) fn path_is_dropped(layout: &LayoutResult, path: &[usize]) -> bool {
  let mut current = layout;
  for &index in path {
    let Some(child) = current.children.get(index) else {
      return false;
    };
    current = &child.result;
    if current.dropped {
      return true;
    }
  }
  false
}

fn next_stop(
  scope: &Node,
  layout: Option<&LayoutResult>,
  focused: Option<NodeId>,
  reverse: bool,
) -> Option<FocusTarget> {
  let mut walk = Walk {
    focused,
    focused_order: None,
    visited: 0,
    stops: Vec::new(),
  };
  collect_stops(scope, layout, None, false, &mut walk);
  let stops = &mut walk.stops;
  if stops.is_empty() {
    return None;
  }
  stops.sort_by_key(|stop| stop.key);

  let last = stops.len() - 1;
  let current = focused.and_then(|id| stops.iter().position(|stop| stop.target.input_id == id));
  let next = match (current, walk.focused_order) {
    (Some(index), _) if reverse => index.checked_sub(1).unwrap_or(last),
    (Some(index), _) => (index + 1) % stops.len(),
    // Focus is on a node that is not a stop (a clicked button without a tab
    // index): continue from its place in tree order, like a browser.
    (None, Some(order)) => {
      let key = stop_key(0, order);
      if reverse {
        stops.iter().rposition(|stop| stop.key < key).unwrap_or(last)
      } else {
        stops.iter().position(|stop| stop.key > key).unwrap_or(0)
      }
    }
    (None, None) if reverse => last,
    (None, None) => 0,
  };
  Some(stops[next].target)
}

fn collect_stops(
  node: &Node,
  layout: Option<&LayoutResult>,
  focus_event_id: Option<NodeId>,
  in_form: bool,
  walk: &mut Walk,
) {
  if layout.is_some_and(|layout| layout.dropped) {
    return;
  }
  let order = walk.visited;
  walk.visited += 1;
  if walk.focused == Some(node.node_id()) {
    walk.focused_order = Some(order);
  }
  let focus_event_id = if !node.events.on_focus.is_empty() || !node.events.on_blur.is_empty() {
    Some(node.node_id())
  } else {
    focus_event_id
  };
  let in_form = in_form || is_form(node);
  let tab_index = node.tab_index_value().or(in_form.then_some(0));
  if let Some(tab_index) = tab_index
    && tab_index >= 0
    && node.is_focusable()
  {
    walk.stops.push(Stop {
      target: FocusTarget {
        input_id: node.node_id(),
        event_id: focus_event_id.unwrap_or_else(|| node.node_id()),
      },
      key: stop_key(tab_index, order),
    });
    // A button's content is part of the button, not separate stops.
    if node.button_kind_value().is_some() {
      return;
    }
  }

  for (index, child) in node.children().iter().enumerate() {
    let child_layout = layout.and_then(|layout| layout.children.get(index));
    collect_stops(
      child,
      child_layout.map(|child| child.result.as_ref()),
      focus_event_id,
      in_form,
      walk,
    );
  }
}

fn stop_key(tab_index: i32, order: usize) -> StopKey {
  if tab_index > 0 {
    (0, tab_index, order)
  } else {
    (1, 0, order)
  }
}

#[cfg(feature = "form")]
fn is_form(node: &Node) -> bool {
  node.events.on_submit.is_some()
}

#[cfg(not(feature = "form"))]
fn is_form(_: &Node) -> bool {
  false
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
    let dx = axis_delta(target.x, target.width, container.viewport.x, container.viewport.width);
    let dy = axis_delta(target.y, target.height, container.viewport.y, container.viewport.height);
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
  state: &'n crate::layout::layout_kind::ScrollState,
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
    LayoutKind::ScrollModifier { state, .. } => {
      containers.push(ScrollContainer { state, viewport: rect });
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
