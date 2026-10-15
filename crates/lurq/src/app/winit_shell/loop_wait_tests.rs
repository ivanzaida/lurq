//! An idle loop sleeps until its earliest deadline instead of polling: a minute-long interval and a stream that
//! waits for its next item leave nothing to poll for.

use std::{
  future::pending,
  time::{Duration, Instant},
};

use super::loop_wait::{LoopNeeds, LoopWait};
use crate::{
  app::{
    App, Tree,
    component::Component,
    ctx::{Ctx, Interval, StreamEmitter},
  },
  node::Element,
};

const MINUTE: Duration = Duration::from_secs(60);

struct Clock {
  _interval: Interval,
}

impl Component for Clock {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    let interval = ctx.create_interval(MINUTE, || {});
    interval.start();
    Self { _interval: interval }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    ctx.stream((), |_, _emitter: StreamEmitter<u32, ()>| pending::<()>());
    Element::new()
  }
}

fn needs(tree: &Tree) -> LoopNeeds {
  LoopNeeds {
    polling: tree.needs_polling(),
    next_redraw: tree.next_scheduled_redraw(),
    next_timer: tree.next_timer_at(),
    ..LoopNeeds::default()
  }
}

#[test]
fn an_idle_window_with_a_minute_interval_and_a_waiting_stream_sleeps_until_the_interval() {
  let mut app = App::new();
  let mut tree = Tree::new();
  let mounted_at = Instant::now();
  tree.mount_root::<Clock>(&mut app, ());
  let started_by = Instant::now();
  tree.tick_timers();
  tree.tick_futures();
  assert!(
    tree.has_active_tick_sources(),
    "the interval and the stream are running"
  );

  let LoopWait::WaitUntil(deadline) = LoopWait::of(needs(&tree), Instant::now()) else {
    panic!(
      "an idle loop must not poll: {:?}",
      LoopWait::of(needs(&tree), Instant::now())
    );
  };
  assert!(
    deadline >= mounted_at + MINUTE && deadline <= started_by + MINUTE,
    "the loop wakes when the interval fires"
  );
}

#[test]
fn a_tree_with_nothing_scheduled_waits_for_an_event() {
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.set_root(Element::new());
  tree.pass_headless(&mut app);

  assert_eq!(LoopWait::of(needs(&tree), Instant::now()), LoopWait::Wait);
}

#[test]
fn the_earliest_of_a_scheduled_redraw_and_a_timer_wakes_the_loop() {
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.mount_root::<Clock>(&mut app, ());
  let soon = Instant::now() + Duration::from_millis(16);
  tree.request_redraw_at(soon);

  assert_eq!(LoopWait::of(needs(&tree), Instant::now()), LoopWait::WaitUntil(soon));
}

#[test]
fn video_polls_and_a_pending_redraw_waits_for_its_event() {
  let video = LoopNeeds {
    continuous_video: true,
    redraw_pending: true,
    next_timer: Some(Instant::now() + MINUTE),
    ..LoopNeeds::default()
  };
  assert_eq!(LoopWait::of(video, Instant::now()), LoopWait::Poll);

  let pending = LoopNeeds {
    redraw_pending: true,
    polling: true,
    ..LoopNeeds::default()
  };
  assert_eq!(LoopWait::of(pending, Instant::now()), LoopWait::Wait);
}

#[test]
fn a_pending_redraw_still_wakes_the_loop_for_the_next_timer() {
  // A redraw a minimized or occluded window never gets must not stall its timers.
  let now = Instant::now();
  let timer = now + MINUTE;
  let pending = LoopNeeds {
    redraw_pending: true,
    polling: true,
    next_timer: Some(timer),
    ..LoopNeeds::default()
  };
  assert_eq!(LoopWait::of(pending, now), LoopWait::WaitUntil(timer));

  // A scheduled frame already due is what the pending redraw serves: waking for it would spin while the redraw
  // does not come, and it must not hide the timer after it.
  let overdue_frame = LoopNeeds {
    next_redraw: Some(now - Duration::from_millis(1)),
    ..pending
  };
  assert_eq!(LoopWait::of(overdue_frame, now), LoopWait::WaitUntil(timer));
  let overdue_frame_only = LoopNeeds {
    next_timer: None,
    ..overdue_frame
  };
  assert_eq!(LoopWait::of(overdue_frame_only, now), LoopWait::Wait);

  // A later frame still counts.
  let later_frame = LoopNeeds {
    next_redraw: Some(now + Duration::from_millis(16)),
    ..pending
  };
  assert_eq!(
    LoopWait::of(later_frame, now),
    LoopWait::WaitUntil(now + Duration::from_millis(16))
  );
}

#[test]
fn a_timer_already_due_wakes_the_loop_at_once_even_with_a_redraw_pending() {
  // A 1 ms interval in a minimized window: by the time the turn decides how to wait, its next fire may be due.
  // The next turn fires it, so it keeps firing.
  let now = Instant::now();
  let due = now - Duration::from_micros(200);
  let pending = LoopNeeds {
    redraw_pending: true,
    next_redraw: Some(now - Duration::from_millis(1)),
    next_timer: Some(due),
    ..LoopNeeds::default()
  };
  assert_eq!(LoopWait::of(pending, now), LoopWait::WaitUntil(due));
}

#[test]
fn polling_work_wins_over_a_deadline() {
  let needs = LoopNeeds {
    polling: true,
    next_timer: Some(Instant::now() + MINUTE),
    ..LoopNeeds::default()
  };
  assert_eq!(LoopWait::of(needs, Instant::now()), LoopWait::Poll);
}
