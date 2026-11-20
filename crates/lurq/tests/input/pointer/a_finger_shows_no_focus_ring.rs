//! A finger shows no focus ring: the ring is drawn from the keyboard alone, and a finger that lands, whether it
//! taps or pans, takes away the ring the keyboard showed.

use lurq::{
  app::{Tree, events::TouchPhase},
  components::{Column, Rect, ScrollVertical},
  core::Signal,
  ui::{Button, SegmentedControl},
};

use crate::support::run_pass;

/// A list that scrolls, with a segmented control and a button at its top.
fn a_list_with_controls() -> Tree {
  let chosen = Signal::new("week");
  let mut tree = Tree::new();
  tree.resize(400, 600);
  tree.set_root(
    ScrollVertical::new(
      Column::new()
        .width(400.0)
        .spacing(8.0)
        .child(SegmentedControl::new(chosen, [("day", "Day"), ("week", "Week"), ("month", "Month")]).id("period"))
        .child(Button::new("Save").id("save"))
        .child(Rect::new(400.0, 2000.0)),
    )
    .size(400.0, 600.0),
  );
  run_pass(&mut tree);
  tree
}

fn tab(tree: &mut Tree) {
  tree.key_down("Tab".to_owned(), "Tab".to_owned(), false, false, false);
  tree.key_up("Tab".to_owned(), "Tab".to_owned(), false, false, false);
  run_pass(tree);
}

#[test]
fn a_finger_that_pans_takes_away_the_ring_the_keyboard_showed() {
  let mut tree = a_list_with_controls();
  tab(&mut tree);
  assert!(tree.focus_visible(), "Tab shows the ring");
  // A pull down the list, from below the controls.
  tree.finger(1, TouchPhase::Start, 200.0, 300.0, None);
  for step in 1..=10 {
    tree.finger(1, TouchPhase::Move, 200.0, 300.0 + 8.0 * step as f32, None);
  }
  tree.finger(1, TouchPhase::End, 200.0, 380.0, None);
  run_pass(&mut tree);
  assert!(!tree.focus_visible(), "a finger shows no ring");
}

#[test]
fn a_finger_that_taps_a_control_shows_no_ring() {
  let mut tree = a_list_with_controls();
  tab(&mut tree);
  let (x, y, width, height) = tree
    .find_element(|element| element.id() == Some("save"))
    .expect("the button")
    .rect();
  tree.finger(1, TouchPhase::Start, x + width / 2.0, y + height / 2.0, None);
  tree.finger(1, TouchPhase::End, x + width / 2.0, y + height / 2.0, None);
  run_pass(&mut tree);
  assert!(!tree.focus_visible(), "a tap shows no ring");
}
