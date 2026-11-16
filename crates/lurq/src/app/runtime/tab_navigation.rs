//! Tab / Shift+Tab traversal. Revealing the newly focused node is [`super::scroll_into_view`].
//!
//! The Tab scope is the topmost open modal, or the whole window when no modal is open, and Tab wraps around at its
//! ends. Only nodes with an explicit `tab_index >= 0` are stops, except inside forms, where controls are stops without
//! one. A form is part of its scope's order like in a browser: Tab from its last control moves on to the next stop.
//!
//! An open modal traps Tab: focus behind it is never a stop, and Tab stays put when the modal has no stops.
//!
//! A child its Row/Column dropped to make room (`ShrinkLimit::Drop`) is not drawn, so nothing inside it is a stop. Nor
//! is anything inside an inert element (`.inert(true)`).
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

/// Where Tab goes on from while nothing has the focus, the sequential focus navigation starting point of a
/// browser: see [`FocusStart`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum StartPoint {
  /// On the node: Tab goes to the first stop after it, Shift+Tab to the last stop before it.
  On(NodeId),
  /// Just before the node: the one that took the place of what was there, or an element Tab passes by
  /// whole (disabled, inert, dropped) that holds it.
  Before(NodeId),
  /// Just after the node and all it holds: what was its last child has gone.
  After(NodeId),
}

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
  /// Where the focus goes as a modal opens, in place of its first stop (`autofocus`).
  autofocus: bool,
  /// The stop the stop is in: a button that has stops inside it (`stops_inside`), a row of a table holding its
  /// own buttons.
  owner: Option<NodeId>,
  /// The depth of the one-stop focus group the stop is out of, where it is in a `FocusGroup::none()` in that group:
  /// the new tab button beside the tabs. That group keeps it as a stop of its own.
  apart: Option<usize>,
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
  /// The stop the walk is in, where it is in a button that has stops inside it.
  owner: Option<NodeId>,
  /// The depth of the one-stop focus group the walk is out of, in a `FocusGroup::none()` in it.
  apart: Option<usize>,
  /// Where Tab goes on from while nothing has the focus.
  start: Option<StartPoint>,
  /// The orders `start` is at, where the walk met it: the first a stop has to have for Tab to go to it, and
  /// the one a stop has to be under for Shift+Tab.
  start_bounds: Option<(usize, usize)>,
}

/// `dropped` holds the nodes the last layout dropped from their Row/Column
/// (see [`dropped_node_ids`]); nothing inside them is a stop. `moved` holds
/// the layers of the overlays that move the focus, with their anchors
/// ([`super::Tree::focus_moving_layers`]). `start` is where Tab goes on from
/// when nothing has the focus.
pub(super) fn tab_move(
  root: &Node,
  dropped: &[NodeId],
  moved: &[(OwnedElementRef, &Node)],
  focused: Option<(NodeId, &[usize])>,
  start: Option<StartPoint>,
  reverse: bool,
) -> TabMove {
  let modal = top_modal_path(root);
  let focused = focused.filter(|(_, path)| modal.as_deref().is_none_or(|modal| path.starts_with(modal)));

  let scope = match &modal {
    Some(path) => find_node_by_path(root, path),
    None => Some(root),
  };
  match scope.and_then(|scope| next_stop(scope, dropped, moved, focused.map(|(id, _)| id), start, reverse)) {
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
      // Only a child its line may drop is ever dropped: a subtree with none has nothing to collect.
      if child.layout_cache.subtree().droppable {
        collect(child, &child_layout.result, dropped);
      }
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

/// Whether the node at `path`, or an element it is in, is inert.
pub(super) fn path_is_inert(root: &Node, path: &[usize]) -> bool {
  let mut node = root;
  if node.is_inert() {
    return true;
  }
  for &index in path {
    let Some(child) = node.children().get(index) else {
      return false;
    };
    node = child;
    if node.is_inert() {
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
  start: Option<StartPoint>,
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
    owner: None,
    apart: None,
    start: if focused.is_none() { start } else { None },
    start_bounds: None,
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
  let start = walk.start_bounds;
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
    // The focus was dropped (a key blurred it, what had it left the tree or was disabled) or a press on what takes
    // none put the place there: Tab goes on from it, as a browser's does from its starting point.
    (None, None) => match start {
      Some((_, back)) if reverse => stops
        .iter()
        .rposition(|stop| stop.key < stop_key(0, back))
        .unwrap_or(last),
      Some((forward, _)) => stops
        .iter()
        .position(|stop| stop.key >= stop_key(0, forward))
        .unwrap_or(0),
      None if reverse => last,
      // Nothing has the focus: a modal opening, or Tab in a window with no focus. A stop marked to take it first.
      None => stops.iter().position(|stop| stop.autofocus).unwrap_or(0),
    },
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

/// The stops in `node` and under it, where the walk is at it, and the orders of the starting point where it is
/// at `node`.
fn collect_node<'a>(
  node: &'a Node,
  dropped: &[NodeId],
  focus_event_id: Option<NodeId>,
  in_form: bool,
  walk: &mut Walk<'a>,
) {
  let id = node.node_id();
  let before = walk.visited;
  if walk.start == Some(StartPoint::Before(id)) {
    walk.start_bounds = Some((before, before));
  }
  visit_node(node, dropped, focus_event_id, in_form, walk);
  if walk.start == Some(StartPoint::On(id)) {
    // Its own order is `before` where the walk counted it, a stop or not.
    walk.start_bounds = Some(if walk.visited > before {
      (before + 1, before)
    } else {
      (before, before)
    });
  }
  if walk.start == Some(StartPoint::After(id)) {
    walk.start_bounds = Some((walk.visited, walk.visited));
  }
}

fn visit_node<'a>(
  node: &'a Node,
  dropped: &[NodeId],
  focus_event_id: Option<NodeId>,
  in_form: bool,
  walk: &mut Walk<'a>,
) {
  // Nothing in a disabled element takes focus, so Tab passes all of it by.
  // Nor in a layer that is on its way out, which is only painted.
  if dropped.contains(&node.node_id()) || node.is_disabled() || node.is_inert() || node.is_leaving_layer() {
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
      autofocus: node.is_autofocus(),
      owner: walk.owner,
      apart: walk.apart,
    });
    // A button's content is part of the button, not separate stops, unless the button says otherwise.
    if node.button_kind_value().is_some() && !node.has_stops_inside() {
      return;
    }
  }
  let owner = walk.owner;
  if tab_index.is_some_and(|index| index >= 0) && node.is_focusable() && node.has_stops_inside() {
    walk.owner = Some(node.node_id());
  }

  let first_inside = walk.stops.len();
  let one_stop = node.focus_group_value().is_some_and(|group| group.one_stop);
  if one_stop {
    walk.in_one_stop += 1;
  }
  // What is in a group of none inside a one-stop group is out of that group: its stops are its own.
  let apart = walk.apart;
  if walk.in_one_stop > 0 && node.focus_group_value().is_some_and(|group| group.is_none()) {
    walk.apart = Some(walk.in_one_stop);
  }
  for child in node.children() {
    collect_stops(child, dropped, focus_event_id, in_form, walk);
  }
  walk.apart = apart;
  walk.owner = owner;
  let depth = walk.in_one_stop;
  if one_stop {
    walk.in_one_stop -= 1;
  }
  // A group that is one stop keeps one of the stops in it: the focused one, so that Tab from inside leaves
  // the group; or the one marked as its entry; or its first. Where the focus is on a stop inside another (a
  // button of a row of a table), that other is the one kept. The stops inside the kept one stay after it: Tab
  // goes from a row into what it holds, and on out of the group.
  if one_stop && walk.stops.len() > first_inside + 1 {
    let inside = &walk.stops[first_inside..];
    let is_inside = |id: NodeId| inside.iter().any(|stop| stop.target.input_id == id);
    // Out of the group is what a group of none in it holds beside its stops (the new tab button beside the tabs),
    // not what is in a stop of the group (a button in a cell of a row): that stays with the stop it is in.
    let apart_here = |stop: &Stop| stop.apart == Some(depth) && !stop.owner.is_some_and(is_inside);
    let ours = || inside.iter().filter(|stop| !apart_here(stop));
    let kept = ours()
      .find(|stop| walk.focused == Some(stop.target.input_id))
      .map(|stop| {
        stop
          .owner
          .filter(|owner| is_inside(*owner))
          .unwrap_or(stop.target.input_id)
      })
      .or_else(|| ours().find(|stop| stop.entry).map(|stop| stop.target.input_id))
      .or_else(|| ours().next().map(|stop| stop.target.input_id));
    let apart: Vec<bool> = inside.iter().map(apart_here).collect();
    let held: Vec<Stop> = walk
      .stops
      .drain(first_inside..)
      .zip(apart)
      .filter(|(stop, apart)| {
        *apart || kept.is_some_and(|kept| stop.target.input_id == kept || stop.owner == Some(kept))
      })
      .map(|(mut stop, _)| {
        // Out of this group, or in it, it is in the groups around it as any stop is.
        if stop.apart == Some(depth) {
          stop.apart = None;
        }
        stop
      })
      .collect();
    walk.stops.extend(held);
  }
  // The layers whose anchors are in the group come after it, once it is done.
  if one_stop && walk.in_one_stop == 0 {
    for layer in std::mem::take(&mut walk.pending) {
      collect_node(layer, dropped, None, false, walk);
    }
  }
}

/// The place Tab goes on from while nothing has the focus: the node the focus was dropped from, or the one a press
/// that took no focus was on, with its path (child indices from the root) then. The root wraps the page in an
/// overlay host while a layer is open and not otherwise, so the path says which it was from.
#[derive(Clone, Debug)]
pub(super) struct FocusStart {
  node: NodeId,
  path: Vec<usize>,
  hosted: bool,
}

impl FocusStart {
  pub(super) fn new(root: &Node, node: NodeId, path: Vec<usize>) -> Self {
    Self {
      node,
      path,
      hosted: root.has_synthetic_role(SyntheticNodeRole::OverlayHost),
    }
  }

  /// Where Tab goes on from now: the node where it still is, else what took its place at its path, else the end
  /// of the deepest element of the path still there. A node inside an element Tab passes by whole (disabled,
  /// inert, dropped, leaving) is where that element is.
  pub(super) fn point(&self, root: &Node, dropped: &[NodeId]) -> StartPoint {
    let passed = |node: &Node| {
      dropped.contains(&node.node_id()) || node.is_disabled() || node.is_inert() || node.is_leaving_layer()
    };
    let (path, found) = match super::find_path_by_id(root, self.node) {
      Some(path) => (path, true),
      None => {
        // The page is the overlay host's first child.
        let hosted = root.has_synthetic_role(SyntheticNodeRole::OverlayHost);
        let path = match (self.hosted, hosted) {
          (true, false) => match self.path.split_first() {
            Some((0, page)) => page.to_vec(),
            // It was in a layer that has gone: the end of the page.
            _ => return StartPoint::After(root.node_id()),
          },
          (false, true) => std::iter::once(0).chain(self.path.iter().copied()).collect(),
          _ => self.path.clone(),
        };
        (path, false)
      }
    };
    let mut at = root;
    if passed(at) {
      return StartPoint::Before(at.node_id());
    }
    for &index in &path {
      let Some(child) = at.children().get(index) else {
        return StartPoint::After(at.node_id());
      };
      if passed(child) {
        return StartPoint::Before(child.node_id());
      }
      at = child;
    }
    if found {
      StartPoint::On(self.node)
    } else {
      StartPoint::Before(at.node_id())
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
  let walk = walk_group(group, dropped, around);
  (walk.stops, walk.before)
}

/// The stops of [`group_stops`], each with its node: for what reads more of each than where it is, the level
/// of an element of a nested group, the label typeahead matches, in one walk of the group.
pub(super) fn group_stop_nodes<'a>(group: &'a Node, dropped: &[NodeId]) -> (Vec<FocusTarget>, Vec<&'a Node>) {
  let walk = walk_group(group, dropped, None);
  (walk.stops, walk.nodes)
}

struct GroupWalk<'a> {
  stops: Vec<FocusTarget>,
  nodes: Vec<&'a Node>,
  around: Option<NodeId>,
  before: Option<usize>,
}

fn walk_group<'a>(group: &'a Node, dropped: &[NodeId], around: Option<NodeId>) -> GroupWalk<'a> {
  fn collect<'a>(
    node: &'a Node,
    root: bool,
    dropped: &[NodeId],
    focus_event_id: Option<NodeId>,
    in_form: bool,
    walk: &mut GroupWalk<'a>,
  ) {
    if walk.around == Some(node.node_id()) {
      walk.before = Some(walk.stops.len());
    }
    if dropped.contains(&node.node_id()) || node.is_disabled() || node.is_inert() || node.is_leaving_layer() {
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
      walk.nodes.push(node);
      if node.button_kind_value().is_some() && !node.has_stops_inside() {
        return;
      }
    }
    for child in node.children() {
      collect(child, false, dropped, focus_event_id, in_form, walk);
    }
  }
  let mut walk = GroupWalk {
    stops: Vec::new(),
    nodes: Vec::new(),
    around,
    before: None,
  };
  collect(group, true, dropped, None, false, &mut walk);
  walk
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
