//! Pasting into the focused text input.
//!
//! Ctrl+V (Cmd+V on macOS) and Shift+Insert read the OS clipboard, a
//! [`WindowHandle::paste_from_clipboard`] request (an app's context menu)
//! does the same from the shell, and [`Tree::paste`] takes content from the
//! caller. All of them deliver one [`PasteEvent`] to the focused input and up
//! its ancestors before the default inserts the text; a handler that calls
//! `prevent_default()` takes the content itself.
//!
//! A key shortcut delivered as synthetic input (an MCP client, an applied
//! [`SyntheticInput`](crate::app::synthetic_input::SyntheticInput)) never
//! reads the OS clipboard: the person's clipboard is theirs. Such a paste
//! carries its own content ([`SyntheticInput::paste`]) or is refused.
//!
//! [`WindowHandle::paste_from_clipboard`]: crate::app::window::WindowHandle::paste_from_clipboard
//! [`SyntheticInput::paste`]: crate::app::synthetic_input::SyntheticInput::paste

use std::sync::Arc;

use super::{Tree, find_node_by_path, find_path_by_id};
use crate::{
  app::events::{ClipboardContent, KeyboardEvent, PasteEvent, is_synthetic_input},
  core::NodeId,
  node::{node::Node, node_kind::NodeKind},
};

impl Tree {
  /// Pastes `content` into the focused text input, as Ctrl+V pastes the OS
  /// clipboard: the input's `on_paste` handlers and its ancestors' see the
  /// content first, and unless one prevents the default, the text is
  /// inserted at the caret. `false` when no text input has focus, or nothing
  /// was inserted and no handler prevented it.
  ///
  /// A shell calls this with what it read from the OS; a headless test calls
  /// it with the content it wants to paste, or applies
  /// [`SyntheticInput::paste`](crate::app::synthetic_input::SyntheticInput::paste),
  /// which marks the input synthetic.
  pub fn paste(&mut self, content: ClipboardContent) -> bool {
    self.rebuild_if_dirty();
    let keyboard = KeyboardEvent::new("Paste", "Paste", false, false, false, false, NodeId::UNASSIGNED);
    let handled = self.paste_content(Arc::new(content), &keyboard);
    if handled {
      self.needs_redraw = true;
      self.reset_text_input_caret_blink();
    }
    self.apply_reactive_updates_after_event();
    handled
  }

  /// The paste a key shortcut asks for. OS input reads the OS clipboard;
  /// synthetic input is refused, since it must not read the person's
  /// clipboard, and carries its own content with `SyntheticInput::paste`
  /// instead.
  pub(super) fn paste_from_clipboard_shortcut(&mut self, keyboard: &KeyboardEvent) -> bool {
    if is_synthetic_input() {
      return false;
    }
    let Some(content) = read_clipboard_content() else {
      return false;
    };
    self.paste_content(Arc::new(content), keyboard)
  }

  fn paste_content(&mut self, content: Arc<ClipboardContent>, keyboard: &KeyboardEvent) -> bool {
    let Some(root) = &self.root else {
      return false;
    };
    let path = self
      .focused_path
      .clone()
      .or_else(|| self.focused_node.and_then(|id| find_path_by_id(root, id)));
    let Some(path) = path else {
      return false;
    };
    let Some(input) = find_node_by_path(root, &path) else {
      return false;
    };
    let NodeKind::TextInput { state, .. } = input.node_kind() else {
      return false;
    };
    let state = state.clone();
    let event = PasteEvent::new(content, input.node_id());
    dispatch_paste(root, &path, &event);
    if event.default_prevented() {
      return true;
    }
    let Some(text) = event.text() else {
      return false;
    };
    state.insert(text, keyboard)
  }
}

/// Runs the `on_paste` handlers of the input at `path`, then of each
/// ancestor up to the root, until one stops the propagation.
fn dispatch_paste(root: &Node, path: &[usize], event: &PasteEvent) {
  let mut chain = Vec::with_capacity(path.len() + 1);
  let mut current = root;
  chain.push(current);
  for &index in path {
    let Some(child) = current.children().get(index) else {
      break;
    };
    chain.push(child);
    current = child;
  }
  for node in chain.into_iter().rev() {
    let event = event.at(node.node_id());
    for handler in &node.events.on_paste {
      handler.call(&event);
      if event.immediate_propagation_stopped() {
        return;
      }
    }
    if event.propagation_stopped() {
      return;
    }
  }
}

#[cfg(feature = "clipboard")]
fn read_clipboard_content() -> Option<ClipboardContent> {
  crate::clipboard::read_clipboard()
}

#[cfg(not(feature = "clipboard"))]
fn read_clipboard_content() -> Option<ClipboardContent> {
  None
}
