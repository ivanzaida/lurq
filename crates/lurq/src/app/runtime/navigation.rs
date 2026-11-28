//! Going back and going forward, from every source, in one place ([`NavigationCommand`]).
//!
//! A key ([`KeyboardEvent::navigation_command`](crate::app::events::KeyboardEvent::navigation_command): Alt+←, Cmd+[,
//! a Back key, Android's back), the mouse's side buttons, a swipe (`on_history_swipe`'s default), a browser's
//! buttons and a request without input ([`Ctx::history_back`](crate::app::ctx::Ctx::history_back), a stack's back
//! button, MCP) are all run the same way:
//!
//! 1. From where the command starts ([`HistoryEvent`] says where for each source) out to the root, innermost first:
//!    each element's `on_history_back` or `on_history_forward` handlers, where one whose handlers all pass it on
//!    ([`HistoryEvent::pass_on`]) has done nothing; then the router mounted in that element, if any, where it can move.
//!    The first that does something takes it. A pointer's command goes out from what is under the pointer first, then
//!    from the focus; where nothing on the way out from the focus takes it, from the last element in the tree's order
//!    with a handler for it, as with nothing focused (a focus on the window's caption buttons is outside the content).
//! 2. With nothing on the way taking it, the routers mounted elsewhere in the window, the last in the tree's order
//!    first (an inner one is after the one it is in), until one moves.

use std::collections::HashSet;

use super::{Tree, find_node_by_path, find_path_by_id, hit_test_tree, paste::nodes_along_path};
use crate::{
  app::{
    events::{HistoryEvent, HistoryMover, NavigationCommand, NavigationSource},
    window::NavigationOrigin,
  },
  core::{ElementRef, NodeId},
  node::{EventHandler as Callback, node::Node},
};

/// The handlers of `node` for `command`.
fn handlers(node: &Node, command: NavigationCommand) -> &[Callback<HistoryEvent>] {
  match command {
    NavigationCommand::Back => &node.events.on_history_back,
    NavigationCommand::Forward => &node.events.on_history_forward,
  }
}

/// Whether anything in the tree under `node` goes back or forward: a handler of either, or a router.
pub(super) fn hears_history(node: &Node) -> bool {
  !node.events.on_history_back.is_empty()
    || !node.events.on_history_forward.is_empty()
    || node.events.history_mover.is_some()
    || node.children().iter().any(hears_history)
}

/// The path of the element `reference` is on, under `node` at `path`.
fn path_of_ref(node: &Node, reference: &ElementRef, path: &mut Vec<usize>) -> bool {
  if node
    .element_ref
    .as_ref()
    .is_some_and(|current| current.same_handle(reference))
  {
    return true;
  }
  for (index, child) in node.children().iter().enumerate() {
    path.push(index);
    if path_of_ref(child, reference, path) {
      return true;
    }
    path.pop();
  }
  false
}

/// The path of the last node in the tree's order under `node` at `path` that has a handler for `command`.
fn last_hearing(node: &Node, command: NavigationCommand, path: &mut Vec<usize>, found: &mut Option<Vec<usize>>) {
  if !handlers(node, command).is_empty() {
    *found = Some(path.clone());
  }
  for (index, child) in node.children().iter().enumerate() {
    path.push(index);
    last_hearing(child, command, path, found);
    path.pop();
  }
}

/// The routers under `node`, in the tree's order, with their nodes.
fn movers(node: &Node, out: &mut Vec<(NodeId, HistoryMover)>) {
  if let Some(mover) = &node.events.history_mover {
    out.push((node.node_id(), mover.clone()));
  }
  for child in node.children() {
    movers(child, out);
  }
}

/// One element on the command's way: its handlers, and the router mounted in it.
struct Step {
  id: NodeId,
  handlers: Vec<Callback<HistoryEvent>>,
  mover: Option<HistoryMover>,
}

impl Tree {
  /// Where the command starts in the part of the tree at `scope`, in order: the focused element where it is in it,
  /// then the last element in it that has a handler for `command`, or else the part itself. What is on the way out
  /// from the focus is asked first; where nothing there takes it, the window's own handlers are, as with nothing
  /// focused: a focus outside the content, on the window's caption buttons or its title bar, finds them too.
  fn start_in(&self, root: &Node, scope: Vec<usize>, command: NavigationCommand) -> Vec<Vec<usize>> {
    let mut starts = Vec::new();
    let focused = self
      .focused_path
      .clone()
      .or_else(|| self.focused_node.and_then(|id| find_path_by_id(root, id)));
    if let Some(focused) = focused
      && focused.starts_with(&scope)
      && find_node_by_path(root, &focused).is_some()
    {
      starts.push(focused);
    }
    let Some(part) = find_node_by_path(root, &scope) else {
      if starts.is_empty() {
        starts.push(Vec::new());
      }
      return starts;
    };
    let mut found = None;
    let mut path = scope.clone();
    last_hearing(part, command, &mut path, &mut found);
    match found {
      Some(found) => starts.push(found),
      None if starts.is_empty() => starts.push(scope),
      None => {}
    }
    starts
  }

  /// The path of the innermost element under `(x, y)`, logical.
  fn path_under(&self, root: &Node, x: f32, y: f32) -> Option<Vec<usize>> {
    let layout = self.last_layout.as_ref()?;
    let mut hits = Vec::new();
    hit_test_tree(root, layout, 0.0, 0.0, x, y, &self.layout_engine.radii(), &mut hits);
    let (innermost, _) = hits.first()?;
    find_path_by_id(root, innermost.node_id())
  }

  /// Runs `command` from `origin`, out to the root, then on the routers elsewhere. Whether something took it.
  pub(crate) fn dispatch_navigation(
    &mut self,
    command: NavigationCommand,
    source: NavigationSource,
    origin: &NavigationOrigin,
  ) -> bool {
    let Some(root) = &self.root else {
      return false;
    };
    let mut paths = Vec::new();
    match origin {
      NavigationOrigin::Focus => paths.extend(self.start_in(root, Vec::new(), command)),
      NavigationOrigin::Point(x, y) => {
        paths.extend(self.path_under(root, *x, *y));
        paths.extend(self.start_in(root, Vec::new(), command));
      }
      NavigationOrigin::Part(part) => {
        let mut path = Vec::new();
        let scope = if path_of_ref(root, part, &mut path) {
          path
        } else {
          Vec::new()
        };
        paths.extend(self.start_in(root, scope, command));
      }
    }
    // What is on the way is taken from the tree first: a handler that changes the app's state runs with nothing
    // borrowed.
    let mut seen = HashSet::new();
    let mut steps = Vec::new();
    for path in &paths {
      for node in nodes_along_path(root, path).into_iter().rev() {
        if !seen.insert(node.node_id()) {
          continue;
        }
        let handlers = handlers(node, command).to_vec();
        let mover = node.events.history_mover.clone();
        if !handlers.is_empty() || mover.is_some() {
          steps.push(Step {
            id: node.node_id(),
            handlers,
            mover,
          });
        }
      }
    }
    let origin_id = paths
      .first()
      .and_then(|path| find_node_by_path(root, path))
      .map_or(root.node_id(), Node::node_id);
    let mut elsewhere = Vec::new();
    movers(root, &mut elsewhere);
    elsewhere.retain(|(id, _)| !seen.contains(id));

    let event = HistoryEvent::new(command, source, origin_id);
    let taken = 'taken: {
      for step in steps {
        let event = event.at(step.id);
        let mut taken = false;
        for handler in &step.handlers {
          event.reset();
          handler.call(&event);
          taken |= !event.passed_on();
        }
        if taken || step.mover.is_some_and(|mover| mover(command, source)) {
          break 'taken true;
        }
      }
      elsewhere.into_iter().rev().any(|(_, mover)| mover(command, source))
    };
    if taken {
      self.needs_redraw = true;
    }
    taken
  }

  /// Goes back or forward as the app asks, without input: from the focused element and out from it, as
  /// [`Ctx::history_back`](crate::app::ctx::Ctx::history_back) does but at once. Whether something took it. Public
  /// for tests and tools.
  #[doc(hidden)]
  pub fn run_navigation(&mut self, command: NavigationCommand) -> bool {
    self.navigate_from_input(command, NavigationSource::App, NavigationOrigin::Focus)
  }

  /// Runs `command` from `origin` as input that went to the window, and renders what it changed.
  pub(crate) fn navigate_from_input(
    &mut self,
    command: NavigationCommand,
    source: NavigationSource,
    origin: NavigationOrigin,
  ) -> bool {
    let taken = self.dispatch_navigation(command, source, &origin);
    self.apply_reactive_updates_after_event();
    taken
  }

  /// Runs the commands the app asked for without input since the last time, in order. Whether one was taken.
  pub(super) fn run_requested_navigations(&mut self) -> bool {
    let mut taken = false;
    for request in self.window.take_navigations() {
      let done = self.dispatch_navigation(request.command, request.source, &request.origin);
      if let Some(after) = request.done {
        after(done);
      }
      taken |= done;
    }
    taken
  }
}
