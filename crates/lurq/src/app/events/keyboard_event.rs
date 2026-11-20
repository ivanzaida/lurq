use std::sync::Arc;

use super::{EditCommand, EventControl, NavigationCommand};
use crate::core::NodeId;

#[derive(Clone)]
pub struct KeyboardEvent {
  pub key: String,
  pub code: String,
  pub shift: bool,
  pub ctrl: bool,
  pub alt: bool,
  pub meta: bool,
  pub target_id: NodeId,
  /// Whether a text input holds keyboard focus when this event fires.
  /// `on_key_down` handlers run before the focused input consumes the key,
  /// so global shortcuts should check this and step aside while the user is
  /// typing — the input's own editing takes the key otherwise. An app's own
  /// undo, redo, copy and paste are no shortcuts to check for: they are
  /// `on_undo`, `on_redo`, `on_copy` and `on_paste` handlers, which a focused
  /// input's own editing goes before ([`edit_command`](Self::edit_command)).
  pub text_input_focused: bool,
  /// Whether this key belongs to an input method (IME) composition, like the
  /// DOM's `isComposing`.
  ///
  /// The keys the input method takes never arrive as presses: Windows
  /// reports them as `Process` (the `Enter` that confirms a composition
  /// included) and macOS not at all, so `on_key_down` handlers and lurq's own
  /// key handling do not see them. A press that does arrive while a
  /// composition is shown, a key the input method passed on, has this set,
  /// and lurq handles it as usual, except an `Enter`: no lurq default (form
  /// submit, button or select activation, newline, blur) acts on it. A
  /// handler that sends on `Enter` can skip it the same way. Releases of the
  /// input method's keys reach `on_key_up` handlers with this set. It is also
  /// set on the event a committed composition passes to `on_input` handlers
  /// (`TextInputEvent::keyboard`), whose `key` is the committed text.
  pub composing: bool,
  /// Whether the keyboard's focus is on the node whose handler is being
  /// called, or on a node inside it. A key event reaches every node's
  /// `on_key_down`, focused or not: a handler that acts only for what has the
  /// focus (an item of a menu opening its submenu on Right) checks this.
  pub focus_within: bool,
  /// Whether what has the keyboard's focus does a thing of its own with Enter: a button (which it presses), a link
  /// (which it follows), a select, or a text input of several lines (where it starts a line). A handler that gives
  /// Enter a meaning of its own around them, as a dialog that confirms on it, steps aside where this is set. A
  /// text input of one line is not one: Enter there submits a form at most.
  pub focus_takes_enter: bool,
  pub(crate) control: EventControl,
}

impl KeyboardEvent {
  pub fn new(
    key: impl Into<String>,
    code: impl Into<String>,
    shift: bool,
    ctrl: bool,
    alt: bool,
    meta: bool,
    target_id: NodeId,
  ) -> Self {
    Self {
      key: key.into(),
      code: code.into(),
      shift,
      ctrl,
      alt,
      meta,
      target_id,
      text_input_focused: false,
      composing: false,
      focus_within: false,
      focus_takes_enter: false,
      control: EventControl::new(),
    }
  }

  /// The command of editing the key is, if it is one, on the platform lurq is built for. Never with Alt held. With
  /// the shortcut key (Ctrl, or Command on macOS and iOS): Z is [`EditCommand::Undo`] and Shift+Z
  /// [`EditCommand::Redo`], as Y without Shift is except on Apple's platforms; X is [`EditCommand::Cut`], C
  /// [`EditCommand::Copy`] and V [`EditCommand::Paste`], with Shift or without; A without Shift is
  /// [`EditCommand::SelectAll`]. Ctrl+Insert is a copy, Shift+Insert a paste and Shift+Delete a cut. A letter is
  /// read as the key says it where it is a Latin one, and by the key's place where it is another (in a Ukrainian
  /// layout Z says "я" and C "с"), so the keys work in every layout.
  ///
  /// This is the one place lurq knows these keys: lurq runs the command on what has the focus (`on_undo`,
  /// `on_redo`, `on_select_all`, `on_copy`, `on_paste`), and a component never matches them itself.
  pub fn edit_command(&self) -> Option<EditCommand> {
    EditCommand::of_key(self)
  }

  /// Going back or forward, if the key is one, on the platform lurq is built for: Alt+← is
  /// [`NavigationCommand::Back`] and Alt+→ [`NavigationCommand::Forward`], or Cmd+[ and Cmd+] on Apple's platforms,
  /// where Option and an arrow move the caret by a word. A bracket is read as the key says it, and by the key's place
  /// where it says a letter of another alphabet. A keyboard's own Back and Forward keys, and Android's back, are the
  /// commands on every platform.
  ///
  /// This is the one place lurq knows these keys: lurq runs the command on what has the focus (`on_history_back`,
  /// `on_history_forward`, then the router), and a component never matches them itself. A text input does not take
  /// Alt and an arrow on Windows and Linux (its word moves are Ctrl's), so they go back in it too, as in a browser.
  pub fn navigation_command(&self) -> Option<NavigationCommand> {
    NavigationCommand::of_key(self)
  }

  pub fn prevent_default(&self) {
    self.control.prevent_default();
  }

  pub fn default_prevented(&self) -> bool {
    self.control.default_prevented()
  }

  pub fn stop_propagation(&self) {
    self.control.stop_propagation();
  }

  pub fn propagation_stopped(&self) -> bool {
    self.control.propagation_stopped()
  }

  pub fn stop_immediate_propagation(&self) {
    self.control.stop_immediate_propagation();
  }

  pub fn immediate_propagation_stopped(&self) -> bool {
    self.control.immediate_propagation_stopped()
  }

  /// Moves the focus, once this key's handlers have run and what they changed is rendered, to the element of the
  /// [focus group](crate::node::FocusGroup) around the focused element whose key (`.key(..)`) is `key`, as the
  /// group's arrows move it: from the keyboard, so that it shows its ring, and into view. The key's default is
  /// prevented, so the group does not move the focus as well. For a group whose elements are reached by what the app
  /// knows of them and not by their order: a calendar's day a week on, which may be in the month the key shows next.
  /// Nothing moves where the group is not there after the render, or has no element of that key that takes the
  /// focus. The last request of a key's handlers wins.
  ///
  /// ```
  /// use lurq::{app::events::KeyboardEvent, components::{Button, Row}, node::FocusGroup};
  ///
  /// // Each day says where the keys take the focus from it: Right to the next day, by its key.
  /// let day = |n: u32| {
  ///   Button::new(&n.to_string()).tab_index(0).key(format!("day-{n}")).on_key_down(move |event: KeyboardEvent| {
  ///     if event.focus_within && event.key == "ArrowRight" {
  ///       event.focus_key(format!("day-{}", n + 1));
  ///     }
  ///   })
  /// };
  /// let week = Row::new().focus_group(FocusGroup::horizontal().one_stop(true)).child(day(1)).child(day(2));
  /// ```
  pub fn focus_key(&self, key: impl Into<Arc<str>>) {
    self.control.prevent_default();
    self.control.request_focus_key(key.into());
  }
}
