//! How a key reaches the handlers of the tree: every node in the order of the tree, from the root down, until one
//! stops it. An element that hears keys first (`.keys_first(true)`, see `NodeUpdate::keys_first`) and is on the way
//! to the focus is told first, with what is in it; unless one of them stopped the key, the rest of the tree is told
//! after, that element left out. Where two such elements are on the way to the focus, the outer one is told first.

use crate::{app::events::KeyboardEvent, node::Node};

/// Which handlers a key calls: those of a key that went down, or those of one let go.
#[derive(Clone, Copy)]
pub(super) enum KeyPhase {
  Down,
  Up,
}

/// Tells the handlers under `root` of `evt`, `focus` being the path from `root` to the focused node: first those of
/// the element on that path that hears keys first, if there is one, then the others.
pub(super) fn fire_key(root: &Node, evt: &mut KeyboardEvent, focus: Option<&[usize]>, phase: KeyPhase) {
  let first = focus.and_then(|focus| first_on(root, focus).map(|depth| focus.split_at(depth)));
  if let Some((to, rest)) = first {
    let node = to
      .iter()
      .try_fold(root, |node, index| node.children().get(*index))
      .expect("the path was found in the tree");
    if fire(node, evt, Some(rest), None, phase) {
      return;
    }
  }
  fire(root, evt, focus, first.map(|(to, _)| to), phase);
}

/// How deep on `focus`, a path from `root`, the outermost element that hears keys first stands, where one does.
fn first_on(root: &Node, focus: &[usize]) -> Option<usize> {
  if root.keys_first() {
    return Some(0);
  }
  let mut node = root;
  for (depth, index) in focus.iter().enumerate() {
    node = node.children().get(*index)?;
    if node.keys_first() {
      return Some(depth + 1);
    }
  }
  None
}

/// The rest of `path` from the child at `index` of a node on it: `None` where the child is not on it.
fn through(path: Option<&[usize]>, index: usize) -> Option<&[usize]> {
  let (first, rest) = path?.split_first()?;
  (*first == index).then_some(rest)
}

/// Calls the handlers of `node` and of everything in it, but the element at `skip`, a path from `node` (`Some(&[])`
/// is `node` itself). `focus` is the path from `node` to the focused node, while `node` is on the way to it. Returns
/// whether a handler stopped the key.
fn fire(
  node: &Node,
  evt: &mut KeyboardEvent,
  focus: Option<&[usize]>,
  skip: Option<&[usize]>,
  phase: KeyPhase,
) -> bool {
  if skip.is_some_and(<[usize]>::is_empty) {
    return false;
  }
  evt.target_id = node.node_id();
  evt.focus_within = focus.is_some();
  let handlers = match phase {
    KeyPhase::Down => &node.events.on_key_down,
    KeyPhase::Up => &node.events.on_key_up,
  };
  for handler in handlers {
    handler.call(evt);
    if evt.immediate_propagation_stopped() {
      return true;
    }
  }
  if evt.propagation_stopped() {
    return true;
  }
  for (index, child) in node.children().iter().enumerate() {
    if fire(child, evt, through(focus, index), through(skip, index), phase) {
      return true;
    }
  }
  false
}
