//! Declarative menus, available headlessly everywhere and hosted by AppKit on macOS.
use std::sync::{Arc, Mutex};

use super::synthetic_input::SyntheticModifiers;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenuBarSupport {
  Native,
  Unavailable,
}

/// A lowercase character or named key (Tab, Enter, Escape, Backspace, Delete,
/// ArrowUp/Down/Left/Right, F1..F24). Use the shift modifier for shifted keys.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Accelerator {
  pub key: Arc<str>,
  pub modifiers: SyntheticModifiers,
}
impl Accelerator {
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
