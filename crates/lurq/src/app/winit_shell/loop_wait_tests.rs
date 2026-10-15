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
    next_deadline: tree.next_wake_at(),
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

  let LoopWait::WaitUntil(deadline) = LoopWait::of(needs(&tree)) else {
    panic!("an idle loop must not poll: {:?}", LoopWait::of(needs(&tree)));
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

  assert_eq!(LoopWait::of(needs(&tree)), LoopWait::Wait);
}

#[test]
fn the_earliest_of_a_scheduled_redraw_and_a_timer_wakes_the_loop() {
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.mount_root::<Clock>(&mut app, ());
  let soon = Instant::now() + Duration::from_millis(16);
  tree.request_redraw_at(soon);

  assert_eq!(LoopWait::of(needs(&tree)), LoopWait::WaitUntil(soon));
}

#[test]
fn video_polls_and_a_pending_redraw_waits_for_its_event() {
  let deadline = Some(Instant::now() + MINUTE);
  let video = LoopNeeds {
    continuous_video: true,
    redraw_pending: true,
    next_deadline: deadline,
    ..LoopNeeds::default()
  };
  assert_eq!(LoopWait::of(video), LoopWait::Poll);

  let pending = LoopNeeds {
    redraw_pending: true,
    polling: true,
    next_deadline: deadline,
    ..LoopNeeds::default()
  };
  assert_eq!(LoopWait::of(pending), LoopWait::Wait);
}

#[test]
fn polling_work_wins_over_a_deadline() {
  let needs = LoopNeeds {
    polling: true,
    next_deadline: Some(Instant::now() + MINUTE),
    ..LoopNeeds::default()
  };
  assert_eq!(LoopWait::of(needs), LoopWait::Poll);
}
