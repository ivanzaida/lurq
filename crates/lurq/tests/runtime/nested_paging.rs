//! A list that scrolls by pages inside another: a strip of an album's pictures, across, in a feed of albums, down.
//! A finger's gesture is one list's, the one along the way the finger first went, and whatever ends the gesture
//! every list it moved comes to rest on a page.
//!
//! The fingers are given through [`Tree::finger`] on the tree's clock of presses, as a shell gives them.

use std::time::{Duration, Instant};

use lurq::{
  app::{App, Tree, events::TouchPhase},
  components::{Column, Row, ScrollHorizontal, ScrollVertical, Spacer},
  layout::{layout_kind::ScrollState, scrollbar::ScrollBarStyle},
};

use crate::support::{TestSurface, render_pass_with_app};

const FINGER: u64 = 1;
/// A page of the feed, and of a strip in it.
const WIDTH: f32 = 100.0;
const HEIGHT: f32 = 200.0;

struct Stage {
  app: App,
  tree: Tree,
  /// The feed, which scrolls down by pages, and the strips of its albums, which scroll across by pages.
  feed: ScrollState,
  strips: Vec<ScrollState>,
  now: Instant,
}

impl Stage {
  /// Three albums of three pictures each.
  fn new() -> Self {
    let feed = ScrollState::new();
    feed.set_paging(true);
    let strips: Vec<ScrollState> = (0..3)
      .map(|_| {
        let strip = ScrollState::new();
        strip.set_paging(true);
        strip
      })
      .collect();
    let albums = Column::new().with_children(strips.iter().map(|strip| {
      ScrollHorizontal::new(Row::new().with_children((0..3).map(|_| Spacer::new().width(WIDTH).height(HEIGHT))))
        .with_scroll_state(strip.clone())
        .scrollbar(ScrollBarStyle::hidden())
        .size(WIDTH, HEIGHT)
    }));
    let mut stage = Self {
      app: App::new(),
      tree: Tree::new(),
      feed: feed.clone(),
      strips,
      now: Instant::now(),
    };
    stage.tree.resize(200, 400);
    stage.tree.set_root(
      ScrollVertical::new(albums)
        .with_scroll_state(feed)
        .scrollbar(ScrollBarStyle::hidden())
        .size(WIDTH, HEIGHT),
    );
    stage.tree.set_press_clock(Some(stage.now));
    render_pass_with_app(&mut stage.tree, &mut stage.app);
    stage
  }

  fn finger(&mut self, millis: u64, phase: TouchPhase, x: f32, y: f32) {
    self.now += Duration::from_millis(millis);
    self.tree.set_press_clock(Some(self.now));
    self.tree.finger(FINGER, phase, x, y, None);
    self.tree.pass(&mut self.app, &TestSurface);
  }

  /// What time brings, a step every 8 ms, for `millis`.
  fn run(&mut self, millis: u64) {
    let until = self.now + Duration::from_millis(millis);
    while self.now < until {
      self.now += Duration::from_millis(8);
      self.tree.set_press_clock(Some(self.now));
      self.tree.finger_tick(self.now);
      self.tree.pass(&mut self.app, &TestSurface);
    }
  }

  /// A finger that lands at `from` and goes by `travel` in steps 16 ms apart, at 600 a second.
  fn drag(&mut self, from: (f32, f32), travel: (f32, f32)) -> (f32, f32) {
    self.finger(0, TouchPhase::Start, from.0, from.1);
    let steps = (travel.0.hypot(travel.1) / 600.0 * 1000.0 / 16.0).ceil().max(1.0) as u32;
    let mut at = from;
    for step in 1..=steps {
      let share = step as f32 / steps as f32;
      at = (from.0 + travel.0 * share, from.1 + travel.1 * share);
      self.finger(16, TouchPhase::Move, at.0, at.1);
    }
    at
  }

  /// Every list is on a page, and nothing moves it.
  fn assert_at_rest_on_pages(&self, why: &str) {
    assert_eq!(
      self.feed.scroll_y() % HEIGHT,
      0.0,
      "the feed, {why}: {}",
      self.feed.scroll_y()
    );
    assert!(!self.feed.moving(), "the feed still moves, {why}");
    for (index, strip) in self.strips.iter().enumerate() {
      assert_eq!(
        strip.scroll_x() % WIDTH,
        0.0,
        "strip {index}, {why}: {}",
        strip.scroll_x()
      );
      assert!(!strip.moving(), "strip {index} still moves, {why}");
    }
  }
}

#[test]
fn a_slanted_drag_scrolls_the_list_along_the_way_it_first_went_and_no_other() {
  let mut stage = Stage::new();

  // Left and further down, from the first album: the way a swipe to the left that turns downward goes.
  let at = stage.drag((72.0, 60.0), (-42.0, 55.0));
  assert_eq!(
    stage.strips[0].scroll_x(),
    0.0,
    "the gesture is the feed's: the strip stays"
  );
  // It rests, so no momentum, and lifts.
  stage.finger(300, TouchPhase::End, at.0, at.1);
  stage.run(2000);

  stage.assert_at_rest_on_pages("after a drag left and down");
  assert_eq!(stage.strips[0].scroll_x(), 0.0);
}

#[test]
fn a_slanted_drag_up_goes_to_the_next_album_and_leaves_its_strip_on_its_page() {
  let mut stage = Stage::new();

  // Up and less to the left: the feed's, far enough for its next page.
  let at = stage.drag((80.0, 180.0), (-70.0, -130.0));
  stage.finger(300, TouchPhase::End, at.0, at.1);
  stage.run(2000);

  stage.assert_at_rest_on_pages("after a drag up and left");
  assert_eq!(stage.feed.scroll_y(), HEIGHT, "the next album");
  assert_eq!(stage.strips[0].scroll_x(), 0.0);
}

#[test]
fn a_drag_across_is_the_strips_whatever_it_does_after() {
  let mut stage = Stage::new();

  // Left first, then it turns down: the strip's all the way.
  stage.finger(0, TouchPhase::Start, 90.0, 60.0);
  stage.finger(16, TouchPhase::Move, 70.0, 62.0);
  stage.finger(16, TouchPhase::Move, 30.0, 70.0);
  assert_eq!(stage.strips[0].scroll_x(), 40.0);
  // Far more down than across now: the strip still follows the finger across, and the feed stays.
  stage.finger(16, TouchPhase::Move, 5.0, 120.0);
  assert_eq!(stage.feed.scroll_y(), 0.0, "the feed stays");
  assert_eq!(stage.strips[0].scroll_x(), 65.0, "the strip follows the finger across");
  stage.finger(300, TouchPhase::End, 5.0, 120.0);
  stage.run(2000);

  stage.assert_at_rest_on_pages("after a drag left that turned down");
  assert_eq!(
    stage.strips[0].scroll_x(),
    WIDTH,
    "more than half a page across: the next picture"
  );
  assert_eq!(stage.feed.scroll_y(), 0.0);
}

#[test]
fn a_touch_taken_away_in_the_middle_of_a_drag_leaves_the_list_on_a_page() {
  let mut stage = Stage::new();

  stage.finger(0, TouchPhase::Start, 90.0, 60.0);
  stage.finger(16, TouchPhase::Move, 70.0, 60.0);
  stage.finger(16, TouchPhase::Move, 40.0, 60.0);
  let between = stage.strips[0].scroll_x();
  assert!(between > 0.0 && between < WIDTH, "between two pages: {between}");
  // The system takes the touch: the mouse of an emulated phone left the window.
  stage.finger(16, TouchPhase::Cancel, 40.0, 60.0);
  stage.run(2000);

  stage.assert_at_rest_on_pages("after a touch that was taken away");
}

#[test]
fn a_press_that_stops_a_list_on_its_way_and_lifts_lets_it_go_on_to_a_page() {
  let mut stage = Stage::new();

  // A flick to the left: the strip goes on to its next page.
  stage.finger(0, TouchPhase::Start, 90.0, 60.0);
  stage.finger(8, TouchPhase::Move, 75.0, 60.0);
  stage.finger(8, TouchPhase::Move, 60.0, 60.0);
  stage.finger(4, TouchPhase::End, 50.0, 60.0);
  stage.run(40);
  let between = stage.strips[0].scroll_x();
  assert!(between > 0.0 && between < WIDTH, "on its way: {between}");

  // A press stops it there, and lifts without a move.
  stage.finger(0, TouchPhase::Start, 50.0, 100.0);
  stage.finger(60, TouchPhase::End, 50.0, 100.0);
  stage.run(2000);

  stage.assert_at_rest_on_pages("after a press that stopped it");
}

#[test]
fn a_list_stopped_on_its_way_goes_on_to_its_page_while_the_finger_scrolls_another() {
  let mut stage = Stage::new();

  // A flick to the left: the strip goes on to its next page.
  stage.finger(0, TouchPhase::Start, 90.0, 60.0);
  stage.finger(8, TouchPhase::Move, 75.0, 60.0);
  stage.finger(8, TouchPhase::Move, 60.0, 60.0);
  stage.finger(4, TouchPhase::End, 50.0, 60.0);
  stage.run(40);
  let between = stage.strips[0].scroll_x();
  assert!(between > 0.0 && between < WIDTH, "on its way: {between}");

  // The finger is back at once, and goes up: the feed's gesture. The strip is not left where it was caught.
  stage.finger(0, TouchPhase::Start, 50.0, 180.0);
  stage.finger(16, TouchPhase::Move, 50.0, 160.0);
  assert!(stage.strips[0].scroll_x() < WIDTH, "it does not jump to its page");
  assert!(stage.strips[0].moving(), "it is still on its way");
  stage.finger(16, TouchPhase::Move, 50.0, 120.0);
  stage.run(40);
  let on = stage.strips[0].scroll_x();
  assert!(
    on > between,
    "it goes on while the finger has the feed: {between} then {on}"
  );
  stage.finger(16, TouchPhase::Move, 50.0, 40.0);
  stage.finger(300, TouchPhase::End, 50.0, 40.0);
  stage.run(2000);

  stage.assert_at_rest_on_pages("after the finger went on to the feed");
  assert_eq!(stage.strips[0].scroll_x(), WIDTH, "the page it was going to");
  assert_eq!(stage.feed.scroll_y(), HEIGHT);
}
