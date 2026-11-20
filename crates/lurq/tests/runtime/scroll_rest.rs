//! What a scroll container tells of content that moves and comes to rest: an `on_scroll` event after every
//! move, the last one at the offset the content stays at; `on_scroll_end` once, when the momentum, the way to a
//! page and the spring back a finger left are over, and not at the lift; and the page the content rests on
//! (`on_page_change`, `ScrollState::page_signal`), whatever took it there.
//!
//! The fingers are given through [`Tree::finger`] on the tree's clock of presses, as a shell gives them.

use std::{
  sync::{Arc, Mutex},
  time::{Duration, Instant},
};

use lurq::{
  app::{
    App, Tree,
    events::{ScrollEvent, ScrollPhase, ScrollSource, TouchPhase},
  },
  components::{ScrollHorizontal, ScrollVertical, Spacer},
  layout::layout_kind::ScrollState,
  node::Element,
};

use crate::support::{TestSurface, render_pass_with_app};

const FINGER: u64 = 1;

/// What a container told, in order: `start`, `scroll <source> <offset>` for an event with no delta, `end
/// <source> <offset>` and `page <n>`.
type Told = Arc<Mutex<Vec<String>>>;

struct Stage {
  app: App,
  tree: Tree,
  state: ScrollState,
  told: Told,
  now: Instant,
}

impl Stage {
  /// A list of `pages` pages, each as large as its 100 by 100 viewport, that scrolls across or down.
  fn new(across: bool, pages: f32, paging: bool) -> Self {
    let state = ScrollState::new();
    state.set_paging(paging);
    let told = Told::default();
    let offset = {
      let state = state.clone();
      move || if across { state.scroll_x() } else { state.scroll_y() }
    };
    let (start, scrolled, ended, paged) = (told.clone(), told.clone(), told.clone(), told.clone());
    let bottom = told.clone();
    let (at, rest) = (offset.clone(), offset);
    let root: Element = if across {
      ScrollHorizontal::new(Spacer::new().width(100.0 * pages).height(100.0))
        .with_scroll_state(state.clone())
        .size(100.0, 100.0)
        .on_scroll_start(move |_: ScrollEvent| start.lock().unwrap().push("start".into()))
        .on_scroll(move |event: ScrollEvent| {
          if (event.delta_x, event.delta_y) == (0.0, 0.0) {
            let line = format!("scroll {} {}", event.source.as_str(), at());
            scrolled.lock().unwrap().push(line);
          }
        })
        .on_scroll_end(move |event: ScrollEvent| {
          let line = format!("end {} {}", event.source.as_str(), rest());
          ended.lock().unwrap().push(line);
        })
        .on_page_change(move |page: usize| paged.lock().unwrap().push(format!("page {page}")))
        .into()
    } else {
      ScrollVertical::new(Spacer::new().width(100.0).height(100.0 * pages))
        .with_scroll_state(state.clone())
        .size(100.0, 100.0)
        .on_scroll_start(move |_: ScrollEvent| start.lock().unwrap().push("start".into()))
        .on_scroll(move |event: ScrollEvent| {
          if (event.delta_x, event.delta_y) == (0.0, 0.0) {
            let line = format!("scroll {} {}", event.source.as_str(), at());
            scrolled.lock().unwrap().push(line);
          }
        })
        .on_scroll_end(move |event: ScrollEvent| {
          let line = format!("end {} {}", event.source.as_str(), rest());
          ended.lock().unwrap().push(line);
        })
        .on_page_change(move |page: usize| paged.lock().unwrap().push(format!("page {page}")))
        .on_scroll_reach_bottom(move |event: ScrollEvent| {
          let line = format!("bottom {}", event.source.as_str());
          bottom.lock().unwrap().push(line);
        })
        .into()
    };
    let mut stage = Self {
      app: App::new(),
      tree: Tree::new(),
      state,
      told,
      now: Instant::now(),
    };
    stage.tree.resize(200, 200);
    stage.tree.set_root(root);
    stage.tree.set_press_clock(Some(stage.now));
    render_pass_with_app(&mut stage.tree, &mut stage.app);
    stage
  }

  fn told(&self) -> Vec<String> {
    self.told.lock().unwrap().clone()
  }

  fn count(&self, start: &str) -> usize {
    self.told().iter().filter(|line| line.starts_with(start)).count()
  }

  /// The clock goes on by `millis`, and the tree is given a finger's event at the time it comes to.
  fn finger(&mut self, millis: u64, phase: TouchPhase, x: f32, y: f32) {
    self.now += Duration::from_millis(millis);
    self.tree.set_press_clock(Some(self.now));
    self.tree.finger(FINGER, phase, x, y, None);
  }

  /// What time brings the finger, a step every 8 ms, until the tree has nothing more to step or `millis`
  /// passed. Tells how many steps ran.
  fn run(&mut self, millis: u64) -> usize {
    let until = self.now + Duration::from_millis(millis);
    let mut steps = 0;
    while self.now < until && self.tree.finger_deadline().is_some() {
      self.now += Duration::from_millis(8);
      self.tree.finger_tick(self.now);
      self.tree.pass(&mut self.app, &TestSurface);
      steps += 1;
    }
    steps
  }
}

/// A finger that lands at `from` and goes `travel` along the list's way in three moves 8 ms apart, the first of
/// which only tells the scroll from a tap, and lifts at once: a flick.
fn flick(stage: &mut Stage, across: bool, from: f32, travel: f32) {
  let at = |along: f32| if across { (along, 50.0) } else { (50.0, along) };
  let (x, y) = at(from);
  stage.finger(0, TouchPhase::Start, x, y);
  let first = from + 15.0 * travel.signum();
  for step in 0..3 {
    let (x, y) = at(first + travel * step as f32 / 2.0);
    stage.finger(8, TouchPhase::Move, x, y);
  }
  let (x, y) = at(first + travel);
  stage.finger(4, TouchPhase::End, x, y);
}

#[test]
fn a_short_flick_tells_the_page_it_goes_to_at_the_lift_and_every_step_of_the_way_there() {
  let mut stage = Stage::new(true, 5.0, true);
  let signal = stage.state.page_signal();

  flick(&mut stage, true, 90.0, -20.0);

  // The finger has lifted 20 into the first page, thrown at the second.
  assert_eq!(stage.state.scroll_x(), 20.0);
  assert_eq!(stage.state.page(), (0, 0), "the offset is still nearest the first page");
  assert_eq!(
    stage.state.target_page(),
    (1, 0),
    "where it is going is known at the lift"
  );
  assert_eq!(stage.state.rested_page(), (0, 0));
  assert!(stage.state.moving());
  assert_eq!(stage.count("start"), 1);
  assert_eq!(stage.count("end"), 0, "the lift is not the end: {:?}", stage.told());
  assert_eq!(stage.count("page"), 0);

  let steps = stage.run(3000);
  assert!(steps > 3, "the list went there over several frames: {steps}");
  assert_eq!(stage.state.scroll_x(), 100.0);
  assert!(!stage.state.moving());

  let told = stage.told();
  let snaps: Vec<&String> = told.iter().filter(|line| line.starts_with("scroll snap")).collect();
  assert!(snaps.len() > 3, "every step of the way to the page is told: {told:?}");
  assert_eq!(
    snaps.last().map(|line| line.as_str()),
    Some("scroll snap 100"),
    "the last one on the page"
  );
  // The page is told once, when the list is on it to the eye: within a fiftieth of a page of it, with the
  // tail of its way still to come. The end comes last, on the page itself.
  let paged = told.iter().position(|line| line == "page 1").expect("the page is told");
  let before: f32 = told[paged - 1]
    .strip_prefix("scroll snap ")
    .and_then(|offset| offset.parse().ok())
    .unwrap_or_else(|| panic!("the step before the page: {told:?}"));
  assert!((98.0..100.0).contains(&before), "the page was told at {before}");
  let after = told[paged + 1..]
    .iter()
    .filter(|line| line.starts_with("scroll snap"))
    .count();
  // Every step is 8 ms: the page is told a good while before the list is still.
  assert!(after >= 12, "the page is told {after} steps before the rest: {told:?}");
  assert_eq!(told.last().map(String::as_str), Some("end snap 100"), "{told:?}");
  assert_eq!(
    stage.count("start"),
    1,
    "one start for the finger and all it left: {told:?}"
  );
  assert_eq!(stage.count("end"), 1);
  assert_eq!(stage.count("page"), 1);
  assert_eq!(stage.state.rested_page(), (1, 0));
  assert_eq!(signal.get_untracked(), (1, 0));
}

#[test]
fn a_drag_tells_the_offset_after_every_move() {
  let mut stage = Stage::new(false, 10.0, false);

  stage.finger(0, TouchPhase::Start, 50.0, 90.0);
  stage.finger(8, TouchPhase::Move, 50.0, 75.0);
  stage.finger(8, TouchPhase::Move, 50.0, 60.0);
  assert_eq!(stage.state.scroll_y(), 15.0);
  stage.finger(8, TouchPhase::Move, 50.0, 40.0);
  assert_eq!(stage.state.scroll_y(), 35.0);

  let told = stage.told();
  assert_eq!(told, ["start", "scroll finger 15", "scroll finger 35"]);
  assert!(stage.state.moving(), "a finger has the content");
}

#[test]
fn momentum_ends_once_where_the_content_stops() {
  let mut stage = Stage::new(false, 100.0, false);

  flick(&mut stage, false, 90.0, -40.0);
  let lifted_at = stage.state.scroll_y();
  assert_eq!(stage.count("end"), 0, "the lift is not the end: {:?}", stage.told());
  assert!(stage.state.moving());

  stage.run(20_000);
  let rest = stage.state.scroll_y();
  assert!(
    rest > lifted_at + 20.0,
    "momentum carried the content on: {lifted_at} to {rest}"
  );
  assert!(!stage.state.moving());

  let told = stage.told();
  let ends: Vec<&String> = told.iter().filter(|line| line.starts_with("end")).collect();
  assert_eq!(ends, [&format!("end momentum {rest}")], "{told:?}");
  let last_scroll = told.iter().rfind(|line| line.starts_with("scroll")).unwrap();
  assert_eq!(
    last_scroll,
    &format!("scroll momentum {rest}"),
    "the last step is told where it stays"
  );
  assert_eq!(stage.count("start"), 1, "momentum is not a scroll of its own: {told:?}");
  // The end came after everything else but the page.
  let end_at = told.iter().position(|line| line.starts_with("end")).unwrap();
  assert!(
    told[end_at + 1..].iter().all(|line| line.starts_with("page")),
    "{told:?}"
  );
}

#[test]
fn a_finger_that_stops_before_it_lifts_ends_the_scroll_at_the_lift() {
  let mut stage = Stage::new(false, 10.0, false);

  stage.finger(0, TouchPhase::Start, 50.0, 90.0);
  stage.finger(8, TouchPhase::Move, 50.0, 75.0);
  stage.finger(8, TouchPhase::Move, 50.0, 20.0);
  assert_eq!(stage.count("end"), 0);
  // It rests for longer than the stretch its speed is measured over, and lifts: no momentum.
  stage.finger(300, TouchPhase::End, 50.0, 20.0);

  assert_eq!(stage.state.scroll_y(), 55.0);
  assert!(!stage.state.moving());
  let told = stage.told();
  assert_eq!(told, ["start", "scroll finger 55", "end finger 55", "page 1"]);
}

#[test]
fn a_finger_that_catches_a_list_on_its_way_to_a_page_holds_it_and_does_not_end_the_scroll() {
  let mut stage = Stage::new(true, 5.0, true);

  flick(&mut stage, true, 90.0, -20.0);
  stage.run(40);
  let between = stage.state.scroll_x();
  assert!(between > 20.0 && between < 100.0, "on its way: {between}");

  // A finger lands on it and rests: the list is held between the two pages.
  stage.finger(0, TouchPhase::Start, 50.0, 50.0);
  assert!(stage.state.moving(), "held, not at rest");
  assert_eq!(stage.count("end"), 0);
  assert_eq!(stage.count("page"), 0);

  // It lets go without a move: the list goes on to the nearest page, and ends there.
  stage.finger(50, TouchPhase::End, 50.0, 50.0);
  stage.run(3000);
  let told = stage.told();
  assert_eq!(stage.count("start"), 1, "{told:?}");
  assert_eq!(stage.count("end"), 1, "{told:?}");
  assert!(!stage.state.moving());
  let page = stage.state.rested_page().0;
  assert_eq!(stage.state.scroll_x(), page as f32 * 100.0);
}

#[test]
fn a_list_that_comes_to_its_last_page_reaches_its_bottom() {
  let mut stage = Stage::new(false, 3.0, true);

  flick(&mut stage, false, 90.0, -20.0);
  stage.run(3000);
  assert_eq!(stage.state.rested_page(), (0, 1));
  assert_eq!(stage.count("bottom"), 0, "the middle page is no end");

  flick(&mut stage, false, 90.0, -20.0);
  stage.run(3000);
  assert_eq!(stage.state.scroll_y(), 200.0);
  let told = stage.told();
  assert_eq!(stage.count("bottom"), 1, "{told:?}");
  assert!(told.contains(&"bottom snap".to_owned()), "{told:?}");
}

#[test]
fn an_offset_the_app_sets_changes_the_page_and_tells_no_scroll() {
  let mut stage = Stage::new(true, 5.0, true);
  let signal = stage.state.page_signal();

  stage.state.set_scroll(300.0, 0.0);
  stage.tree.pass(&mut stage.app, &TestSurface);

  assert_eq!(stage.told(), ["page 3"]);
  assert_eq!(stage.state.rested_page(), (3, 0));
  assert_eq!(signal.get_untracked(), (3, 0));

  // An offset on the same page changes nothing.
  stage.state.set_scroll(320.0, 0.0);
  stage.tree.pass(&mut stage.app, &TestSurface);
  assert_eq!(stage.told(), ["page 3"]);
}

#[test]
fn a_touchpad_changes_the_page_when_the_offset_is_nearer_the_next() {
  let mut stage = Stage::new(false, 5.0, false);

  stage.tree.scroll(50.0, 50.0, 0.0, -40.0, ScrollPhase::Scroll);
  stage.tree.pass(&mut stage.app, &TestSurface);
  // Its event is the one with its delta, before the move, and it tells no end.
  assert!(stage.told().is_empty(), "{:?}", stage.told());
  assert_eq!(stage.state.rested_page(), (0, 0));

  stage.tree.scroll(50.0, 50.0, 0.0, -40.0, ScrollPhase::Scroll);
  stage.tree.pass(&mut stage.app, &TestSurface);
  assert_eq!(stage.told(), ["page 1"]);
  assert_eq!(stage.state.rested_page(), (0, 1));
}

#[test]
fn a_mouses_wheel_ends_its_scroll_where_the_notch_sent_it() {
  let mut stage = Stage::new(false, 5.0, false);
  let start = Instant::now();
  stage.tree.set_animation_clock(Some(start));
  stage.tree.pass(&mut stage.app, &TestSurface);

  // Two notches down, 80: the content goes there over the frames that follow.
  stage.tree.wheel(50.0, 50.0, 0.0, -80.0, ScrollPhase::Scroll);
  assert!(stage.state.moving());
  assert_eq!(stage.state.target_page(), (0, 1), "where the wheel is taking it");
  assert_eq!(stage.state.page(), (0, 0));
  let mut at = 0;
  while stage.state.moving() {
    at += 8;
    assert!(at < 1000, "the wheel's scroll never ended");
    stage.tree.set_animation_clock(Some(start + Duration::from_millis(at)));
    stage.tree.pass(&mut stage.app, &TestSurface);
  }

  let told = stage.told();
  assert_eq!(stage.state.scroll_y(), 80.0);
  assert_eq!(stage.count("end"), 1, "{told:?}");
  let tail: Vec<&str> = told.iter().rev().take(3).rev().map(String::as_str).collect();
  assert_eq!(tail, ["scroll wheel 80", "end wheel 80", "page 1"], "{told:?}");
}

#[test]
fn a_scroll_event_tells_what_moves_the_content() {
  assert_eq!(ScrollSource::default(), ScrollSource::Wheel);
  let names: Vec<&str> = [
    ScrollSource::Finger,
    ScrollSource::Wheel,
    ScrollSource::Momentum,
    ScrollSource::Snap,
    ScrollSource::Scrollbar,
    ScrollSource::Keyboard,
    ScrollSource::Autoscroll,
    ScrollSource::Code,
  ]
  .iter()
  .map(|source| source.as_str())
  .collect();
  assert_eq!(
    names,
    [
      "finger",
      "wheel",
      "momentum",
      "snap",
      "scrollbar",
      "keyboard",
      "autoscroll",
      "code"
    ]
  );
}

#[test]
fn a_finger_that_takes_the_list_back_after_its_page_was_told_tells_the_page_it_rests_on() {
  let mut stage = Stage::new(true, 5.0, true);
  flick(&mut stage, true, 90.0, -20.0);
  // Until the page is told, and no further: the list is still on the last of its way.
  let mut steps = 0;
  while stage.count("page") == 0 {
    steps += 1;
    assert!(steps < 400, "the page is never told");
    stage.now += Duration::from_millis(8);
    stage.tree.finger_tick(stage.now);
    stage.tree.pass(&mut stage.app, &TestSurface);
  }
  assert_eq!(stage.state.rested_page(), (1, 0));
  assert!(stage.state.moving(), "its scroll has not ended");
  assert_eq!(stage.count("end"), 0);

  // A finger catches it there and drags it back to the first page.
  stage.finger(8, TouchPhase::Start, 20.0, 50.0);
  for step in 1..=8 {
    stage.finger(40, TouchPhase::Move, 20.0 + 10.0 * step as f32, 50.0);
  }
  stage.finger(200, TouchPhase::End, 100.0, 50.0);
  stage.run(3000);
  assert_eq!(stage.state.scroll_x(), 0.0);
  assert_eq!(stage.state.rested_page(), (0, 0));
  let pages: Vec<String> = stage
    .told()
    .into_iter()
    .filter(|line| line.starts_with("page"))
    .collect();
  assert_eq!(pages, ["page 1", "page 0"]);
  assert_eq!(
    stage.count("end"),
    1,
    "one scroll, which ended once: {:?}",
    stage.told()
  );
}
