//! A meter: the board's segments in the tones of their parts of the range, the value written, the description in
//! the tone of a warning, an optimum at the top, one tone without parts, a value outside the range, what tooling
//! reads, and a theme's rules and renderer.

use std::sync::Arc;

use lurq::{
  app::theme::{PaletteColor, ThemePalette},
  components::{Column, Text},
  layout::quad::{Quad, QuadContent},
  node::color::Color,
  ui::{Meter, MeterPart, MeterRenderer, MeterVariant, MeterView},
};

use super::stage::Stage;

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

/// The segments as painted, from the start: 8 high.
fn segments(stage: &Stage) -> Vec<Quad> {
  let mut segments: Vec<Quad> = stage
    .quads()
    .into_iter()
    .filter(|quad| matches!(&quad.content, QuadContent::Rect { color, .. } if color.a() > 0) && quad.height == 8.0)
    .collect();
  segments.sort_by(|a, b| a.x.total_cmp(&b.x));
  segments
}

fn colors(stage: &Stage) -> Vec<Color> {
  segments(stage)
    .iter()
    .map(|quad| match quad.content {
      QuadContent::Rect { color, .. } => color,
      _ => unreachable!(),
    })
    .collect()
}

/// The board's meter: 96 of 128 GB of a disk, good under 64, a warning to 96.
fn board() -> Meter {
  Meter::new(96)
    .max(128)
    .low(64)
    .high(96)
    .label("Disk space")
    .value_text(|used| format!("{used} of 128 GB"))
    .description("Running low. Free up space soon.")
    .id("meter")
}

fn stage_of(meter: impl Fn() -> Meter + Send + Sync + 'static) -> Stage {
  Stage::new(move |_| Column::new().width(260.0).child(meter()).into())
}

#[test]
fn sixteen_segments_in_the_tones_of_their_parts() {
  let mut stage = stage_of(board);
  let (x, y, width, _) = stage.bounds("meter");
  assert_eq!(width, 260.0);
  let quads = segments(&stage);
  assert_eq!(quads.len(), 16);
  // 2 apart, sharing the width, 2 round.
  assert!((quads[0].x - x).abs() < 0.5);
  let share = (260.0 - 15.0 * 2.0) / 16.0;
  assert!((quads[0].width - share).abs() < 0.5, "{}", quads[0].width);
  assert!((quads[1].x - (quads[0].x + quads[0].width + 2.0)).abs() < 0.5);
  // 8 good, 4 a warning (64 to 96), the rest empty.
  let tones = colors(&stage);
  assert!(tones[..8].iter().all(|color| *color == light(PaletteColor::Success)));
  assert!(tones[8..12].iter().all(|color| *color == light(PaletteColor::Warning)));
  assert!(tones[12..].iter().all(|color| *color == light(PaletteColor::Secondary)));

  // The label and the value over them, the description under them in the warning's tone.
  assert_eq!(stage.text("Disk space"), (light(PaletteColor::Foreground), 13.0));
  assert_eq!(stage.text("96 of 128 GB"), (light(PaletteColor::MutedForeground), 11.0));
  let value = stage.text_quad("96 of 128 GB").unwrap();
  assert!(
    (value.x + value.width - (x + 260.0)).abs() < 1.0,
    "the value at the end"
  );
  assert_eq!(
    stage.text("Running low. Free up space soon."),
    (light(PaletteColor::Warning), 12.0)
  );
  let label = stage.text_quad("Disk space").unwrap();
  assert!(label.y < quads[0].y && (quads[0].y - (label.y + label.height) - 8.0).abs() < 3.0);
  let _ = y;
}

#[test]
fn the_optimum_decides_which_part_is_good() {
  // A battery: the higher the better. 18 is under 20: bad.
  let stage = stage_of(|| {
    Meter::new(18)
      .low(20)
      .high(80)
      .optimum(100)
      .label("Battery")
      .description("Plug in soon.")
  });
  let tones = colors(&stage);
  assert_eq!(
    tones[0],
    light(PaletteColor::Destructive),
    "the low part is the far one"
  );
  assert_eq!(tones[3], light(PaletteColor::Secondary), "18 of 100 fills 3 of 16");
  assert_eq!(stage.text("Plug in soon.").0, light(PaletteColor::Destructive));
  assert!(stage.paints("18 of 100"), "the value written as it is");

  // Full: good at the top, a warning in the middle, bad at the bottom.
  let stage = stage_of(|| Meter::new(100).low(20).high(80).optimum(100));
  let tones = colors(&stage);
  assert_eq!(tones[0], light(PaletteColor::Destructive));
  assert_eq!(tones[8], light(PaletteColor::Warning));
  assert_eq!(tones[15], light(PaletteColor::Success));

  // A good value says its description in the muted color.
  let stage = stage_of(|| Meter::new(10).low(64).high(96).max(128).description("Plenty of room."));
  assert_eq!(stage.text("Plenty of room.").0, light(PaletteColor::MutedForeground));
}

#[test]
fn one_tone_without_parts_and_values_outside_the_range() {
  let stage = stage_of(|| Meter::new(0.5).max(1).segments(4).variant(MeterVariant::Info));
  let tones = colors(&stage);
  assert_eq!(tones.len(), 4);
  assert_eq!(tones[..2], [light(PaletteColor::Info); 2]);
  assert_eq!(tones[2..], [light(PaletteColor::Secondary); 2]);
  assert!(stage.paints("0.5 of 1"));
  let stage = stage_of(|| Meter::new(40).segments(5));
  assert_eq!(
    colors(&stage)[..2],
    [light(PaletteColor::Primary); 2],
    "neutral is the primary color"
  );

  // Over the range all are filled, under it none; the text says the value.
  let stage = stage_of(|| Meter::new(150).segments(8));
  assert!(
    colors(&stage)
      .iter()
      .all(|color| *color == light(PaletteColor::Primary))
  );
  assert!(stage.paints("150 of 100"));
  let stage = stage_of(|| Meter::new(-5).segments(8));
  assert!(
    colors(&stage)
      .iter()
      .all(|color| *color == light(PaletteColor::Secondary))
  );
}

#[test]
fn a_rule_and_a_renderer_of_the_theme_change_it() {
  let stage = stage_of(|| board().style(MeterPart::Segment, |s| s.height(8.0).radius(0.0)));
  assert!(
    segments(&stage)
      .iter()
      .all(|quad| quad.border_radius.is_none_or(|radius| radius.top_left == 0.0))
  );
  let app = lurq::app::App::new();
  let renderer: MeterRenderer = Arc::new(|view: MeterView| Text::new(&format!("meter {}", view.value_text)).into());
  app.theme().set_renderer::<Meter>(renderer);
  let stage = Stage::in_app(app, |_| board().into());
  assert!(stage.paints("meter 96 of 128 GB"));
}

/// A meter is a meter to tooling: its name, its value, its range and its text.
#[cfg(any(feature = "mcp", feature = "devtools"))]
#[test]
fn a_meter_tells_tooling_its_value() {
  let mut stage = stage_of(board);
  let mut said = |name: &str| {
    stage
      .tree
      .get_element_by_id_mut("meter")
      .and_then(|element| element.description(name))
  };
  assert_eq!(said("a11y_role").as_deref(), Some("meter"));
  assert_eq!(said("a11y_name").as_deref(), Some("Disk space"));
  assert_eq!(said("a11y_valuenow").as_deref(), Some("96"));
  assert_eq!(said("a11y_valuemin").as_deref(), Some("0"));
  assert_eq!(said("a11y_valuemax").as_deref(), Some("128"));
  assert_eq!(said("a11y_valuetext").as_deref(), Some("96 of 128 GB"));
}
