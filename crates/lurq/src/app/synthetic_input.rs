//! Synthetic input: driving a running window as if the OS had delivered the
//! events.
//!
//! Automation harnesses (screenshot-driven UI tests, agent tooling) need to
//! click and type into a real window. Posting native messages from outside the
//! process does not work reliably — the shell tracks its own cursor position
//! and modifier state, so a bare button-press arrives with no hover established
//! and lands on whatever the toolkit last thought was under the pointer.
//!
//! These events are queued through the same [`WindowCommand`] channel the
//! window's own controls use and are drained by the shell inside its event
//! loop, so they reach exactly the entry points real winit events reach and
//! observe the same ordering guarantees.
//!
//! [`WindowCommand`]: crate::app::window::WindowCommand

use crate::app::events::MouseButton;
use crate::app::runtime::Tree;
use crate::app::events::ScrollPhase;

/// Modifier state to apply to a synthetic event, matching what the shell would
/// have read from `ModifiersChanged`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SyntheticModifiers {
  pub shift: bool,
  pub ctrl: bool,
  pub alt: bool,
  pub meta: bool,
}

impl SyntheticModifiers {
  pub fn shift(mut self) -> Self {
    self.shift = true;
    self
  }

  pub fn ctrl(mut self) -> Self {
    self.ctrl = true;
    self
  }

  pub fn alt(mut self) -> Self {
    self.alt = true;
    self
  }

  pub fn meta(mut self) -> Self {
    self.meta = true;
    self
  }
}

/// What a synthetic event does. Positions are **physical** pixels in the
/// window's own coordinate space — the same units `WindowEvent::CursorMoved`
/// carries, and the same units a captured frame is measured in, so a caller can
/// click what it sees in a screenshot without converting.
#[derive(Clone, Debug, PartialEq)]
pub enum SyntheticInputKind {
  MouseMove {
    x: f32,
    y: f32,
  },
  MouseDown {
    x: f32,
    y: f32,
    button: MouseButton,
  },
  MouseUp {
    x: f32,
    y: f32,
    button: MouseButton,
  },
  /// Move, press, release — in that order.
  ///
  /// Hover is established before the press because that is what real input
  /// does: hit-testing runs off the last motion event, so a press with no
  /// preceding move is delivered against a stale position.
  Click {
    x: f32,
    y: f32,
    button: MouseButton,
  },
  Wheel {
    x: f32,
    y: f32,
    delta_x: f32,
    delta_y: f32,
  },
  KeyDown {
    key: String,
    code: String,
  },
  KeyUp {
    key: String,
    code: String,
  },
  /// One printable character, delivered as a press/release pair.
  Char(char),
}

/// A queued synthetic event.
#[derive(Clone, Debug, PartialEq)]
pub struct SyntheticInput {
  kind: SyntheticInputKind,
  modifiers: SyntheticModifiers,
}

impl SyntheticInput {
  pub fn new(kind: SyntheticInputKind) -> Self {
    Self {
      kind,
      modifiers: SyntheticModifiers::default(),
    }
  }

  pub fn with_modifiers(mut self, modifiers: SyntheticModifiers) -> Self {
    self.modifiers = modifiers;
    self
  }

  pub fn kind(&self) -> &SyntheticInputKind {
    &self.kind
  }

  pub fn modifiers(&self) -> SyntheticModifiers {
    self.modifiers
  }

  pub fn mouse_move(x: f32, y: f32) -> Self {
    Self::new(SyntheticInputKind::MouseMove { x, y })
  }

  pub fn mouse_down(x: f32, y: f32, button: MouseButton) -> Self {
    Self::new(SyntheticInputKind::MouseDown { x, y, button })
  }

  pub fn mouse_up(x: f32, y: f32, button: MouseButton) -> Self {
    Self::new(SyntheticInputKind::MouseUp { x, y, button })
  }

  pub fn click(x: f32, y: f32) -> Self {
    Self::new(SyntheticInputKind::Click {
      x,
      y,
      button: MouseButton::Left,
    })
  }

  pub fn click_button(x: f32, y: f32, button: MouseButton) -> Self {
    Self::new(SyntheticInputKind::Click { x, y, button })
  }

  pub fn wheel(x: f32, y: f32, delta_x: f32, delta_y: f32) -> Self {
    Self::new(SyntheticInputKind::Wheel {
      x,
      y,
      delta_x,
      delta_y,
    })
  }

  /// A named key such as `Enter`, `Tab`, `Escape`, `ArrowDown`, or a single
  /// character. `code` defaults to the key name when the caller has no
  /// physical-key information, which is what every non-layout-sensitive
  /// consumer wants.
  pub fn key_down(key: impl Into<String>) -> Self {
    let key = key.into();
    Self::new(SyntheticInputKind::KeyDown {
      code: physical_code_for(&key),
      key,
    })
  }

  pub fn key_up(key: impl Into<String>) -> Self {
    let key = key.into();
    Self::new(SyntheticInputKind::KeyUp {
      code: physical_code_for(&key),
      key,
    })
  }

  pub fn char(character: char) -> Self {
    Self::new(SyntheticInputKind::Char(character))
  }

  /// Expand a string into one [`SyntheticInputKind::Char`] per character.
  pub fn text(value: &str) -> Vec<Self> {
    value.chars().map(Self::char).collect()
  }
}

/// Deliver one synthetic event into the tree through the same entry points the
/// winit shell uses for real events.
///
/// Kept beside the event type rather than in the shell so both shells share one
/// implementation and the ordering rules are stated once. Public because a
/// headless harness drives a [`Tree`] directly, with no window to queue through.
pub fn apply(tree: &mut Tree, input: &SyntheticInput) {
  let m = input.modifiers();
  match input.kind() {
    SyntheticInputKind::MouseMove { x, y } => {
      tree.mouse_move_with_modifiers(*x, *y, m.shift, m.ctrl, m.alt);
    }
    SyntheticInputKind::MouseDown { x, y, button } => {
      tree.mouse_down_with_modifiers(*x, *y, *button, m.shift, m.ctrl, m.alt);
    }
    SyntheticInputKind::MouseUp { x, y, button } => {
      tree.mouse_up_with_modifiers(*x, *y, *button, m.shift, m.ctrl, m.alt);
    }
    SyntheticInputKind::Click { x, y, button } => {
      // Hover first: hit-testing reads the last motion event, so a press with
      // no preceding move lands wherever the pointer was left.
      tree.mouse_move_with_modifiers(*x, *y, m.shift, m.ctrl, m.alt);
      tree.mouse_down_with_modifiers(*x, *y, *button, m.shift, m.ctrl, m.alt);
      tree.mouse_up_with_modifiers(*x, *y, *button, m.shift, m.ctrl, m.alt);
    }
    SyntheticInputKind::Wheel {
      x,
      y,
      delta_x,
      delta_y,
    } => {
      // A wheel notch is a complete gesture: scroll containers latch on Start
      // and release on End, so omitting either leaves one latched.
      tree.mouse_move_with_modifiers(*x, *y, m.shift, m.ctrl, m.alt);
      tree.scroll(*x, *y, 0.0, 0.0, ScrollPhase::Start);
      tree.scroll(*x, *y, *delta_x, *delta_y, ScrollPhase::Scroll);
      tree.scroll(*x, *y, 0.0, 0.0, ScrollPhase::End);
    }
    SyntheticInputKind::KeyDown { key, code } => {
      tree.key_down_with_meta(key.clone(), code.clone(), m.shift, m.ctrl, m.alt, m.meta);
    }
    SyntheticInputKind::KeyUp { key, code } => {
      tree.key_up_with_meta(key.clone(), code.clone(), m.shift, m.ctrl, m.alt, m.meta);
    }
    SyntheticInputKind::Char(character) => {
      let key = character.to_string();
      let code = physical_code_for(&key);
      tree.key_down_with_meta(key.clone(), code.clone(), m.shift, m.ctrl, m.alt, m.meta);
      tree.key_up_with_meta(key, code, m.shift, m.ctrl, m.alt, m.meta);
    }
  }
}

/// Best-effort `KeyboardEvent.code` for a key name.
///
/// The shell derives this from the physical scancode; a synthetic caller
/// usually has only the logical key, and consumers that branch on `code` are
/// looking for letters and digits.
fn physical_code_for(key: &str) -> String {
  let mut chars = key.chars();
  match (chars.next(), chars.next()) {
    (Some(single), None) if single.is_ascii_alphabetic() => {
      format!("Key{}", single.to_ascii_uppercase())
    }
    (Some(single), None) if single.is_ascii_digit() => format!("Digit{single}"),
    _ => key.to_owned(),
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn letters_and_digits_get_physical_codes() {
    assert_eq!(physical_code_for("a"), "KeyA");
    assert_eq!(physical_code_for("Z"), "KeyZ");
    assert_eq!(physical_code_for("7"), "Digit7");
    assert_eq!(physical_code_for("Enter"), "Enter");
    assert_eq!(physical_code_for("ArrowDown"), "ArrowDown");
  }

  #[test]
  fn text_expands_to_one_event_per_character() {
    let events = SyntheticInput::text("hi!");
    assert_eq!(events.len(), 3);
    assert_eq!(events[0].kind(), &SyntheticInputKind::Char('h'));
    assert_eq!(events[2].kind(), &SyntheticInputKind::Char('!'));
  }

  #[test]
  fn modifiers_are_carried_with_the_event() {
    let event = SyntheticInput::click(1.0, 2.0)
      .with_modifiers(SyntheticModifiers::default().shift().ctrl());
    assert!(event.modifiers().shift);
    assert!(event.modifiers().ctrl);
    assert!(!event.modifiers().alt);
  }
}
