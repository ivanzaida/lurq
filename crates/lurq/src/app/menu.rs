//! Declarative menus, available headlessly everywhere and hosted by AppKit on macOS.
use std::sync::{Arc, Mutex};

use super::{events::EditCommand, synthetic_input::SyntheticModifiers};

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

  /// An item of the Edit menu: runs `command` on what has the focus, as its key does
  /// ([`WindowHandle::edit`](crate::app::window::WindowHandle::edit)), with the key and the word of the app's
  /// locale. A menu of the app's bar that has one is its Edit menu, and lurq adds none of its own.
  pub fn edit(command: EditCommand) -> Self {
    let locale = crate::ui::current_locale();
    Self::edit_in(command, &MenuWords::of(&locale))
  }

  fn edit_in(command: EditCommand, words: &MenuWords) -> Self {
    let (name, key, shift, label) = match command {
      EditCommand::Undo => ("undo", "z", false, &words.undo),
      EditCommand::Redo => ("redo", "z", true, &words.redo),
      EditCommand::Cut => ("cut", "x", false, &words.cut),
      EditCommand::Copy => ("copy", "c", false, &words.copy),
      EditCommand::Paste => ("paste", "v", false, &words.paste),
      EditCommand::SelectAll => ("select_all", "a", false, &words.select_all),
    };
    let mut accelerator = Accelerator::command(key);
    accelerator.modifiers.shift = shift;
    Self::new(format!("{EDIT_PREFIX}{name}"), label.clone()).accelerator(accelerator)
  }

  /// The command of editing an item of [`MenuAction::edit`] runs.
  pub fn edit_command(&self) -> Option<EditCommand> {
    let name = self.id.strip_prefix(EDIT_PREFIX)?;
    EditCommand::ALL.into_iter().find(|command| {
      Self::edit_in(*command, &MenuWords::english())
        .id
        .strip_prefix(EDIT_PREFIX)
        == Some(name)
    })
  }
}

/// What the id of an item of editing starts with.
const EDIT_PREFIX: &str = "lurq.edit.";
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

impl Menu {
  /// The Edit menu: Undo, Redo, Cut, Copy, Paste and Select All, in the words of the app's locale. lurq puts it in
  /// an app's bar on a Mac by itself; an app that wants it elsewhere among its menus, or with items of its own,
  /// puts this one there (and on a Mac the system adds its Emoji & Symbols to it).
  pub fn edit() -> Self {
    Self::edit_in(&MenuWords::of(&crate::ui::current_locale()))
  }

  pub(crate) fn edit_in(words: &MenuWords) -> Self {
    let item = |command| MenuItem::Item(MenuAction::edit_in(command, words));
    Self {
      title: words.navigation.edit.clone(),
      items: vec![
        item(EditCommand::Undo),
        item(EditCommand::Redo),
        MenuItem::Separator,
        item(EditCommand::Cut),
        item(EditCommand::Copy),
        item(EditCommand::Paste),
        item(EditCommand::SelectAll),
      ],
    }
  }

  /// Whether an item of editing is in it, or in a menu in it.
  pub fn has_edit_items(&self) -> bool {
    self.items.iter().any(|item| match item {
      MenuItem::Item(action) => action.edit_command().is_some(),
      MenuItem::Submenu(menu) => menu.has_edit_items(),
      MenuItem::Separator => false,
    })
  }
}

/// The words of a Mac's menu bar that lurq says itself: what the system puts in the menu of the app, and the Edit
/// menu's.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct MenuWords {
  pub navigation: crate::core::NavigationWords,
  pub undo: Arc<str>,
  pub redo: Arc<str>,
  pub cut: Arc<str>,
  pub copy: Arc<str>,
  pub paste: Arc<str>,
  pub select_all: Arc<str>,
}

impl MenuWords {
  pub(crate) fn of(locale: &crate::core::Locale) -> Self {
    Self {
      navigation: locale.navigation.clone(),
      undo: locale.editing.undo.clone(),
      redo: locale.editing.redo.clone(),
      cut: locale.editing.cut.clone(),
      copy: locale.editing.copy.clone(),
      paste: locale.editing.paste.clone(),
      select_all: locale.navigation.select_all.clone(),
    }
  }

  pub(crate) fn english() -> Self {
    Self::of(&crate::core::Locale::english())
  }
}

impl Default for MenuWords {
  fn default() -> Self {
    Self::english()
  }
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
/// Called "Application" until the app names it, with a Quit item: in the words of the app's locale
/// ([`Locale::navigation`](crate::core::Locale::navigation)) where it is made in a render, in English outside one.
impl Default for ApplicationMenu {
  fn default() -> Self {
    let locale = crate::ui::current_locale();
    Self {
      name: locale.navigation.application.clone(),
      about: None,
      preferences: None,
      quit: MenuAction::new("quit", locale.navigation.quit.clone()).accelerator(Accelerator::command("q")),
    }
  }
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MenuBar {
  pub application: ApplicationMenu,
  pub menus: Vec<Menu>,
}
impl MenuBar {
  /// Whether one of its menus is an Edit menu: has an item of editing ([`MenuAction::edit`]).
  pub fn has_edit_menu(&self) -> bool {
    self.menus.iter().any(Menu::has_edit_items)
  }

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
  /// The words of the app's locale that the system's own items of the menu and the Edit menu say, once the app set a
  /// locale.
  words: Option<Arc<MenuWords>>,
  /// Whether lurq leaves the bar without an Edit menu of its own: [`MenuController::set_edit_menu`].
  no_edit_menu: bool,
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
  /// The words the items lurq and the system put in the bar say ("Hide", "Show All", the Edit menu): the app's
  /// locale's, which [`App::set_locale`](crate::app::App::set_locale) gives here, and the menu is built again in them.
  pub(crate) fn set_words(&self, locale: &crate::core::Locale) {
    let words = MenuWords::of(locale);
    let waker = {
      let mut state = self.inner.lock().unwrap();
      if state.words.as_deref() == Some(&words) {
        return;
      }
      state.words = Some(Arc::new(words));
      state.version = state.version.wrapping_add(1);
      state.waker.clone()
    };
    if let Some(waker) = waker {
      waker();
    }
  }
  /// What [`Self::set_words`] gave, English before.
  #[cfg_attr(not(target_os = "macos"), allow(dead_code))]
  pub(crate) fn words(&self) -> Arc<MenuWords> {
    let words = self.inner.lock().unwrap().words.clone();
    words.unwrap_or_else(|| Arc::new(MenuWords::english()))
  }
  /// Whether lurq puts an Edit menu in the bar on a Mac, as it does until told not to. Where the app's bar has a
  /// menu with an item of editing ([`MenuAction::edit`]), that is the Edit menu, and lurq adds none either way.
  pub fn set_edit_menu(&self, shown: bool) {
    let waker = {
      let mut state = self.inner.lock().unwrap();
      if state.no_edit_menu == !shown {
        return;
      }
      state.no_edit_menu = !shown;
      state.version = state.version.wrapping_add(1);
      state.waker.clone()
    };
    if let Some(waker) = waker {
      waker();
    }
  }
  /// The bar as it is shown: the app's, or none where the app has none and lurq adds nothing to the system's; with
  /// lurq's Edit menu after the menu of the app where the bar has none. The app's own bar is [`Self::model`].
  pub fn shown(&self) -> Option<MenuBar> {
    let state = self.inner.lock().unwrap();
    Self::shown_of(&state)
  }
  fn shown_of(state: &MenuState) -> Option<MenuBar> {
    let edit = !state.no_edit_menu;
    let mut bar = match &state.bar {
      Some(bar) => bar.clone(),
      None if edit => MenuBar::default(),
      None => return None,
    };
    if edit && !bar.has_edit_menu() {
      let words = state.words.clone().unwrap_or_else(|| Arc::new(MenuWords::english()));
      bar.menus.insert(0, Menu::edit_in(&words));
    }
    Some(bar)
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
  pub(crate) fn dispatch(&self, id: &str, window: &super::window::Window, in_front: &super::window::Window) -> bool {
    let (handler, quit, edit) = {
      let state = self.inner.lock().unwrap();
      let Some(bar) = Self::shown_of(&state) else {
        return false;
      };
      let matches: Vec<_> = bar.actions().into_iter().filter(|a| a.id.as_ref() == id).collect();
      if matches.len() != 1 || !matches[0].enabled {
        return false;
      }
      (
        state.handler.clone(),
        bar.application.quit.id.as_ref() == id,
        matches[0].edit_command(),
      )
    };
    // An item of editing runs its command, as its key does.
    if let Some(command) = edit {
      in_front.request_edit(command);
      return true;
    }
    if let Some(handler) = handler {
      handler(id);
    } else if quit {
      window.dispatch_close_request(super::CloseRequestSource::Os);
    }
    true
  }
  /// Run queued activations on the event-loop thread, or explicitly in a headless harness.
  pub fn drain(&self, window: &super::window::Window) {
    self.drain_to(window, window);
  }
  /// [`Self::drain`], with the window in front, whose focus an item of editing runs on, beside the app's main one.
  pub(crate) fn drain_to(&self, window: &super::window::Window, in_front: &super::window::Window) {
    let pending = std::mem::take(&mut self.inner.lock().unwrap().pending);
    for id in pending {
      self.dispatch(&id, window, in_front);
    }
  }
  #[cfg(feature = "winit")]
  pub(crate) fn set_waker(&self, waker: super::window::WindowWaker) {
    self.inner.lock().unwrap().waker = Some(waker);
  }
  #[cfg(all(feature = "winit", target_os = "macos"))]
  pub(crate) fn snapshot(&self, previous: Option<u64>) -> Option<(u64, Option<MenuBar>, bool)> {
    let state = self.inner.lock().unwrap();
    if previous == Some(state.version) {
      None
    } else {
      Some((state.version, Self::shown_of(&state), state.bar.is_some()))
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
      focus_takes_enter: false,
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
    assert!(!menus.dispatch("open", &window, &window));
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
    assert!(menus.dispatch("quit", &window, &window));
    assert!(window.take_commands().is_empty());
    window.handle().clear_close_requested_handler();
    assert!(menus.dispatch("quit", &window, &window));
    assert_eq!(window.take_commands(), vec![WindowCommand::Close]);
  }
  fn titles(bar: &MenuBar) -> Vec<String> {
    bar.menus.iter().map(|menu| menu.title.to_string()).collect()
  }

  #[test]
  fn an_app_with_no_bar_of_its_own_has_the_edit_menu() {
    let menus = MenuController::default();
    let shown = menus.shown().expect("a bar is shown");
    assert_eq!(titles(&shown), vec!["Edit"]);
    let commands: Vec<_> = shown.menus[0]
      .items
      .iter()
      .filter_map(|item| match item {
        MenuItem::Item(action) => Some((action.edit_command(), action.label.to_string())),
        _ => None,
      })
      .collect();
    assert_eq!(
      commands,
      [
        (Some(EditCommand::Undo), "Undo"),
        (Some(EditCommand::Redo), "Redo"),
        (Some(EditCommand::Cut), "Cut"),
        (Some(EditCommand::Copy), "Copy"),
        (Some(EditCommand::Paste), "Paste"),
        (Some(EditCommand::SelectAll), "Select All"),
      ]
      .map(|(command, label)| (command, label.to_owned()))
    );
    assert_eq!(menus.model(), None, "the app's own bar is still none");
  }

  #[test]
  fn the_edit_menu_is_after_the_menu_of_the_app_and_before_the_apps_menus() {
    let menus = MenuController::default();
    menus.set(bar(true));
    assert_eq!(titles(&menus.shown().unwrap()), vec!["Edit", "File"]);
  }

  #[test]
  fn the_edit_menu_is_in_the_apps_locale() {
    let menus = MenuController::default();
    menus.set_words(&crate::core::Locale::for_tag("uk"));
    let shown = menus.shown().unwrap();
    assert_eq!(titles(&shown), vec!["Редагування"]);
    let MenuItem::Item(undo) = &shown.menus[0].items[0] else {
      panic!("an item first");
    };
    assert_eq!(&*undo.label, "Скасувати");
  }

  #[test]
  fn an_app_turns_the_edit_menu_off() {
    let menus = MenuController::default();
    menus.set_edit_menu(false);
    assert_eq!(
      menus.shown(),
      None,
      "no bar of the app's: the system's is left as it is"
    );
    menus.set(bar(true));
    assert_eq!(titles(&menus.shown().unwrap()), vec!["File"]);
    menus.set_edit_menu(true);
    assert_eq!(titles(&menus.shown().unwrap()), vec!["Edit", "File"]);
  }

  #[test]
  fn a_menu_of_the_apps_with_an_item_of_editing_is_the_edit_menu() {
    let menus = MenuController::default();
    let mut own = bar(true);
    let mut edit = Menu::edit();
    edit.items.push(MenuAction::new("find", "Find").into());
    own.menus.push(edit);
    menus.set(own.clone());
    assert_eq!(menus.shown(), Some(own), "where the app put it, as it made it");

    let mut copy_alone = bar(true);
    copy_alone.menus[0]
      .items
      .push(MenuAction::edit(EditCommand::Copy).into());
    menus.set(copy_alone);
    assert_eq!(titles(&menus.shown().unwrap()), vec!["File"]);
  }

  #[test]
  fn an_item_of_editing_runs_its_command_on_the_window_in_front() {
    let menus = MenuController::default();
    let (main, in_front) = (Window::new(), Window::new());
    let handled = Arc::new(Mutex::new(Vec::new()));
    let seen = handled.clone();
    menus.on_activate(move |id| seen.lock().unwrap().push(id.to_owned()));
    menus.activate(MenuAction::edit(EditCommand::Paste).id);
    menus.activate("quit");
    menus.drain_to(&main, &in_front);
    assert_eq!(in_front.take_edits(), vec![(EditCommand::Paste, false)]);
    assert!(main.take_edits().is_empty());
    assert_eq!(
      *handled.lock().unwrap(),
      vec!["quit"],
      "the app's handler is told of its own items alone"
    );
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
