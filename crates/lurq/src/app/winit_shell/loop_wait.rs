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
  /// A redraw was requested and its event is on its way.
  pub(super) redraw_pending: bool,
  /// Something has work for the next turn that nothing wakes the loop for: an `on_tick` callback, a pending click
  /// waiting for a double click, a query ready to be ticked.
  pub(super) polling: bool,
  /// The earliest scheduled redraw (animation frame, caret blink, query expiry) or timer.
  pub(super) next_deadline: Option<Instant>,
}

/// How the loop waits for its next turn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum LoopWait {
  Poll,
  WaitUntil(Instant),
  Wait,
}

impl LoopWait {
  pub(super) fn of(needs: LoopNeeds) -> Self {
    if needs.continuous_video {
      return Self::Poll;
    }
    if needs.redraw_pending {
      return Self::Wait;
    }
    if needs.polling {
      return Self::Poll;
    }
    match needs.next_deadline {
      Some(deadline) => Self::WaitUntil(deadline),
      None => Self::Wait,
    }
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
