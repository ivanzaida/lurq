//! An element that takes every key of the window: one built with `.captures_keys(true)` (see
//! `NodeUpdate::captures_keys`). While one is in the tree a key goes to its own `on_key_down` and `on_key_up`
//! handlers alone, before anything the window does with a key: no default of the runtime (Tab, Enter, Escape, the keys
//! of a focus group, a row of menus' Alt, typing), no other element's handlers. A host that handles keys before the
//! window (the key equivalents of a Mac's menu) asks [`keys_captured`] first, so that the accelerators of the app's
//! menu are not run either.

use std::sync::atomic::{AtomicUsize, Ordering};

use super::{Tree, find_node_by_path};
use crate::{
  app::events::{EventControl, KeyboardEvent},
  node::Node,
};

/// How many windows of the process have an element that takes every key.
static CAPTURING_WINDOWS: AtomicUsize = AtomicUsize::new(0);

/// Whether an element of a window of the process takes every key now. Asked by a Mac's menu before it takes a key.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub(crate) fn keys_captured() -> bool {
  CAPTURING_WINDOWS.load(Ordering::Relaxed) > 0
}

/// The path from `node` to the first element under it, in the order of the tree, that takes every key: not one that
/// is inert or disabled, or in one that is.
fn capturing_path(node: &Node, path: &mut Vec<usize>) -> bool {
  if node.is_inert() || node.is_disabled() {
    return false;
  }
  if node.captures_keys() {
    return true;
  }
  for (index, child) in node.children().iter().enumerate() {
    path.push(index);
    if capturing_path(child, path) {
      return true;
    }
    path.pop();
  }
  false
}

impl Tree {
  /// The path of the element that takes every key, where one does.
  fn key_capture(&self) -> Option<Vec<usize>> {
    let root = self.root.as_ref()?;
    let mut path = Vec::new();
    capturing_path(root, &mut path).then_some(path)
  }

  /// Notes for the process whether this window has an element that takes every key: after a rebuild, which may
  /// have built one or taken it away.
  pub(super) fn note_key_capture(&mut self) {
    self.set_key_capture(self.key_capture().is_some());
  }

  pub(super) fn set_key_capture(&mut self, captures: bool) {
    if captures != self.keys_captured {
      self.keys_captured = captures;
      if captures {
        CAPTURING_WINDOWS.fetch_add(1, Ordering::Relaxed);
      } else {
        CAPTURING_WINDOWS.fetch_sub(1, Ordering::Relaxed);
      }
    }
  }

  /// Gives a key that went down (`down`) or up to the element that takes every key, alone. Returns whether one
  /// took it: the window does nothing else with the key then.
  #[allow(clippy::too_many_arguments)]
  pub(super) fn dispatch_captured_key(
    &mut self,
    down: bool,
    key: &str,
    code: &str,
    shift: bool,
    ctrl: bool,
    alt: bool,
    meta: bool,
  ) -> bool {
    let Some(path) = self.key_capture() else {
      self.set_key_capture(false);
      return false;
    };
    let Some(node) = self.root.as_ref().and_then(|root| find_node_by_path(root, &path)) else {
      return false;
    };
    let mut event = KeyboardEvent {
      key: key.to_owned(),
      code: code.to_owned(),
      shift,
      ctrl,
      alt,
      meta,
      target_id: node.node_id(),
      text_input_focused: false,
      composing: false,
      focus_within: true,
      control: EventControl::new(),
    };
    let handlers = if down {
      &node.events.on_key_down
    } else {
      &node.events.on_key_up
    };
    for handler in handlers {
      handler.call(&mut event);
      if event.immediate_propagation_stopped() {
        break;
      }
    }
    // A key taken here is no Alt alone, nor one pressed with it.
    self.alt_alone = false;
    self.needs_redraw = true;
    self.apply_reactive_updates_after_event();
    self.note_key_capture();
    true
  }
}
