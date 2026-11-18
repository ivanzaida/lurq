//! An app's own frames: a callback run once in each frame a window draws, in step with its display
//! ([`Ctx::on_frame`], [`WindowHandle::on_frame`](crate::app::window::WindowHandle::on_frame)).
//!
//! While any callback of a window is running the window draws a frame each display refresh, as it does while an
//! animation runs: the runtime counts the callbacks as a timeline ([`Tree::has_active_timeline`]) whose next change
//! is due at once ([`Tree::next_timeline_change`]), so the shell paces them as it paces a transition's frames, and a
//! pass that runs them lays the window out and paints it. A callback runs at the start of the pass, before the
//! components it dirtied are rendered and the window is laid out: what it sets is in the frame it ran for.
//!
//! [`Tree::has_active_timeline`]: crate::app::Tree::has_active_timeline
//! [`Tree::next_timeline_change`]: crate::app::Tree::next_timeline_change

use std::sync::{Arc, Weak};

use parking_lot::Mutex;

use super::Ctx;
use crate::{
  app::window::WindowWaker,
  time::{Duration, Instant},
};

/// One frame a callback of [`Ctx::on_frame`] runs for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Frame {
  /// The frame's time, on the runtime's clock: the time its animations are drawn at.
  pub time: Instant,
  /// The time since the frame this callback last ran for. Zero on its first frame, on the first after it was started
  /// again, and on the first after the window was hidden (minimized or covered) and shown again: a callback that
  /// steps a simulation by it does not jump by the time the window was not seen. A frame the machine was late for
  /// is given its real delta.
  pub delta: Duration,
}

impl Frame {
  /// [`Frame::delta`] in seconds, for a step of a simulation.
  pub fn delta_secs(&self) -> f32 {
    self.delta.as_secs_f32()
  }
}

type FrameFn = Box<dyn FnMut(Frame) -> bool + Send>;

/// A callback of [`Ctx::on_frame`] or [`WindowHandle::on_frame`](crate::app::window::WindowHandle::on_frame), and
/// the handle that stops it and starts it again. Clones are the same callback.
///
/// A component's callbacks are dropped with the component; a window's stay until they stop and their handles are
/// dropped.
#[derive(Clone)]
pub struct FrameCallback {
  inner: Arc<FrameEntry>,
}

struct FrameEntry {
  state: Mutex<FrameState>,
  /// Locked only while the callback runs, apart from `state`: the callback may cancel or start itself.
  callback: Mutex<FrameFn>,
  /// Wakes the event loop when the callback is started from outside a turn of it.
  wake: Option<WindowWaker>,
  /// The window's list a window's callback is kept in, which it goes back into when it is started again.
  home: Option<Weak<Mutex<Vec<FrameCallback>>>>,
}

#[derive(Default)]
struct FrameState {
  active: bool,
  /// The time of the last frame it ran for, since it started; `None` before the first.
  last: Option<Instant>,
}

impl FrameCallback {
  fn with_home(callback: FrameFn, wake: Option<WindowWaker>, home: Option<Weak<Mutex<Vec<FrameCallback>>>>) -> Self {
    Self {
      inner: Arc::new(FrameEntry {
        state: Mutex::new(FrameState {
          active: true,
          last: None,
        }),
        callback: Mutex::new(callback),
        wake,
        home,
      }),
    }
  }

  /// Stops the callback: it runs for no frame from now on, and the window draws no frame for it. Called from the
  /// callback itself, it is the frame it runs for that is its last, whatever it returns.
  pub fn cancel(&self) {
    self.inner.state.lock().active = false;
  }

  /// Starts the callback again once it stopped (it returned `false`, or was cancelled), from the next frame, which
  /// the window draws for it: its delta is zero there, as on a first frame. A running callback goes on as it was.
  pub fn start(&self) {
    {
      let mut state = self.inner.state.lock();
      if state.active {
        return;
      }
      state.active = true;
      state.last = None;
    }
    if let Some(home) = self.inner.home.as_ref().and_then(Weak::upgrade) {
      let mut list = home.lock();
      if !list.iter().any(|callback| callback.same(self)) {
        list.push(self.clone());
      }
    }
    if let Some(wake) = &self.inner.wake {
      wake();
    }
  }

  /// Whether the callback runs for the next frame.
  pub fn is_active(&self) -> bool {
    self.inner.state.lock().active
  }

  fn same(&self, other: &Self) -> bool {
    Arc::ptr_eq(&self.inner, &other.inner)
  }

  /// Puts `callback` in place of the one this runs, keeping whether it runs and when it last ran: a render that
  /// calls [`Ctx::on_frame`] again at the same place.
  fn replace(&self, callback: FrameFn) {
    *self.inner.callback.lock() = callback;
  }

  /// Runs the callback for the frame at `now`, if it runs. Returns whether it ran.
  fn run(&self, now: Instant) -> bool {
    let delta = {
      let mut state = self.inner.state.lock();
      if !state.active {
        return false;
      }
      let delta = state
        .last
        .map_or(Duration::ZERO, |last| now.saturating_duration_since(last));
      state.last = Some(now);
      delta
    };
    let more = (self.inner.callback.lock())(Frame { time: now, delta });
    if !more {
      self.inner.state.lock().active = false;
    }
    true
  }

  /// The next frame's delta is zero: the window was not seen since the last.
  fn forget_last(&self) {
    self.inner.state.lock().last = None;
  }
}

/// The callbacks a window keeps by itself, those of
/// [`WindowHandle::on_frame`](crate::app::window::WindowHandle::on_frame).
#[derive(Clone, Default)]
pub(crate) struct WindowFrames {
  list: Arc<Mutex<Vec<FrameCallback>>>,
}

impl WindowFrames {
  pub(crate) fn add(&self, callback: FrameFn, wake: Option<WindowWaker>) -> FrameCallback {
    let callback = FrameCallback::with_home(callback, wake, Some(Arc::downgrade(&self.list)));
    self.list.lock().push(callback.clone());
    callback
  }

  pub(crate) fn any_active(&self) -> bool {
    self.list.lock().iter().any(FrameCallback::is_active)
  }

  /// Runs each callback for the frame at `now`, outside the list's lock (a callback may add another), and drops
  /// those that stopped. Returns whether any ran.
  pub(crate) fn run(&self, now: Instant) -> bool {
    let callbacks: Vec<FrameCallback> = self.list.lock().clone();
    let mut ran = false;
    for callback in &callbacks {
      ran |= callback.run(now);
    }
    self.list.lock().retain(FrameCallback::is_active);
    ran
  }

  pub(crate) fn forget_last(&self) {
    for callback in self.list.lock().iter() {
      callback.forget_last();
    }
  }
}

/// A component's callbacks: those it made outside a render, kept until it unmounts, and those of its render, by the
/// place of the call.
#[derive(Default)]
pub(super) struct ComponentFrames {
  stable: Vec<FrameCallback>,
  slots: Vec<FrameCallback>,
  cursor: usize,
}

impl ComponentFrames {
  pub(super) fn begin_render(&mut self) {
    self.cursor = 0;
  }

  /// Drops the callbacks of render places this render did not reach.
  pub(super) fn end_render(&mut self) {
    self.slots.truncate(self.cursor);
  }

  fn iter(&self) -> impl Iterator<Item = &FrameCallback> {
    self.stable.iter().chain(&self.slots)
  }

  pub(super) fn any_active(&self) -> bool {
    self.iter().any(FrameCallback::is_active)
  }

  pub(super) fn run(&self, now: Instant) -> bool {
    let mut ran = false;
    for callback in self.iter() {
      ran |= callback.run(now);
    }
    ran
  }

  pub(super) fn forget_last(&self) {
    for callback in self.iter() {
      callback.forget_last();
    }
  }

  pub(super) fn capacity_bytes(&self) -> usize {
    (self.stable.capacity() + self.slots.capacity()) * std::mem::size_of::<FrameCallback>()
  }
}

impl Ctx {
  /// Runs `callback` once in each frame the window draws, in step with its display, for as long as it returns
  /// `true`: lurq's `requestAnimationFrame`, for a spring, a body with physics, a drag's inertia, a game, or a value
  /// that follows another smoothly. It is told the frame's time and the time since the frame it last ran for
  /// ([`Frame`]).
  ///
  /// It runs at the start of the frame, before the window is rendered and laid out: a signal it sets is shown in
  /// that frame. While it runs the window draws a frame each refresh of its display, laid out and painted, as it
  /// does for an animation; once it returns `false`, or is cancelled ([`FrameCallback::cancel`]), it runs no more
  /// and the window draws nothing for it. [`FrameCallback::start`] starts it again. It is paused while the window
  /// is minimized or covered, as animations are, and its first frame after has a delta of zero. Reduced motion does
  /// not stop it: what it moves is the app's, which reads [`Ctx::reduced_motion`] where it should move less.
  ///
  /// Called in `create`, the callback is the component's until it unmounts. Called in a render, it is the callback
  /// of that place in the render, as a [`Ctx::future`] is: each render puts its closure in place of the last one's,
  /// keeping whether it runs, and a render that no longer makes the call drops it. Either way it is dropped with the
  /// component, and a component kept off the stage runs none. Starts running at once.
  pub fn on_frame(&mut self, callback: impl FnMut(Frame) -> bool + Send + 'static) -> FrameCallback {
    let callback: FrameFn = Box::new(callback);
    let wake = self.window.as_ref().map(crate::app::window::Window::loop_waker);
    if !self.rendering {
      let made = FrameCallback::with_home(callback, wake, None);
      self.frames.stable.push(made.clone());
      return made;
    }
    let cursor = self.frames.cursor;
    self.frames.cursor += 1;
    if let Some(slot) = self.frames.slots.get(cursor) {
      slot.replace(callback);
      return slot.clone();
    }
    let made = FrameCallback::with_home(callback, wake, None);
    self.frames.slots.push(made.clone());
    made
  }

  /// Whether a callback of [`Ctx::on_frame`] of this component or a mounted descendant runs for the next frame.
  pub(crate) fn has_active_frame_callbacks(&self) -> bool {
    self.frames.any_active()
      || self
        .children
        .iter()
        .any(|slot| !slot.offstage && slot.ctx.has_active_frame_callbacks())
  }

  /// Runs the callbacks of this component and its mounted descendants for the frame at `now`, parents first.
  /// Returns whether any ran.
  pub(crate) fn run_frame_callbacks(&self, now: Instant) -> bool {
    let mut ran = self.frames.run(now);
    for slot in &self.children {
      if slot.offstage {
        // Off the stage it runs none, and its first frame back is as a first frame.
        slot.ctx.forget_last_frames();
      } else {
        ran |= slot.ctx.run_frame_callbacks(now);
      }
    }
    ran
  }

  /// The next frame of every callback has a delta of zero: the window was not seen since the last.
  pub(crate) fn forget_last_frames(&self) {
    self.frames.forget_last();
    for slot in &self.children {
      slot.ctx.forget_last_frames();
    }
  }
}
