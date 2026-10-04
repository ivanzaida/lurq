/// Input method (IME) composition input for [`Tree::ime`](crate::app::Tree::ime),
/// the platform-neutral form of winit's `Ime` window event.
///
/// An input method composes text before it reaches the field: Japanese,
/// Chinese and Korean input, dead keys and the macOS accent menu. While a
/// composition is in progress its text (the *preedit*) is shown in the focused
/// `TextInput` at the caret, underlined, without changing the input's value;
/// [`ImeEvent::Commit`] inserts the final text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ImeEvent {
  /// The input method became active for the window.
  Enabled,
  /// The composition text changed. An empty `text` ends the composition
  /// without inserting anything (it was cancelled, or a commit follows).
  /// `cursor` is the input method's cursor (or selected clause) within
  /// `text`, as UTF-8 byte offsets `(start, end)`; `None` hides the caret.
  Preedit {
    text: String,
    cursor: Option<(usize, usize)>,
  },
  /// The composition finished with `text`, which the focused text input
  /// inserts like typed text (its `on_input` handlers run first).
  Commit(String),
  /// The input method was turned off for the window; any composition ends
  /// without inserting anything.
  Disabled,
}

/// Where the input method should place its candidate window: the caret of
/// the focused text input, in logical pixels from the window's top-left
/// corner. See [`Tree::ime_cursor_area`](crate::app::Tree::ime_cursor_area).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ImeCursorArea {
  pub x: f32,
  pub y: f32,
  pub width: f32,
  pub height: f32,
}
