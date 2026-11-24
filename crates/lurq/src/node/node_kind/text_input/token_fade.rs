//! The look a token takes under the pointer, coming and going by degrees: where its style says how long
//! ([`InputTokenStyle::fade`](crate::components::InputTokenStyle::fade)), the colors of a token the pointer comes
//! onto run from its rest look to that one, and those of the token it leaves back, over that time.
//!
//! The input keeps which token the look leaves and when the change was first laid out; the layout mixes the two looks
//! by how far the change is. While a change is on its way the window draws frames for it: the inputs whose looks are
//! changing are kept here, laid out again at each frame, and let go once their change is over.

use std::{cell::RefCell, time::Duration};

use super::TextInputState;
use crate::time::Instant;

/// A change of the token under the pointer, on its way.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct TokenFade {
  /// The token the look leaves, where one had it.
  pub(crate) from: Option<usize>,
  /// When the change was first laid out: `None` until then.
  pub(crate) start: Option<Instant>,
}

thread_local! {
  /// The inputs whose token looks are changing, and the moment the frame being laid out is drawn for.
  static FADING: RefCell<Vec<TextInputState>> = const { RefCell::new(Vec::new()) };
  static FRAME: std::cell::Cell<Option<Instant>> = const { std::cell::Cell::new(None) };
}

/// Sets the moment the frame being laid out is drawn for, and lays out again each input whose token looks are
/// changing. Called by the runtime as it starts to lay a frame out.
pub(crate) fn begin_frame(now: Instant) {
  FRAME.with(|frame| frame.set(Some(now)));
  FADING.with(|fading| {
    fading.borrow_mut().retain(|state| {
      let going = state.token_fade_going(now);
      state.mark_layout_dirty_keeping_scroll();
      going
    })
  });
}

/// Whether a token look is changing: the window draws frames for it.
pub(crate) fn active() -> bool {
  FADING.with(|fading| !fading.borrow().is_empty())
}

/// The moment the frame being laid out is drawn for.
pub(crate) fn now() -> Instant {
  FRAME.with(|frame| frame.get()).unwrap_or_else(Instant::now)
}

/// Keeps `state` laid out at each frame while its token look changes.
pub(super) fn follow(state: &TextInputState) {
  FADING.with(|fading| {
    let mut fading = fading.borrow_mut();
    if !fading
      .iter()
      .any(|held| std::sync::Arc::ptr_eq(&held.inner, &state.inner))
    {
      fading.push(state.clone());
    }
  });
}

/// How much of the look under the pointer the token at `index` has, between its rest look (0) and that one (1):
/// `hovered` is the token under the pointer, `fade` the change on its way, `start` when it was first laid out and
/// `length` how long it takes.
pub(crate) fn share(
  index: usize,
  hovered: Option<usize>,
  fade: Option<TokenFade>,
  now: Instant,
  length: Duration,
) -> f32 {
  let on = hovered == Some(index);
  let Some(fade) = fade.filter(|_| !length.is_zero()) else {
    return if on { 1.0 } else { 0.0 };
  };
  let start = fade.start.unwrap_or(now);
  let progress = (now.saturating_duration_since(start).as_secs_f32() / length.as_secs_f32()).clamp(0.0, 1.0);
  match (on, fade.from == Some(index)) {
    (true, _) => progress,
    (false, true) => 1.0 - progress,
    (false, false) => 0.0,
  }
}
