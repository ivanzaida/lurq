//! A finger that travels past the slop before it lifts is no tap: it clicks nothing, whichever way it went,
//! whether or not anything under it scrolls that way, and after it rested into a long press too.

use std::{
  sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
  },
  time::{Duration, Instant},
};

use lurq::{
  app::{Tree, events::TouchPhase},
  components::{Column, Rect, ScrollVertical},
};

use crate::support::run_pass;

/// A list that scrolls up and down, with a box at its top that is clicked, 300 by 200, and how many times it was.
fn a_list_with_a_link() -> (Tree, Arc<AtomicUsize>) {
  let clicked = Arc::new(AtomicUsize::new(0));
  let counted = clicked.clone();
  let mut tree = Tree::new();
  tree.resize(400, 600);
  tree.set_root(
    ScrollVertical::new(
      Column::new()
        .width(400.0)
        .child(Rect::new(300.0, 200.0).on_click(move |_| {
          counted.fetch_add(1, Ordering::SeqCst);
        }))
        .child(Rect::new(400.0, 2000.0)),
    )
    .size(400.0, 600.0),
  );
  run_pass(&mut tree);
  (tree, clicked)
}

/// A box that is clicked, 300 by 300, with nothing that scrolls, and how many times it was.
fn a_box() -> (Tree, Arc<AtomicUsize>) {
  let clicked = Arc::new(AtomicUsize::new(0));
  let counted = clicked.clone();
  let mut tree = Tree::new();
  tree.resize(400, 400);
  tree.set_root(Rect::new(300.0, 300.0).on_click(move |_| {
    counted.fetch_add(1, Ordering::SeqCst);
  }));
  run_pass(&mut tree);
  (tree, clicked)
}

/// The finger that is down at `from` goes `by` away in ten steps, and lifts there.
fn travel(tree: &mut Tree, from: (f32, f32), by: (f32, f32)) {
  for step in 1..=10 {
    let part = step as f32 / 10.0;
    tree.finger(1, TouchPhase::Move, from.0 + by.0 * part, from.1 + by.1 * part, None);
  }
  tree.finger(1, TouchPhase::End, from.0 + by.0, from.1 + by.1, None);
  run_pass(tree);
}

/// A finger lands at `from`, goes `by` away and lifts.
fn drag(tree: &mut Tree, from: (f32, f32), by: (f32, f32)) {
  tree.finger(1, TouchPhase::Start, from.0, from.1, None);
  travel(tree, from, by);
}

#[test]
fn a_finger_that_travels_over_a_list_clicks_nothing_whichever_way_it_goes() {
  for by in [(40.0, 0.0), (-40.0, 0.0), (0.0, 40.0)] {
    let (mut tree, clicked) = a_list_with_a_link();
    drag(&mut tree, (150.0, 100.0), by);
    assert_eq!(clicked.load(Ordering::SeqCst), 0, "a drag {by:?} is no tap");
  }
  let (mut tree, clicked) = a_list_with_a_link();
  drag(&mut tree, (150.0, 100.0), (3.0, 2.0));
  assert_eq!(clicked.load(Ordering::SeqCst), 1, "a finger within the slop taps");
}

#[test]
fn a_finger_that_travels_over_a_box_where_nothing_scrolls_clicks_nothing() {
  let (mut tree, clicked) = a_box();
  drag(&mut tree, (100.0, 100.0), (40.0, 0.0));
  assert_eq!(clicked.load(Ordering::SeqCst), 0, "a drag is no tap");
  drag(&mut tree, (100.0, 100.0), (2.0, 3.0));
  assert_eq!(clicked.load(Ordering::SeqCst), 1, "a finger within the slop taps");
}

#[test]
fn a_finger_that_rested_into_a_long_press_and_then_travels_clicks_nothing() {
  let (mut tree, clicked) = a_list_with_a_link();
  let start = Instant::now();
  tree.set_press_clock(Some(start));
  tree.finger(1, TouchPhase::Start, 150.0, 100.0, None);
  let rested = start + Duration::from_millis(700);
  tree.set_press_clock(Some(rested));
  tree.finger_tick(rested);
  travel(&mut tree, (150.0, 100.0), (40.0, 0.0));
  assert_eq!(clicked.load(Ordering::SeqCst), 0);
}
