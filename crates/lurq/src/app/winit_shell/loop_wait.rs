//! How long the event loop may sleep once a turn has handled its events. An idle loop sleeps until the next
//! deadline or a wake: input, a window command, an MCP call, a menu action, a future or query that can make
//! progress, and a timer started from another thread all wake it.

use std::time::Instant;

use winit::event_loop::ControlFlow;

/// What the windows of the loop need from it after a turn.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct LoopNeeds {
  /// A video is presented directly from the loop, every turn.
  pub(super) continuous_video: bool,
  /// A redraw was requested and its event is on its way (unless the window is minimized or occluded).
  pub(super) redraw_pending: bool,
  /// Something has work for the next turn that nothing wakes the loop for: an `on_tick` callback, a pending click
  /// waiting for a double click, a query ready to be ticked.
  pub(super) polling: bool,
  /// The earliest scheduled redraw: an animation frame, the caret's blink, a query's expiry.
  pub(super) next_redraw: Option<Instant>,
  /// When the earliest running timer fires.
  pub(super) next_timer: Option<Instant>,
}

/// How the loop waits for its next turn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum LoopWait {
  Poll,
  WaitUntil(Instant),
  Wait,
}

impl LoopWait {
  /// `now` is when the turn decided how to wait.
  pub(super) fn of(needs: LoopNeeds, now: Instant) -> Self {
    if needs.continuous_video {
      return Self::Poll;
    }
    if needs.redraw_pending {
      // The redraw's event brings the next turn, but a window that is minimized or occluded may not get it for
      // a long time, so timers still wake the loop: at their time, or at once when one is already due (the next
      // turn fires it, so even a 1 ms interval keeps firing). A scheduled frame already due is what the pending
      // redraw serves: it is left out, or waking for it would spin while the redraw does not come, and it must
      // not hide a later deadline.
      let next_redraw = needs.next_redraw.filter(|redraw| *redraw > now);
      return Self::until(next_redraw.into_iter().chain(needs.next_timer).min());
    }
    if needs.polling {
      return Self::Poll;
    }
    Self::until(needs.next_redraw.into_iter().chain(needs.next_timer).min())
  }

  fn until(deadline: Option<Instant>) -> Self {
    deadline.map_or(Self::Wait, Self::WaitUntil)
  }

  pub(super) fn control_flow(self) -> ControlFlow {
    match self {
      Self::Poll => ControlFlow::Poll,
      Self::WaitUntil(deadline) => ControlFlow::WaitUntil(deadline),
      Self::Wait => ControlFlow::Wait,
    }
  }

  /// The name the loop cadence log counts.
  pub(super) fn name(self) -> &'static str {
    match self {
      Self::Poll => "poll",
      Self::WaitUntil(_) => "wait_until",
      Self::Wait => "wait",
    }
  }
}
