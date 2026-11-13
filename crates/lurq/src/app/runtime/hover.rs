//! Hover under a pointer that holds still. A scroll, a list that grows or any
//! other relayout moves content under the pointer without a mouse event, so
//! after each layout the hover is hit-tested again at the pointer's last
//! position, like the web does: the node that moved away gets its
//! `on_mouse_leave` (and with it, its tooltip goes), the node that moved under
//! the pointer its `on_mouse_enter`. They run with the source of the input
//! that last put the pointer there, so an agent's pointer stays synthetic.
//!
//! A hover style that moves its own node away from the pointer (a `hovered`
//! size or offset with a transition) unhovers it, which moves it back under
//! the pointer, and so on: like CSS `:hover`, such a node flickers.
//!
//! A hover that shows nothing asks for no frame. The hover moves whenever the
//! pointer comes onto another node, a text inside its row say, and most nodes
//! look the same under the pointer: a frame for each of those moves draws the
//! pixels of the frame before it. One is asked for when a node that gained or
//! lost the hover is drawn differently for it or ran a handler for it (see
//! [`set_hover`]), and for every change while DevTools shows the tree.

use super::{
  FocusTarget, Tree, find_node_by_id, find_path_by_id, focus_visible, hovered_cursor, scrollbar_hit_at,
  set_node_hovered, take_disabled_hit, trim_hits_to_scrollbar,
};
use crate::{
  app::{
    events::{InputSource, input_source, with_input_source},
    hit_test::{HitRect, hit_test_tree},
  },
  core::NodeId,
  node::{Node, VoidEventHandler},
};

/// Where the pointer last was, in physical pixels, and where the input that
/// put it there came from.
#[derive(Clone, Copy)]
pub(super) struct PointerAt {
  x: f32,
  y: f32,
  source: InputSource,
}

/// What a move of the hover did.
#[derive(Clone, Copy, Default)]
pub(super) struct HoverMove {
  /// A node gained or lost the hover.
  pub(super) changed: bool,
  /// One of those nodes is drawn differently for it, or ran a handler for it.
  pub(super) shows: bool,
}

impl std::ops::BitOr for HoverMove {
  type Output = Self;

  fn bitor(self, other: Self) -> Self {
    Self {
      changed: self.changed || other.changed,
      shows: self.shows || other.shows,
    }
  }
}

impl std::ops::BitOrAssign for HoverMove {
  fn bitor_assign(&mut self, other: Self) {
    *self = *self | other;
  }
}

impl Tree {
  /// Remembers where the pointer is, in physical pixels, for
  /// [`Self::refresh_hover_at_pointer`]. Called by the public pointer entry
  /// points only: a click held back for a double click is delivered later at
  /// its own position, which is no longer where the pointer is.
  pub(super) fn track_pointer(&mut self, x: f32, y: f32) {
    self.hover_held_by_keys = false;
    self.pointer = Some(PointerAt {
      x,
      y,
      source: input_source(),
    });
  }

  /// Hit-tests the hover again at the pointer's last position against the
  /// last layout. Does nothing while the pointer is outside the window or a
  /// drag holds it. Returns whether the hovered nodes changed.
  pub(super) fn refresh_hover_at_pointer(&mut self) -> bool {
    let Some(PointerAt { x, y, source }) = self.pointer else {
      return false;
    };
    if self.has_active_input_interaction() || self.hover_held_by_keys {
      return false;
    }
    let scale = self.scale_factor();
    let (lx, ly) = (x / scale, y / scale);
    let (Some(root), Some(result)) = (&self.root, &self.last_layout) else {
      return false;
    };
    let mut hits = Vec::new();
    hit_test_tree(root, result, 0.0, 0.0, lx, ly, &self.layout_engine.radii(), &mut hits);
    // A disabled element is not hovered; the elements it is in are.
    let disabled = take_disabled_hit(&mut hits);
    trim_hits_to_scrollbar(&mut hits, lx, ly);

    let before = self.disabled_hover;
    let moved = with_input_source(source, || {
      leave_disabled(root, before, disabled)
        | move_hover(root, &self.hover_path, &hits)
        | enter_disabled(before, disabled)
    });
    self.disabled_hover = disabled.map(Node::node_id);
    let scrollable_in_disabled = if disabled.is_some() {
      super::scroll_containers_in_disabled(root, result, lx, ly, &self.layout_engine.radii(), &hits)
    } else {
      Vec::new()
    };
    let mut thumb_changed = false;
    for (node, _) in scrollable_in_disabled.iter().chain(&hits) {
      if let Some((state, _)) = node.scroll_container() {
        let on_thumb = scrollbar_hit_at(node, lx, ly).is_some();
        if on_thumb != state.is_thumb_hovered() {
          state.set_thumb_hovered(on_thumb);
          thumb_changed = true;
        }
      }
    }
    let follow = pointer_focus(root, &self.hover_path, self.focused_path.as_deref(), &hits);
    self.hover_path = hits.iter().map(|(node, _)| node.node_id()).collect();
    self.cursor = hovered_cursor(disabled.into_iter().chain(hits.iter().map(|(node, _)| *node)));
    let draws_hover = self.draws_hover(moved);
    if draws_hover {
      self.cached_render_list = None;
    }
    if draws_hover || thumb_changed {
      self.needs_redraw = true;
    }
    self.follow_pointer_focus(follow);
    moved.changed
  }

  /// Moves the focus to `target`, the element under the pointer that the
  /// focus follows it to ([`pointer_focus`]), with no ring for it.
  pub(super) fn follow_pointer_focus(&mut self, target: Option<NodeId>) {
    if let Some(target) = target
      && self.focused_node != Some(target)
    {
      self.set_input_modality(focus_visible::InputModality::Pointer);
      self.focus_node(FocusTarget {
        input_id: target,
        event_id: target,
      });
      self.needs_redraw = true;
    }
  }

  /// Whether a move of the hover needs a frame: it shows, or DevTools is open
  /// on the tree, whose element tree and inspector show which nodes are hovered
  /// and are brought up to date after a frame.
  pub(super) fn draws_hover(&self, moved: HoverMove) -> bool {
    #[cfg(feature = "devtools")]
    if moved.changed && self.devtools_is_open() {
      return true;
    }
    moved.shows
  }
}

/// The element under the pointer that the focus follows it to: the innermost
/// one that takes the focus, where it is in a focus group that follows the
/// pointer ([`FocusGroup::follow_pointer`](crate::node::FocusGroup::follow_pointer))
/// and the focus, at `focused_path`, is in that group already. Only an element
/// the pointer has just come onto, not in `hover_path`: a pointer that holds
/// still while the keyboard moves the focus away leaves it where it went.
pub(super) fn pointer_focus(
  root: &Node,
  hover_path: &[NodeId],
  focused_path: Option<&[usize]>,
  hits: &[(&Node, HitRect)],
) -> Option<NodeId> {
  let focused_path = focused_path?;
  let stop = hits
    .iter()
    .rev()
    .map(|(node, _)| *node)
    .find(|node| node.is_focusable() && !node.is_focus_disabled())?;
  if hover_path.contains(&stop.node_id()) {
    return None;
  }
  let path = find_path_by_id(root, stop.node_id())?;
  // The innermost group on the way to the element that follows the pointer, by the length of its path.
  let mut group = None;
  let mut node = root;
  for (depth, &index) in path.iter().enumerate() {
    if node.focus_group_value().is_some_and(|group| group.follow_pointer) {
      group = Some(depth);
    }
    node = node.children().get(index)?;
  }
  let depth = group?;
  (focused_path.len() > depth && focused_path[..depth] == path[..depth]).then(|| stop.node_id())
}

/// Moves the hover from the nodes in `hover_path` to the nodes in `hits`: a
/// node the pointer left is unhovered and gets its `on_mouse_leave`, then a
/// node it came onto is hovered and gets its `on_mouse_enter`.
pub(super) fn move_hover(root: &Node, hover_path: &[NodeId], hits: &[(&Node, HitRect)]) -> HoverMove {
  let mut moved = HoverMove::default();
  for old_id in hover_path {
    if hits.iter().any(|(node, _)| node.node_id() == *old_id) {
      continue;
    }
    let Some(node) = find_node_by_id(root, *old_id) else {
      continue;
    };
    moved.changed = true;
    moved.shows |= leave(node);
  }
  for (node, _) in hits {
    if hover_path.contains(&node.node_id()) {
      continue;
    }
    moved.changed = true;
    moved.shows |= set_hover(node, true, &node.events.on_mouse_enter);
  }
  moved
}

/// The pointer left `before`, the disabled element it was on, for `now` (another disabled element, or none): runs
/// `before`'s `on_mouse_leave`. A disabled element hears the pointer come and go and nothing else of it: it is not
/// hovered, sets no cursor and takes no press. Called before the hover moves, as a leave comes before an enter.
pub(super) fn leave_disabled(root: &Node, before: Option<NodeId>, now: Option<&Node>) -> HoverMove {
  let Some(before) = before.filter(|before| Some(*before) != now.map(Node::node_id)) else {
    return HoverMove::default();
  };
  let Some(node) = find_node_by_id(root, before) else {
    return HoverMove::default();
  };
  run(&node.events.on_mouse_leave)
}

/// The pointer came onto `now`, a disabled element, from `before`: runs `now`'s `on_mouse_enter`, after the hover
/// moved. See [`leave_disabled`].
pub(super) fn enter_disabled(before: Option<NodeId>, now: Option<&Node>) -> HoverMove {
  match now {
    Some(node) if Some(node.node_id()) != before => run(&node.events.on_mouse_enter),
    _ => HoverMove::default(),
  }
}

/// Runs `handlers` of a move that changes no hover: a frame for them where any ran.
fn run(handlers: &[VoidEventHandler]) -> HoverMove {
  for handler in handlers {
    handler.call();
  }
  HoverMove {
    changed: false,
    shows: !handlers.is_empty(),
  }
}

/// Takes the hover from `node` and runs its `on_mouse_leave`. Returns whether
/// that needs a frame.
pub(super) fn leave(node: &Node) -> bool {
  set_hover(node, false, &node.events.on_mouse_leave)
}

/// Sets `node`'s hover and runs the `handlers` of that change. Returns whether
/// it needs a frame:
///
/// - the node is drawn differently under the pointer ([`Node::shows_hover`]);
/// - one of its state styles sizes it, so the change dirtied its layout;
/// - a handler ran. What a handler changes mostly marks the tree by itself (a signal, a scroll state, a canvas), but it
///   may also reach what is drawn through something that marks nothing, so its run is drawn whatever it did.
///
/// Nothing else of a hover is drawn. The cursor is the shell's, which reads
/// [`Tree::cursor`] after each pointer event. The hover an app reads from an
/// element ref or an interaction state is no signal: it renders no component,
/// so a frame for it would draw the frame before again.
fn set_hover(node: &Node, hovered: bool, handlers: &[VoidEventHandler]) -> bool {
  let relaid = set_node_hovered(node, hovered);
  for handler in handlers {
    handler.call();
  }
  relaid || node.shows_hover() || !handlers.is_empty()
}
