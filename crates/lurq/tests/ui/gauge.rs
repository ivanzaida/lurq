//! A gauge: the board's half ring of 140 by 72, the value's share told by the point at its end, its reading, its
//! range and its label, a range of the app's, the tone of a part, a value outside the range, and what tooling reads.

use lurq::{
  app::theme::{PaletteColor, ThemePalette},
  canvas::CanvasItemShape,
  components::Column,
  layout::text_style::FontWeight,
  node::color::Color,
  ui::Gauge,
};

use super::stage::Stage;

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

/// Where the value's share of the ring ends, from the top left of the ring's canvas.
fn end_of_value(stage: &mut Stage) -> (f32, f32) {
  let canvas = stage
    .tree
    .find_element(|element| element.as_canvas().is_some())
    .and_then(|element| element.as_canvas())
    .expect("the ring is a canvas");
  match canvas.items()[0].shape {
    CanvasItemShape::Point { x, y, .. } => (x, y),
    _ => panic!("a point"),
  }
}

fn stage_of(gauge: impl Fn() -> Gauge + Send + Sync + 'static) -> Stage {
  Stage::new(move |_| Column::new().child(gauge().id("gauge")).into())
}

/// The ring's middle line: its centre at the middle of the canvas's foot, 70 - 15.4 / 2 from it.
const RADIUS: f32 = 70.0 - 15.4 / 2.0;

#[test]
fn the_board_s_ring_its_reading_and_its_range() {
  let mut stage = stage_of(|| Gauge::new(70).label("CPU"));
  let (x, y, width, _) = stage.bounds("gauge");
  assert_eq!(width, 140.0);
  // 70% of the half ring from its start, the left: 126 degrees round.
  let (end_x, end_y) = end_of_value(&mut stage);
  let angle = std::f32::consts::PI * 1.7;
  assert!((end_x - (70.0 + RADIUS * angle.cos())).abs() < 0.01);
  assert!((end_y - (70.0 + RADIUS * angle.sin())).abs() < 0.01);
  // The reading in the ring, 20 at 600; the bounds and the label 4 under it.
  assert_eq!(stage.text("70%"), (light(PaletteColor::Foreground), 20.0));
  let reading = stage.text_quad("70%").unwrap();
  assert!(
    (reading.x + reading.width / 2.0 - (x + 70.0)).abs() < 1.0,
    "in the middle"
  );
  assert!(
    (reading.y + reading.height - (y + 62.0)).abs() < 2.0,
    "its foot 10 over the ring's: {} {}",
    reading.y - y,
    reading.height
  );
  let weight = stage
    .quads()
    .iter()
    .find_map(|quad| match &quad.content {
      lurq::layout::quad::QuadContent::Text { text, style, .. } if text == "70%" => Some(style.weight),
      _ => None,
    })
    .unwrap();
  assert_eq!(weight, FontWeight::SemiBold);
  let (low, label, high) = (
    stage.text_quad("0").unwrap(),
    stage.text_quad("CPU").unwrap(),
    stage.text_quad("100").unwrap(),
  );
  assert!((low.x - x).abs() < 0.5 && (high.x + high.width - (x + 140.0)).abs() < 1.0);
  assert!(low.y >= y + 72.0 + 3.0, "under the ring");
  assert_eq!(stage.text("0"), (light(PaletteColor::MutedForeground), 11.0));
  assert_eq!(stage.text("CPU"), (light(PaletteColor::Foreground), 12.0));
  assert!(label.x > low.x + low.width && label.x + label.width < high.x);
}

#[test]
fn a_range_of_the_app_s_and_values_outside_it() {
  // 60 of 20 to 100 is half the ring: its end at the top.
  let mut stage = stage_of(|| {
    Gauge::new(60)
      .min(20)
      .max(100)
      .value_text(|degrees| format!("{degrees}°"))
  });
  let (end_x, end_y) = end_of_value(&mut stage);
  assert!((end_x - 70.0).abs() < 0.01 && (end_y - (70.0 - RADIUS)).abs() < 0.01);
  assert!(stage.paints("60°") && stage.paints("20") && stage.paints("100"));
  // Past the range: the whole ring, its reading what it is.
  let mut stage = stage_of(|| Gauge::new(130));
  let (end_x, _) = end_of_value(&mut stage);
  assert!((end_x - (70.0 + RADIUS)).abs() < 0.01);
  assert!(stage.paints("130%"));
  // Under it: none of it.
  let mut stage = stage_of(|| Gauge::new(-10));
  let (end_x, _) = end_of_value(&mut stage);
  assert!((end_x - (70.0 - RADIUS)).abs() < 0.01);
  // Bounds as the app writes them.
  let stage = stage_of(|| Gauge::new(5).max(10).bound_text(|value| format!("{value} GB")));
  assert!(stage.paints("0 GB") && stage.paints("10 GB"));
}

#[cfg(any(feature = "mcp", feature = "devtools"))]
#[test]
fn a_gauge_tells_tooling_its_value() {
  let mut stage = stage_of(|| Gauge::new(70).label("CPU"));
  let mut said = |name: &str| {
    stage
      .tree
      .get_element_by_id_mut("gauge")
      .and_then(|element| element.description(name))
  };
  assert_eq!(said("a11y_role").as_deref(), Some("meter"));
  assert_eq!(said("a11y_name").as_deref(), Some("CPU"));
  assert_eq!(said("a11y_valuenow").as_deref(), Some("70"));
  assert_eq!(said("a11y_valuemin").as_deref(), Some("0"));
  assert_eq!(said("a11y_valuemax").as_deref(), Some("100"));
  assert_eq!(said("a11y_valuetext").as_deref(), Some("70%"));
}
