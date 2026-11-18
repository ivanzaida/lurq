use std::sync::{
  Arc,
  atomic::{AtomicBool, Ordering},
};

use super::KeyboardEvent;
use crate::{
  app::{Accelerator, SyntheticModifiers},
  core::NodeId,
};

/// A command of editing: what the keys every app shares ask for (Ctrl+Z, Ctrl+C, Ctrl+V and the rest; Cmd on a Mac),
/// and an Edit menu's items.
///
/// It is the one place in lurq that knows which keys they are: a key's command is
/// [`KeyboardEvent::edit_command`], and lurq runs it on what has the focus, a focused text input first and then the
/// focused element and its ancestors, innermost first: `on_undo`, `on_redo` and `on_select_all` handlers
/// ([`EditEvent`]), `on_copy` for a copy and a cut ([`CopyEvent`](super::CopyEvent)), `on_paste`
/// ([`PasteEvent`](super::PasteEvent)). A menu or a toolbar runs it without a key with
/// [`Ctx::edit`](crate::app::ctx::Ctx::edit).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EditCommand {
  /// Takes back the last change: Ctrl+Z.
  Undo,
  /// Does again the change last taken back: Ctrl+Shift+Z and Ctrl+Y (Cmd+Shift+Z on a Mac).
  Redo,
  /// Puts what is selected on the clipboard and takes it away: Ctrl+X and Shift+Delete.
  Cut,
  /// Puts what is selected on the clipboard: Ctrl+C and Ctrl+Insert.
  Copy,
  /// Puts in what the clipboard holds: Ctrl+V and Shift+Insert.
  Paste,
  /// Selects all of what has the focus: Ctrl+A.
  SelectAll,
}

/// Whether the platform is Apple's, whose shortcut key is Command and which has no Ctrl+Y.
const APPLE: bool = cfg!(any(target_os = "macos", target_os = "ios"));

impl EditCommand {
  /// Every command, in the order an Edit menu has them.
  pub const ALL: [Self; 6] = [
    Self::Undo,
    Self::Redo,
    Self::Cut,
    Self::Copy,
    Self::Paste,
    Self::SelectAll,
  ];

  /// The command a key that went down asks for, on the platform lurq is built for; see
  /// [`KeyboardEvent::edit_command`].
  pub fn of_key(event: &KeyboardEvent) -> Option<Self> {
    Self::of_key_on(event, APPLE)
  }

  /// The command a key asks for, on Apple's platforms (`apple`) or another's.
  pub(crate) fn of_key_on(event: &KeyboardEvent, apple: bool) -> Option<Self> {
    if event.alt {
      return None;
    }
    let named = |name: &str| event.key == name || event.code == name;
    // The keys of the old IBM keyboard, which Windows and Linux keep: Ctrl+Insert, Shift+Insert and Shift+Delete.
    if named("Insert") && event.ctrl && !event.shift {
      return Some(Self::Copy);
    }
    if named("Insert") && event.shift && !event.ctrl {
      return Some(Self::Paste);
    }
    if named("Delete") && event.shift && !event.ctrl {
      return Some(Self::Cut);
    }
    let shortcut = if apple { event.meta } else { event.ctrl };
    if !shortcut {
      return None;
    }
    // The letter the key says where it is a Latin one, which is where a layout puts it (on a German keyboard Z is
    // said by the key in Y's place, and Y by the one in Z's). A key that says another letter, as "я" in a Ukrainian
    // layout, is read by its place: the letter of a US keyboard there.
    let mut chars = event.key.chars();
    let letter = match (chars.next(), chars.next()) {
      (Some(letter), None) if letter.is_ascii_alphabetic() => letter.to_ascii_lowercase(),
      _ => match event.code.strip_prefix("Key").map(str::as_bytes) {
        Some([letter]) => letter.to_ascii_lowercase() as char,
        _ => return None,
      },
    };
    match letter {
      'z' if event.shift => Some(Self::Redo),
      'z' => Some(Self::Undo),
      'y' if !event.shift && !apple => Some(Self::Redo),
      'x' => Some(Self::Cut),
      'c' => Some(Self::Copy),
      'v' => Some(Self::Paste),
      'a' if !event.shift => Some(Self::SelectAll),
      _ => None,
    }
  }

  /// The combination that runs the command on the platform lurq is built for, to show beside an Edit menu's item
  /// or in a tool's tooltip, or to give a native menu's item: Ctrl+Z, Ctrl+Y, Ctrl+X, Ctrl+C, Ctrl+V and Ctrl+A,
  /// or Cmd and Cmd+Shift+Z for redo on Apple's platforms. The other keys of a command (Ctrl+Shift+Z,
  /// Shift+Insert) run it too, but are not the one shown.
  pub fn accelerator(self) -> Accelerator {
    let modifiers = if APPLE {
      SyntheticModifiers::default().meta()
    } else {
      SyntheticModifiers::default().ctrl()
    };
    match self {
      Self::Undo => Accelerator::new("z", modifiers),
      Self::Redo if APPLE => Accelerator::new("z", modifiers.shift()),
      Self::Redo => Accelerator::new("y", modifiers),
      Self::Cut => Accelerator::new("x", modifiers),
      Self::Copy => Accelerator::new("c", modifiers),
      Self::Paste => Accelerator::new("v", modifiers),
      Self::SelectAll => Accelerator::new("a", modifiers),
    }
  }

  /// The key a command run without one reports, where a key is asked for (a text input's `on_input`): the
  /// command's [`accelerator`](Self::accelerator), from no element.
  pub(crate) fn keyboard_event(self) -> KeyboardEvent {
    let accelerator = self.accelerator();
    let m = accelerator.modifiers;
    let code = format!("Key{}", accelerator.key.to_ascii_uppercase());
    let key = if m.shift {
      accelerator.key.to_ascii_uppercase()
    } else {
      accelerator.key.to_string()
    };
    KeyboardEvent::new(key, code, m.shift, m.ctrl, m.alt, m.meta, NodeId::UNASSIGNED)
  }
}

/// An undo, a redo or a select all on the focused element, or on an ancestor it went out to: what an element's
/// `on_undo`, `on_redo` or `on_select_all` handler is told. The handler takes back (or does again, or selects) what
/// is its own; one that had nothing to do says so with [`pass_on`](Self::pass_on), and the command goes on out to the
/// elements around it, innermost first, until one takes it. A pad with no stroke to take back, in a form whose app
/// keeps its own history.
#[derive(Clone)]
pub struct EditEvent {
  command: EditCommand,
  /// The node whose handlers run now: the focused element, or an ancestor the command went out to.
  pub target_id: NodeId,
  /// The focused element the command was run on.
  pub focus_id: NodeId,
  /// Whether the handler being called passed the command on.
  passed: Arc<AtomicBool>,
}

impl EditEvent {
  pub(crate) fn new(command: EditCommand, focus_id: NodeId) -> Self {
    Self {
      command,
      target_id: focus_id,
      focus_id,
      passed: Arc::new(AtomicBool::new(false)),
    }
  }

  /// The event as the handlers of `target_id` are told it.
  pub(crate) fn at(&self, target_id: NodeId) -> Self {
    Self {
      command: self.command,
      target_id,
      focus_id: self.focus_id,
      passed: Arc::clone(&self.passed),
    }
  }

  /// Undo, redo or select all.
  pub fn command(&self) -> EditCommand {
    self.command
  }

  /// Says that this handler did nothing with the command: it had nothing to take back, or to do again. Where every
  /// handler of an element passes it on, it goes on to the element around it.
  pub fn pass_on(&self) {
    self.passed.store(true, Ordering::Relaxed);
  }

  /// Whether the handler passed the command on.
  pub fn passed_on(&self) -> bool {
    self.passed.load(Ordering::Relaxed)
  }

  /// Starts the next handler afresh: it has not passed the command on.
  pub(crate) fn reset(&self) {
    self.passed.store(false, Ordering::Relaxed);
  }
}

impl std::fmt::Debug for EditEvent {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("EditEvent")
      .field("command", &self.command)
      .field("target_id", &self.target_id)
      .field("focus_id", &self.focus_id)
      .finish()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn key(key: &str, code: &str, shift: bool, ctrl: bool, alt: bool, meta: bool) -> KeyboardEvent {
    KeyboardEvent::new(key, code, shift, ctrl, alt, meta, NodeId::UNASSIGNED)
  }

  #[test]
  fn apple_takes_command_and_has_no_y() {
    let of = |event: &KeyboardEvent| EditCommand::of_key_on(event, true);
    assert_eq!(
      of(&key("z", "KeyZ", false, false, false, true)),
      Some(EditCommand::Undo)
    );
    assert_eq!(of(&key("Z", "KeyZ", true, false, false, true)), Some(EditCommand::Redo));
    assert_eq!(
      of(&key("я", "KeyZ", false, false, false, true)),
      Some(EditCommand::Undo)
    );
    assert_eq!(
      of(&key("z", "KeyZ", false, true, false, false)),
      None,
      "Ctrl is no shortcut on a Mac"
    );
    assert_eq!(
      of(&key("y", "KeyY", false, false, false, true)),
      None,
      "Cmd+Y is no redo on a Mac"
    );
    assert_eq!(of(&key("z", "KeyZ", false, false, true, true)), None, "Option held");
    assert_eq!(
      of(&key("с", "KeyC", false, false, false, true)),
      Some(EditCommand::Copy)
    );
    assert_eq!(
      of(&key("м", "KeyV", false, false, false, true)),
      Some(EditCommand::Paste)
    );
    assert_eq!(
      of(&key("a", "KeyA", false, false, false, true)),
      Some(EditCommand::SelectAll)
    );
    assert_eq!(
      of(&key("c", "KeyC", false, true, false, false)),
      None,
      "Ctrl+C is no copy on a Mac"
    );
  }

  #[test]
  fn elsewhere_takes_ctrl_and_y() {
    let of = |event: &KeyboardEvent| EditCommand::of_key_on(event, false);
    assert_eq!(
      of(&key("z", "KeyZ", false, true, false, false)),
      Some(EditCommand::Undo)
    );
    assert_eq!(
      of(&key("y", "KeyY", false, true, false, false)),
      Some(EditCommand::Redo)
    );
    assert_eq!(of(&key("Y", "KeyY", true, true, false, false)), None, "Ctrl+Shift+Y");
    assert_eq!(
      of(&key("z", "KeyZ", false, false, false, true)),
      None,
      "Meta is no shortcut here"
    );
  }

  #[test]
  fn a_german_keyboard_is_read_by_the_letters_it_says() {
    let of = |event: &KeyboardEvent| EditCommand::of_key_on(event, false);
    // On a German keyboard Z is where a US one has Y, and Y where it has Z.
    assert_eq!(
      of(&key("z", "KeyY", false, true, false, false)),
      Some(EditCommand::Undo)
    );
    assert_eq!(
      of(&key("y", "KeyZ", false, true, false, false)),
      Some(EditCommand::Redo)
    );
  }

  #[test]
  fn a_command_without_a_key_reports_its_accelerator() {
    use EditCommand::*;
    for command in [Undo, Redo, Cut, Copy, Paste, SelectAll] {
      assert_eq!(EditCommand::of_key(&command.keyboard_event()), Some(command));
      let accelerator = command.accelerator();
      assert!(accelerator.matches(&command.keyboard_event()), "{command:?}");
    }
  }
}
