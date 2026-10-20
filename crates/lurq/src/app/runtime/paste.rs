//! Pasting into the focused text input.
//!
//! Ctrl+V (Cmd+V on macOS) and Shift+Insert read the OS clipboard, a
//! [`WindowHandle::paste_from_clipboard`] request (an app's context menu)
//! does the same from the shell, and [`Tree::paste`] takes content from the
//! caller. All of them deliver one [`PasteEvent`] to the focused input and up
//! its ancestors before the default inserts the text; a handler that calls
//! `prevent_default()` takes the content itself.
//!
//! The clipboard is read as far as the paste needs: with no `on_paste`
//! handler on the input or its ancestors only the text is read, as before
//! there were paste events; with one, the image and the files too.
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

/// Where a paste shortcut reads from: the OS clipboard (with the `clipboard`
/// feature), or what a test installs with [`Tree::set_clipboard_reader`].
pub trait ClipboardReader: Send + Sync {
  /// The clipboard's text, for a paste no handler looks at.
  fn read_text(&self) -> Option<String>;
  /// Everything the clipboard holds, for a paste a handler looks at.
  fn read(&self) -> Option<ClipboardContent>;
}

/// The OS clipboard.
#[cfg(feature = "clipboard")]
struct OsClipboard;

#[cfg(feature = "clipboard")]
impl ClipboardReader for OsClipboard {
  fn read_text(&self) -> Option<String> {
    crate::clipboard::read_from_clipboard()
  }

  fn read(&self) -> Option<ClipboardContent> {
    crate::clipboard::read_clipboard()
  }
}

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
    let keyboard = paste_keyboard();
    let handled = self.paste_content(|_| Some(content), &keyboard);
    self.finish_paste(handled);
    handled
  }

  /// Reads the clipboard and pastes it, as a `WindowHandle::paste_from_clipboard`
  /// request asks: refused when `synthetic`, since the request was made
  /// while lurq delivered synthetic input and the person's clipboard is
  /// theirs. `false` when refused or nothing was pasted.
  pub fn paste_from_os_clipboard(&mut self, synthetic: bool) -> bool {
    if synthetic {
      return false;
    }
    self.rebuild_if_dirty();
    let keyboard = paste_keyboard();
    let handled = self.paste_from_reader(&keyboard);
    self.finish_paste(handled);
    handled
  }

  /// Where paste shortcuts and [`paste_from_os_clipboard`](Self::paste_from_os_clipboard)
  /// read from instead of the OS clipboard: a test's clipboard, which the
  /// OS never sees. `None` goes back to the OS clipboard (feature
  /// `clipboard`; nothing without it).
  pub fn set_clipboard_reader(&mut self, reader: Option<Arc<dyn ClipboardReader>>) {
    self.clipboard_reader = reader;
  }

  /// The paste a key shortcut asks for. OS input reads the clipboard;
  /// synthetic input is refused, since it must not read the person's
  /// clipboard, and carries its own content with `SyntheticInput::paste`
  /// instead.
  pub(super) fn paste_from_clipboard_shortcut(&mut self, keyboard: &KeyboardEvent) -> bool {
    if is_synthetic_input() {
      return false;
    }
    self.paste_from_reader(keyboard)
  }

  fn finish_paste(&mut self, handled: bool) {
    if handled {
      self.needs_redraw = true;
      self.reset_text_input_caret_blink();
    }
    self.apply_reactive_updates_after_event();
  }

  fn paste_from_reader(&mut self, keyboard: &KeyboardEvent) -> bool {
    let Some(reader) = self.clipboard_reader.clone().or_else(os_clipboard) else {
      return false;
    };
    self.paste_content(
      |wants_everything| {
        if wants_everything {
          reader.read()
        } else {
          reader.read_text().map(ClipboardContent::text)
        }
      },
      keyboard,
    )
  }

  /// `read` gets whether an `on_paste` handler on the input or an ancestor
  /// will look at the content, so a plain paste reads only the text.
  fn paste_content(&mut self, read: impl FnOnce(bool) -> Option<ClipboardContent>, keyboard: &KeyboardEvent) -> bool {
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
    let chain = nodes_along_path(root, &path);
    let has_handlers = chain.iter().any(|node| !node.events.on_paste.is_empty());
    let Some(content) = read(has_handlers) else {
      return false;
    };
    let event = PasteEvent::new(Arc::new(content), input.node_id());
    dispatch_paste(&chain, &event);
    if event.default_prevented() {
      return true;
    }
    let Some(text) = event.text() else {
      return false;
    };
    state.insert(text, keyboard)
  }
}

/// The key a paste without a key reports to `on_input` handlers.
fn paste_keyboard() -> KeyboardEvent {
  KeyboardEvent::new("Paste", "Paste", false, false, false, false, NodeId::UNASSIGNED)
}

#[cfg(feature = "clipboard")]
fn os_clipboard() -> Option<Arc<dyn ClipboardReader>> {
  Some(Arc::new(OsClipboard))
}

#[cfg(not(feature = "clipboard"))]
fn os_clipboard() -> Option<Arc<dyn ClipboardReader>> {
  None
}

/// The root, each ancestor and the node at `path`, in that order.
fn nodes_along_path<'a>(root: &'a Node, path: &[usize]) -> Vec<&'a Node> {
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
  chain
}

/// Runs the `on_paste` handlers of the input, then of each ancestor up to
/// the root, until one stops the propagation.
fn dispatch_paste(chain: &[&Node], event: &PasteEvent) {
  for node in chain.iter().rev() {
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
