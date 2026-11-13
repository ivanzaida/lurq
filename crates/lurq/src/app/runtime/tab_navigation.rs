//! Tab / Shift+Tab traversal. Revealing the newly focused node is [`super::scroll_into_view`].
//!
//! The Tab scope is the topmost open modal, or the whole window when no modal is open, and Tab wraps around at its
//! ends. Only nodes with an explicit `tab_index >= 0` are stops, except inside forms, where controls are stops without
//! one. A form is part of its scope's order like in a browser: Tab from its last control moves on to the next stop.
//!
//! An open modal traps Tab: focus behind it is never a stop, and Tab stays put when the modal has no stops.
//!
//! A child its Row/Column dropped to make room (`ShrinkLimit::Drop`) is not drawn, so nothing inside it is a stop.
//!
//! The stops of an overlay that takes the focus (`Overlay::takes_focus`: a popover, a menu) come right after its
//! anchor's, as a web popover's come after its invoker's: Tab from the trigger goes into it, and Tab from its last
//! stop goes on to what follows the trigger. Where the anchor is in a focus group Tab stops in once, they come
//! after the group.

use super::{FocusTarget, find_node_by_path};
use crate::{
  core::{ElementRef as OwnedElementRef, NodeId},
  layout::layout_result::LayoutResult,
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
  /// Where Tab enters the one-stop focus group the stop is in.
  entry: bool,
}

struct Walk<'a> {
  focused: Option<NodeId>,
  focused_order: Option<usize>,
  visited: usize,
  stops: Vec<Stop>,
  /// The layers of the overlays that move the focus, each with its anchor: walked after the anchor, not where
  /// the host has them.
  moved: &'a [(OwnedElementRef, &'a Node)],
  /// Which of `moved` were walked.
  walked: Vec<NodeId>,
  /// How many focus groups that are one stop the walk is in: a layer met in one waits for it to end.
  in_one_stop: usize,
  /// The layers met in a focus group that is one stop, walked when the group ends.
  pending: Vec<&'a Node>,
}

/// `dropped` holds the nodes the last layout dropped from their Row/Column
/// (see [`dropped_node_ids`]); nothing inside them is a stop. `moved` holds
/// the layers of the overlays that move the focus, with their anchors
/// ([`super::Tree::focus_moving_layers`]).
pub(super) fn tab_move(
  root: &Node,
  dropped: &[NodeId],
  moved: &[(OwnedElementRef, &Node)],
  focused: Option<(NodeId, &[usize])>,
  reverse: bool,
) -> TabMove {
  let modal = top_modal_path(root);
  let focused = focused.filter(|(_, path)| modal.as_deref().is_none_or(|modal| path.starts_with(modal)));

  let scope = match &modal {
    Some(path) => find_node_by_path(root, path),
    None => Some(root),
  };
  match scope.and_then(|scope| next_stop(scope, dropped, moved, focused.map(|(id, _)| id), reverse)) {
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

/// The nodes `layout` (the layout of `root`) dropped from their Row/Column,
/// outermost only: what lies inside them is dropped with them.
pub(super) fn dropped_node_ids(root: &Node, layout: &LayoutResult) -> Vec<NodeId> {
  fn collect(node: &Node, layout: &LayoutResult, dropped: &mut Vec<NodeId>) {
    if layout.dropped {
      dropped.push(node.node_id());
      return;
    }
    for (child, child_layout) in node.children().iter().zip(&layout.children) {
      collect(child, &child_layout.result, dropped);
    }
  }
  let mut dropped = Vec::new();
  collect(root, layout, &mut dropped);
  dropped
}

/// Whether the node at `path` (child indices from `root`) is one of the
/// `dropped` nodes or lies inside one.
/// Whether the node at `path`, or an element it is in, is disabled.
pub(super) fn path_is_disabled(root: &Node, path: &[usize]) -> bool {
  let mut node = root;
  if node.is_disabled() {
    return true;
  }
  for &index in path {
    let Some(child) = node.children().get(index) else {
      return false;
    };
    node = child;
    if node.is_disabled() {
      return true;
    }
  }
  false
}

pub(super) fn path_is_dropped(root: &Node, dropped: &[NodeId], path: &[usize]) -> bool {
  if dropped.is_empty() {
    return false;
  }
  let mut node = root;
  for &index in path {
    let Some(child) = node.children().get(index) else {
      return false;
    };
    node = child;
    if dropped.contains(&node.node_id()) {
      return true;
    }
  }
  false
}

fn next_stop<'a>(
  scope: &'a Node,
  dropped: &[NodeId],
  moved: &'a [(OwnedElementRef, &'a Node)],
  focused: Option<NodeId>,
  reverse: bool,
) -> Option<FocusTarget> {
  let mut walk = Walk {
    focused,
    focused_order: None,
    visited: 0,
    stops: Vec::new(),
    moved,
    walked: Vec::new(),
    in_one_stop: 0,
    pending: Vec::new(),
  };
  collect_stops(scope, dropped, None, false, &mut walk);
  // A layer whose anchor the walk did not reach (in a disabled element) is where the host has it: after the
  // page. Only a layer of this scope: a modal's holds none.
  for (_, layer) in moved {
    if !walk.walked.contains(&layer.node_id()) && scope.children().iter().any(|child| std::ptr::eq(child, *layer)) {
      walk.walked.push(layer.node_id());
      collect_node(layer, dropped, None, false, &mut walk);
    }
  }
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

fn collect_stops<'a>(
  node: &'a Node,
  dropped: &[NodeId],
  focus_event_id: Option<NodeId>,
  in_form: bool,
  walk: &mut Walk<'a>,
) {
  // A layer that moves the focus is walked after its anchor.
  if walk.moved.iter().any(|(_, layer)| std::ptr::eq(*layer, node)) {
    return;
  }
  collect_node(node, dropped, focus_event_id, in_form, walk);
  let Some(anchor) = &node.element_ref else {
    return;
  };
  let moved = walk.moved;
  for (_, layer) in moved.iter().filter(|(owner, _)| owner.same_handle(anchor)) {
    if walk.walked.contains(&layer.node_id()) {
      continue;
    }
    walk.walked.push(layer.node_id());
    if walk.in_one_stop > 0 {
      walk.pending.push(layer);
    } else {
      collect_node(layer, dropped, None, false, walk);
    }
  }
}

/// The stops in `node` and under it, where the walk is at it.
fn collect_node<'a>(
  node: &'a Node,
  dropped: &[NodeId],
  focus_event_id: Option<NodeId>,
  in_form: bool,
  walk: &mut Walk<'a>,
) {
  // Nothing in a disabled element takes focus, so Tab passes all of it by.
  // Nor in a layer that is on its way out, which is only painted.
  if dropped.contains(&node.node_id()) || node.is_disabled() || node.is_leaving_layer() {
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
      entry: node.is_focus_entry(),
    });
    // A button's content is part of the button, not separate stops, unless the button says otherwise.
    if node.button_kind_value().is_some() && !node.has_stops_inside() {
      return;
    }
  }

  let first_inside = walk.stops.len();
  let one_stop = node.focus_group_value().is_some_and(|group| group.one_stop);
  if one_stop {
    walk.in_one_stop += 1;
  }
  for child in node.children() {
    collect_stops(child, dropped, focus_event_id, in_form, walk);
  }
  if one_stop {
    walk.in_one_stop -= 1;
  }
  // A group that is one stop keeps one of the stops in it: the focused one, so that Tab from inside leaves
  // the group; or the one marked as its entry; or its first.
  if one_stop && walk.stops.len() > first_inside + 1 {
    let inside = &walk.stops[first_inside..];
    let kept = inside
      .iter()
      .position(|stop| walk.focused == Some(stop.target.input_id))
      .or_else(|| inside.iter().position(|stop| stop.entry))
      .unwrap_or(0);
    let stop = walk.stops.swap_remove(first_inside + kept);
    walk.stops.truncate(first_inside);
    walk.stops.push(stop);
  }
  // The layers whose anchors are in the group come after it, once it is done.
  if one_stop && walk.in_one_stop == 0 {
    for layer in std::mem::take(&mut walk.pending) {
      collect_node(layer, dropped, None, false, walk);
    }
  }
}

/// What the arrow keys move between in the focus group `group`: every stop of Tab inside it, in the order
/// of the tree, whether or not Tab stops in the group once. What is in a group of no keys inside it
/// (`FocusGroup::none`) is not among them.
pub(super) fn group_stops(group: &Node, dropped: &[NodeId]) -> Vec<FocusTarget> {
  group_stops_around(group, dropped, None).0
}

/// The stops of [`group_stops`], and how many of them are before the node `around` in the order of the
/// tree, which may be disabled and so not a stop itself: `None` when it is not in the group.
pub(super) fn group_stops_around(
  group: &Node,
  dropped: &[NodeId],
  around: Option<NodeId>,
) -> (Vec<FocusTarget>, Option<usize>) {
  struct Walk {
    stops: Vec<FocusTarget>,
    around: Option<NodeId>,
    before: Option<usize>,
  }
  fn collect(
    node: &Node,
    root: bool,
    dropped: &[NodeId],
    focus_event_id: Option<NodeId>,
    in_form: bool,
    walk: &mut Walk,
  ) {
    if walk.around == Some(node.node_id()) {
      walk.before = Some(walk.stops.len());
    }
    if dropped.contains(&node.node_id()) || node.is_disabled() || node.is_leaving_layer() {
      return;
    }
    if !root && node.focus_group_value().is_some_and(|group| group.is_none()) {
      return;
    }
    let focus_event_id = if !node.events.on_focus.is_empty() || !node.events.on_blur.is_empty() {
      Some(node.node_id())
    } else {
      focus_event_id
    };
    let in_form = in_form || is_form(node);
    if node
      .tab_index_value()
      .or(in_form.then_some(0))
      .is_some_and(|index| index >= 0)
      && node.is_focusable()
    {
      walk.stops.push(FocusTarget {
        input_id: node.node_id(),
        event_id: focus_event_id.unwrap_or_else(|| node.node_id()),
      });
      if node.button_kind_value().is_some() && !node.has_stops_inside() {
        return;
      }
    }
    for child in node.children() {
      collect(child, false, dropped, focus_event_id, in_form, walk);
    }
  }
  let mut walk = Walk {
    stops: Vec::new(),
    around,
    before: None,
  };
  collect(group, true, dropped, None, false, &mut walk);
  (walk.stops, walk.before)
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
