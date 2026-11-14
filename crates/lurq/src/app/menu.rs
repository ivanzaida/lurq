//! Declarative menus, available headlessly everywhere and hosted by AppKit on macOS.
use std::sync::{Arc, Mutex};

use super::synthetic_input::SyntheticModifiers;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenuBarSupport {
  Native,
  Unavailable,
}

/// A key combination: a key with the modifiers held as it goes down. What an item of the app's menu is chosen by,
/// what [`ui::Kbd`](crate::ui::Kbd) draws, and what a [`ui::ShortcutRecorder`](crate::ui::ShortcutRecorder) holds.
///
/// The key is a lowercase character or a named key (Tab, Enter, Escape, Backspace, Delete, Space as `" "`,
/// ArrowUp/Down/Left/Right, Home, End, PageUp, PageDown, Insert, F1..F24). A shifted key is its unshifted one with the
/// shift modifier: Shift+1, not `!`.
///
/// ```
/// use lurq::app::Accelerator;
///
/// let palette = Accelerator::parse("Ctrl+Shift+P").unwrap();
/// assert_eq!(palette.to_string(), "Ctrl+Shift+P");
/// assert!(palette.modifiers.ctrl && palette.modifiers.shift);
/// // `Mod` is Cmd on a Mac and Ctrl elsewhere.
/// let save = Accelerator::parse("Mod+S").unwrap();
/// assert!(save.has_modifier());
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Accelerator {
  pub key: Arc<str>,
  pub modifiers: SyntheticModifiers,
}

/// Keys that are held with another and are no combination alone, and what names no key.
const NOT_A_KEY: &[&str] = &[
  "Shift",
  "Control",
  "Alt",
  "AltGraph",
  "Meta",
  "OS",
  "Super",
  "Hyper",
  "Fn",
  "FnLock",
  "CapsLock",
  "NumLock",
  "ScrollLock",
  "Dead",
  "Unidentified",
  "Process",
  "",
];

/// A named key as a person writes it, in lowercase, and the name lurq gives it.
fn named_key(folded: &str) -> Option<&'static str> {
  Some(match folded {
    "enter" | "return" => "Enter",
    "escape" | "esc" => "Escape",
    "backspace" => "Backspace",
    "delete" | "del" => "Delete",
    "tab" => "Tab",
    "space" | "spacebar" => " ",
    "up" | "arrowup" => "ArrowUp",
    "down" | "arrowdown" => "ArrowDown",
    "left" | "arrowleft" => "ArrowLeft",
    "right" | "arrowright" => "ArrowRight",
    "home" => "Home",
    "end" => "End",
    "pageup" | "pgup" => "PageUp",
    "pagedown" | "pgdn" => "PageDown",
    "insert" | "ins" => "Insert",
    _ => return None,
  })
}

impl Accelerator {
  /// `key` with `modifiers` held.
  pub fn new(key: impl Into<Arc<str>>, modifiers: SyntheticModifiers) -> Self {
    Self {
      key: key.into(),
      modifiers,
    }
  }

  /// The combination a key that went down makes: its key, unshifted where it is a letter or a digit, and the
  /// modifiers held. `None` for a modifier alone (Shift, Control, Alt, Meta) and for what names no key.
  pub fn from_key_event(event: &crate::app::events::KeyboardEvent) -> Option<Self> {
    if NOT_A_KEY.contains(&event.key.as_str()) {
      return None;
    }
    let code = event.code.as_str();
    let key: Arc<str> = if let Some(letter) = code.strip_prefix("Key").filter(|rest| rest.len() == 1) {
      Arc::from(letter.to_lowercase())
    } else if let Some(digit) = code.strip_prefix("Digit").filter(|rest| rest.len() == 1) {
      Arc::from(digit)
    } else if event.key.chars().count() == 1 {
      Arc::from(event.key.to_lowercase())
    } else {
      Arc::from(event.key.as_str())
    };
    Some(Self::new(
      key,
      SyntheticModifiers {
        shift: event.shift,
        ctrl: event.ctrl,
        alt: event.alt,
        meta: event.meta,
      },
    ))
  }

  /// Whether a key that went down is this combination: the same key, with these modifiers held and no other.
  pub fn matches(&self, event: &crate::app::events::KeyboardEvent) -> bool {
    Self::from_key_event(event).as_ref() == Some(self)
  }

  /// Whether a modifier is held with the key: Ctrl, Alt, Shift or Meta.
  pub fn has_modifier(&self) -> bool {
    let m = self.modifiers;
    m.ctrl || m.alt || m.shift || m.meta
  }

  /// A combination as a person writes it: keys parted by `+` in any case, the modifiers `Ctrl`, `Alt` (`Option`),
  /// `Shift`, `Meta` (`Cmd`, `Super`, `Win`) and `Mod` (Cmd on a Mac, Ctrl elsewhere), then one key: `Ctrl+Shift+P`,
  /// `Mod+K`, `Alt+F4`, `Ctrl++`. `None` where there is no key, or more than one.
  pub fn parse(text: &str) -> Option<Self> {
    let mut parts: Vec<String> = Vec::new();
    let mut part = String::new();
    for character in text.chars() {
      if character == '+' && !part.trim().is_empty() {
        parts.push(std::mem::take(&mut part));
      } else {
        part.push(character);
      }
    }
    parts.push(part);
    let mut modifiers = SyntheticModifiers::default();
    let mut key = None;
    for part in parts {
      let written = if part.trim().is_empty() {
        part.as_str()
      } else {
        part.trim()
      };
      if written.is_empty() {
        continue;
      }
      let folded = written.to_lowercase();
      match folded.as_str() {
        "ctrl" | "control" => modifiers.ctrl = true,
        "alt" | "option" | "opt" => modifiers.alt = true,
        "shift" => modifiers.shift = true,
        "meta" | "cmd" | "command" | "super" | "win" | "windows" => modifiers.meta = true,
        "mod" if cfg!(any(target_os = "macos", target_os = "ios")) => modifiers.meta = true,
        "mod" => modifiers.ctrl = true,
        _ if key.is_some() => return None,
        _ => {
          key = Some(match named_key(&folded) {
            Some(name) => Arc::from(name),
            None if folded.starts_with('f') && folded[1..].parse::<u32>().is_ok_and(|n| (1..=24).contains(&n)) => {
              Arc::from(folded.to_uppercase())
            }
            None if written.chars().count() == 1 => Arc::from(folded),
            None => Arc::from(written),
          })
        }
      }
    }
    Some(Self::new(key?, modifiers))
  }

  pub fn command(key: impl Into<Arc<str>>) -> Self {
    Self {
      key: key.into(),
      modifiers: SyntheticModifiers::default().meta(),
    }
  }
  pub fn key_equivalent(&self) -> String {
    match self.key.as_ref() {
      "Tab" => "\t".into(),
      "Enter" => "\r".into(),
      "Escape" => "\u{1b}".into(),
      "Backspace" => "\u{8}".into(),
      "Delete" => "\u{7f}".into(),
      "ArrowUp" => "\u{f700}".into(),
      "ArrowDown" => "\u{f701}".into(),
      "ArrowLeft" => "\u{f702}".into(),
      "ArrowRight" => "\u{f703}".into(),
      key => {
        if let Some(n) = key.strip_prefix('F').and_then(|n| n.parse::<u32>().ok())
          && (1..=24).contains(&n)
        {
          return char::from_u32(0xf703 + n).unwrap().to_string();
        }
        if key.chars().count() == 1 {
          key.to_lowercase()
        } else {
          String::new()
        }
      }
    }
  }
  pub fn display(&self) -> String {
    let m = self.modifiers;
    format!(
      "{}{}{}{}{}",
      if m.ctrl { "⌃" } else { "" },
      if m.alt { "⌥" } else { "" },
      if m.shift { "⇧" } else { "" },
      if m.meta { "⌘" } else { "" },
      self.key.to_uppercase()
    )
  }
}

/// A combination written as [`Accelerator::parse`] reads it: the modifiers held, in the order Ctrl, Alt, Shift, Meta,
/// then the key, a letter in capitals: `Ctrl+Shift+P`, `Alt+Space`. What an app stores; what a person reads is drawn
/// by [`ui::Kbd`](crate::ui::Kbd) in the names of their platform.
impl std::fmt::Display for Accelerator {
  fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    let m = self.modifiers;
    for (held, name) in [(m.ctrl, "Ctrl"), (m.alt, "Alt"), (m.shift, "Shift"), (m.meta, "Meta")] {
      if held {
        write!(formatter, "{name}+")?;
      }
    }
    match self.key.as_ref() {
      " " => write!(formatter, "Space"),
      key if key.chars().count() == 1 => write!(formatter, "{}", key.to_uppercase()),
      key => write!(formatter, "{key}"),
    }
  }
}

#[cfg(feature = "devtools")]
impl crate::app::component::DevtoolsInspectable for Accelerator {
  fn inspect(&self, formatter: &mut crate::app::component::DevtoolsFormatter<'_>) {
    formatter.value_debug(&self.to_string());
  }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MenuAction {
  pub id: Arc<str>,
  pub label: Arc<str>,
  pub accelerator: Option<Accelerator>,
  pub enabled: bool,
}
impl MenuAction {
  pub fn new(id: impl Into<Arc<str>>, label: impl Into<Arc<str>>) -> Self {
    Self {
      id: id.into(),
      label: label.into(),
      accelerator: None,
      enabled: true,
    }
  }
  pub fn accelerator(mut self, accelerator: Accelerator) -> Self {
    self.accelerator = Some(accelerator);
    self
  }
  pub fn enabled(mut self, enabled: bool) -> Self {
    self.enabled = enabled;
    self
  }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MenuItem {
  Item(MenuAction),
  Separator,
  Submenu(Menu),
}
impl From<MenuAction> for MenuItem {
  fn from(action: MenuAction) -> Self {
    Self::Item(action)
  }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Menu {
  pub title: Arc<str>,
  pub items: Vec<MenuItem>,
}

/// AppKit supplies Services and Hide/Show around these application commands.
/// Quit is an action, never a direct terminate selector.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ApplicationMenu {
  pub name: Arc<str>,
  pub about: Option<MenuAction>,
  pub preferences: Option<MenuAction>,
  pub quit: MenuAction,
}
impl Default for ApplicationMenu {
  fn default() -> Self {
    Self {
      name: "Application".into(),
      about: None,
      preferences: None,
      quit: MenuAction::new("quit", "Quit").accelerator(Accelerator::command("q")),
    }
  }
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MenuBar {
  pub application: ApplicationMenu,
  pub menus: Vec<Menu>,
}
impl MenuBar {
  pub fn actions(&self) -> Vec<&MenuAction> {
    fn collect<'a>(items: &'a [MenuItem], out: &mut Vec<&'a MenuAction>) {
      for item in items {
        match item {
          MenuItem::Item(action) => out.push(action),
          MenuItem::Submenu(menu) => collect(&menu.items, out),
          MenuItem::Separator => {}
        }
      }
    }
    let mut out = Vec::new();
    out.extend(self.application.about.iter());
    out.extend(self.application.preferences.iter());
    out.push(&self.application.quit);
    for menu in &self.menus {
      collect(&menu.items, &mut out);
    }
    out
  }
}
type ActivateHandler = Arc<dyn Fn(&str) + Send + Sync>;
#[derive(Default)]
struct MenuState {
  bar: Option<MenuBar>,
  version: u64,
  handler: Option<ActivateHandler>,
  waker: Option<super::window::WindowWaker>,
  pending: Vec<Arc<str>>,
}
/// Clone into a signal effect to update labels, enabled state, or item presence.
/// Equal models are ignored. Callbacks run when the event loop drains activations.
#[derive(Clone, Default)]
pub struct MenuController {
  inner: Arc<Mutex<MenuState>>,
}
impl MenuController {
  pub fn set(&self, bar: MenuBar) {
    let waker = {
      let mut state = self.inner.lock().unwrap();
      if state.bar.as_ref() == Some(&bar) {
        return;
      }
      state.bar = Some(bar);
      state.version = state.version.wrapping_add(1);
      state.waker.clone()
    };
    if let Some(waker) = waker {
      waker();
    }
  }
  pub fn model(&self) -> Option<MenuBar> {
    self.inner.lock().unwrap().bar.clone()
  }
  pub fn on_activate(&self, handler: impl Fn(&str) + Send + Sync + 'static) {
    self.inner.lock().unwrap().handler = Some(Arc::new(handler));
  }
  /// Queue activation. Unknown, ambiguous or disabled ids are ignored at execution.
  pub fn activate(&self, id: impl Into<Arc<str>>) {
    let waker = {
      let mut state = self.inner.lock().unwrap();
      state.pending.push(id.into());
      state.waker.clone()
    };
    if let Some(waker) = waker {
      waker();
    }
  }
  pub(crate) fn dispatch(&self, id: &str, window: &super::window::Window) -> bool {
    let (handler, quit) = {
      let state = self.inner.lock().unwrap();
      let Some(bar) = &state.bar else {
        return false;
      };
      let matches: Vec<_> = bar.actions().into_iter().filter(|a| a.id.as_ref() == id).collect();
      if matches.len() != 1 || !matches[0].enabled {
        return false;
      }
      (state.handler.clone(), bar.application.quit.id.as_ref() == id)
    };
    if let Some(handler) = handler {
      handler(id);
    } else if quit {
      window.dispatch_close_request(super::CloseRequestSource::Os);
    }
    true
  }
  /// Run queued activations on the event-loop thread, or explicitly in a headless harness.
  pub fn drain(&self, window: &super::window::Window) {
    let pending = std::mem::take(&mut self.inner.lock().unwrap().pending);
    for id in pending {
      self.dispatch(&id, window);
    }
  }
  #[cfg(feature = "winit")]
  pub(crate) fn set_waker(&self, waker: super::window::WindowWaker) {
    self.inner.lock().unwrap().waker = Some(waker);
  }
  #[cfg(all(feature = "winit", target_os = "macos"))]
  pub(crate) fn snapshot(&self, previous: Option<u64>) -> Option<(u64, Option<MenuBar>)> {
    let state = self.inner.lock().unwrap();
    if previous == Some(state.version) {
      None
    } else {
      Some((state.version, state.bar.clone()))
    }
  }
}

#[cfg(test)]
mod accelerator_tests {
  use super::*;
  use crate::{
    app::events::{EventControl, KeyboardEvent},
    core::NodeId,
  };

  fn pressed(key: &str, code: &str, shift: bool, ctrl: bool) -> KeyboardEvent {
    KeyboardEvent {
      key: key.to_owned(),
      code: code.to_owned(),
      shift,
      ctrl,
      alt: false,
      meta: false,
      target_id: NodeId::UNASSIGNED,
      text_input_focused: false,
      composing: false,
      focus_within: true,
      control: EventControl::new(),
    }
  }

  #[test]
  fn a_key_that_goes_down_makes_its_combination_unshifted() {
    let palette = Accelerator::from_key_event(&pressed("P", "KeyP", true, true)).unwrap();
    assert_eq!(palette, Accelerator::parse("Ctrl+Shift+P").unwrap());
    assert_eq!(palette.key.as_ref(), "p");
    // Shift+1 is 1 with shift, not the `!` it types.
    let one = Accelerator::from_key_event(&pressed("!", "Digit1", true, false)).unwrap();
    assert_eq!(one.to_string(), "Shift+1");
    assert_eq!(
      Accelerator::from_key_event(&pressed("F5", "F5", false, false))
        .unwrap()
        .to_string(),
      "F5"
    );
    // A modifier alone is no combination.
    for modifier in ["Shift", "Control", "Alt", "Meta"] {
      assert_eq!(
        Accelerator::from_key_event(&pressed(modifier, modifier, true, true)),
        None,
        "{modifier}"
      );
    }
    assert!(palette.matches(&pressed("P", "KeyP", true, true)));
    assert!(!palette.matches(&pressed("P", "KeyP", false, true)), "another modifier");
  }

  #[test]
  fn a_combination_is_written_as_it_is_read() {
    for written in [
      "Ctrl+Shift+P",
      "Alt+F4",
      "Ctrl+Alt+Shift+Meta+K",
      "Space",
      "Ctrl+Enter",
      "Shift+ArrowUp",
      "Ctrl++",
    ] {
      assert_eq!(Accelerator::parse(written).unwrap().to_string(), written, "{written}");
    }
    assert_eq!(
      Accelerator::parse("ctrl + shift + p").unwrap().to_string(),
      "Ctrl+Shift+P"
    );
    assert_eq!(Accelerator::parse("Cmd+Esc").unwrap().to_string(), "Meta+Escape");
    assert_eq!(Accelerator::parse("Ctrl+Shift"), None, "no key");
    assert_eq!(Accelerator::parse("Ctrl+A+B"), None, "two keys");
    assert!(!Accelerator::parse("F5").unwrap().has_modifier());
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::app::{Window, window::WindowCommand};
  fn bar(enabled: bool) -> MenuBar {
    MenuBar {
      menus: vec![Menu {
        title: "File".into(),
        items: vec![MenuItem::Submenu(Menu {
          title: "Recent".into(),
          items: vec![MenuAction::new("open", "Open").enabled(enabled).into()],
        })],
      }],
      ..Default::default()
    }
  }
  #[test]
  fn menu_activation_checks_live_availability_presence_and_identity() {
    let menus = MenuController::default();
    let window = Window::new();
    let received = Arc::new(Mutex::new(Vec::new()));
    let output = received.clone();
    let reentrant = menus.clone();
    menus.on_activate(move |id| {
      output.lock().unwrap().push(id.to_owned());
      reentrant.set(bar(false));
    });
    menus.set(bar(true));
    menus.activate("open");
    menus.activate("open");
    menus.activate("missing");
    menus.drain(&window);
    assert_eq!(*received.lock().unwrap(), vec!["open"]);
    menus.set(bar(true));
    menus.activate("open");
    menus.set(MenuBar::default());
    menus.drain(&window);
    assert_eq!(received.lock().unwrap().len(), 1);
    let mut duplicate = bar(true);
    duplicate.menus[0].items.push(MenuAction::new("open", "Other").into());
    menus.set(duplicate);
    assert!(!menus.dispatch("open", &window));
  }
  #[test]
  fn menu_quit_defaults_to_veto_and_model_updates_are_coalesced() {
    let menus = MenuController::default();
    let window = Window::new();
    menus.set(MenuBar::default());
    let version = menus.inner.lock().unwrap().version;
    menus.set(MenuBar::default());
    assert_eq!(menus.inner.lock().unwrap().version, version);
    window.handle().on_close_requested(|r| r.cancel());
    assert!(menus.dispatch("quit", &window));
    assert!(window.take_commands().is_empty());
    window.handle().clear_close_requested_handler();
    assert!(menus.dispatch("quit", &window));
    assert_eq!(window.take_commands(), vec![WindowCommand::Close]);
  }
  #[test]
  fn accelerator_rendering_covers_characters_modifiers_and_named_keys() {
    let mut a = Accelerator::command("s");
    a.modifiers.shift = true;
    assert_eq!(a.display(), "⇧⌘S");
    assert_eq!(a.key_equivalent(), "s");
    for (key, expected) in [
      ("Tab", "\t"),
      ("Enter", "\r"),
      ("ArrowLeft", "\u{f702}"),
      ("F1", "\u{f704}"),
      ("F24", "\u{f71b}"),
      ("F25", ""),
      ("invalid", ""),
    ] {
      assert_eq!(Accelerator::command(key).key_equivalent(), expected);
    }
  }
}
