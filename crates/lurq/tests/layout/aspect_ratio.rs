//! `.aspect_ratio(...)`: the axis of a box that is auto follows the one that is known, as CSS `aspect-ratio`.

use lurq::{
  app::Tree,
  components::{Column, Rect, Row, Spacer},
  layout::{Alignment, Constraints, Size},
  node::{Element, dimension::Dimension},
};

use super::PassLayoutExt;

fn room() -> Constraints {
  Constraints::loose(Size::new(400.0, 300.0))
}

fn laid_out(root: impl Into<Element>, constraints: Constraints) -> lurq::layout::layout_result::LayoutResult {
  let mut tree = Tree::new();
  tree.set_root(root.into());
  tree.pass_layout(constraints).unwrap()
}

#[test]
fn a_said_width_gives_the_height() {
  let result = laid_out(Spacer::new().width(160.0).aspect_ratio(16.0 / 9.0), room());
  assert_eq!((result.size.width, result.size.height), (160.0, 90.0));
}

#[test]
fn a_said_height_gives_the_width() {
  let result = laid_out(Spacer::new().height(90.0).aspect_ratio(16.0 / 9.0), room());
  assert_eq!((result.size.width, result.size.height), (160.0, 90.0));
}

#[test]
fn both_said_and_the_ratio_does_nothing() {
  let result = laid_out(Rect::new(120.0, 30.0).aspect_ratio(1.0), room());
  assert_eq!((result.size.width, result.size.height), (120.0, 30.0));
}

#[test]
fn neither_said_and_the_box_is_as_wide_as_its_room() {
  let result = laid_out(Spacer::new().aspect_ratio(2.0), room());
  assert_eq!((result.size.width, result.size.height), (400.0, 200.0));
}

#[test]
fn a_width_with_no_bound_leaves_the_height_of_the_room_to_give_it() {
  let unbounded = Constraints {
    min_width: 0.0,
    max_width: f32::INFINITY,
    min_height: 0.0,
    max_height: 100.0,
  };
  let result = laid_out(Spacer::new().aspect_ratio(2.0), unbounded);
  assert_eq!((result.size.width, result.size.height), (200.0, 100.0));
}

#[test]
fn a_percentage_of_a_known_parent_is_a_known_axis() {
  let column = Column::with(
    0.0,
    Alignment::Start,
    vec![Element::from(
      Spacer::new().width(Dimension::Pct(50.0)).aspect_ratio(4.0 / 3.0),
    )],
  )
  .width(240.0);
  let result = laid_out(column, room());
  let child = &result.children[0].result;
  assert_eq!((child.size.width, child.size.height), (120.0, 90.0));
}

#[test]
fn a_width_the_parent_fixes_gives_the_height() {
  // A child that fills its row is as wide as the row leaves it: the ratio makes it square.
  let row = Row::with(
    0.0,
    Alignment::Start,
    vec![
      Element::from(Rect::new(100.0, 10.0)),
      Element::from(Spacer::new().flex(1.0).aspect_ratio(1.0)),
    ],
  )
  .width(180.0);
  let result = laid_out(row, room());
  let child = &result.children[1].result;
  assert_eq!((child.size.width, child.size.height), (80.0, 80.0));
  assert_eq!(result.size.height, 80.0, "the row is as high as the square");
}

#[test]
fn the_bounds_of_the_given_axis_still_hold() {
  let capped = laid_out(Spacer::new().width(300.0).aspect_ratio(1.0).max_height(120.0), room());
  assert_eq!((capped.size.width, capped.size.height), (300.0, 120.0));
  let raised = laid_out(Spacer::new().width(40.0).aspect_ratio(4.0).min_height(24.0), room());
  assert_eq!((raised.size.width, raised.size.height), (40.0, 24.0));
}

#[test]
fn a_ratio_that_is_no_positive_number_is_ignored() {
  for ratio in [0.0, -2.0, f32::NAN, f32::INFINITY] {
    let result = laid_out(Spacer::new().width(100.0).aspect_ratio(ratio), room());
    assert_eq!(
      (result.size.width, result.size.height),
      (100.0, 0.0),
      "a ratio of {ratio} gives no height"
    );
  }
}

#[test]
fn a_box_with_children_keeps_its_ratio_over_what_they_ask() {
  let column = Column::with(0.0, Alignment::Start, vec![Element::from(Rect::new(20.0, 20.0))])
    .width(200.0)
    .aspect_ratio(2.0);
  let result = laid_out(column, room());
  assert_eq!((result.size.width, result.size.height), (200.0, 100.0));
}

#[test]
fn a_changed_ratio_lays_the_box_out_again() {
  let mut tree = Tree::new();
  tree.set_root(Spacer::new().width(120.0).aspect_ratio(1.0));
  let square = tree.pass_layout(room()).unwrap();
  assert_eq!(square.size.height, 120.0);
  tree.set_root(Spacer::new().width(120.0).aspect_ratio(2.0));
  let wide = tree.pass_layout(room()).unwrap();
  assert_eq!(
    wide.size.height, 60.0,
    "the new ratio is not served from the cache of the old"
  );
}

#[cfg(feature = "raster")]
mod image {
  use lurq::{components::Image, images::ImageData};

  use super::{laid_out, room};

  fn pixels(width: u32, height: u32) -> ImageData {
    ImageData::from_rgba(vec![255; (width * height * 4) as usize], width, height)
  }

  #[test]
  #[allow(deprecated)]
  fn a_ratio_stands_for_the_pictures_own() {
    // 80 by 40 pixels, in a box said to be square.
    let result = laid_out(Image::new(pixels(80, 40)).width(60.0).aspect_ratio(1.0), room());
    assert_eq!((result.size.width, result.size.height), (60.0, 60.0));
  }

  #[test]
  #[allow(deprecated)]
  fn with_neither_axis_said_the_picture_keeps_its_own_width() {
    let result = laid_out(Image::new(pixels(80, 40)).aspect_ratio(1.0), room());
    assert_eq!((result.size.width, result.size.height), (80.0, 80.0));
  }

  #[test]
  #[allow(deprecated)]
  fn without_a_ratio_the_pictures_own_still_gives_the_other_axis() {
    let result = laid_out(Image::new(pixels(80, 40)).width(60.0), room());
    assert_eq!((result.size.width, result.size.height), (60.0, 30.0));
  }
}
