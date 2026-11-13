//! A wheel or a trackpad scrolls one container on each axis: the innermost
//! under the pointer that can still move that way takes that axis of the
//! event whole. What it cannot take, reaching its end within the event, is
//! dropped and not handed to the container around it, so a small list
//! scrolled inside a page does not take the page along. A container already
//! at its end takes nothing, and the event goes to the one around it. A
//! finger's scroll has its own targets and still hands the rest on.

use std::sync::{
  Arc,
  atomic::{AtomicUsize, Ordering},
};

use lurq::{
  app::{Tree, events::ScrollPhase},
  components::{Column, Rect, ScrollBoth, ScrollHorizontal, ScrollVertical},
  layout::layout_kind::ScrollState,
};

use crate::support::run_pass;

/// Over the list or the strip at the top of the page, and still over it once
/// the page has scrolled by a hundred.
const POINTER: (f32, f32) = (150.0, 25.0);
/// How far the list and the strip scroll: their content is this much larger
/// than their viewport.
const OVERFLOW: f32 = 30.0;

/// One wheel event at the pointer, and the frame after it.
fn wheel(tree: &mut Tree, delta_x: f32, delta_y: f32) {
  tree.scroll(POINTER.0, POINTER.1, delta_x, delta_y, ScrollPhase::Scroll);
  run_pass(tree);
}

fn vertical(page: &ScrollState, list: &ScrollState) -> (f32, f32) {
  (list.scroll_y(), page.scroll_y())
}

/// A 300x200 page scrolling down, with a 300x150 list at its top whose content
/// is `OVERFLOW` taller, and more below. Gives the page's state and the list's.
fn list_in_a_page() -> (Tree, ScrollState, ScrollState) {
  let (page, list) = (ScrollState::new(), ScrollState::new());
  let mut tree = Tree::new();
  tree.set_root(
    ScrollVertical::new(
      Column::new()
        .spacing(0.0)
        .child(
          ScrollVertical::new(Rect::new(300.0, 150.0 + OVERFLOW).background("#1f2937"))
            .with_scroll_state(list.clone())
            .size(300.0, 150.0),
        )
        .child(Rect::new(300.0, 1000.0).background("#111827")),
    )
    .with_scroll_state(page.clone())
    .size(300.0, 200.0),
  );
  run_pass(&mut tree);
  assert_eq!(list.content_height() - list.viewport_height(), OVERFLOW);
  (tree, page, list)
}

#[test]
fn an_event_larger_than_the_lists_overflow_moves_the_list_alone() {
  let (mut tree, page, list) = list_in_a_page();
  wheel(&mut tree, 0.0, -100.0);
  assert_eq!(
    vertical(&page, &list),
    (OVERFLOW, 0.0),
    "the list goes to its end, and the page gets none of what the list could not take"
  );
}

#[test]
fn the_next_event_the_same_way_moves_the_page() {
  let (mut tree, page, list) = list_in_a_page();
  wheel(&mut tree, 0.0, -100.0);
  wheel(&mut tree, 0.0, -100.0);
  assert_eq!(
    vertical(&page, &list),
    (OVERFLOW, 100.0),
    "the list is at its end: the whole event is the page's"
  );
}

#[test]
fn an_event_the_other_way_moves_the_list_again() {
  let (mut tree, page, list) = list_in_a_page();
  wheel(&mut tree, 0.0, -100.0);
  wheel(&mut tree, 0.0, -100.0);
  wheel(&mut tree, 0.0, 100.0);
  assert_eq!(
    vertical(&page, &list),
    (0.0, 100.0),
    "the list has room that way again and takes the event; the page keeps its place"
  );
  wheel(&mut tree, 0.0, 60.0);
  assert_eq!(
    vertical(&page, &list),
    (0.0, 40.0),
    "at its start the list leaves the next one to the page"
  );
}

#[test]
fn many_small_events_fill_the_list_before_the_page_moves() {
  let (mut tree, page, list) = list_in_a_page();
  // A trackpad's moves: 12, 24, then 30 with 6 of the third dropped, then the page.
  for _ in 0..3 {
    wheel(&mut tree, 0.0, -12.0);
  }
  assert_eq!(vertical(&page, &list), (OVERFLOW, 0.0));
  wheel(&mut tree, 0.0, -12.0);
  assert_eq!(vertical(&page, &list), (OVERFLOW, 12.0));
}

#[test]
fn the_list_that_reached_its_end_hears_it_and_the_page_does_not() {
  let (list_reached, page_reached) = (Arc::new(AtomicUsize::new(0)), Arc::new(AtomicUsize::new(0)));
  let (list_count, page_count) = (list_reached.clone(), page_reached.clone());
  let list = ScrollState::new();
  let mut tree = Tree::new();
  tree.set_root(
    ScrollVertical::new(
      Column::new()
        .spacing(0.0)
        .child(
          ScrollVertical::new(Rect::new(300.0, 150.0 + OVERFLOW).background("#1f2937"))
            .with_scroll_state(list.clone())
            .on_scroll_reach_bottom(move |_| {
              list_count.fetch_add(1, Ordering::SeqCst);
            })
            .size(300.0, 150.0),
        )
        // The page's own overflow is smaller than the event: handed the rest, it would reach its end too.
        .child(Rect::new(300.0, 100.0).background("#111827")),
    )
    .on_scroll_reach_bottom(move |_| {
      page_count.fetch_add(1, Ordering::SeqCst);
    })
    .size(300.0, 200.0),
  );
  run_pass(&mut tree);

  wheel(&mut tree, 0.0, -100.0);
  assert_eq!(list.scroll_y(), OVERFLOW);
  assert_eq!(
    (list_reached.load(Ordering::SeqCst), page_reached.load(Ordering::SeqCst)),
    (1, 0),
    "the list moved to its end; the page did not move"
  );

  wheel(&mut tree, 0.0, -100.0);
  assert_eq!(
    (list_reached.load(Ordering::SeqCst), page_reached.load(Ordering::SeqCst)),
    (1, 1),
    "the next event is the page's, which reaches its own end"
  );
}

/// A 400x200 page with a 400x50 strip scrolling sideways at its top, whose
/// content is `OVERFLOW` wider, and 800x1000 of content below. `page` makes
/// the page from its content.
fn strip_in_a_page<P: Into<lurq::node::Element>>(
  page: impl FnOnce(Column, ScrollState) -> P,
) -> (Tree, ScrollState, ScrollState) {
  let (page_state, strip) = (ScrollState::new(), ScrollState::new());
  let mut tree = Tree::new();
  let content = Column::new()
    .spacing(0.0)
    .child(
      ScrollHorizontal::new(Rect::new(400.0 + OVERFLOW, 50.0).background("#1f2937"))
        .with_scroll_state(strip.clone())
        .size(400.0, 50.0),
    )
    .child(Rect::new(800.0, 1000.0).background("#111827"));
  tree.set_root(page(content, page_state.clone()));
  run_pass(&mut tree);
  assert_eq!(strip.content_width() - strip.viewport_width(), OVERFLOW);
  (tree, page_state, strip)
}

#[test]
fn a_strip_takes_the_sideways_part_of_a_slanted_event_and_the_page_the_vertical() {
  let (mut tree, page, strip) =
    strip_in_a_page(|content, state| ScrollVertical::new(content).with_scroll_state(state).size(400.0, 200.0));
  wheel(&mut tree, -20.0, -10.0);
  assert_eq!(
    (strip.scroll_x(), page.scroll_y()),
    (20.0, 10.0),
    "each axis has its own container"
  );
}

#[test]
fn each_axis_of_a_slanted_event_is_handed_on_by_itself() {
  let (mut tree, page, strip) =
    strip_in_a_page(|content, state| ScrollBoth::new(content).with_scroll_state(state).size(400.0, 200.0));

  wheel(&mut tree, -100.0, -40.0);
  assert_eq!(strip.scroll_x(), OVERFLOW, "the strip goes to its end");
  assert_eq!(
    (page.scroll_x(), page.scroll_y()),
    (0.0, 40.0),
    "the page takes the vertical part, and none of the sideways rest of the strip's"
  );

  wheel(&mut tree, -100.0, -40.0);
  assert_eq!(strip.scroll_x(), OVERFLOW);
  assert_eq!(
    (page.scroll_x(), page.scroll_y()),
    (100.0, 80.0),
    "with the strip at its end both parts are the page's"
  );
}

#[test]
fn a_list_that_scrolls_both_ways_hands_on_only_the_axis_it_is_at_the_end_of() {
  let (page, list) = (ScrollState::new(), ScrollState::new());
  let mut tree = Tree::new();
  tree.set_root(
    ScrollVertical::new(
      Column::new()
        .spacing(0.0)
        .child(
          ScrollBoth::new(Rect::new(900.0, 150.0 + OVERFLOW).background("#1f2937"))
            .with_scroll_state(list.clone())
            .size(300.0, 150.0),
        )
        .child(Rect::new(300.0, 1000.0).background("#111827")),
    )
    .with_scroll_state(page.clone())
    .size(300.0, 200.0),
  );
  run_pass(&mut tree);

  wheel(&mut tree, -50.0, -100.0);
  assert_eq!((list.scroll_x(), list.scroll_y()), (50.0, OVERFLOW));
  assert_eq!(page.scroll_y(), 0.0, "the list took the vertical part, to its end");

  wheel(&mut tree, -50.0, -100.0);
  assert_eq!(
    (list.scroll_x(), list.scroll_y()),
    (100.0, OVERFLOW),
    "the list still has room sideways"
  );
  assert_eq!(
    page.scroll_y(),
    100.0,
    "down, it is at its end: that part is the page's"
  );
}

#[test]
fn three_containers_deep_the_event_goes_to_the_innermost_with_room() {
  let (page, middle, list) = (ScrollState::new(), ScrollState::new(), ScrollState::new());
  let mut tree = Tree::new();
  tree.set_root(
    ScrollVertical::new(
      Column::new()
        .spacing(0.0)
        .child(
          ScrollVertical::new(
            Column::new()
              .spacing(0.0)
              .child(
                ScrollVertical::new(Rect::new(300.0, 100.0 + OVERFLOW).background("#1f2937"))
                  .with_scroll_state(list.clone())
                  .size(300.0, 100.0),
              )
              .child(Rect::new(300.0, 50.0 + OVERFLOW).background("#374151")),
          )
          .with_scroll_state(middle.clone())
          .size(300.0, 150.0),
        )
        .child(Rect::new(300.0, 1000.0).background("#111827")),
    )
    .with_scroll_state(page.clone())
    .size(300.0, 200.0),
  );
  run_pass(&mut tree);

  wheel(&mut tree, 0.0, -100.0);
  assert_eq!(
    (list.scroll_y(), middle.scroll_y(), page.scroll_y()),
    (OVERFLOW, 0.0, 0.0)
  );
  wheel(&mut tree, 0.0, -100.0);
  assert_eq!(
    (list.scroll_y(), middle.scroll_y(), page.scroll_y()),
    (OVERFLOW, OVERFLOW, 0.0),
    "the list at its end is passed by, and the one around it takes the event alone"
  );
  wheel(&mut tree, 0.0, -100.0);
  assert_eq!(
    (list.scroll_y(), middle.scroll_y(), page.scroll_y()),
    (OVERFLOW, OVERFLOW, 100.0)
  );
}

#[test]
fn a_fingers_scroll_still_hands_on_what_a_container_inside_could_not_take() {
  let (mut tree, page, list) = list_in_a_page();
  wheel(&mut tree, 0.0, -100.0);
  assert_eq!(vertical(&page, &list), (OVERFLOW, 0.0));

  // The list is at its end the way the finger goes, so the page takes the gesture, and the list inside
  // it is still one of the gesture's targets.
  tree.touch_scroll_begin(POINTER.0, POINTER.1, 0.0, -40.0);
  tree.touch_scroll(POINTER.0, POINTER.1, 0.0, -40.0, ScrollPhase::Scroll, false);
  run_pass(&mut tree);
  assert_eq!(vertical(&page, &list), (OVERFLOW, 40.0));

  // The finger turns back by more than the list holds: the list takes what it can, and the rest of the
  // same event goes on to the page, which a wheel's would not.
  tree.touch_scroll(POINTER.0, POINTER.1, 0.0, 50.0, ScrollPhase::Scroll, false);
  run_pass(&mut tree);
  assert_eq!(vertical(&page, &list), (0.0, 20.0));
  tree.touch_scroll(POINTER.0, POINTER.1, 0.0, 0.0, ScrollPhase::End, false);
}
