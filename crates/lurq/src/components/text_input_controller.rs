//! Selection-preserving edits of one mounted plain-string input.
use std::{ops::Range, sync::Arc};

use parking_lot::Mutex;

#[cfg(feature = "markdown")]
mod styled;

use crate::{
  app::{ctx::Ctx, events::KeyboardEvent},
  core::{ElementRef, ElementRefMut, NodeId},
  node::node_kind::{TextInputState, WeakTextInputState},
};

/// UTF-8 byte positions in the source string, including selection direction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InputSelection {
  pub caret: usize,
  pub anchor: Option<usize>,
}

impl InputSelection {
  pub fn range(self) -> Range<usize> {
    let anchor = self.anchor.unwrap_or(self.caret);
    anchor.min(self.caret)..anchor.max(self.caret)
  }
}

/// Where a caret stands in an input's text: see [`TextInputController::caret_point`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InputCaretPoint {
  pub x: f32,
  /// The top of the text's band at the caret.
  pub y: f32,
  /// The height of the text's band at the caret.
  pub height: f32,
}

#[derive(Debug, PartialEq, Eq)]
pub enum InputEditError {
  NotMounted,
  Masked,
  Composing,
  InvalidSelection,
}

impl std::fmt::Display for InputEditError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.write_str(match self {
      Self::NotMounted => "input controller not mounted",
      Self::Masked => "masked input does not expose controller selection",
      Self::Composing => "finish the input-method composition before editing",
      Self::InvalidSelection => "selection is not a source UTF-8 boundary",
    })
  }
}
impl std::error::Error for InputEditError {}

#[derive(Default)]
struct Binding {
  state: Option<WeakTextInputState>,
}

/// Retain one controller per keyed input. Bind it with [`super::TextInput::controller`].
/// Edits use the keyboard's existing on_input, validation and undo path. Values stay plain strings.
#[derive(Clone, Default)]
pub struct TextInputController {
  binding: Arc<Mutex<Binding>>,
  element: ElementRefMut,
}

impl TextInputController {
  pub fn new() -> Self {
    Self::default()
  }

  pub(crate) fn bind(&self, state: &TextInputState) {
    let mut binding = self.binding.lock();
    // Runtime reconciliation owns identity and editing-state preservation.
    // The controller observes that state and must not retain a removed field.
    binding.state = Some(state.downgrade());
  }

  pub(crate) fn reference(&self) -> ElementRefMut {
    self.element.clone()
  }

  fn state(&self) -> Result<TextInputState, InputEditError> {
    if !self.element.is_attached() {
      return Err(InputEditError::NotMounted);
    }
    let state = self
      .binding
      .lock()
      .state
      .as_ref()
      .and_then(WeakTextInputState::upgrade)
      .ok_or(InputEditError::NotMounted)?;
    if state.is_masked() {
      return Err(InputEditError::Masked);
    }
    if state.is_composing() {
      return Err(InputEditError::Composing);
    }
    Ok(state)
  }

  /// Reads the retained caret/selection even after a picker took keyboard focus.
  pub fn selection(&self) -> Result<InputSelection, InputEditError> {
    let (caret, anchor) = self.state()?.controller_selection();
    Ok(InputSelection { caret, anchor })
  }

  /// Explicit selection using UTF-8 byte boundaries; invalid positions never clamp silently.
  pub fn select(&self, anchor: usize, caret: usize) -> Result<(), InputEditError> {
    if self.state()?.controller_select(anchor, caret) {
      Ok(())
    } else {
      Err(InputEditError::InvalidSelection)
    }
  }

  /// Replaces the selection or inserts at its retained caret, as one undo operation.
  /// Returns false for a prevented/no-op edit. Active IME composition is refused, never discarded.
  /// The initiating editor command carries no physical key target (`NodeId::UNASSIGNED`).
  pub fn replace_selection(&self, text: &str) -> Result<bool, InputEditError> {
    let state = self.state()?;
    state.sync_external_value();
    let keyboard = KeyboardEvent::new(text, "InsertText", false, false, false, false, NodeId::UNASSIGNED);
    Ok(state.replace_selection_text(text, &keyboard))
  }

  /// Where the caret before `byte` stood when the input was last laid out, for what is hung from a place in
  /// the text, as a list of suggestions under a word being typed. It is measured from the start of the
  /// text's box, inside the input's padding (and, for a single line, the room that centres it), with what
  /// the input scrolled taken off. Layout follows an edit, so in the render an edit causes the point is
  /// that of the text before it: a byte before the edit stands where it stood.
  pub fn caret_point(&self, byte: usize) -> Result<InputCaretPoint, InputEditError> {
    let (x, y, height) = self.state()?.controller_point(byte);
    Ok(InputCaretPoint { x, y, height })
  }

  /// Request normal runtime focus/scroll after the picker closes in the following render.
  pub fn focus(&self, ctx: &Ctx) {
    ctx.focus(&self.element.as_ref());
  }

  pub fn element_ref(&self) -> ElementRef {
    self.element.as_ref()
  }
}
