//! A press where a rounded clip cuts its content off does not reach that
//! content, as it is not drawn there: past the corner of a rounded panel, and
//! past it for what lies along the panel's edge inside a clip of its own.

use lurq::{
  app::{Tree, events::MouseButton},
  components::{Column, Rect, Row, Stack},
  core::Signal,
};

use crate::support::{pointer_click, run_pass};

fn counting_rect(width: f32, height: f32, clicks: &Signal<u32>) -> Rect {
  Rect::new(width, height).background("#f04040").on_click({
    let clicks = clicks.clone();
    move |_| clicks.update(|count| *count += 1)
  })
}

#[test]
fn a_press_past_a_rounded_corner_does_not_reach_what_the_corner_cuts_off() {
  let clicks = Signal::new(0);
  let mut tree = Tree::new();
  tree.set_root(
    Stack::new()
      .size(200.0, 100.0)
      .rounded(20.0)
      .clip()
      .child(counting_rect(200.0, 100.0, &clicks)),
  );
  run_pass(&mut tree);

  pointer_click(&mut tree, 2.0, 2.0, MouseButton::Left);
  pointer_click(&mut tree, 198.0, 98.0, MouseButton::Left);
  assert_eq!(clicks.get(), 0, "past the corners");

  pointer_click(&mut tree, 100.0, 50.0, MouseButton::Left);
  pointer_click(&mut tree, 100.0, 1.0, MouseButton::Left);
  assert_eq!(clicks.get(), 2, "inside the corners");
}

#[test]
fn a_press_past_the_panel_corner_does_not_reach_a_rail_along_its_edge() {
  let clicks = Signal::new(0);
  let mut tree = Tree::new();
  tree.set_root(
    Column::new()
      .spacing(0.0)
      .size(200.0, 100.0)
      .rounded(20.0)
      .clip()
      .child(Rect::new(200.0, 80.0))
      .child(Row::new().size(200.0, 20.0).child(counting_rect(200.0, 20.0, &clicks))),
  );
  run_pass(&mut tree);

  pointer_click(&mut tree, 2.0, 98.0, MouseButton::Left);
  assert_eq!(clicks.get(), 0, "past the panel's corner");

  pointer_click(&mut tree, 100.0, 90.0, MouseButton::Left);
  pointer_click(&mut tree, 15.0, 85.0, MouseButton::Left);
  assert_eq!(clicks.get(), 2, "on the rail inside the corner");
}

#[test]
fn content_that_overflows_visibly_is_hit_past_the_corner() {
  let clicks = Signal::new(0);
  let mut tree = Tree::new();
  tree.set_root(
    Stack::new()
      .size(200.0, 100.0)
      .rounded(20.0)
      .overflow_visible()
      .child(counting_rect(200.0, 100.0, &clicks)),
  );
  run_pass(&mut tree);

  pointer_click(&mut tree, 2.0, 2.0, MouseButton::Left);
  assert_eq!(clicks.get(), 1);
}
