//! A container that scrolls both ways under a finger: it goes wherever the finger goes, on a slant too, alone
//! and with a list that scrolls one way inside it or around it. The lock to one way (`nested_paging`) is a list's
//! that scrolls one way, and is not this container's.
//!
//! The fingers are given through [`Tree::finger`] on the tree's clock of presses, as a shell gives them.

use std::time::{Duration, Instant};

use lurq::{
  app::{App, Tree, events::TouchPhase},
  components::{Column, ScrollBoth, ScrollHorizontal, ScrollVertical, Spacer},
  layout::{layout_kind::ScrollState, scrollbar::ScrollBarStyle},
  node::Element,
};

use crate::support::{TestSurface, render_pass_with_app};

const FINGER: u64 = 1;

struct Stage {
  app: App,
  tree: Tree,
  now: Instant,
}

impl Stage {
  fn new(root: impl Into<Element>) -> Self {
    let mut stage = Self {
      app: App::new(),
      tree: Tree::new(),
      now: Instant::now(),
    };
    stage.tree.resize(400, 400);
    stage.tree.set_root(root);
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

  /// A finger that lands at `from`, goes by `travel` in eight steps 16 ms apart, stands, and lifts.
  fn drag(&mut self, from: (f32, f32), travel: (f32, f32)) {
    self.finger(0, TouchPhase::Start, from.0, from.1);
    for step in 1..=8 {
      let share = step as f32 / 8.0;
      self.finger(
        16,
        TouchPhase::Move,
        from.0 + travel.0 * share,
        from.1 + travel.1 * share,
      );
    }
    self.finger(300, TouchPhase::End, from.0 + travel.0, from.1 + travel.1);
    // What the lift leaves, if anything.
    for _ in 0..200 {
      self.now += Duration::from_millis(8);
      self.tree.set_press_clock(Some(self.now));
      self.tree.finger_tick(self.now);
      self.tree.pass(&mut self.app, &TestSurface);
    }
  }
}

/// A board of 600 by 600 in a window of 200 by 200 onto it, which scrolls both ways.
fn board(state: &ScrollState, content: impl Into<Element>) -> ScrollBoth {
  ScrollBoth::new(content)
    .with_scroll_state(state.clone())
    .scrollbar(ScrollBarStyle::hidden())
    .size(200.0, 200.0)
}

fn at(state: &ScrollState) -> (f32, f32) {
  (state.scroll_x(), state.scroll_y())
}

#[test]
fn a_container_that_scrolls_both_ways_follows_a_finger_on_a_slant() {
  let state = ScrollState::new();
  let mut stage = Stage::new(board(&state, Spacer::new().width(600.0).height(600.0)));
  stage.drag((150.0, 150.0), (-80.0, -60.0));
  // All of the finger's travel but the first step, which tells a scroll from a tap: along both ways at once.
  assert_eq!(at(&state), (70.0, 52.5));
  // And back up on another slant.
  stage.drag((100.0, 100.0), (40.0, 80.0));
  assert_eq!(at(&state), (35.0, 0.0), "it is held at its start, and goes on across");
}

#[test]
fn it_follows_the_finger_both_ways_inside_a_list_that_scrolls_one_way() {
  let (list, inner) = (ScrollState::new(), ScrollState::new());
  let content = Column::new()
    .child(board(&inner, Spacer::new().width(600.0).height(600.0)))
    .child(Spacer::new().width(200.0).height(600.0));
  let mut stage = Stage::new(
    ScrollVertical::new(content)
      .with_scroll_state(list.clone())
      .scrollbar(ScrollBarStyle::hidden())
      .size(200.0, 300.0),
  );
  stage.drag((150.0, 150.0), (-80.0, -60.0));
  assert_eq!(at(&inner), (70.0, 52.5), "the board goes both ways");
  assert_eq!(at(&list), (0.0, 0.0), "and the list around it stays");
  // Mostly down, the way the list scrolls: still the board's, both ways, while it has room to go.
  stage.drag((150.0, 150.0), (-20.0, -90.0));
  assert_eq!(at(&inner), (87.5, 131.25));
  assert_eq!(at(&list), (0.0, 0.0));
}

#[test]
fn a_strip_inside_it_takes_the_finger_that_goes_across_and_the_board_the_one_that_goes_down() {
  let (outer, strip) = (ScrollState::new(), ScrollState::new());
  let content = Column::new()
    .child(
      ScrollHorizontal::new(Spacer::new().width(900.0).height(100.0))
        .with_scroll_state(strip.clone())
        .scrollbar(ScrollBarStyle::hidden())
        .size(300.0, 100.0),
    )
    .child(Spacer::new().width(600.0).height(500.0));
  let mut stage = Stage::new(board(&outer, content));
  // On the strip, across first: the gesture is the strip's, which scrolls one way and takes that way alone.
  // One gesture scrolls one list: the board does not go down with what the finger went down meanwhile.
  stage.drag((150.0, 50.0), (-80.0, -30.0));
  assert_eq!(at(&strip), (70.0, 0.0));
  assert_eq!(at(&outer), (0.0, 0.0));
  // On the strip, down first: the gesture is the board's, which goes both ways with it, and the strip stays.
  stage.drag((150.0, 50.0), (-30.0, -80.0));
  assert_eq!(at(&strip), (70.0, 0.0));
  assert_eq!(at(&outer), (26.25, 70.0));
  // Beside the strip the board is all there is.
  stage.drag((150.0, 150.0), (-80.0, -60.0));
  assert_eq!(at(&strip), (70.0, 0.0));
  assert_eq!(at(&outer), (96.25, 122.5));
}
