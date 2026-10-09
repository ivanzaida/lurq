//! Focus-visible: whether the focused element shows a focus ring, like CSS `:focus-visible`.
//!
//! The tree remembers the modality of the last input that can move focus:
//!
//! - Keyboard: Tab / Shift+Tab, and any key press other than a bare modifier (Shift, Control, Alt, AltGraph,
//!   Super/Meta) made without Control, Alt or Super/Meta held. A shortcut such as Ctrl+C on a clicked button leaves the
//!   modality as it was; a plain key press (arrows, Space, Enter, Escape, letters) turns the ring on, like a browser.
//! - Pointer: a mouse button press anywhere in the window (also one that closes a popup), and
//!   [`super::ElementHandle::click`]. It turns the ring of the current focus off.
//!
//! Focus is visible while the modality is keyboard, so a programmatic focus request (`Ctx::focus`,
//! [`super::ElementHandle::focus`]) inherits the last input's modality: a request made from a key handler shows the
//! ring, one made from a click handler does not. Before any input the modality is pointer: an app that focuses
//! something at startup shows no ring until the user touches the keyboard.
//!
//! A focused text input is always focus-visible, whatever moved focus there, as browsers do for text fields: the
//! caret alone does not show which field takes the keys.

use super::{Tree, find_node_by_path};
use crate::node::{Node, node_kind::NodeKind};

/// What the last focus-moving input was.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum InputModality {
  #[default]
  Pointer,
  Keyboard,
}

/// Bare modifier presses (as the winit shell and the MCP held-input names report them) do not count as keyboard use.
const MODIFIER_KEYS: [&str; 6] = ["Shift", "Control", "Alt", "AltGraph", "Super", "Meta"];

/// A key press that makes the focus visible: not a bare modifier and not part of a Control / Alt / Super shortcut.
fn is_keyboard_use(key: &str, ctrl: bool, alt: bool, meta: bool) -> bool {
  !(ctrl || alt || meta || MODIFIER_KEYS.contains(&key))
}

/// Whether `input`, the focused control, shows its focus ring under `modality`.
pub(super) fn shows_focus_ring(modality: InputModality, input: &Node) -> bool {
  modality == InputModality::Keyboard || matches!(input.node_kind(), NodeKind::TextInput { .. })
}

/// Sets `node`'s focus-visible flag everywhere a reader looks: its style state, its
/// [`crate::node::interaction_state::InteractionState`] and its element ref.
pub(super) fn set_node_focus_visible(node: &Node, visible: bool) {
  node.set_style_focus_visible(visible);
  if let Some(state) = &node.interaction {
    state.set_focus_visible(visible);
  }
  if let Some(element_ref) = &node.element_ref {
    element_ref.set_focus_visible(visible);
  }
}

impl Tree {
  /// Whether the focused element is focus-visible, like CSS `:focus-visible`, and so shows its
  /// `focus_visible_style`; `false` when nothing has focus.
  ///
  /// Focus is visible after keyboard input (Tab, or any key press that is not a bare modifier or a
  /// Control / Alt / Super shortcut) and not after a pointer press; a focus request inherits the last
  /// input's modality, and before any input the modality is pointer. A focused text input is always
  /// focus-visible.
  pub fn focus_visible(&self) -> bool {
    let Some(root) = self.root.as_ref() else {
      return false;
    };
    self
      .focused_path
      .as_deref()
      .and_then(|path| find_node_by_path(root, path))
      .is_some_and(Node::is_style_focus_visible)
  }

  /// A key press: switches to keyboard modality unless it is a bare modifier or a shortcut.
  pub(super) fn note_key_press_modality(&mut self, key: &str, ctrl: bool, alt: bool, meta: bool) {
    if is_keyboard_use(key, ctrl, alt, meta) {
      self.set_input_modality(InputModality::Keyboard);
    }
  }

  /// Records the modality of the input being handled and updates the current focus's ring.
  pub(super) fn set_input_modality(&mut self, modality: InputModality) {
    if self.input_modality == modality {
      return;
    }
    self.input_modality = modality;
    if self.focused_node.is_some() {
      self.refresh_focus_ids();
      self.needs_redraw = true;
    }
  }
}
