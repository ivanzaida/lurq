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
//! A paste shortcut on a focused element that is not a text input (a cell of
//! a data grid) delivers the same event to it and its ancestors where one of
//! them has an `on_paste` handler, with nothing inserted by default. A copy or
//! a cut shortcut there delivers a [`CopyEvent`]: a handler says the text,
//! and the engine writes it to the clipboard, or to what a test installs with
//! [`Tree::set_clipboard_writer`].
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
  app::events::{ClipboardContent, CopyEvent, KeyboardEvent, PasteEvent, is_synthetic_input},
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
  /// The Markdown a styled input copied beside `text`, where the clipboard still holds that text: what
  /// [`ClipboardWriter::write_rich`] was given with it. `None` where the clipboard keeps none.
  fn read_markdown(&self, _text: &str) -> Option<String> {
    None
  }
}

/// Where a copy writes: the OS clipboard (with the `clipboard` feature), or
/// what a test installs with [`Tree::set_clipboard_writer`].
pub trait ClipboardWriter: Send + Sync {
  /// Puts `text` on the clipboard: whether it was written.
  fn write_text(&self, text: &str) -> bool;
  /// Puts `text` on the clipboard with its Markdown beside it, as a styled input with
  /// [`rich_clipboard`](crate::components::TextInput::rich_clipboard) copies: whether it was written. A clipboard
  /// that keeps no second form writes the text alone.
  fn write_rich(&self, text: &str, _markdown: &str) -> bool {
    self.write_text(text)
  }
}

/// What lurq copied last with its Markdown beside it. The OS clipboard holds one text, so a copy of formatted text
/// gives it the Markdown, which another app or another lurq process reads as it is; this process keeps the plain
/// text and the Markdown beside it, so that while the clipboard still holds what was written a plain input of the
/// app pastes the plain text and a styled one the Markdown.
#[cfg(feature = "clipboard")]
#[cfg_attr(not(feature = "markdown"), allow(dead_code))]
struct CopiedRich {
  /// The selection's text, as a plain input pastes it.
  plain: String,
  /// The selection with its marks and the kinds of its paragraphs.
  markdown: String,
  /// What the OS clipboard was given: the Markdown where the selection has formatting, else the plain text.
  written: String,
}

#[cfg(feature = "clipboard")]
static COPIED_RICH: parking_lot::Mutex<Option<CopiedRich>> = parking_lot::Mutex::new(None);

/// A text as two clipboards hold the same lines.
#[cfg(feature = "clipboard")]
fn same_lines(a: &str, b: &str) -> bool {
  a == b
    || a.replace(
      "
", "
",
    ) == b.replace(
      "
", "
",
    )
}

/// The Markdown lurq copied beside `text`, where `text` is the plain text of what it copied last.
#[cfg(all(feature = "clipboard", not(target_arch = "wasm32")))]
fn copied_markdown(text: &str) -> Option<String> {
  let copied = COPIED_RICH.lock();
  let copied = copied.as_ref()?;
  same_lines(&copied.plain, text).then(|| copied.markdown.clone())
}

/// What lurq copied last, where the OS clipboard still holds what it wrote: the plain text and the Markdown.
#[cfg(feature = "clipboard")]
pub(crate) fn copied_rich(clipboard_text: &str) -> Option<(String, String)> {
  let copied = COPIED_RICH.lock();
  let copied = copied.as_ref()?;
  same_lines(&copied.written, clipboard_text).then(|| (copied.plain.clone(), copied.markdown.clone()))
}

/// The OS clipboard.
#[cfg(feature = "clipboard")]
struct OsClipboard;

#[cfg(all(feature = "clipboard", not(target_arch = "wasm32")))]
impl ClipboardReader for OsClipboard {
  fn read_text(&self) -> Option<String> {
    let text = crate::clipboard::read_from_clipboard()?;
    // The Markdown this app wrote is the plain text it copied, to an input of the app.
    Some(copied_rich(&text).map_or(text, |(plain, _)| plain))
  }

  fn read(&self) -> Option<ClipboardContent> {
    let content = crate::clipboard::read_clipboard()?;
    match content.text_content().and_then(copied_rich) {
      Some((plain, _)) => Some(content.with_text(plain)),
      None => Some(content),
    }
  }

  fn read_markdown(&self, text: &str) -> Option<String> {
    copied_markdown(text)
  }
}

/// A browser answers a read of the clipboard through a promise: the paste is
/// asked for here and delivered by the shell once the text has arrived, as
/// [`Tree::paste`]. See `clipboard::web`.
#[cfg(all(feature = "clipboard", target_arch = "wasm32"))]
impl ClipboardReader for OsClipboard {
  fn read_text(&self) -> Option<String> {
    crate::clipboard::web::request_paste();
    None
  }

  fn read(&self) -> Option<ClipboardContent> {
    crate::clipboard::web::request_paste();
    None
  }
}

impl Tree {
  /// Pastes `content` into the focused text input, as Ctrl+V pastes the OS
  /// clipboard: the input's `on_paste` handlers and its ancestors' see the
  /// content first, and unless one prevents the default, the text is
  /// inserted at the caret. A focused element that is not an input gets the
  /// event where it or an ancestor has an `on_paste` handler, and nothing is
  /// inserted. `false` when nothing has focus, or nothing was inserted and no
  /// handler prevented it or heard it.
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
    if synthetic || is_synthetic_input() {
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

  /// Where copies and cuts write instead of the OS clipboard: a test's
  /// clipboard, which the OS never sees. `None` goes back to the OS clipboard
  /// (feature `clipboard`; nothing without it).
  pub fn set_clipboard_writer(&mut self, writer: Option<Arc<dyn ClipboardWriter>>) {
    self.clipboard_writer = writer;
  }

  /// Writes `text` where copies go: whether it was written.
  pub(super) fn write_clipboard_text(&self, text: &str) -> bool {
    match &self.clipboard_writer {
      Some(writer) => writer.write_text(text),
      None => {
        // What was copied with its Markdown is no longer what the clipboard holds.
        #[cfg(feature = "clipboard")]
        COPIED_RICH.lock().take();
        super::write_os_clipboard_text(text)
      }
    }
  }

  /// Writes `text` where copies go, with its Markdown beside it: whether it was written. The OS clipboard, which
  /// holds one text, is given the Markdown where the text has formatting (marks, a picture, or a paragraph that is
  /// not body text), so that another app or another lurq process has it; this process keeps the plain text beside
  /// it.
  #[cfg(feature = "markdown")]
  pub(super) fn write_clipboard_rich(&self, text: &str, markdown: &str) -> bool {
    match &self.clipboard_writer {
      Some(writer) => writer.write_rich(text, markdown),
      None => {
        let rich = crate::components::RichText::from_markdown(markdown);
        let formatted = !rich.spans.is_empty()
          || !rich.objects.is_empty()
          || rich
            .paragraphs
            .iter()
            .any(|kind| *kind != crate::components::ParagraphKind::Body);
        let written = if formatted { markdown } else { text };
        let done = super::write_os_clipboard_text(written);
        #[cfg(feature = "clipboard")]
        {
          *COPIED_RICH.lock() = done.then(|| CopiedRich {
            plain: text.to_owned(),
            markdown: markdown.to_owned(),
            written: written.to_owned(),
          });
        }
        done
      }
    }
  }

  /// Writes what a text input copies or cuts: its text, and beside it the Markdown of it where the input keeps
  /// its formatting through the clipboard.
  pub(super) fn write_input_copy(&self, state: &crate::node::node_kind::TextInputState, text: &str) -> bool {
    #[cfg(feature = "markdown")]
    if let Some(markdown) = state.clipboard_markdown() {
      return self.write_clipboard_rich(text, &markdown);
    }
    #[cfg(not(feature = "markdown"))]
    let _ = state;
    self.write_clipboard_text(text)
  }

  /// A copy, a cut or a paste shortcut on a focused element that is not a
  /// text input, where it or an ancestor has a handler for it: the copy
  /// event's text is written to the clipboard, the paste event carries what
  /// the clipboard holds. `false` where it is no such shortcut, or nothing
  /// hears it.
  pub(super) fn clipboard_shortcut_on_focus(&mut self, keyboard: &KeyboardEvent) -> bool {
    let (key, code) = (keyboard.key.as_str(), keyboard.code.as_str());
    let shortcut = Self::platform_shortcut_modifier(keyboard.ctrl, keyboard.meta);
    let plain_ctrl = keyboard.ctrl && !keyboard.shift && !keyboard.alt;
    let copy = (shortcut && matches!((key, code), ("c" | "C", _) | (_, "KeyC")))
      || (plain_ctrl && matches!((key, code), ("Insert", _) | (_, "Insert")));
    let cut = (shortcut && matches!((key, code), ("x" | "X", _) | (_, "KeyX")))
      || (keyboard.shift && !keyboard.ctrl && matches!((key, code), ("Delete", _) | (_, "Delete")));
    let paste = (shortcut && matches!((key, code), ("v" | "V", _) | (_, "KeyV")))
      || (keyboard.shift && !keyboard.ctrl && matches!((key, code), ("Insert", _) | (_, "Insert")));
    if !(copy || cut || paste) || keyboard.alt {
      return false;
    }
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
    if matches!(focused.node_kind(), NodeKind::TextInput { .. }) {
      return false;
    }
    if paste {
      return !is_synthetic_input() && self.paste_from_reader(keyboard);
    }
    let chain = nodes_along_path(root, &path);
    if chain.iter().all(|node| node.events.on_copy.is_empty()) {
      return false;
    }
    let event = CopyEvent::new(focused.node_id(), cut);
    dispatch_copy(&chain, &event);
    if let Some(text) = event.text() {
      self.write_clipboard_text(&text);
    }
    true
  }

  fn paste_from_reader(&mut self, keyboard: &KeyboardEvent) -> bool {
    let Some(reader) = self.clipboard_reader.clone().or_else(os_clipboard) else {
      return false;
    };
    self.paste_content(
      |wants_everything| {
        let content = if wants_everything {
          reader.read()
        } else {
          reader.read_text().map(ClipboardContent::text)
        }?;
        // The Markdown a styled input copied beside this text, where the clipboard keeps it.
        let markdown = match (content.markdown_content(), content.text_content()) {
          (None, Some(text)) => reader.read_markdown(text),
          _ => None,
        };
        Some(match markdown {
          Some(markdown) => content.with_markdown(markdown),
          None => content,
        })
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
      // An element that is not an input: its handlers and its ancestors' take the content, and nothing is
      // inserted.
      let chain = nodes_along_path(root, &path);
      if chain.iter().all(|node| node.events.on_paste.is_empty()) {
        return false;
      }
      let Some(content) = read(true) else {
        return false;
      };
      let event = PasteEvent::new(Arc::new(content), input.node_id());
      dispatch_paste(&chain, &event);
      return true;
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
    // An input that keeps its formatting through the clipboard takes the Markdown beside the text, or a text that
    // reads as Markdown, with its marks and kinds.
    #[cfg(feature = "markdown")]
    if let Some(pasted) = state.paste_rich(text, event.markdown(), keyboard) {
      return pasted;
    }
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

/// Runs the `on_copy` handlers of the focused element, then of each ancestor
/// up to the root, until one stops the propagation.
fn dispatch_copy(chain: &[&Node], event: &CopyEvent) {
  for node in chain.iter().rev() {
    let event = event.at(node.node_id());
    for handler in &node.events.on_copy {
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
