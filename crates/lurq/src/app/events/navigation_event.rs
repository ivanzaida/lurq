use std::sync::{
  Arc,
  atomic::{AtomicBool, Ordering},
};

use super::{KeyboardEvent, MouseButton};
use crate::{
  app::{Accelerator, SyntheticModifiers},
  core::NodeId,
};

/// Going back or going forward: what every source of it asks for, in one place, as [`EditCommand`](super::EditCommand)
/// is for editing.
///
/// Its sources are the mouse's side buttons ([`MouseButton::Back`] and [`MouseButton::Forward`], on their release),
/// Alt+← and Alt+→ (Cmd+[ and Cmd+] on Apple's platforms), a keyboard's own Back and Forward keys and Android's
/// back ([`KeyboardEvent::navigation_command`]), a trackpad's swipe between pages and a finger in from the edge of
/// the window (`on_history_swipe`, whose default it is), a browser's back and forward buttons (the router that
/// follows the page's address), and an app that asks without any of them:
/// [`Ctx::history_back`](crate::app::ctx::Ctx::history_back),
/// [`WindowHandle::history_back`](crate::app::window::WindowHandle::history_back), a back button of a stack of
/// screens, MCP's `lurq_interact` actions `history_back` and `history_forward`.
///
/// lurq runs it on the elements from where it was asked outward, innermost first: their `on_history_back` or
/// `on_history_forward` handlers ([`HistoryEvent`]) and the routers mounted there, until one takes it; with none
/// taking it, the routers mounted elsewhere in the window, the innermost first. See
/// [`HistoryEvent`] for where it starts.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NavigationCommand {
  /// Goes back to where one was before: Alt+← (Cmd+[ on Apple's platforms), the mouse's back button.
  Back,
  /// Goes forward again to where one went back from: Alt+→ (Cmd+] on Apple's platforms), the mouse's forward
  /// button.
  Forward,
}

/// Whether the platform is Apple's, whose keys for going back are Command and the brackets: Option and an arrow
/// move the caret by a word there.
const APPLE: bool = cfg!(any(target_os = "macos", target_os = "ios"));

impl NavigationCommand {
  /// The command a key that went down asks for, on the platform lurq is built for; see
  /// [`KeyboardEvent::navigation_command`].
  pub fn of_key(event: &KeyboardEvent) -> Option<Self> {
    Self::of_key_on(event, APPLE)
  }

  /// The command a key asks for, on Apple's platforms (`apple`) or another's.
  pub fn of_key_on(event: &KeyboardEvent, apple: bool) -> Option<Self> {
    let named = |name: &str| event.key == name || event.code == name;
    // A key of its own for it: a keyboard's Back and Forward keys, and Android's back, which winit names so.
    if !event.ctrl && !event.alt && !event.meta {
      if named("BrowserBack") {
        return Some(Self::Back);
      }
      if named("BrowserForward") {
        return Some(Self::Forward);
      }
    }
    if event.shift {
      return None;
    }
    if apple {
      if !event.meta || event.ctrl || event.alt {
        return None;
      }
      // The bracket the key says; a key that says a letter of another alphabet (Ukrainian "х" and "ї", German "ü"
      // and "+") is read by its place, the brackets of a US keyboard.
      return match event.key.as_str() {
        "[" => Some(Self::Back),
        "]" => Some(Self::Forward),
        key if key.is_ascii() && !key.is_empty() && key != "Dead" => None,
        _ => match event.code.as_str() {
          "BracketLeft" => Some(Self::Back),
          "BracketRight" => Some(Self::Forward),
          _ => None,
        },
      };
    }
    if !event.alt || event.ctrl || event.meta {
      return None;
    }
    if named("ArrowLeft") {
      Some(Self::Back)
    } else if named("ArrowRight") {
      Some(Self::Forward)
    } else {
      None
    }
  }

  /// The command a mouse's button is: its back button goes back and its forward button forward.
  pub fn of_button(button: MouseButton) -> Option<Self> {
    match button {
      MouseButton::Back => Some(Self::Back),
      MouseButton::Forward => Some(Self::Forward),
      _ => None,
    }
  }

  /// The combination that runs the command on the platform lurq is built for, to show beside a menu's item or in a
  /// tooltip: Alt+← and Alt+→, or Cmd+[ and Cmd+] on Apple's platforms.
  pub fn accelerator(self) -> Accelerator {
    Self::accelerator_on(self, APPLE)
  }

  pub(crate) fn accelerator_on(self, apple: bool) -> Accelerator {
    if apple {
      let meta = SyntheticModifiers::default().meta();
      match self {
        Self::Back => Accelerator::new("[", meta),
        Self::Forward => Accelerator::new("]", meta),
      }
    } else {
      let alt = SyntheticModifiers::default().alt();
      match self {
        Self::Back => Accelerator::new("ArrowLeft", alt),
        Self::Forward => Accelerator::new("ArrowRight", alt),
      }
    }
  }

  /// The other way.
  pub fn reversed(self) -> Self {
    match self {
      Self::Back => Self::Forward,
      Self::Forward => Self::Back,
    }
  }
}

/// What asked to go back or forward: what a [`HistoryEvent`] says it came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum NavigationSource {
  /// A key: Alt+← or Alt+→ (Cmd+[ or Cmd+] on Apple's platforms), a keyboard's Back or Forward key, Android's back.
  Key,
  /// The mouse's back or forward button.
  Mouse,
  /// A trackpad's swipe between pages.
  Swipe,
  /// A finger in from the left or the right edge of the window.
  Edge,
  /// A browser's back or forward button, or its history moved by the page's own script, for the router that follows
  /// the page's address.
  Browser,
  /// The app, with no input of its own: `Ctx::history_back`, `WindowHandle::history_back`, a back button lurq draws
  /// (a stack's), an MCP client.
  App,
}

/// What a router mounted in an element does with a command that reached it: whether it moved.
pub(crate) type HistoryMover = Arc<dyn Fn(NavigationCommand, NavigationSource) -> bool + Send + Sync>;

/// Going back or forward on an element: what its `on_history_back` or `on_history_forward` handler is told
/// ([`NavigationCommand`]).
///
/// A handler that has something of its own to go back from does it (a dialog closes, a wizard goes to its last step,
/// a viewer to the last picture); one that has nothing to do says so with [`pass_on`](Self::pass_on), and the
/// command goes on out to the elements around it, innermost first, until one takes it. A router mounted in an
/// element takes it after the element's own handlers when it can move; with nothing taking it, the routers mounted
/// elsewhere in the window are asked, the innermost first.
///
/// Where it starts:
///
/// - From a key, Android's back, a browser's buttons and an app's request: at the focused element. With nothing
///   focused, at the last element in the tree's order that has a handler for the command (the topmost of what is open),
///   or at the window's root.
/// - From the mouse's side buttons and a swipe (a trackpad's, a finger's from the edge): at the element under the
///   pointer, then, where nothing there took it, as from a key.
/// - From a part of the window ([`HistoryRunner::run_in`](crate::app::ctx::HistoryRunner::run_in), a stack's back
///   button): at the focused element where it is in that part, otherwise at the last element of the part that has a
///   handler for it, or at the part.
#[derive(Clone)]
pub struct HistoryEvent {
  command: NavigationCommand,
  source: NavigationSource,
  /// The node whose handlers run now: where the command started, or an ancestor it went out to.
  pub target_id: NodeId,
  /// Where the command started.
  pub origin_id: NodeId,
  /// Whether the handler being called passed the command on.
  passed: Arc<AtomicBool>,
}

impl HistoryEvent {
  pub(crate) fn new(command: NavigationCommand, source: NavigationSource, origin_id: NodeId) -> Self {
    Self {
      command,
      source,
      target_id: origin_id,
      origin_id,
      passed: Arc::new(AtomicBool::new(false)),
    }
  }

  /// The event as the handlers of `target_id` are told it.
  pub(crate) fn at(&self, target_id: NodeId) -> Self {
    Self {
      command: self.command,
      source: self.source,
      target_id,
      origin_id: self.origin_id,
      passed: Arc::clone(&self.passed),
    }
  }

  /// Back or forward.
  pub fn command(&self) -> NavigationCommand {
    self.command
  }

  /// What asked for it.
  pub fn source(&self) -> NavigationSource {
    self.source
  }

  /// Says that this handler did nothing with the command: it had nowhere to go back to. Where every handler of an
  /// element passes it on, it goes on to the element around it.
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

impl std::fmt::Debug for HistoryEvent {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("HistoryEvent")
      .field("command", &self.command)
      .field("source", &self.source)
      .field("target_id", &self.target_id)
      .field("origin_id", &self.origin_id)
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
  fn elsewhere_alt_and_an_arrow() {
    let of = |event: &KeyboardEvent| NavigationCommand::of_key_on(event, false);
    use NavigationCommand::*;
    assert_eq!(
      of(&key("ArrowLeft", "ArrowLeft", false, false, true, false)),
      Some(Back)
    );
    assert_eq!(
      of(&key("ArrowRight", "ArrowRight", false, false, true, false)),
      Some(Forward)
    );
    assert_eq!(
      of(&key("ArrowLeft", "ArrowLeft", false, false, false, false)),
      None,
      "an arrow alone"
    );
    assert_eq!(
      of(&key("ArrowLeft", "ArrowLeft", false, true, true, false)),
      None,
      "Ctrl+Alt"
    );
    assert_eq!(
      of(&key("ArrowLeft", "ArrowLeft", true, false, true, false)),
      None,
      "Shift+Alt"
    );
    assert_eq!(
      of(&key("[", "BracketLeft", false, false, false, true)),
      None,
      "Meta+[ is Apple's"
    );
  }

  #[test]
  fn apple_command_and_a_bracket() {
    let of = |event: &KeyboardEvent| NavigationCommand::of_key_on(event, true);
    use NavigationCommand::*;
    assert_eq!(of(&key("[", "BracketLeft", false, false, false, true)), Some(Back));
    assert_eq!(of(&key("]", "BracketRight", false, false, false, true)), Some(Forward));
    // A Ukrainian layout says "х" and "ї" there: read by the place.
    assert_eq!(of(&key("х", "BracketLeft", false, false, false, true)), Some(Back));
    assert_eq!(of(&key("ї", "BracketRight", false, false, false, true)), Some(Forward));
    assert_eq!(
      of(&key("{", "BracketLeft", true, false, false, true)),
      None,
      "Cmd+Shift+[ is the tab before"
    );
    assert_eq!(
      of(&key("ArrowLeft", "ArrowLeft", false, false, true, false)),
      None,
      "Option+← moves by a word on a Mac"
    );
    assert_eq!(of(&key("[", "BracketLeft", false, true, false, false)), None, "Ctrl+[");
    assert_eq!(
      of(&key("^", "BracketLeft", false, false, false, true)),
      None,
      "a key that says another sign of its own"
    );
  }

  #[test]
  fn a_key_of_its_own_on_every_platform() {
    for apple in [false, true] {
      let of = |event: &KeyboardEvent| NavigationCommand::of_key_on(event, apple);
      assert_eq!(
        of(&key("BrowserBack", "", false, false, false, false)),
        Some(NavigationCommand::Back),
        "Android's back, as winit names it"
      );
      assert_eq!(
        of(&key("", "BrowserForward", false, false, false, false)),
        Some(NavigationCommand::Forward)
      );
      assert_eq!(of(&key("BrowserBack", "", false, true, false, false)), None);
    }
  }

  #[test]
  fn the_accelerator_is_a_key_of_the_command() {
    for apple in [false, true] {
      for command in [NavigationCommand::Back, NavigationCommand::Forward] {
        let accelerator = command.accelerator_on(apple);
        let m = accelerator.modifiers;
        let event = KeyboardEvent::new(
          accelerator.key.to_string(),
          String::new(),
          m.shift,
          m.ctrl,
          m.alt,
          m.meta,
          NodeId::UNASSIGNED,
        );
        assert_eq!(NavigationCommand::of_key_on(&event, apple), Some(command));
      }
    }
  }

  #[test]
  fn the_side_buttons() {
    assert_eq!(
      NavigationCommand::of_button(MouseButton::Back),
      Some(NavigationCommand::Back)
    );
    assert_eq!(
      NavigationCommand::of_button(MouseButton::Forward),
      Some(NavigationCommand::Forward)
    );
    assert_eq!(NavigationCommand::of_button(MouseButton::Left), None);
    assert_eq!(NavigationCommand::of_button(MouseButton::Other(8)), None);
  }
}
