//! The focus of a modal that takes it (`Modal::takes_focus`), as a browser's
//! `<dialog>` opened with `showModal()` has it: as the modal opens, the focus
//! goes to its first stop of Tab, and as it closes, back to what had it before.
//! An overlay that takes it (`Overlay::takes_focus`) does not trap Tab, and is
//! told from another by its anchor: see `overlay_layers.rs`.
//!
//! A modal is told from another between passes by the node that declares it.
//! The focus goes back only when it was lost with the modal: one the app or a
//! press has put somewhere else that is still there stays there. Where it has
//! nothing to go back to, Tab goes on from where the focus was as the modal
//! opened.
//!
//! Modals are stacked in the order they opened, as a browser's top layer
//! holds them: one opened over another is over it, takes the pointer, Tab and
//! Escape from it, whatever their order in the tree. An app's search declared
//! in its shell before the page opens over a dialog of the page.

use super::{FocusTarget, Tree, find_path_by_id, tab_navigation};
use crate::{app::ctx::ModalLayer, layout::layout_engine::OverlayLayoutRecord};

impl Tree {
  /// Gives the focus back from the modals that closed since the last pass,
  /// the latest first, and moves it into those that opened.
  pub(super) fn sync_modal_focus(&mut self) {
    let built = std::mem::take(&mut self.built_focus_modals);
    let mut index = self.modal_focus.len();
    while index > 0 {
      index -= 1;
      if built.contains(&self.modal_focus[index].0) {
        continue;
      }
      let (_, returns_to, start) = self.modal_focus.remove(index);
      if self.focus_is_in_tree() {
        continue;
      }
      if let Some(target) = returns_to {
        self.focus_node(target);
      }
      // What had the focus has gone, or nothing had it: Tab goes on from its place.
      if self.focused_node.is_none() && start.is_some() {
        self.focus_start = start;
      }
    }
    for origin in built {
      if self.modal_focus.iter().any(|(open, ..)| *open == origin) {
        continue;
      }
      let had = self
        .focused_node
        .zip(self.focused_event_node)
        .map(|(input_id, event_id)| FocusTarget { input_id, event_id });
      let start = match (self.root.as_ref(), self.focused_node, self.focused_path.as_ref()) {
        (Some(root), Some(id), Some(path)) => Some(tab_navigation::FocusStart::new(root, id, path.clone())),
        _ => self.focus_start.clone(),
      };
      self.modal_focus.push((origin, had, start));
      // The first stop of the topmost modal, which is the one that has just opened.
      let first = self.root.as_ref().and_then(|root| {
        match tab_navigation::tab_move(root, &self.dropped_nodes, &[], None, None, None, false) {
          tab_navigation::TabMove::Focus(target) => Some(target),
          _ => None,
        }
      });
      if let Some(target) = first {
        self.focus_node(target);
      }
    }
  }

  /// Puts the modals among `records`, the layers of one layout in the order of the tree, in the order they
  /// opened; the other layers keep their places. A modal that opened in this pass goes over those open before
  /// it, and among those opened together the order of the tree holds.
  pub(super) fn stack_modals_in_open_order(&mut self, records: &mut Vec<OverlayLayoutRecord>) {
    let mut slots = Vec::new();
    for (slot, record) in records.iter().enumerate() {
      if let OverlayLayoutRecord::Modal { origin, spec, .. } = record
        && spec.layer == ModalLayer::Dialog
      {
        if let Some(origin) = *origin {
          self.modals_met.push(origin);
          if !self.modal_open_order.contains(&origin) {
            self.modal_open_order.push(origin);
          }
        }
        slots.push(slot);
      }
    }
    if slots.len() < 2 {
      return;
    }
    let rank = |record: &OverlayLayoutRecord| {
      match record {
        OverlayLayoutRecord::Modal {
          origin: Some(origin), ..
        } => self.modal_open_order.iter().position(|open| open == origin),
        _ => None,
      }
      .unwrap_or(usize::MAX)
    };
    let mut stacked = slots.clone();
    // Stable: those of one rank keep the order of the tree.
    stacked.sort_by_key(|&slot| rank(&records[slot]));
    if stacked == slots {
      return;
    }
    let mut taken: Vec<Option<OverlayLayoutRecord>> = std::mem::take(records).into_iter().map(Some).collect();
    let modals: Vec<OverlayLayoutRecord> = stacked.iter().filter_map(|&slot| taken[slot].take()).collect();
    for (&slot, modal) in slots.iter().zip(modals) {
      taken[slot] = Some(modal);
    }
    *records = taken.into_iter().flatten().collect();
  }

  /// Forgets the order of the modals the pass just built did not meet: they closed.
  pub(super) fn forget_closed_modals(&mut self) {
    let met = std::mem::take(&mut self.modals_met);
    self.modal_open_order.retain(|open| met.contains(open));
  }

  /// Whether the focused node is still in the tree.
  fn focus_is_in_tree(&self) -> bool {
    let (Some(root), Some(focused)) = (self.root.as_ref(), self.focused_node) else {
      return false;
    };
    find_path_by_id(root, focused).is_some()
  }
}
