//! Home, End, Page Up and Page Down on a slider: to the ends, and by a tenth of the range.

use lurq::{
  app::Tree,
  components::{Column, Slider},
  core::Signal,
};

use crate::support::run_pass;

fn key(tree: &mut Tree, key: &str, shift: bool) {
  tree.key_down(key.to_owned(), key.to_owned(), shift, false, false);
  tree.key_up(key.to_owned(), key.to_owned(), shift, false, false);
}

/// Mounts `slider` and Tabs to it.
fn focused(slider: Slider) -> Tree {
  let mut tree = Tree::new();
  tree.set_root(Column::new().child(slider.tab_index(0)));
  run_pass(&mut tree);
  key(&mut tree, "Tab", false);
  tree
}

#[test]
fn home_and_end_take_a_slider_to_its_ends() {
  for vertical in [false, true] {
    let value = Signal::new(40);
    let slider = Slider::new(value.clone()).range(10, 90);
    let mut tree = focused(if vertical { slider.vertical() } else { slider });

    key(&mut tree, "End", false);
    assert_eq!(value.get(), 90, "vertical: {vertical}");
    key(&mut tree, "Home", false);
    assert_eq!(value.get(), 10, "vertical: {vertical}");
  }
}

#[test]
fn page_up_and_down_move_a_tenth_of_the_range() {
  for vertical in [false, true] {
    let value = Signal::new(40);
    let slider = Slider::new(value.clone()).range(0, 100);
    let mut tree = focused(if vertical { slider.vertical() } else { slider });

    key(&mut tree, "PageUp", false);
    assert_eq!(value.get(), 50, "vertical: {vertical}");
    key(&mut tree, "PageDown", false);
    key(&mut tree, "PageDown", false);
    assert_eq!(value.get(), 30, "vertical: {vertical}");
    for _ in 0..5 {
      key(&mut tree, "PageDown", false);
    }
    assert_eq!(value.get(), 0, "no further than the least");
  }
}

#[test]
fn a_page_is_whole_steps_and_at_least_one() {
  // 0 to 1000 in steps of 5: a tenth is 100, twenty steps.
  let long = Signal::new(500);
  let mut tree = focused(Slider::new(long.clone()).range(0, 1000).step(5.0));
  key(&mut tree, "PageUp", false);
  assert_eq!(long.get(), 600);

  // 0 to 10: a tenth is one step.
  let short = Signal::new(5);
  let mut tree = focused(Slider::new(short.clone()).range(0, 10));
  key(&mut tree, "PageUp", false);
  assert_eq!(short.get(), 6);

  // 0 to 4 in steps of 2: a tenth is less than a step, and a page is still one.
  let coarse = Signal::new(2);
  let mut tree = focused(Slider::new(coarse.clone()).range(0, 4).step(2.0));
  key(&mut tree, "PageDown", false);
  assert_eq!(coarse.get(), 0);

  // Fractions: 0 to 1 in hundredths, a page of a tenth.
  let fine = Signal::new(0.5_f32);
  let mut tree = focused(Slider::new_f32(fine.clone()).range_f32(0.0, 1.0));
  key(&mut tree, "PageUp", false);
  assert!((fine.get() - 0.6).abs() < 1e-4, "{}", fine.get());
}

#[test]
fn the_keys_move_the_focused_thumb_of_a_range_and_it_stops_at_the_other() {
  for vertical in [false, true] {
    let values = Signal::new((30, 60));
    let slider = Slider::new_range(values.clone()).range(0, 100);
    let mut tree = focused(if vertical { slider.vertical() } else { slider });

    // The low thumb: Home to the least, End no further than the high thumb.
    key(&mut tree, "Home", false);
    assert_eq!(values.get(), (0, 60), "vertical: {vertical}");
    key(&mut tree, "PageUp", false);
    assert_eq!(values.get(), (10, 60));
    key(&mut tree, "End", false);
    assert_eq!(values.get(), (60, 60));
    key(&mut tree, "PageDown", false);
    assert_eq!(values.get(), (50, 60));

    // The high thumb: Home no further than the low thumb, End to the greatest.
    key(&mut tree, "Tab", false);
    key(&mut tree, "Home", false);
    assert_eq!(values.get(), (50, 50), "vertical: {vertical}");
    key(&mut tree, "PageUp", false);
    key(&mut tree, "PageUp", false);
    assert_eq!(values.get(), (50, 70));
    key(&mut tree, "End", false);
    assert_eq!(values.get(), (50, 100));
  }
}

#[test]
fn a_disabled_slider_takes_no_keys() {
  let value = Signal::new(40);
  let mut tree = focused(Slider::new(value.clone()).range(0, 100).disabled(true));
  for name in ["Home", "End", "PageUp", "PageDown"] {
    key(&mut tree, name, false);
  }
  assert_eq!(value.get(), 40);
}
