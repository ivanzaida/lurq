//! Pointer buttons and keys that MCP clients hold down across `lurq_interact`
//! calls, per window.
//!
//! Each hold belongs to the session that pressed it. Anything a client can no
//! longer release itself is released for it: when its session ends, when the
//! window loses focus or closes, when `lurq_interact` becomes unavailable, and
//! when the server stops (`tools::held`).

use super::sessions::SessionId;
use crate::app::{events::MouseButton, synthetic_input::SyntheticModifiers};

/// Why held input was released without a `release` or `key_up`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ReleaseReason {
  SessionEnded,
  FocusLost,
  WindowClosed,
  Unavailable,
  ServerStopped,
}

impl ReleaseReason {
  pub(crate) fn describe(self) -> &'static str {
    match self {
      Self::SessionEnded => "the MCP session that held it ended",
      Self::FocusLost => "the window lost focus",
      Self::WindowClosed => "the window closed",
      Self::Unavailable => "lurq_interact became unavailable (MCP disabled, scope removed or tool denied)",
      Self::ServerStopped => "the MCP server stopped",
    }
  }
}

pub(crate) struct HeldButton {
  pub(crate) button: MouseButton,
  pub(crate) session: SessionId,
}

pub(crate) struct HeldKey {
  pub(crate) key: String,
  pub(crate) session: SessionId,
}

/// What MCP clients hold in one window.
pub(crate) struct WindowHolds {
  pub(crate) buttons: Vec<HeldButton>,
  pub(crate) keys: Vec<HeldKey>,
  /// Where the last MCP pointer action left the pointer, in physical pixels.
  pub(crate) pointer: Option<(f32, f32)>,
  /// The window's focus-loss count when its first hold began; the next loss releases everything.
  pub(crate) focus_losses: u64,
}

impl WindowHolds {
  fn new(focus_losses: u64) -> Self {
    Self {
      buttons: Vec::new(),
      keys: Vec::new(),
      pointer: None,
      focus_losses,
    }
  }

  pub(crate) fn is_empty(&self) -> bool {
    self.buttons.is_empty() && self.keys.is_empty()
  }

  pub(crate) fn holds_button(&self, button: MouseButton) -> bool {
    self.buttons.iter().any(|held| held.button == button)
  }

  pub(crate) fn holds_key(&self, key: &str) -> bool {
    self.keys.iter().any(|held| held.key == key)
  }

  /// The modifiers the held keys put down.
  pub(crate) fn modifiers(&self) -> SyntheticModifiers {
    key_modifiers(&self.keys)
  }

  /// Splits off what the `ended` sessions hold.
  fn take_sessions(&mut self, ended: &[SessionId]) -> Self {
    let mut taken = Self::new(self.focus_losses);
    taken.pointer = self.pointer;
    (taken.buttons, self.buttons) = std::mem::take(&mut self.buttons)
      .into_iter()
      .partition(|held| ended.contains(&held.session));
    (taken.keys, self.keys) = std::mem::take(&mut self.keys)
      .into_iter()
      .partition(|held| ended.contains(&held.session));
    taken
  }

  pub(crate) fn report(&self) -> serde_json::Value {
    serde_json::json!({
      "buttons": self.buttons.iter().map(|held| button_name(held.button)).collect::<Vec<_>>(),
      "keys": self.keys.iter().map(|held| held.key.as_str()).collect::<Vec<_>>(),
    })
  }
}

/// The modifiers held modifier keys put down, as the shell reports them from `ModifiersChanged`.
pub(crate) fn key_modifiers(keys: &[HeldKey]) -> SyntheticModifiers {
  let mut modifiers = SyntheticModifiers::default();
  for held in keys {
    match held.key.as_str() {
      "Shift" => modifiers.shift = true,
      "Control" => modifiers.ctrl = true,
      "Alt" => modifiers.alt = true,
      "Meta" | "Super" => modifiers.meta = true,
      _ => {}
    }
  }
  modifiers
}

pub(crate) fn button_name(button: MouseButton) -> &'static str {
  match button {
    MouseButton::Left => "left",
    MouseButton::Middle => "middle",
    MouseButton::Right => "right",
    MouseButton::Other(_) => "other",
  }
}

/// Holds of every window, by canonical window id (`main` or `w<id>`).
#[derive(Default)]
pub(crate) struct HeldInput {
  windows: Vec<(String, WindowHolds)>,
  /// Why each window's input was last released for its client, until the next hold or release there.
  released: Vec<(String, ReleaseReason)>,
}

impl HeldInput {
  pub(crate) fn is_empty(&self) -> bool {
    self.windows.is_empty()
  }

  pub(crate) fn window_ids(&self) -> Vec<String> {
    self.windows.iter().map(|(id, _)| id.clone()).collect()
  }

  pub(crate) fn get(&self, window: &str) -> Option<&WindowHolds> {
    self.windows.iter().find(|(id, _)| id == window).map(|(_, holds)| holds)
  }

  pub(crate) fn get_mut(&mut self, window: &str) -> Option<&mut WindowHolds> {
    self
      .windows
      .iter_mut()
      .find(|(id, _)| id == window)
      .map(|(_, holds)| holds)
  }

  /// The window's holds, begun now (at its current focus-loss count) if it holds nothing yet.
  pub(crate) fn entry(&mut self, window: &str, focus_losses: u64) -> &mut WindowHolds {
    let index = match self.windows.iter().position(|(id, _)| id == window) {
      Some(index) => index,
      None => {
        self.windows.push((window.to_owned(), WindowHolds::new(focus_losses)));
        self.windows.len() - 1
      }
    };
    &mut self.windows[index].1
  }

  /// After a client's own hold or release: drop the window's entry once it holds nothing, and its
  /// last automatic release, which no longer explains its state.
  pub(crate) fn settle(&mut self, window: &str) {
    self.windows.retain(|(id, holds)| id != window || !holds.is_empty());
    self.released.retain(|(id, _)| id != window);
  }

  /// Takes everything the window holds, released for `reason`.
  pub(crate) fn take(&mut self, window: &str, reason: ReleaseReason) -> Option<WindowHolds> {
    let index = self.windows.iter().position(|(id, _)| id == window)?;
    let (_, holds) = self.windows.remove(index);
    self.record(window, reason);
    Some(holds)
  }

  /// Takes what the `ended` sessions hold in the window, if anything.
  pub(crate) fn take_sessions(&mut self, window: &str, ended: &[SessionId]) -> Option<WindowHolds> {
    if ended.is_empty() {
      return None;
    }
    let holds = self.get_mut(window)?;
    let taken = holds.take_sessions(ended);
    if taken.is_empty() {
      return None;
    }
    self.windows.retain(|(_, holds)| !holds.is_empty());
    self.record(window, ReleaseReason::SessionEnded);
    Some(taken)
  }

  fn record(&mut self, window: &str, reason: ReleaseReason) {
    self.released.retain(|(id, _)| id != window);
    self.released.push((window.to_owned(), reason));
  }

  pub(crate) fn released_because(&self, window: &str) -> Option<ReleaseReason> {
    self
      .released
      .iter()
      .find(|(id, _)| id == window)
      .map(|(_, reason)| *reason)
  }

  /// `{"buttons": [...], "keys": [...]}` held in the window.
  pub(crate) fn report(&self, window: &str) -> serde_json::Value {
    match self.get(window) {
      Some(holds) => holds.report(),
      None => serde_json::json!({ "buttons": [], "keys": [] }),
    }
  }
}
