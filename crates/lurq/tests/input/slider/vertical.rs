//! A vertical slider: its least value at the bottom, laid out, painted and dragged along its height.

use lurq::{
  app::{Tree, events::MouseButton},
  components::{Column, Slider},
  core::Signal,
  layout::Alignment,
  node::{SliderOrientation, color::Color},
};

use crate::support::{RectSnapshot, RenderSnapshot, pointer_click, render_pass, run_pass};

const TRACK: &str = "#64748b";
const FILL: &str = "#22c55e";
const THUMB: &str = "#f97316";

fn vertical(value: Signal<i32>) -> Slider {
  Slider::new(value)
    .range(0, 10)
    .vertical()
    .size(20.0, 110.0)
    .track(|style| style.width(4.0).background(TRACK))
    .fill(|style| style.background(FILL))
    .thumb(|style| style.size(10.0, 10.0).background(THUMB))
}

fn rect<'a>(snapshot: &'a RenderSnapshot, hex: &str) -> &'a RectSnapshot {
  snapshot
    .rects
    .iter()
    .find(|rect| rect.color == Color::from_hex(hex))
    .unwrap_or_else(|| panic!("{hex} should be painted"))
}

fn key(tree: &mut Tree, key: &str) {
  tree.key_down(key.to_owned(), key.to_owned(), false, false, false);
  tree.key_up(key.to_owned(), key.to_owned(), false, false, false);
}

#[test]
fn vertical_slider_lays_its_parts_along_the_height() {
  let value = Signal::new(2);
  let mut tree = Tree::new();
  tree.set_root(vertical(value));
  let snapshot = render_pass(&mut tree);
  let bounds = tree.find_element(|_| true).unwrap().bounds();

  let track = rect(&snapshot, TRACK);
  assert_eq!((track.width, track.height), (4.0, 110.0));
  assert_eq!(track.x, bounds.x + 8.0);

  // The thumb travels 100 from the bottom: at 2 of 10 its center is 20 above where it starts.
  let thumb = rect(&snapshot, THUMB);
  let thumb_center_y = thumb.y + thumb.height / 2.0;
  assert!(
    (thumb_center_y - (bounds.y + 105.0 - 20.0)).abs() <= 0.5,
    "thumb at {thumb_center_y}"
  );
  assert_eq!(thumb.x + thumb.width / 2.0, track.x + track.width / 2.0);

  // The fill runs from the thumb down to the bottom of the track, as thick as the track.
  let fill = rect(&snapshot, FILL);
  assert_eq!(fill.width, 4.0);
  assert!((fill.y - thumb_center_y).abs() <= 0.5);
  assert!((fill.y + fill.height - (track.y + track.height)).abs() <= 0.5);
}

#[test]
fn vertical_slider_has_its_least_value_at_the_bottom() {
  let value = Signal::new(5);
  let mut tree = Tree::new();
  tree.set_root(vertical(value.clone()));
  run_pass(&mut tree);
  let bounds = tree.find_element(|_| true).unwrap().bounds();
  let x = bounds.x + bounds.width / 2.0;

  pointer_click(&mut tree, x, bounds.y + 1.0, MouseButton::Left);
  assert_eq!(value.get(), 10, "a press at the top takes the greatest value");

  pointer_click(&mut tree, x, bounds.y + bounds.height - 1.0, MouseButton::Left);
  assert_eq!(value.get(), 0, "a press at the bottom takes the least");
}

#[test]
fn dragging_a_vertical_slider_up_raises_it() {
  let value = Signal::new(0);
  let mut tree = Tree::new();
  tree.set_root(vertical(value.clone()));
  run_pass(&mut tree);
  let bounds = tree.find_element(|_| true).unwrap().bounds();
  let x = bounds.x + bounds.width / 2.0;
  let bottom = bounds.y + 105.0;

  tree.mouse_down(x, bottom, MouseButton::Left);
  // Sideways the pointer moves nothing; up it raises the value.
  tree.mouse_move(x + 40.0, bottom);
  assert_eq!(value.get(), 0);
  tree.mouse_move(x, bottom - 70.0);
  tree.mouse_up(x, bottom - 70.0, MouseButton::Left);

  assert_eq!(value.get(), 7);
}

#[test]
fn up_and_down_move_a_vertical_slider() {
  let value = Signal::new(5);
  let mut tree = Tree::new();
  tree.set_root(vertical(value.clone()).tab_index(0));
  run_pass(&mut tree);

  key(&mut tree, "Tab");
  key(&mut tree, "ArrowUp");
  assert_eq!(value.get(), 6);
  key(&mut tree, "ArrowDown");
  key(&mut tree, "ArrowDown");
  assert_eq!(value.get(), 4);
}

#[test]
fn vertical_slider_asks_for_a_tall_box() {
  let mut tree = Tree::new();
  tree.set_root(
    Column::new().align_items(Alignment::Start).child(
      Slider::new(Signal::new(0))
        .id("slider")
        .orientation(SliderOrientation::Vertical),
    ),
  );
  run_pass(&mut tree);
  let bounds = tree
    .get_element_by_id_mut("slider")
    .and_then(|slider| slider.bounds())
    .unwrap();

  assert_eq!((bounds.width, bounds.height), (20.0, 120.0));
}

#[test]
fn disabled_vertical_slider_takes_no_press() {
  let value = Signal::new(5);
  let mut tree = Tree::new();
  tree.set_root(vertical(value.clone()).disabled(true));
  run_pass(&mut tree);
  let bounds = tree.find_element(|_| true).unwrap().bounds();

  pointer_click(
    &mut tree,
    bounds.x + bounds.width / 2.0,
    bounds.y + 1.0,
    MouseButton::Left,
  );

  assert_eq!(value.get(), 5);
}
