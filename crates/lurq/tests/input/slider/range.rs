//! A range slider: two thumbs on one track bound to a low value and a high one.

use lurq::{
  app::{Tree, events::MouseButton},
  components::{Column, Slider},
  core::Signal,
  node::{SliderPartStyle, color::Color},
};

use crate::support::{RectSnapshot, RenderSnapshot, pointer_click, render_pass, run_pass};

const TRACK: &str = "#64748b";
const FILL: &str = "#22c55e";
const THUMB: &str = "#f97316";
const RING: &str = "#2563eb";

/// A track of 110 and thumbs of 10: the thumbs' centers travel 100, from 5 to 105, a step of 10 each.
fn range(values: Signal<(i32, i32)>) -> Slider {
  Slider::new_range(values)
    .id("price")
    .range(0, 10)
    .size(110.0, 20.0)
    .tab_index(0)
    .track(|style| style.height(4.0).background(TRACK))
    .fill(|style| style.background(FILL))
    .thumb(|style| style.size(10.0, 10.0).background(THUMB))
}

fn thumbs(snapshot: &RenderSnapshot) -> Vec<&RectSnapshot> {
  snapshot
    .rects
    .iter()
    .filter(|rect| rect.color == Color::from_hex(THUMB))
    .collect()
}

fn key(tree: &mut Tree, key: &str, shift: bool) {
  tree.key_down(key.to_owned(), key.to_owned(), shift, false, false);
  tree.key_up(key.to_owned(), key.to_owned(), shift, false, false);
}

fn origin(tree: &mut Tree) -> (f32, f32) {
  let bounds = tree.find_element(|_| true).unwrap().bounds();
  (bounds.x, bounds.y + bounds.height / 2.0)
}

fn focused_id(tree: &Tree) -> Option<String> {
  tree
    .focused_element()
    .and_then(|element| element.id().map(str::to_owned))
}

#[test]
fn range_slider_paints_two_thumbs_and_fills_between_them() {
  let mut tree = Tree::new();
  tree.set_root(range(Signal::new((2, 7))));
  let snapshot = render_pass(&mut tree);
  let (x, _) = origin(&mut tree);

  let thumbs = thumbs(&snapshot);
  assert_eq!(thumbs.len(), 2);
  let centers: Vec<f32> = thumbs.iter().map(|thumb| thumb.x + thumb.width / 2.0 - x).collect();
  assert!((centers[0] - 25.0).abs() <= 0.5, "low thumb at {}", centers[0]);
  assert!((centers[1] - 75.0).abs() <= 0.5, "high thumb at {}", centers[1]);

  let fill = snapshot
    .rects
    .iter()
    .find(|rect| rect.color == Color::from_hex(FILL))
    .expect("the range between the thumbs should be filled");
  assert!((fill.x - x - 25.0).abs() <= 0.5);
  assert!((fill.width - 50.0).abs() <= 0.5);
  assert_eq!(fill.height, 4.0);
}

#[test]
fn a_press_on_the_track_moves_the_nearer_thumb() {
  let values = Signal::new((2, 7));
  let mut tree = Tree::new();
  tree.set_root(range(values.clone()));
  run_pass(&mut tree);
  let (x, y) = origin(&mut tree);

  pointer_click(&mut tree, x + 5.0 + 90.0, y, MouseButton::Left);
  assert_eq!(values.get(), (2, 9));

  pointer_click(&mut tree, x + 5.0 + 40.0, y, MouseButton::Left);
  assert_eq!(values.get(), (4, 9));

  pointer_click(&mut tree, x + 5.0, y, MouseButton::Left);
  assert_eq!(values.get(), (0, 9));
}

#[test]
fn a_drag_moves_the_thumb_it_started_on_and_stops_at_the_other() {
  let values = Signal::new((2, 7));
  let mut tree = Tree::new();
  tree.set_root(range(values.clone()));
  run_pass(&mut tree);
  let (x, y) = origin(&mut tree);

  // On the low thumb, then past the high one: the low stops where the high is.
  tree.mouse_down(x + 25.0, y, MouseButton::Left);
  tree.mouse_move(x + 65.0, y);
  assert_eq!(values.get(), (6, 7));
  tree.mouse_move(x + 100.0, y);
  tree.mouse_up(x + 100.0, y, MouseButton::Left);
  assert_eq!(values.get(), (7, 7));

  // Together, a press beyond them takes the thumb on that side, and they part.
  tree.mouse_down(x + 95.0, y, MouseButton::Left);
  tree.mouse_up(x + 95.0, y, MouseButton::Left);
  assert_eq!(values.get(), (7, 9));
  tree.mouse_down(x + 5.0 + 70.0, y, MouseButton::Left);
  tree.mouse_move(x + 5.0 + 30.0, y);
  tree.mouse_up(x + 5.0 + 30.0, y, MouseButton::Left);
  assert_eq!(values.get(), (3, 9));
}

#[test]
fn each_thumb_is_a_stop_of_tab_and_the_arrows_move_the_focused_one() {
  let values = Signal::new((2, 7));
  let mut tree = Tree::new();
  tree.set_root(
    Column::new()
      .child(range(values.clone()))
      .child(Slider::new(Signal::new(0)).id("after").range(0, 10).tab_index(0)),
  );
  run_pass(&mut tree);

  key(&mut tree, "Tab", false);
  assert_eq!(focused_id(&tree).as_deref(), Some("price"));
  key(&mut tree, "ArrowRight", false);
  assert_eq!(values.get(), (3, 7), "the low thumb is the first stop");

  key(&mut tree, "Tab", false);
  assert_eq!(
    focused_id(&tree).as_deref(),
    Some("price"),
    "the high thumb is the second"
  );
  key(&mut tree, "ArrowLeft", false);
  key(&mut tree, "ArrowLeft", false);
  assert_eq!(values.get(), (3, 5));
  // The arrows stop it at the low thumb's value.
  for _ in 0..4 {
    key(&mut tree, "ArrowDown", false);
  }
  assert_eq!(values.get(), (3, 3));

  key(&mut tree, "Tab", false);
  assert_eq!(focused_id(&tree).as_deref(), Some("after"));

  // Back into the range, Shift+Tab reaches its high thumb first, then its low one.
  key(&mut tree, "Tab", true);
  assert_eq!(focused_id(&tree).as_deref(), Some("price"));
  key(&mut tree, "ArrowUp", false);
  assert_eq!(values.get(), (3, 4));
  key(&mut tree, "Tab", true);
  assert_eq!(focused_id(&tree).as_deref(), Some("price"));
  key(&mut tree, "ArrowLeft", false);
  assert_eq!(values.get(), (2, 4));
}

#[test]
fn the_focus_ring_is_on_the_focused_thumb() {
  let mut tree = Tree::new();
  tree.set_root(Column::new().child(
    range(Signal::new((2, 7))).thumb_focused_style(SliderPartStyle::new().border_inside(2.0, Color::from_hex(RING))),
  ));
  run_pass(&mut tree);
  let (x, _) = origin(&mut tree);
  let ringed = |snapshot: &RenderSnapshot| {
    snapshot
      .rects
      .iter()
      .filter(|rect| rect.stroke_color == Color::from_hex(RING) && rect.stroke.iter().any(|width| *width > 0.0))
      .map(|rect| rect.x + rect.width / 2.0 - x)
      .collect::<Vec<_>>()
  };

  key(&mut tree, "Tab", false);
  let rings = ringed(&render_pass(&mut tree));
  assert_eq!(rings.len(), 1);
  assert!((rings[0] - 25.0).abs() <= 0.5, "ring on the low thumb, at {}", rings[0]);

  key(&mut tree, "Tab", false);
  let rings = ringed(&render_pass(&mut tree));
  assert_eq!(rings.len(), 1);
  assert!(
    (rings[0] - 75.0).abs() <= 0.5,
    "ring on the high thumb, at {}",
    rings[0]
  );
}

#[test]
fn a_disabled_range_slider_takes_nothing() {
  let values = Signal::new((2, 7));
  let mut tree = Tree::new();
  tree.set_root(range(values.clone()).disabled(true));
  run_pass(&mut tree);
  let (x, y) = origin(&mut tree);

  pointer_click(&mut tree, x + 100.0, y, MouseButton::Left);
  tree.mouse_down(x + 25.0, y, MouseButton::Left);
  tree.mouse_move(x + 60.0, y);
  tree.mouse_up(x + 60.0, y, MouseButton::Left);
  key(&mut tree, "Tab", false);
  key(&mut tree, "ArrowRight", false);

  assert_eq!(values.get(), (2, 7));
  assert_eq!(focused_id(&tree), None);
}

#[test]
fn a_range_slider_of_fractions_keeps_its_values_in_order() {
  let values = Signal::new((0.8_f32, 0.2_f32));
  let mut tree = Tree::new();
  tree.set_root(Slider::new_range_f32(values.clone()).range_f32(0.0, 1.0).step(0.1));
  run_pass(&mut tree);

  let (low, high) = values.get();
  assert!(low <= high, "{low} should not be past {high}");
}
