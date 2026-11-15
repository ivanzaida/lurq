//! How an image's pixels are placed in its box: the fit, and `.image_anchor(...)` where it is cut or does not
//! fill the box.

#![cfg(feature = "raster")]
#![allow(deprecated)]

use lurq::{
  app::Tree,
  components::{Image, Spacer},
  images::ImageData,
  layout::{Constraints, Size, quad::QuadContent},
  node::{BackgroundSize, Element},
};

use super::PassLayoutExt;

/// Where an image quad lies and the part of the image it draws: `(x, y, width, height)`, `uv_min`, `uv_max`.
type Placed = ((f32, f32, f32, f32), [f32; 2], [f32; 2]);

fn pixels(width: u32, height: u32) -> ImageData {
  ImageData::from_rgba(vec![255; (width * height * 4) as usize], width, height)
}

fn placed(root: impl Into<Element>) -> Placed {
  let mut tree = Tree::new();
  tree.set_root(root.into());
  let layout = tree.pass_layout(Constraints::loose(Size::new(400.0, 400.0))).unwrap();
  let quads = tree.resolve_quads(&layout);
  let quad = quads
    .iter()
    .find(|quad| matches!(quad.content, QuadContent::Image { .. }))
    .expect("an image quad");
  match quad.content {
    QuadContent::Image { uv_min, uv_max, .. } => ((quad.x, quad.y, quad.width, quad.height), uv_min, uv_max),
    _ => unreachable!(),
  }
}

/// A picture 200 by 100 in a box 100 by 100.
fn wide_in_a_square(fit: BackgroundSize) -> Image {
  Image::new(pixels(200, 100)).size(100.0, 100.0).fit(fit)
}

/// A picture 100 by 200 in a box 100 by 100.
fn tall_in_a_square(fit: BackgroundSize) -> Image {
  Image::new(pixels(100, 200)).size(100.0, 100.0).fit(fit)
}

const WHOLE_BOX: (f32, f32, f32, f32) = (0.0, 0.0, 100.0, 100.0);

#[test]
fn a_stretched_picture_fills_its_box_and_has_nothing_to_hold() {
  for anchor in [(0.0, 0.0), (0.5, 0.5), (1.0, 1.0)] {
    let picture = wide_in_a_square(BackgroundSize::Stretch).image_anchor(anchor.0, anchor.1);
    assert_eq!(placed(picture), (WHOLE_BOX, [0.0, 0.0], [1.0, 1.0]));
  }
}

#[test]
fn a_cover_is_cut_about_the_middle_unless_told() {
  assert_eq!(
    placed(wide_in_a_square(BackgroundSize::Cover)),
    (WHOLE_BOX, [0.25, 0.0], [0.75, 1.0])
  );
  assert_eq!(
    placed(tall_in_a_square(BackgroundSize::Cover)),
    (WHOLE_BOX, [0.0, 0.25], [1.0, 0.75])
  );
}

#[test]
fn a_cover_keeps_the_side_it_is_anchored_to() {
  let left = wide_in_a_square(BackgroundSize::Cover).image_anchor(0.0, 0.5);
  assert_eq!(
    placed(left),
    (WHOLE_BOX, [0.0, 0.0], [0.5, 1.0]),
    "the left half is kept"
  );
  let right = wide_in_a_square(BackgroundSize::Cover).image_anchor(1.0, 0.5);
  assert_eq!(
    placed(right),
    (WHOLE_BOX, [0.5, 0.0], [1.0, 1.0]),
    "the right half is kept"
  );
  let top = tall_in_a_square(BackgroundSize::Cover).image_anchor(0.5, 0.0);
  assert_eq!(placed(top), (WHOLE_BOX, [0.0, 0.0], [1.0, 0.5]), "the top half is kept");
  let bottom = tall_in_a_square(BackgroundSize::Cover).image_anchor(0.5, 1.0);
  assert_eq!(
    placed(bottom),
    (WHOLE_BOX, [0.0, 0.5], [1.0, 1.0]),
    "the bottom half is kept"
  );
}

#[test]
fn a_contained_picture_stands_where_it_is_anchored() {
  let whole = ([0.0, 0.0], [1.0, 1.0]);
  let middle = placed(wide_in_a_square(BackgroundSize::Contain));
  assert_eq!(middle, ((0.0, 25.0, 100.0, 50.0), whole.0, whole.1));
  let top = placed(wide_in_a_square(BackgroundSize::Contain).image_anchor(0.5, 0.0));
  assert_eq!(top, ((0.0, 0.0, 100.0, 50.0), whole.0, whole.1));
  let bottom = placed(wide_in_a_square(BackgroundSize::Contain).image_anchor(0.5, 1.0));
  assert_eq!(bottom, ((0.0, 50.0, 100.0, 50.0), whole.0, whole.1));
  let right = placed(tall_in_a_square(BackgroundSize::Contain).image_anchor(1.0, 0.5));
  assert_eq!(right, ((50.0, 0.0, 50.0, 100.0), whole.0, whole.1));
}

#[test]
fn a_natural_picture_larger_than_its_box_is_cut_at_its_own_size() {
  // 200 by 100 in 100 by 50: half of each axis shows, a pixel to a pixel.
  let picture = |x: f32, y: f32| {
    Image::new(pixels(200, 100))
      .size(100.0, 50.0)
      .fit(BackgroundSize::Natural)
      .image_anchor(x, y)
  };
  let box_100_by_50 = (0.0, 0.0, 100.0, 50.0);
  assert_eq!(placed(picture(0.5, 0.5)), (box_100_by_50, [0.25, 0.25], [0.75, 0.75]));
  assert_eq!(placed(picture(0.0, 0.0)), (box_100_by_50, [0.0, 0.0], [0.5, 0.5]));
  assert_eq!(placed(picture(1.0, 1.0)), (box_100_by_50, [0.5, 0.5], [1.0, 1.0]));
}

#[test]
fn a_natural_picture_smaller_than_its_box_keeps_its_size() {
  let picture = |x: f32, y: f32| {
    Image::new(pixels(40, 20))
      .size(100.0, 100.0)
      .fit(BackgroundSize::Natural)
      .image_anchor(x, y)
  };
  let whole = ([0.0, 0.0], [1.0, 1.0]);
  assert_eq!(placed(picture(0.5, 0.5)), ((30.0, 40.0, 40.0, 20.0), whole.0, whole.1));
  assert_eq!(placed(picture(0.0, 1.0)), ((0.0, 80.0, 40.0, 20.0), whole.0, whole.1));
}

#[test]
fn a_natural_picture_is_cut_on_the_axis_it_is_too_long_on_alone() {
  // 200 by 20 in 100 by 100: cut across, and room left up and down.
  let picture = Image::new(pixels(200, 20))
    .size(100.0, 100.0)
    .fit(BackgroundSize::Natural)
    .image_anchor(0.0, 1.0);
  assert_eq!(placed(picture), ((0.0, 80.0, 100.0, 20.0), [0.0, 0.0], [0.5, 1.0]));
}

#[test]
fn an_anchor_out_of_its_range_is_the_nearest_end() {
  let past = wide_in_a_square(BackgroundSize::Cover).image_anchor(7.0, -3.0);
  assert_eq!(placed(past), (WHOLE_BOX, [0.5, 0.0], [1.0, 1.0]));
  let not_a_number = wide_in_a_square(BackgroundSize::Cover).image_anchor(f32::NAN, f32::NAN);
  assert_eq!(
    placed(not_a_number),
    (WHOLE_BOX, [0.25, 0.0], [0.75, 1.0]),
    "the middle"
  );
}

#[test]
fn a_background_image_is_held_by_the_same_anchor() {
  let cover = Spacer::new()
    .size(100.0, 100.0)
    .background_image(pixels(200, 100))
    .background_cover()
    .image_anchor(1.0, 0.5);
  assert_eq!(placed(cover), (WHOLE_BOX, [0.5, 0.0], [1.0, 1.0]));
  let contain = Spacer::new()
    .size(100.0, 100.0)
    .background_image(pixels(200, 100))
    .background_contain()
    .image_anchor(0.5, 1.0);
  assert_eq!(placed(contain).0, (0.0, 50.0, 100.0, 50.0));
}
