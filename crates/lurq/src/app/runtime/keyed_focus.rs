//! A key's or a press's handler moving the focus within its focus group to an element by its key
//! ([`KeyboardEvent::focus_key`](crate::app::events::KeyboardEvent::focus_key),
//! [`MouseEvent::focus_key`](crate::app::events::MouseEvent::focus_key)), once the render the event causes is done:
//! for a group whose elements the app reaches by what it knows of them, as a calendar reaches the day a week on,
//! which may be in the month the key shows next.

use std::sync::Arc;

use super::{Tree, find_node_by_path, find_path_by_id, focus_visible, tab_navigation};
use crate::core::NodeId;

impl Tree {
  /// The innermost focus group around the focused element that moves the focus: what a request for an element by
  /// its key looks in. A group of no keys around it puts it in none.
  pub(super) fn focus_group_around_focus(&self) -> Option<NodeId> {
    let path = self.focused_path.as_deref()?;
    let mut node = self.root.as_ref()?;
    let mut found = None;
    for &index in path {
      match node.focus_group_value() {
        Some(group) if group.is_none() => found = None,
        Some(_) => found = Some(node.node_id()),
        None => {}
      }
      node = node.children().get(index)?;
    }
    found
  }

  /// Moves the focus to the stop of the group `group` whose key is `key`, and scrolls it into view: from the keyboard
  /// where `keyboard` says so, so that it shows its ring, else as the input that asked left the focus. Nothing moves
  /// where the group or the stop is not there.
  pub(super) fn focus_keyed_in_group(&mut self, group: NodeId, key: &Arc<str>, keyboard: bool) {
    let Some(root) = self.root.as_ref() else {
      return;
    };
    let Some(group) = find_path_by_id(root, group).and_then(|path| find_node_by_path(root, &path)) else {
      return;
    };
    let (stops, nodes) = tab_navigation::group_stop_nodes(group, &self.dropped_nodes);
    let Some(target) = stops
      .iter()
      .zip(nodes)
      .find(|(_, node)| node.component_key() == Some(key.as_ref()))
      .map(|(stop, _)| *stop)
    else {
      return;
    };
    if keyboard {
      self.set_input_modality(focus_visible::InputModality::Keyboard);
    }
    if self.focused_node != Some(target.input_id) {
      self.focus_node(target);
    }
    self.scroll_focused_into_view();
    self.needs_redraw = true;
  }
}
