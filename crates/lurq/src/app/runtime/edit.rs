//! The commands of editing on what has the focus: undo, redo, cut, copy, paste and select all.
//!
//! A key that is one ([`KeyboardEvent::edit_command`], the one place lurq knows those keys) and a request without a
//! key ([`Ctx::edit`](crate::app::ctx::Ctx::edit), [`WindowHandle::edit`](crate::app::window::WindowHandle::edit), an
//! Edit menu) are run the same way, on the focused element and out from it:
//!
//! 1. A focused text input does it to its own text where it can: takes back an edit it has, copies or cuts its
//!    selection, pastes (its `on_paste` handlers and its ancestors' are told first), selects its text.
//! 2. Otherwise the handlers of the focused element, then of each ancestor, innermost first: `on_undo`, `on_redo` and
//!    `on_select_all` ([`EditEvent`]), where one whose handlers all pass the command on ([`EditEvent::pass_on`]) has
//!    done nothing and the command goes on to the element around it, and the first that does not takes it; `on_copy`
//!    for a copy or a cut ([`CopyEvent`](crate::app::events::CopyEvent)), which goes out until a handler stops it, and
//!    is taken where one set a text; `on_paste` ([`PasteEvent`](crate::app::events::PasteEvent)), on a focused element
//!    that is not a text input.
//! 3. A copy that nothing took copies the text selected on the page (a selectable text), focused or not.
//!
//! A key goes to every `on_key_down` handler before: one that prevents the key's default (an app that binds Ctrl+Z
//! to something else) keeps it from being a command, as an element that captures the keys (`.captures_keys(true)`)
//! does. A key that is a command is no other key, whether or not something took it.

use super::{
  Tree, find_node_by_path, find_path_by_id,
  paste::{dispatch_copy, nodes_along_path},
  selected_selectable_text,
};
use crate::{
  app::events::{CopyEvent, EditCommand, EditEvent, KeyboardEvent, is_synthetic_input},
  node::{node::Node, node_kind::NodeKind},
};

impl Tree {
  /// Runs `command` on the focused element and out from it; `keyboard` is the key that asked, where one did, and
  /// `synthetic` whether synthetic input asked, whose paste reads no clipboard. Whether something took it.
  pub(super) fn dispatch_edit(
    &mut self,
    command: EditCommand,
    keyboard: Option<&KeyboardEvent>,
    synthetic: bool,
  ) -> bool {
    let keyboard = keyboard.cloned().unwrap_or_else(|| command.keyboard_event());
    let taken =
      self.edit_focus(command, &keyboard, synthetic) || (command == EditCommand::Copy && self.copy_selected_text());
    if taken {
      self.needs_redraw = true;
    }
    taken
  }

  /// The command on the focused text input, or on the handlers of the focused element and its ancestors.
  fn edit_focus(&mut self, command: EditCommand, keyboard: &KeyboardEvent, synthetic: bool) -> bool {
    let Some(root) = &self.root else {
      return false;
    };
    let Some(path) = self
      .focused_path
      .clone()
      .or_else(|| self.focused_node.and_then(|id| find_path_by_id(root, id)))
    else {
      return false;
    };
    let Some(focused) = find_node_by_path(root, &path) else {
      return false;
    };
    let focus_id = focused.node_id();
    let input = match focused.node_kind() {
      NodeKind::TextInput { state, .. } => Some(state.clone()),
      _ => None,
    };
    if command == EditCommand::Paste {
      // The paste reads the clipboard and tells the `on_paste` handlers of the input or of the element focused, and of
      // their ancestors, before a text input puts the text in.
      if synthetic || is_synthetic_input() {
        return false;
      }
      let pasted = self.paste_from_reader(keyboard);
      if pasted && input.is_some() {
        self.reset_text_input_caret_blink();
      }
      return pasted;
    }
    if let Some(state) = input {
      let done = match command {
        EditCommand::Undo => state.undo(keyboard),
        EditCommand::Redo => state.redo(keyboard),
        EditCommand::SelectAll => {
          state.select_all();
          true
        }
        EditCommand::Copy | EditCommand::Cut => match state.clipboard_selection() {
          Some(selected) => {
            let written = self.write_input_copy(&state, &selected);
            if written && command == EditCommand::Cut {
              let _ = state.cut_selection(keyboard);
            }
            written
          }
          None => false,
        },
        EditCommand::Paste => unreachable!("a paste is run above"),
      };
      if done {
        self.reset_text_input_caret_blink();
        return true;
      }
      if matches!(command, EditCommand::Copy | EditCommand::Cut) {
        // A text input's copy is its own: its handlers' `on_copy` are not told.
        return false;
      }
    }
    let Some(root) = &self.root else {
      return false;
    };
    let chain = nodes_along_path(root, &path);
    if matches!(command, EditCommand::Copy | EditCommand::Cut) {
      if chain.iter().all(|node| node.events.on_copy.is_empty()) {
        return false;
      }
      let event = CopyEvent::new(focus_id, command == EditCommand::Cut);
      dispatch_copy(&chain, &event);
      return event.text().is_some_and(|text| self.write_clipboard_text(&text));
    }
    let handlers = |node: &Node| match command {
      EditCommand::Undo => node.events.on_undo.clone(),
      EditCommand::Redo => node.events.on_redo.clone(),
      _ => node.events.on_select_all.clone(),
    };
    // The handlers are taken from the tree first: one that changes the app's state runs with nothing borrowed.
    let along: Vec<_> = chain
      .iter()
      .rev()
      .map(|node| (node.node_id(), handlers(node)))
      .filter(|(_, handlers)| !handlers.is_empty())
      .collect();
    let event = EditEvent::new(command, focus_id);
    for (id, handlers) in along {
      let event = event.at(id);
      let mut taken = false;
      for handler in &handlers {
        event.reset();
        handler.call(&event);
        taken |= !event.passed_on();
      }
      if taken {
        return true;
      }
    }
    false
  }

  /// Copies the text selected on the page, in a selectable text, where there is one.
  fn copy_selected_text(&mut self) -> bool {
    let (Some(root), Some(layout)) = (&self.root, &self.last_layout) else {
      return false;
    };
    let Some(selected) = selected_selectable_text(root, layout) else {
      return false;
    };
    self.write_clipboard_text(&selected)
  }

  /// Runs the commands the app asked for without a key since the last time, in order. Whether one was taken.
  pub(super) fn run_requested_edits(&mut self) -> bool {
    let mut taken = false;
    for (command, synthetic) in self.window.take_edits() {
      taken |= self.dispatch_edit(command, None, synthetic);
    }
    taken
  }
}
