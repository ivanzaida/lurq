//! A color a person picks from anywhere on the screen: another app's window, a picture, the app itself.
//!
//! [`EyeDropper::pick`] starts picking and gives the color under the pointer when the person presses, or nothing when
//! they press Escape (or the right button, or switch to another window). While it picks, the pointer is a cross and a
//! loupe follows it: the pixels around it magnified, the one it takes outlined in the middle, and its hex beside it.
//! The arrow keys move the pointer by a pixel (ten with Shift), and Enter or Space take the color as a press does.
//!
//! ```no_run
//! use lurq::{app::ctx::Ctx, eyedropper::EyeDropper};
//!
//! # fn pick(ctx: &mut Ctx) {
//! let window = ctx.window();
//! let pick = ctx.future_action(move |()| EyeDropper::new().pick(&window));
//! // In a handler: `pick.run(())`; `pick.state()` then holds `Ok(Some(color))`, `Ok(None)` where the person gave up,
//! // or `Err(Unsupported)` where the platform has no picking.
//! # }
//! ```
//!
//! # What each platform gives
//!
//! - **Windows**: the pick runs on a thread of its own, with the platform's own means. A window that takes the pointer
//!   and the keys lies over every monitor, all but transparent, and a small window draws the loupe beside the pointer;
//!   both are kept out of screen captures (`WDA_EXCLUDEFROMCAPTURE`), so the pixels read with GDI under the pointer are
//!   what the person sees, refreshed as the pointer moves and ten times a second. Where the system cannot keep them out
//!   (before Windows 10 2004), the screen is read once as the pick starts. The thread is aware of each monitor's scale,
//!   so at 150% and across monitors of different scales the pixel taken is the physical pixel under the pointer, and
//!   the loupe is as large on each monitor.
//! - **macOS, Linux, iOS and a browser**: no picking yet. [`EyeDropper::is_supported`] says so, and
//!   [`EyeDropper::pick`] gives `Err(Unsupported)` at once. (A browser's own `EyeDropper` is the way for the web build,
//!   where there is one.)
//!
//! A test, or a tool driving the app, puts a stand-in in place of the screen with [`EyeDropper::stand_in`].

#[cfg(windows)]
mod windows;

pub(crate) mod geometry;

use std::{
  cell::RefCell,
  future::Future,
  pin::Pin,
  sync::{Arc, Mutex},
  task::{Context, Poll, Waker},
};

use crate::{
  app::{
    WindowHandle,
    theme::{Theme, ThemePalette},
  },
  node::color::Color,
};

/// What a pick gives: the color taken, nothing where the person gave up, or [`Unsupported`].
pub type Picked = Result<Option<Color>, Unsupported>;

/// The platform has no picking from the screen: see the [module](self).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, lurq_macros::DevtoolsInspectable)]
pub struct Unsupported;

impl std::fmt::Display for Unsupported {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.write_str("picking a color from the screen is not supported on this platform")
  }
}

impl std::error::Error for Unsupported {}

/// The colors the loupe is drawn in: the theme's popover.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Look {
  pub background: Color,
  pub foreground: Color,
  pub border: Color,
}

impl Look {
  fn of(palette: &ThemePalette) -> Self {
    use crate::app::theme::PaletteColor::{Border, Popover, PopoverForeground};
    Self {
      background: palette.get(Popover),
      foreground: palette.get(PopoverForeground),
      border: palette.get(Border),
    }
  }
}

/// Picking a color from the screen. See the [module](self).
#[derive(Clone, Debug)]
pub struct EyeDropper {
  look: Look,
}

impl Default for EyeDropper {
  fn default() -> Self {
    Self::new()
  }
}

impl EyeDropper {
  /// A pick whose loupe is drawn in the dark theme's popover colors.
  pub fn new() -> Self {
    Self {
      look: Look::of(&ThemePalette::dark()),
    }
  }

  /// The loupe in `theme`'s popover colors.
  pub fn theme(mut self, theme: &Theme) -> Self {
    self.look = Look::of(&theme.palette());
    self
  }

  /// Whether this platform picks colors from the screen, or a stand-in is set on this thread: what a component asks
  /// before it shows an eyedropper.
  pub fn is_supported() -> bool {
    STAND_IN.with(|stand_in| stand_in.borrow().is_some()) || cfg!(windows)
  }

  /// Starts picking and gives what the person picked. The future does not borrow the window: it is for
  /// [`Ctx::future_action`](crate::app::ctx::Ctx::future_action), whose result it is as it is. A second pick asked
  /// for while one runs gives nothing at once.
  pub fn pick(self, window: &WindowHandle) -> impl Future<Output = Picked> + Send + 'static + use<> {
    let _ = window;
    let (answer, answering) = Answer::new();
    self.pick_then(move |picked| answering.give(picked));
    answer
  }

  /// Starts picking and calls `then` with what the person picked: on this thread at once where a stand-in answers,
  /// else on the thread the pick runs on. What a component that has no `Ctx` starts a pick with.
  pub(crate) fn pick_then(self, then: impl FnOnce(Picked) + Send + 'static) {
    if let Some(answer) = STAND_IN.with(|stand_in| stand_in.borrow().clone()) {
      then(answer());
      return;
    }
    native_pick(self.look, Box::new(then));
  }

  /// Puts `answer` in place of the screen on this thread, until the guard is dropped: each pick asks it, and
  /// [`EyeDropper::is_supported`] is true. For a test of what an app does with a pick, and for a tool driving it.
  pub fn stand_in(answer: impl Fn() -> Picked + 'static) -> StandIn {
    let previous = STAND_IN.with(|stand_in| stand_in.borrow_mut().replace(Rc::new(answer)));
    StandIn { previous }
  }
}

use std::rc::Rc;

thread_local! {
  /// What answers a pick on this thread in place of the screen.
  static STAND_IN: RefCell<Option<Rc<dyn Fn() -> Picked>>> = const { RefCell::new(None) };
}

/// While it is held, a stand-in answers the picks of its thread: see [`EyeDropper::stand_in`].
#[must_use = "the stand-in is taken away as the guard is dropped"]
pub struct StandIn {
  previous: Option<Rc<dyn Fn() -> Picked>>,
}

impl Drop for StandIn {
  fn drop(&mut self) {
    let previous = self.previous.take();
    STAND_IN.with(|stand_in| *stand_in.borrow_mut() = previous);
  }
}

#[cfg(windows)]
fn native_pick(look: Look, then: Box<dyn FnOnce(Picked) + Send>) {
  windows::pick(look, then);
}

#[cfg(not(windows))]
fn native_pick(_look: Look, then: Box<dyn FnOnce(Picked) + Send>) {
  then(Err(Unsupported));
}

/// The answer of a pick that runs off the loop: given once by the pick, waited on by the loop.
struct Answer(Arc<Mutex<Slot>>);

#[derive(Default)]
struct Slot {
  picked: Option<Picked>,
  waker: Option<Waker>,
}

/// What gives an [`Answer`].
struct Answering(Arc<Mutex<Slot>>);

impl Answer {
  fn new() -> (Self, Answering) {
    let slot = Arc::new(Mutex::new(Slot::default()));
    (Self(slot.clone()), Answering(slot))
  }
}

impl Future for Answer {
  type Output = Picked;

  fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
    let mut slot = self.0.lock().unwrap();
    match slot.picked.take() {
      Some(picked) => Poll::Ready(picked),
      None => {
        slot.waker = Some(cx.waker().clone());
        Poll::Pending
      }
    }
  }
}

impl Answering {
  fn give(self, picked: Picked) {
    let waker = {
      let mut slot = self.0.lock().unwrap();
      slot.picked = Some(picked);
      slot.waker.take()
    };
    if let Some(waker) = waker {
      waker.wake();
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn ready<F: Future>(future: F) -> F::Output {
    let mut future = std::pin::pin!(future);
    let waker = Waker::noop();
    let mut cx = Context::from_waker(waker);
    match future.as_mut().poll(&mut cx) {
      Poll::Ready(output) => output,
      Poll::Pending => panic!("the pick is not over"),
    }
  }

  #[test]
  fn a_stand_in_answers_a_pick_and_is_taken_away_with_its_guard() {
    let blue = Color::from_hex("#2563EB");
    {
      let _guard = EyeDropper::stand_in(move || Ok(Some(blue)));
      assert!(EyeDropper::is_supported());
      let (answer, answering) = Answer::new();
      EyeDropper::new().pick_then(move |picked| answering.give(picked));
      assert_eq!(ready(answer), Ok(Some(blue)));
      // Escape: nothing picked.
      let _cancel = EyeDropper::stand_in(|| Ok(None));
      let (answer, answering) = Answer::new();
      EyeDropper::new().pick_then(move |picked| answering.give(picked));
      assert_eq!(ready(answer), Ok(None));
    }
    assert_eq!(
      EyeDropper::is_supported(),
      cfg!(windows),
      "the guards took the stand-ins away"
    );
  }

  #[test]
  fn an_answer_given_later_wakes_what_waits_on_it() {
    use std::sync::atomic::{AtomicBool, Ordering};
    struct Flag(AtomicBool);
    impl std::task::Wake for Flag {
      fn wake(self: Arc<Self>) {
        self.0.store(true, Ordering::SeqCst);
      }
    }
    let flag = Arc::new(Flag(AtomicBool::new(false)));
    let waker = Waker::from(flag.clone());
    let mut cx = Context::from_waker(&waker);
    let (answer, answering) = Answer::new();
    let mut answer = std::pin::pin!(answer);
    assert!(answer.as_mut().poll(&mut cx).is_pending(), "picking");
    std::thread::spawn(move || answering.give(Err(Unsupported)))
      .join()
      .unwrap();
    assert!(flag.0.load(Ordering::SeqCst), "woken");
    assert_eq!(answer.as_mut().poll(&mut cx), Poll::Ready(Err(Unsupported)));
  }

  #[cfg(not(windows))]
  #[test]
  fn a_platform_without_picking_says_so_at_once() {
    assert!(!EyeDropper::is_supported());
    let (answer, answering) = Answer::new();
    EyeDropper::new().pick_then(move |picked| answering.give(picked));
    assert_eq!(ready(answer), Err(Unsupported));
  }
}
