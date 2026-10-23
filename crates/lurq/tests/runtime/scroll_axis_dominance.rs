//! A scroll container that scrolls sideways only takes no part of a mostly
//! vertical gesture: a slightly slanted trackpad swipe over it scrolls the
//! page around it and leaves it where it is, while a mostly sideways swipe
//! and a wheel with Shift (a delta on x alone) scroll it. Windows delivers
//! the two axes of a swipe as separate events, so a sideways delta is weighed
//! against the vertical one seen just before it at the same pointer.

use std::time::{Duration, Instant};

use lurq::{
  app::{Tree, events::ScrollPhase},
  components::{Column, Rect, ScrollHorizontal, ScrollVertical},
  layout::layout_kind::ScrollState,
};

use crate::support::run_pass;

const POINTER: (f32, f32) = (200.0, 25.0);

/// A 400x200 page scrolling down, with a 400x50 strip scrolling sideways at
/// its top and more below; the pointer rests on the strip.
fn page() -> (Tree, ScrollState, ScrollState) {
  let (page, strip) = (ScrollState::new(), ScrollState::new());
  let mut tree = Tree::new();
  tree.set_root(
    ScrollVertical::new(
      Column::new()
        .spacing(0.0)
        .child(
          ScrollHorizontal::new(Rect::new(2000.0, 50.0).background("#1f2937"))
            .with_scroll_state(strip.clone())
            .size(400.0, 50.0),
        )
        .child(Rect::new(400.0, 1000.0).background("#111827")),
    )
    .with_scroll_state(page.clone())
    .size(400.0, 200.0),
  );
  run_pass(&mut tree);
  (tree, page, strip)
}

/// A trackpad swipe of `times` moves, each `(dx, dy)`.
fn swipe(tree: &mut Tree, dx: f32, dy: f32, times: usize) {
  tree.scroll(POINTER.0, POINTER.1, 0.0, 0.0, ScrollPhase::Start);
  for _ in 0..times {
    tree.scroll(POINTER.0, POINTER.1, dx, dy, ScrollPhase::Scroll);
  }
  tree.scroll(POINTER.0, POINTER.1, 0.0, 0.0, ScrollPhase::End);
}

#[test]
fn a_mostly_vertical_swipe_scrolls_the_page_and_leaves_the_sideways_strip() {
  let (mut tree, page, strip) = page();
  swipe(&mut tree, -3.0, -6.0, 5);
  assert_eq!(page.scroll_y(), 30.0, "the page takes the whole vertical delta");
  assert_eq!(strip.scroll_x(), 0.0, "the strip is not nudged by the slant");
}

#[test]
fn a_mostly_sideways_swipe_scrolls_the_strip_and_its_slant_the_page() {
  let (mut tree, page, strip) = page();
  swipe(&mut tree, -6.0, -3.0, 5);
  assert_eq!(strip.scroll_x(), 30.0, "the strip takes the sideways delta");
  assert_eq!(page.scroll_y(), 15.0, "the page takes the vertical rest");
}

#[test]
fn a_delta_on_x_alone_scrolls_the_strip() {
  let (mut tree, page, strip) = page();
  tree.scroll(POINTER.0, POINTER.1, -40.0, 0.0, ScrollPhase::Scroll);
  assert_eq!(strip.scroll_x(), 40.0, "Shift+wheel, as the shell delivers it");
  assert_eq!(page.scroll_y(), 0.0);
  swipe(&mut tree, -5.0, -5.0, 2);
  assert_eq!(strip.scroll_x(), 50.0, "an even slant still counts as sideways");
  assert_eq!(page.scroll_y(), 10.0);
}

/// A slanted swipe as Windows delivers it: each move as a vertical event
/// and a horizontal one, `step` apart on the runtime's clock.
fn slanted_swipe_in_separate_events(tree: &mut Tree, start: Instant, step: Duration, moves: u32) {
  for index in 0..moves {
    tree.set_animation_clock(Some(start + step * index));
    tree.scroll(POINTER.0, POINTER.1, 0.0, -6.0, ScrollPhase::Scroll);
    tree.scroll(POINTER.0, POINTER.1, -3.0, 0.0, ScrollPhase::Scroll);
  }
}

#[test]
fn a_slanted_swipe_delivered_as_separate_events_scrolls_the_page_and_leaves_the_strip() {
  let (mut tree, page, strip) = page();
  slanted_swipe_in_separate_events(&mut tree, Instant::now(), Duration::from_millis(10), 5);
  assert_eq!(page.scroll_y(), 30.0, "the vertical events scroll the page");
  assert_eq!(
    strip.scroll_x(),
    0.0,
    "the horizontal events in between are the slant, not a scroll of the strip"
  );
}

#[test]
fn sideways_events_alone_scroll_the_strip() {
  let (mut tree, page, strip) = page();
  tree.set_animation_clock(Some(Instant::now()));
  for _ in 0..5 {
    tree.scroll(POINTER.0, POINTER.1, -3.0, 0.0, ScrollPhase::Scroll);
  }
  assert_eq!(
    strip.scroll_x(),
    15.0,
    "no vertical event before them: a sideways swipe or Shift+wheel"
  );
  assert_eq!(page.scroll_y(), 0.0);
}

#[test]
fn a_sideways_burst_after_the_window_has_passed_scrolls_the_strip() {
  let (mut tree, page, strip) = page();
  let start = Instant::now();
  slanted_swipe_in_separate_events(&mut tree, start, Duration::from_millis(10), 5);
  assert_eq!(strip.scroll_x(), 0.0);
  tree.set_animation_clock(Some(start + Duration::from_millis(400)));
  for _ in 0..5 {
    tree.scroll(POINTER.0, POINTER.1, -3.0, 0.0, ScrollPhase::Scroll);
  }
  assert_eq!(
    strip.scroll_x(),
    15.0,
    "the vertical delta is forgotten after the window"
  );
  assert_eq!(page.scroll_y(), 30.0);
}

#[test]
fn a_sideways_event_at_another_spot_is_not_held_by_a_vertical_one() {
  let (mut tree, page, strip) = page();
  let start = Instant::now();
  tree.set_animation_clock(Some(start));
  tree.scroll(POINTER.0, POINTER.1, 0.0, -6.0, ScrollPhase::Scroll);
  tree.scroll(POINTER.0 + 40.0, POINTER.1, -3.0, 0.0, ScrollPhase::Scroll);
  assert_eq!(strip.scroll_x(), 3.0, "a pointer 40 px away is another gesture");
  assert_eq!(page.scroll_y(), 6.0);
}
