//! Enter on a focused link inside a paragraph ([`crate::node::inline_link`])
//! follows it, like a browser link. Space does not: it is not a button.

use super::{MouseModifiers, Tree, find_node_by_id, find_node_by_path};
use crate::node::node_kind::NodeKind;

impl Tree {
  /// Runs the click handlers of the focused paragraph link, as a click on it
  /// would, with the key's `modifiers`; `false` when the focus is not on such
  /// a link.
  pub(super) fn activate_focused_link(&mut self, modifiers: MouseModifiers) -> bool {
    let Some(focused) = self.focused_node else {
      return false;
    };
    let click = {
      let Some(root) = &self.root else {
        return false;
      };
      let node = self
        .focused_path
        .as_deref()
        .and_then(|path| find_node_by_path(root, path))
        .or_else(|| find_node_by_id(root, focused));
      match node {
        Some(node) if matches!(node.node_kind(), NodeKind::InlineLink { .. }) => node.events.on_click.clone(),
        _ => return false,
      }
    };
    let event = modifiers.keyboard_click(focused);
    for handler in click {
      handler.call(&event);
    }
    true
  }
}
