//! Where the input being dispatched came from: the operating system, or
//! lurq's synthetic input (MCP tools and [`apply`](crate::app::synthetic_input::apply)).
//!
//! Synthetic input reaches the same entry points as OS input, so a handler
//! cannot tell the two apart by the event. While lurq delivers synthetic
//! input, [`input_source`] says so, and an app can ignore it on a control that
//! only a person may use.

use std::cell::Cell;

/// Where the input whose handlers are running came from.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum InputSource {
  /// Input the window received from the operating system, and anything that
  /// is not synthetic input. OS input does not prove that a person made it:
  /// another program can post OS input.
  #[default]
  Os,
  /// lurq's synthetic input: an MCP tool call (`lurq_interact`, `lurq_act`,
  /// `lurq_set_value` and every other tool that reaches the event loop,
  /// including the release of input an MCP client held), a
  /// [`SyntheticInput`](crate::app::synthetic_input::SyntheticInput) applied
  /// with [`apply`](crate::app::synthetic_input::apply) or queued on a window,
  /// and code run through [`with_synthetic_input`].
  Synthetic,
}

thread_local! {
  static SOURCE: Cell<InputSource> = const { Cell::new(InputSource::Os) };
}

/// The source of the input whose handlers run now on this thread: `Synthetic`
/// while lurq delivers synthetic input, `Os` otherwise.
///
/// Read it in an event handler, an `on_input` or `on_change` callback, or a
/// focus handler, which lurq calls while it delivers the event. Work deferred
/// past the delivery (a spawned task, an effect of a later render, a timer)
/// sees `Os`.
///
/// ```
/// use lurq::app::events::{MouseEvent, is_synthetic_input};
///
/// let on_click = |_: MouseEvent| {
///   if is_synthetic_input() {
///     return; // only a person may press this
///   }
///   // ...
/// };
/// # let _ = on_click;
/// ```
pub fn input_source() -> InputSource {
  SOURCE.with(Cell::get)
}

/// Whether the input whose handlers run now is lurq's synthetic input.
pub fn is_synthetic_input() -> bool {
  input_source() == InputSource::Synthetic
}

/// Runs `deliver` with its input marked as synthetic, for an automation layer
/// that drives a [`Tree`](crate::app::Tree) through its input methods
/// directly. lurq's own synthetic input does this already.
pub fn with_synthetic_input<R>(deliver: impl FnOnce() -> R) -> R {
  let _restore = SourceGuard(SOURCE.with(|source| source.replace(InputSource::Synthetic)));
  deliver()
}

/// Restores the previous source, also when a handler panics.
struct SourceGuard(InputSource);

impl Drop for SourceGuard {
  fn drop(&mut self) {
    SOURCE.with(|source| source.set(self.0));
  }
}
