//! A sparkline: its canvas of the board's 80 by 28, a point for every value where the line is drawn through it
//! (the first at the start, the greatest at the top), the app's range, one value, no values, values on both sides
//! of 0, painted again with other values, and what tooling reads.

use lurq::{
  app::theme::PaletteColor,
  canvas::{CanvasItem, CanvasItemShape},
  components::Column,
  ui::{Sparkline, SparklinePart},
};

use super::stage::Stage;

/// The points the sparkline's canvas holds, as `(x, y)` from its top left.
fn points(stage: &mut Stage, id: &str) -> Vec<(f32, f32)> {
  let canvas = stage
    .tree
    .get_element_by_id(id)
    .and_then(|element| element.as_canvas())
    .expect("the sparkline is a canvas");
  canvas
    .items()
    .iter()
    .map(|item: &CanvasItem| match item.shape {
      CanvasItemShape::Point { x, y, .. } => (x, y),
      _ => panic!("a point"),
    })
    .collect()
}

fn stage_of(sparkline: impl Fn() -> Sparkline + Send + Sync + 'static) -> Stage {
  Stage::new(move |_| Column::new().child(sparkline().id("spark")).into())
}

const FRAMES: [f64; 8] = [118.0, 124.0, 121.0, 133.0, 129.0, 141.0, 138.0, 144.0];

#[test]
fn a_point_for_each_value_across_the_board_s_80_by_28() {
  let mut stage = stage_of(|| Sparkline::new(FRAMES).color(PaletteColor::Success));
  assert_eq!(stage.bounds("spark").2, 80.0);
  assert_eq!(stage.bounds("spark").3, 28.0);
  let placed = points(&mut stage, "spark");
  assert_eq!(placed.len(), 8);
  // Half the line's 1.75 in from every edge: the first at the start, the last at the end, the greatest at the top.
  let inset = 1.75 / 2.0;
  assert!((placed[0].0 - inset).abs() < 0.01);
  assert!((placed[7].0 - (80.0 - inset)).abs() < 0.01);
  assert!((placed[7].1 - inset).abs() < 0.01, "144, the greatest, at the top");
  assert!(
    (placed[0].1 - (28.0 - inset)).abs() < 0.01,
    "118, the least, at the bottom"
  );
  // Evenly apart.
  let step = placed[1].0 - placed[0].0;
  assert!((placed[5].0 - placed[4].0 - step).abs() < 0.01);
}

#[test]
fn the_app_s_range_one_value_none_and_values_about_0() {
  // A range of the app's: 0 to 200 puts 144 under the top.
  let mut stage = stage_of(|| Sparkline::new(FRAMES).min(0).max(200));
  let placed = points(&mut stage, "spark");
  let inset = 1.75 / 2.0;
  let expected = inset + (28.0 - 2.0 * inset) * (1.0 - 144.0 / 200.0);
  assert!((placed[7].1 - expected).abs() < 0.01, "{}", placed[7].1);

  // One value: a point in the middle.
  let mut stage = stage_of(|| Sparkline::new([42.0]));
  assert_eq!(points(&mut stage, "spark"), [(40.0, 14.0)]);
  // None: nothing.
  let mut stage = stage_of(|| Sparkline::new(Vec::<f64>::new()));
  assert!(points(&mut stage, "spark").is_empty());
  // All the same: across the middle.
  let mut stage = stage_of(|| Sparkline::new([5.0, 5.0, 5.0]));
  assert!(points(&mut stage, "spark").iter().all(|(_, y)| *y == 14.0));
  // Both sides of 0: 0 is between the least at the bottom and the greatest at the top.
  let mut stage = stage_of(|| Sparkline::new([-10.0, 0.0, 30.0]));
  let placed = points(&mut stage, "spark");
  assert!(placed[0].1 > placed[1].1 && placed[1].1 > placed[2].1);
  let zero = inset + (28.0 - 2.0 * inset) * (1.0 - 10.0 / 40.0);
  assert!((placed[1].1 - zero).abs() < 0.01);
}

#[test]
fn its_size_comes_from_its_props_and_its_recipe() {
  let mut stage = stage_of(|| Sparkline::new(FRAMES).width(160.0).height(40.0));
  assert_eq!((stage.bounds("spark").2, stage.bounds("spark").3), (160.0, 40.0));
  let last = *points(&mut stage, "spark").last().unwrap();
  assert!((last.0 - (160.0 - 1.75 / 2.0)).abs() < 0.01);
  let mut stage = stage_of(|| Sparkline::new(FRAMES).style(SparklinePart::Line, |s| s.width(4.0)));
  assert!(
    (points(&mut stage, "spark")[7].1 - 2.0).abs() < 0.01,
    "a line of 4 is 2 in"
  );
}

/// A sparkline is a picture to tooling, called by its label and described by what its values are.
#[cfg(any(feature = "mcp", feature = "devtools"))]
#[test]
fn a_sparkline_is_a_picture_with_its_words() {
  let mut stage = stage_of(|| Sparkline::new(FRAMES).label("Frames per second, the last minute"));
  let mut said = |name: &str| {
    stage
      .tree
      .get_element_by_id_mut("spark")
      .and_then(|element| element.description(name))
  };
  assert_eq!(said("a11y_role").as_deref(), Some("img"));
  assert_eq!(said("a11y_name").as_deref(), Some("Frames per second, the last minute"));
  assert_eq!(
    said("a11y_description").as_deref(),
    Some("8 values from 118 to 144, the last 144")
  );
  let mut empty = stage_of(|| Sparkline::new(Vec::<f64>::new()).empty_text("Nothing yet"));
  let said = empty
    .tree
    .get_element_by_id_mut("spark")
    .and_then(|element| element.description("a11y_description"));
  assert_eq!(said.as_deref(), Some("Nothing yet"));
}
