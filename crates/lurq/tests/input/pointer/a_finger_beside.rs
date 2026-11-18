//! A finger that taps while another is held: it clicks what it taps, and the finger that is held, the pointer,
//! goes on as it was. Two fingers that land and lift together are a tap of two, and click nothing.

use std::{
  sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
  },
  time::{Duration, Instant},
};

use lurq::{
  app::{Tree, events::TouchPhase},
  components::{Column, Rect},
};

use crate::support::run_pass;

/// Two boxes that are clicked, 100 by 40, one over the other with 20 between, and how many times each was clicked.
fn two_buttons() -> (Tree, Arc<AtomicUsize>, Arc<AtomicUsize>) {
  let (first, second) = (Arc::new(AtomicUsize::new(0)), Arc::new(AtomicUsize::new(0)));
  let (one, two) = (first.clone(), second.clone());
  let mut tree = Tree::new();
  tree.resize(200, 200);
  tree.set_root(
    Column::new()
      .size(200.0, 200.0)
      .spacing(20.0)
      .child(Rect::new(100.0, 40.0).on_click(move |_| {
        one.fetch_add(1, Ordering::SeqCst);
      }))
      .child(Rect::new(100.0, 40.0).on_click(move |_| {
        two.fetch_add(1, Ordering::SeqCst);
      })),
  );
  run_pass(&mut tree);
  (tree, first, second)
}

/// The middles of the two buttons.
const FIRST: (f32, f32) = (50.0, 20.0);
const SECOND: (f32, f32) = (50.0, 80.0);

#[test]
fn a_finger_that_taps_while_another_is_held_clicks_what_it_taps() {
  let (mut tree, first, second) = two_buttons();
  let start = Instant::now();
  tree.set_press_clock(Some(start));
  tree.finger(1, TouchPhase::Start, FIRST.0, FIRST.1, None);
  // Held a while, another finger taps the second button, twice.
  for at in [400, 700] {
    tree.set_press_clock(Some(start + Duration::from_millis(at)));
    tree.finger(2, TouchPhase::Start, SECOND.0, SECOND.1, None);
    tree.set_press_clock(Some(start + Duration::from_millis(at + 60)));
    tree.finger(2, TouchPhase::End, SECOND.0, SECOND.1, None);
    run_pass(&mut tree);
  }
  assert_eq!(second.load(Ordering::SeqCst), 2, "each tap clicks what it is on");
  assert_eq!(
    first.load(Ordering::SeqCst),
    0,
    "the finger that is held has not lifted"
  );
}

#[test]
fn a_finger_beside_that_travels_clicks_nothing() {
  let (mut tree, first, second) = two_buttons();
  let start = Instant::now();
  tree.set_press_clock(Some(start));
  tree.finger(1, TouchPhase::Start, FIRST.0, FIRST.1, None);
  tree.set_press_clock(Some(start + Duration::from_millis(400)));
  tree.finger(2, TouchPhase::Start, SECOND.0, SECOND.1, None);
  tree.finger(2, TouchPhase::Move, SECOND.0 + 60.0, SECOND.1, None);
  tree.finger(2, TouchPhase::Move, SECOND.0, SECOND.1, None);
  tree.finger(2, TouchPhase::End, SECOND.0, SECOND.1, None);
  run_pass(&mut tree);
  assert_eq!(
    (first.load(Ordering::SeqCst), second.load(Ordering::SeqCst)),
    (0, 0),
    "one that went away and came back is no tap"
  );
}
