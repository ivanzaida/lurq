//! How much of an element shows (`ElementRef::observe_visibility`): reckoned from the layout at every pass, so
//! told the same whatever moved the content, and only when the share crosses a threshold.

use std::{
  sync::{Arc, Mutex},
  time::{Duration, Instant},
};

use lurq::{
  app::{App, Tree, events::TouchPhase},
  components::{Column, Rect, Row, ScrollHorizontal, ScrollVertical, Spacer},
  core::{ElementRef, Visibility, VisibilityObserver, VisibilityOptions},
  layout::layout_kind::ScrollState,
  node::Element,
};

use crate::support::{TestSurface, render_pass_with_app};

/// What an observer was told, in order.
type Seen = Arc<Mutex<Vec<Visibility>>>;

fn watch(element: &ElementRef, options: VisibilityOptions) -> (Seen, VisibilityObserver) {
  let seen = Seen::default();
  let told = seen.clone();
  let observer = element.observe_visibility(options, move |visibility| told.lock().unwrap().push(visibility));
  (seen, observer)
}

fn ratios(seen: &Seen) -> Vec<f32> {
  seen.lock().unwrap().iter().map(|visibility| visibility.ratio).collect()
}

struct Stage {
  app: App,
  tree: Tree,
}

impl Stage {
  fn new(root: impl Into<Element>) -> Self {
    let mut stage = Self {
      app: App::new(),
      tree: Tree::new(),
    };
    stage.tree.resize(200, 200);
    stage.tree.set_root(root);
    render_pass_with_app(&mut stage.tree, &mut stage.app);
    stage
  }

  fn pass(&mut self) {
    self.tree.pass(&mut self.app, &TestSurface);
  }
}

/// A list 100 by 100 of `count` rows, each as large as the list's viewport, with a ref on each.
fn list(count: usize) -> (ScrollVertical, ScrollState, Vec<ElementRef>) {
  let state = ScrollState::new();
  let rows: Vec<ElementRef> = (0..count).map(|_| ElementRef::new()).collect();
  let column = Column::new().with_children(rows.iter().map(|row| Rect::new(100.0, 100.0).ref_element(row.clone())));
  let list = ScrollVertical::new(column)
    .with_scroll_state(state.clone())
    .size(100.0, 100.0);
  (list, state, rows)
}

#[test]
fn an_observer_is_told_at_the_first_layout_and_when_the_share_crosses_its_threshold() {
  let (list, state, rows) = list(4);
  let watched: Vec<(Seen, VisibilityObserver)> = rows
    .iter()
    .map(|row| watch(row, VisibilityOptions::new().thresholds([0.99])))
    .collect();
  let mut stage = Stage::new(list);

  // Each is told once of what shows of it: the first row all, the others nothing.
  assert_eq!(ratios(&watched[0].0), [1.0]);
  assert_eq!(ratios(&watched[1].0), [0.0]);
  assert!(!watched[1].0.lock().unwrap()[0].visible);

  // Half way to the second row no share crossed 0.99 but the first row's, which no longer all shows.
  state.set_scroll(0.0, 50.0);
  stage.pass();
  assert_eq!(ratios(&watched[0].0), [1.0, 0.5]);
  assert_eq!(ratios(&watched[1].0), [0.0], "half of it shows, under its threshold");

  // On the second row: it is the one that shows.
  state.set_scroll(0.0, 100.0);
  stage.pass();
  assert_eq!(ratios(&watched[0].0), [1.0, 0.5], "it was under its threshold already");
  assert_eq!(ratios(&watched[1].0), [0.0, 1.0]);
  let second = watched[1].0.lock().unwrap()[1];
  assert_eq!((second.visible_rect.y, second.visible_rect.height), (0.0, 100.0));
  assert_eq!((second.root_rect.width, second.root_rect.height), (100.0, 100.0));

  // A pass with nothing moved tells nothing.
  stage.pass();
  assert_eq!(ratios(&watched[1].0).len(), 2);
}

#[test]
fn every_threshold_crossed_is_told_either_way_and_nothing_between_two() {
  let (list, state, rows) = list(3);
  let (seen, _observer) = watch(&rows[1], VisibilityOptions::new().thresholds([0.0, 0.5, 1.0]));
  let mut stage = Stage::new(list);
  assert_eq!(ratios(&seen), [0.0]);

  for (offset, told) in [
    (10.0, vec![0.0, 0.1]),
    (40.0, vec![0.0, 0.1]),
    (60.0, vec![0.0, 0.1, 0.6]),
    (100.0, vec![0.0, 0.1, 0.6, 1.0]),
    (130.0, vec![0.0, 0.1, 0.6, 1.0, 0.7]),
    (190.0, vec![0.0, 0.1, 0.6, 1.0, 0.7, 0.1]),
    (200.0, vec![0.0, 0.1, 0.6, 1.0, 0.7, 0.1, 0.0]),
  ] {
    state.set_scroll(0.0, offset);
    stage.pass();
    let seen = ratios(&seen);
    assert_eq!(seen.len(), told.len(), "at {offset}: {seen:?}");
    for (seen, told) in seen.iter().zip(&told) {
      assert!((seen - told).abs() < 0.001, "at {offset}: {seen} for {told}");
    }
  }
}

#[test]
fn a_margin_tells_what_is_near_the_viewport() {
  let (list, state, rows) = list(4);
  let (near, _near) = watch(&rows[2], VisibilityOptions::new().margin(100.0));
  let (shown, _shown) = watch(&rows[2], VisibilityOptions::new());
  let mut stage = Stage::new(list);
  // The third row starts 200 down: 100 under the viewport, at the edge of the margin.
  assert_eq!(near.lock().unwrap().len(), 1);
  assert!(!near.lock().unwrap()[0].visible);

  state.set_scroll(0.0, 20.0);
  stage.pass();
  let told = near.lock().unwrap().clone();
  assert_eq!(told.len(), 2, "within the margin");
  assert!(told[1].visible);
  assert!((told[1].ratio - 0.2).abs() < 0.001);
  assert_eq!(
    told[1].root_rect.height, 300.0,
    "the viewport and the margin over and under it"
  );
  assert_eq!(
    shown.lock().unwrap().len(),
    1,
    "nothing of it shows in the viewport itself"
  );
}

#[test]
fn a_mark_of_no_height_at_the_end_of_a_list_shows_when_the_end_does() {
  let state = ScrollState::new();
  let end = ElementRef::new();
  let (seen, _observer) = watch(&end, VisibilityOptions::new());
  let content = Column::new()
    .child(Rect::new(100.0, 300.0))
    .child(Spacer::new().width(100.0).height(0.0).ref_element(end.clone()));
  let mut stage = Stage::new(
    ScrollVertical::new(content)
      .with_scroll_state(state.clone())
      .size(100.0, 100.0),
  );
  assert!(!seen.lock().unwrap()[0].visible);

  state.set_scroll(0.0, 150.0);
  stage.pass();
  assert_eq!(seen.lock().unwrap().len(), 1, "the end is still under the viewport");

  state.set_scroll(0.0, 200.0);
  stage.pass();
  let told = seen.lock().unwrap().clone();
  assert_eq!(told.len(), 2);
  assert!(told[1].visible);
  assert_eq!(told[1].ratio, 1.0);
}

#[test]
fn a_finger_and_what_it_leaves_tell_as_the_apps_own_offset_does() {
  let (list, state, rows) = list(3);
  state.set_paging(true);
  let (seen, _observer) = watch(&rows[1], VisibilityOptions::new().thresholds([0.99]));
  let mut stage = Stage::new(list);
  let mut now = Instant::now();
  stage.tree.set_press_clock(Some(now));

  // A short flick up: the list goes on to its second page by itself.
  let mut finger = |stage: &mut Stage, millis: u64, phase: TouchPhase, y: f32| {
    now += Duration::from_millis(millis);
    stage.tree.set_press_clock(Some(now));
    stage.tree.finger(1, phase, 50.0, y, None);
    now
  };
  finger(&mut stage, 0, TouchPhase::Start, 90.0);
  finger(&mut stage, 8, TouchPhase::Move, 75.0);
  finger(&mut stage, 8, TouchPhase::Move, 65.0);
  let mut at = finger(&mut stage, 4, TouchPhase::End, 55.0);
  stage.pass();
  assert_eq!(ratios(&seen), [0.0], "a fifth of it shows as the finger lifts");

  while stage.tree.finger_deadline().is_some() {
    at += Duration::from_millis(8);
    stage.tree.finger_tick(at);
    stage.pass();
  }
  assert_eq!(state.scroll_y(), 100.0);
  let told = ratios(&seen);
  assert_eq!(
    told.len(),
    2,
    "told once on the way, when the page all but filled the viewport: {told:?}"
  );
  assert!(told[1] >= 0.99, "{told:?}");
}

#[test]
fn an_element_inside_two_lists_shows_where_both_show_it() {
  let (outer, inner) = (ScrollState::new(), ScrollState::new());
  let card = ElementRef::new();
  let (seen, _observer) = watch(&card, VisibilityOptions::new().thresholds([0.0, 1.0]));
  // A strip that scrolls across, 100 down a page that scrolls down: its second card is to the right of it.
  let strip = ScrollHorizontal::new(
    Row::new()
      .child(Rect::new(100.0, 100.0))
      .child(Rect::new(100.0, 100.0).ref_element(card.clone())),
  )
  .with_scroll_state(inner.clone())
  .size(100.0, 100.0);
  let page = Column::new().child(Rect::new(100.0, 100.0)).child(strip);
  let mut stage = Stage::new(
    ScrollVertical::new(page)
      .with_scroll_state(outer.clone())
      .size(100.0, 100.0),
  );
  assert!(!seen.lock().unwrap()[0].visible);

  // The strip brings the card under its own viewport, which the page does not show yet.
  inner.set_scroll(100.0, 0.0);
  stage.pass();
  assert_eq!(
    seen.lock().unwrap().len(),
    1,
    "the strip is still under the page's viewport"
  );

  outer.set_scroll(0.0, 50.0);
  stage.pass();
  let told = seen.lock().unwrap().clone();
  assert_eq!(told.len(), 2);
  assert!(
    (told[1].ratio - 0.5).abs() < 0.001,
    "the half of the strip the page shows"
  );

  outer.set_scroll(0.0, 100.0);
  stage.pass();
  assert_eq!(ratios(&seen).last().copied(), Some(1.0));
}

#[test]
fn an_element_nothing_scrolls_is_measured_against_the_window_or_a_root_of_the_apps() {
  let (frame, card) = (ElementRef::new(), ElementRef::new());
  let (in_window, _window) = watch(&card, VisibilityOptions::new());
  let (in_frame, _frame) = watch(&card, VisibilityOptions::new().root(&frame));
  // A window 200 wide: a frame 100 wide, and a card 100 wide that starts 150 across.
  let _stage = Stage::new(
    Row::new()
      .child(Rect::new(100.0, 100.0).ref_element(frame.clone()))
      .child(Spacer::new().width(50.0).height(100.0))
      .child(Rect::new(100.0, 100.0).ref_element(card.clone())),
  );

  let window = in_window.lock().unwrap()[0];
  assert!(
    (window.ratio - 0.5).abs() < 0.001,
    "half of it is in the window: {}",
    window.ratio
  );
  assert_eq!((window.root_rect.width, window.root_rect.height), (200.0, 200.0));
  let frame = in_frame.lock().unwrap()[0];
  assert!(!frame.visible, "none of it is in the frame");
  assert_eq!(frame.root_rect.width, 100.0);
}

#[test]
fn an_observer_that_is_dropped_is_told_no_more() {
  let (list, state, rows) = list(3);
  let (kept, _kept) = watch(&rows[1], VisibilityOptions::new());
  let (dropped, observer) = watch(&rows[1], VisibilityOptions::new());
  let mut stage = Stage::new(list);
  assert_eq!((kept.lock().unwrap().len(), dropped.lock().unwrap().len()), (1, 1));

  drop(observer);
  state.set_scroll(0.0, 50.0);
  stage.pass();
  assert_eq!((kept.lock().unwrap().len(), dropped.lock().unwrap().len()), (2, 1));
}

#[test]
fn an_observer_added_after_the_layout_is_told_at_the_next_pass() {
  let (list, _state, rows) = list(3);
  let mut stage = Stage::new(list);
  stage.pass();
  assert!(!stage.tree.needs_redraw());

  let (seen, _observer) = watch(&rows[0], VisibilityOptions::new());
  assert!(seen.lock().unwrap().is_empty());
  stage.pass();
  assert_eq!(ratios(&seen), [1.0]);
}
