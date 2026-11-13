//! The focus of a modal that takes it (`Modal::takes_focus`), as a browser's
//! `<dialog>` opened with `showModal()` has it: as the modal opens, the focus
//! goes to its first stop of Tab, and as it closes, back to what had it before.
//! An overlay that takes it (`Overlay::takes_focus`) does not trap Tab, and is
//! told from another by its anchor: see `overlay_layers.rs`.
//!
//! A modal is told from another between passes by the node that declares it.
//! The focus goes back only when it was lost with the modal: one the app or a
//! press has put somewhere else that is still there stays there.

use super::{FocusTarget, Tree, find_path_by_id, tab_navigation};

impl Tree {
  /// Gives the focus back from the modals that closed since the last pass,
  /// the latest first, and moves it into those that opened.
  pub(super) fn sync_modal_focus(&mut self) {
    let built = std::mem::take(&mut self.built_focus_modals);
    let mut index = self.modal_focus.len();
    while index > 0 {
      index -= 1;
      let (origin, returns_to) = self.modal_focus[index];
      if built.contains(&origin) {
        continue;
      }
      self.modal_focus.remove(index);
      if let Some(target) = returns_to
        && !self.focus_is_in_tree()
      {
        self.focus_node(target);
      }
    }
    for origin in built {
      if self.modal_focus.iter().any(|(open, _)| *open == origin) {
        continue;
      }
      let had = self
        .focused_node
        .zip(self.focused_event_node)
        .map(|(input_id, event_id)| FocusTarget { input_id, event_id });
      self.modal_focus.push((origin, had));
      // The first stop of the topmost modal, which is the one that has just opened.
      let first = self.root.as_ref().and_then(|root| {
        match tab_navigation::tab_move(root, &self.dropped_nodes, &[], None, false) {
          tab_navigation::TabMove::Focus(target) => Some(target),
          _ => None,
        }
      });
      if let Some(target) = first {
        self.focus_node(target);
      }
    }
  }

  /// Whether the focused node is still in the tree.
  fn focus_is_in_tree(&self) -> bool {
    let (Some(root), Some(focused)) = (self.root.as_ref(), self.focused_node) else {
      return false;
    };
    find_path_by_id(root, focused).is_some()
  }
}
