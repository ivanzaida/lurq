//! A progress: its rail and what is filled of it, its variants and sizes, its
//! label and value, the fill's move to a new value, the segment of one that
//! does not know how far it is and what reduced motion makes of it, the top
//! bar, the ring, and a theme's other anatomy.

use std::{sync::Arc, time::Duration};

use lurq::{
  app::{
    App,
    theme::{PaletteColor, ThemePalette},
  },
  components::Column,
  core::Signal,
  layout::{
    quad::{Quad, QuadContent},
    render_list::StrokeStyle,
    stroke::{StrokeShape, stroke_coverage},
  },
  node::color::Color,
  ui::{
    PROGRESS_PERIOD, Progress, ProgressPart, ProgressRenderer, ProgressSize, ProgressVariant, ProgressView,
    RingProgress,
  },
};

use super::stage::Stage;

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

/// The filled rectangles of `color` inside the box of the element with `id`, from the start of it.
fn filled(stage: &mut Stage, id: &str, color: Color) -> Vec<Quad> {
  let (x, y, width, height) = stage.bounds(id);
  stage
    .quads()
    .into_iter()
    .filter(|quad| {
      matches!(&quad.content, QuadContent::Rect { color: fill, gradient: None } if *fill == color)
        && quad.width > 0.0
        && quad.y >= y - 0.01
        && quad.y + quad.height <= y + height + 0.01
        && quad.x >= x - 0.01
        && quad.x <= x + width + 0.01
    })
    .collect()
}

/// What is filled of the rail of the progress with `id`: the one rectangle in the primary color, or in `color`.
fn fill(stage: &mut Stage, id: &str, color: Color) -> Quad {
  let mut quads = filled(stage, id, color);
  assert_eq!(quads.len(), 1, "#{id} should have one fill");
  quads.remove(0)
}

fn near(a: f32, b: f32) -> bool {
  (a - b).abs() < 0.01
}

#[test]
fn a_progress_is_a_rail_filled_as_far_as_the_work_is() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .padding(8.0)
      .spacing(12.0)
      .child(Progress::new(0.6).id("sixty"))
      .child(Progress::new(0.25).id("quarter"))
      .into()
  });
  let (x, _, width, height) = stage.bounds("sixty");
  assert_eq!((width, height), (240.0, 8.0));
  // The rail is the whole of the progress, round, in the secondary color.
  let rail = stage.fill("sixty").expect("a rail");
  assert!(matches!(rail.content, QuadContent::Rect { color, .. } if color == light(PaletteColor::Secondary)));
  assert!(rail.border_radius.is_some_and(|radius| radius.top_left >= 4.0), "round");

  let done = fill(&mut stage, "sixty", light(PaletteColor::Primary));
  assert_eq!(done.x, x, "from the start of the rail");
  assert!(near(done.width, 144.0), "60% of 240: {}", done.width);
  assert_eq!(done.height, 8.0);
  assert!(
    done.border_radius.is_some_and(|radius| radius.top_right >= 4.0),
    "round"
  );
  assert!(near(
    fill(&mut stage, "quarter", light(PaletteColor::Primary)).width,
    60.0
  ));
}

#[test]
fn a_value_of_another_range_is_its_share_of_the_greatest_and_is_kept_in_it() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .padding(8.0)
      .spacing(12.0)
      .child(Progress::new(32).max(128).id("quarter"))
      .child(Progress::new(1.5).id("over"))
      .child(Progress::new(-0.5).id("under"))
      .child(Progress::new(3).max(0).id("no room"))
      .into()
  });
  let primary = light(PaletteColor::Primary);
  assert!(near(fill(&mut stage, "quarter", primary).width, 60.0));
  assert!(
    near(fill(&mut stage, "over", primary).width, 240.0),
    "no further than the end"
  );
  assert!(filled(&mut stage, "under", primary).is_empty(), "nothing is done");
  assert!(filled(&mut stage, "no room", primary).is_empty(), "nothing is done");
}

#[test]
fn a_variant_colors_what_is_filled_and_a_size_is_the_rails_height() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .padding(8.0)
      .spacing(12.0)
      .child(Progress::new(0.8).variant(ProgressVariant::Success).id("success"))
      .child(Progress::new(0.6).variant(ProgressVariant::Warning).id("warning"))
      .child(
        Progress::new(0.3)
          .variant(ProgressVariant::Destructive)
          .id("destructive"),
      )
      .child(Progress::new(0.5).size(ProgressSize::Sm).id("sm"))
      .child(Progress::new(0.5).size(ProgressSize::Md).id("md"))
      .child(Progress::new(0.5).size(ProgressSize::Lg).id("lg"))
      .into()
  });
  for (id, role) in [
    ("success", PaletteColor::Success),
    ("warning", PaletteColor::Warning),
    ("destructive", PaletteColor::Destructive),
  ] {
    fill(&mut stage, id, light(role));
    assert_eq!(
      stage.fill_color(id),
      Some(light(PaletteColor::Secondary)),
      "{id}: the rail is the same"
    );
  }
  for (id, height) in [("sm", 4.0), ("md", 6.0), ("lg", 8.0)] {
    assert_eq!(stage.bounds(id).3, height, "{id}");
    assert_eq!(
      fill(&mut stage, id, light(PaletteColor::Primary)).height,
      height,
      "{id}"
    );
  }
}

#[test]
fn the_label_is_over_the_start_of_the_rail_and_the_value_over_its_end() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .padding(8.0)
      .spacing(12.0)
      .child(
        Progress::new(0.62)
          .label("Uploading theme.json")
          .show_value(true)
          .size(ProgressSize::Md)
          .id("upload"),
      )
      .child(
        Progress::new(42)
          .max(128)
          .show_value(true)
          .format(|done, all| format!("{done} of {all}"))
          .id("copy"),
      )
      .child(Progress::new(0.4).label("Hidden value").id("quiet"))
      .into()
  });
  let (x, y, width, height) = stage.bounds("upload");
  let label = stage.text_quad("Uploading theme.json").unwrap();
  assert!(
    (label.x - x).abs() < 0.5 && (label.y - y).abs() < 0.5,
    "the label at the start, at the top"
  );
  assert_eq!(
    stage.text("Uploading theme.json"),
    (light(PaletteColor::Foreground), 13.0)
  );
  let value = stage.text_quad("62%").unwrap();
  assert!(
    (value.x + value.width - (x + width)).abs() < 1.0,
    "the value at the end"
  );
  assert_eq!(stage.text("62%"), (light(PaletteColor::MutedForeground), 11.0));
  // The rail is 8 under the header, at the bottom of the progress.
  let rail = fill(&mut stage, "upload", light(PaletteColor::Primary));
  assert_eq!(rail.y + rail.height, y + height);
  assert!(rail.y - (label.y + label.height) >= 8.0 - 0.01);

  assert!(stage.paints("42 of 128"), "the app's format");
  // Only the value it is given is said.
  assert!(stage.paints("Hidden value"));
  assert!(!stage.paints("40%"));
}

#[test]
fn what_is_filled_moves_to_a_new_value() {
  let value = Signal::new(0.2_f32);
  let read = value.clone();
  let mut stage = Stage::new(move |_| {
    Column::new()
      .padding(8.0)
      .child(Progress::new(read.get()).id("bar"))
      .into()
  });
  let primary = light(PaletteColor::Primary);
  assert!(near(fill(&mut stage, "bar", primary).width, 48.0));
  value.set(0.8);
  stage.pass_after(Duration::ZERO);
  stage.pass_after(Duration::from_millis(60));
  let on_the_way = fill(&mut stage, "bar", primary).width;
  assert!(on_the_way > 48.5 && on_the_way < 191.5, "on the way: {on_the_way}");
  stage.pass_after(Duration::from_secs(1));
  assert!(near(fill(&mut stage, "bar", primary).width, 192.0));
}

#[test]
fn a_progress_that_does_not_know_runs_a_segment_across_its_rail() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .padding(8.0)
      .child(Progress::indeterminate().label("Connecting").show_value(true).id("bar"))
      .into()
  });
  let primary = light(PaletteColor::Primary);
  let (x, _, width, _) = stage.bounds("bar");
  let mut starts = Vec::new();
  for step in 0..4 {
    stage.pass_after(PROGRESS_PERIOD / 4 * step + Duration::from_millis(100));
    for segment in filled(&mut stage, "bar", primary) {
      assert!(
        segment.width <= 90.0 + 0.01,
        "no wider than 90 of 240: {}",
        segment.width
      );
      starts.push(segment.x - x);
    }
  }
  assert!(
    starts.iter().any(|start| (start - starts[0]).abs() > 1.0),
    "it moves: {starts:?}"
  );
  assert!(starts.iter().all(|start| *start >= -0.01 && *start <= width));
  // No value is said while it is not known.
  assert!(stage.paints("Connecting"));
  assert!(!stage.paints("0%"));
}

#[test]
fn reduced_motion_holds_the_segment_still_away_from_the_start() {
  let app = App::new();
  app.theme().set_reduced_motion(Some(true));
  let mut stage = Stage::in_app(app, |_| {
    Column::new()
      .padding(8.0)
      .child(Progress::indeterminate().id("bar"))
      .into()
  });
  let primary = light(PaletteColor::Primary);
  let (x, ..) = stage.bounds("bar");
  let still = fill(&mut stage, "bar", primary);
  assert!(near(still.x - x, 70.0), "70 of 240 from the start: {}", still.x - x);
  assert!(near(still.width, 90.0));
  stage.pass_after(PROGRESS_PERIOD / 3);
  let later = fill(&mut stage, "bar", primary);
  assert_eq!((later.x, later.width), (still.x, still.width), "it does not move");
}

#[test]
fn a_top_bar_is_thin_and_as_wide_as_what_holds_it_with_no_rail() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .width(360.0)
      .child(Progress::new(0.6).top_bar(true).id("bar"))
      .into()
  });
  let (_, _, width, height) = stage.bounds("bar");
  assert_eq!((width, height), (360.0, 3.0));
  assert!(stage.fill("bar").is_none(), "no rail");
  let done = fill(&mut stage, "bar", light(PaletteColor::Primary));
  assert!(near(done.width, 216.0));
  assert!(done.border_radius.is_some_and(|radius| near(radius.top_right, 2.0)));
}

/// The quads with a border in the box of the ring with `id`: the track's and the arc's, as painted.
fn rings(stage: &mut Stage, id: &str) -> Vec<Quad> {
  let (x, y, width, height) = stage.bounds(id);
  stage
    .quads()
    .into_iter()
    .filter(|quad| quad.border.is_some() && (quad.x, quad.y, quad.width, quad.height) == (x, y, width, height))
    .collect()
}

/// The color, the width and the dash of the border of `quad`.
fn stroke(quad: &Quad) -> (Color, f32, [f32; 2]) {
  let side = quad.border.and_then(|border| border.top).expect("a border on top");
  (side.color, side.width, side.dash)
}

/// The arc of the ring with `id`: the border in `color`.
fn arc(stage: &mut Stage, id: &str, color: Color) -> (f32, [f32; 2]) {
  let quads = rings(stage, id);
  let arc = quads
    .iter()
    .map(stroke)
    .find(|(painted, ..)| *painted == color)
    .unwrap_or_else(|| panic!("#{id} should have an arc"));
  (arc.1, arc.2)
}

#[test]
fn a_ring_is_a_track_with_an_arc_of_one_dash_over_it_and_nothing_in_the_middle() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .padding(8.0)
      .spacing(12.0)
      .child(RingProgress::new(0.7).id("ring"))
      .child(RingProgress::new(0.5).size(ProgressSize::Sm).id("sm"))
      .child(RingProgress::new(0.5).size(ProgressSize::Md).show_value(false).id("md"))
      .child(RingProgress::new(1.0).variant(ProgressVariant::Success).id("done"))
      .child(RingProgress::new(0.0).id("none"))
      .into()
  });
  let (x, y, width, height) = stage.bounds("ring");
  assert_eq!((width, height), (72.0, 72.0));
  // The track: an inside border a tenth of the side wide, in the secondary color, all the way round.
  let quads = rings(&mut stage, "ring");
  assert_eq!(quads.len(), 2, "the track and the arc");
  assert!(
    quads
      .iter()
      .all(|quad| quad.border_radius.is_some_and(|radius| radius.top_left >= 36.0))
  );
  let (color, ring_width, dash) = stroke(&quads[0]);
  assert_eq!((color, dash), (light(PaletteColor::Secondary), [0.0, 0.0]));
  assert!(near(ring_width, 7.2));
  // The arc over it: as wide, in the primary color, one dash of 70% of the circle and a gap of the rest.
  let circle = std::f32::consts::PI * 72.0;
  let (arc_width, [dash, gap]) = arc(&mut stage, "ring", light(PaletteColor::Primary));
  assert!(near(arc_width, 7.2));
  assert!(
    (dash - circle * 0.7).abs() < 0.01 && (gap - circle * 0.3).abs() < 0.01,
    "{dash} {gap}"
  );
  // Nothing is painted in the middle: the track and the arc fill nothing.
  assert!(
    stage.quads().iter().all(|quad| {
      let inside = quad.x >= x - 0.01 && quad.y >= y - 0.01 && quad.x + quad.width <= x + 72.01;
      !inside
        || match &quad.content {
          QuadContent::Rect { color, gradient } => color.a() == 0 && gradient.is_none(),
          _ => true,
        }
    }),
    "nothing filled in the ring's box"
  );
  assert_eq!(stage.text("70%").1, 13.0);
  let value = stage.text_quad("70%").unwrap();
  assert!(
    ((value.x + value.width / 2.0) - (x + 36.0)).abs() < 1.0,
    "in the middle"
  );

  // The smaller rings: a ring a tenth of the side, the value in a smaller type or not at all.
  assert_eq!(stage.bounds("sm").2, 40.0);
  assert_eq!(stage.bounds("md").2, 56.0);
  assert!(near(arc(&mut stage, "sm", light(PaletteColor::Primary)).0, 4.0));
  let (_, [dash, gap]) = arc(&mut stage, "md", light(PaletteColor::Primary));
  assert!((dash - std::f32::consts::PI * 28.0).abs() < 0.01 && (gap - dash).abs() < 0.01);
  assert_eq!(
    stage.text("50%").1,
    11.0,
    "the small ring's value; the other shows none"
  );
  // A whole turn is a dash with no gap: a solid line, in the variant's color.
  let (_, [_, gap]) = arc(&mut stage, "done", light(PaletteColor::Success));
  assert_eq!(gap, 0.0);
  // Nothing done: the arc has no color.
  assert!(
    rings(&mut stage, "none")
      .iter()
      .map(stroke)
      .all(|(color, ..)| color == light(PaletteColor::Secondary) || color.a() == 0)
  );
}

/// Whether the dashed border of a ring of `side`, `fraction` of the way round, covers the ring from the top and
/// clockwise as far as the fraction and nowhere else, as the renderers paint it: one dash, starting at 12 o'clock.
fn one_dash_from_the_top(side: f32, fraction: f32) {
  let half = side / 2.0;
  let width = side / 10.0;
  let circle = std::f32::consts::PI * side;
  let dash = circle * fraction;
  let shape = StrokeShape {
    half: [half; 2],
    radii: [half; 4],
    stroke: [width; 4],
    style: StrokeStyle {
      dash: [dash, circle - dash],
      radii_at: 0.0,
    },
  };
  // Along the middle of the ring, a sample every half degree; the ends of the dash have a ramp of a pixel.
  let middle = half - width / 2.0;
  let ramp = 1.5 / circle;
  for step in 0..720 {
    let turn = step as f32 / 720.0;
    let angle = turn * std::f32::consts::TAU;
    let coverage = stroke_coverage(middle * angle.sin(), -middle * angle.cos(), &shape);
    let near_an_end = turn.min(1.0 - turn) < ramp || (turn - fraction).abs() < ramp;
    if near_an_end && fraction < 1.0 {
      continue;
    }
    if turn < fraction {
      assert!(
        coverage > 0.99,
        "side {side}, {fraction}: covered at {turn}: {coverage}"
      );
    } else {
      assert!(coverage < 0.01, "side {side}, {fraction}: bare at {turn}: {coverage}");
    }
  }
}

#[test]
fn the_arcs_dash_starts_at_the_top_and_is_the_only_one_at_every_value_and_size() {
  for side in [40.0, 56.0, 72.0] {
    for fraction in [0.02, 0.1, 0.25, 0.4999, 0.5, 0.5001, 0.7, 0.9, 0.98, 1.0] {
      one_dash_from_the_top(side, fraction);
    }
  }
}

#[test]
fn the_arc_moves_round_to_a_new_value() {
  let value = Signal::new(0.2_f32);
  let read = value.clone();
  let mut stage = Stage::new(move |_| {
    Column::new()
      .padding(8.0)
      .child(RingProgress::new(read.get()).id("ring"))
      .into()
  });
  let primary = light(PaletteColor::Primary);
  let circle = std::f32::consts::PI * 72.0;
  let (_, [dash, _]) = arc(&mut stage, "ring", primary);
  assert!((dash - circle * 0.2).abs() < 0.01);
  value.set(0.8);
  stage.pass_after(Duration::ZERO);
  stage.pass_after(Duration::from_millis(60));
  let (_, [dash, gap]) = arc(&mut stage, "ring", primary);
  assert!(
    dash > circle * 0.2 + 0.5 && dash < circle * 0.8 - 0.5,
    "on the way: {dash}"
  );
  assert!((dash + gap - circle).abs() < 0.01, "one dash all the way");
  stage.pass_after(Duration::from_secs(1));
  let (_, [dash, _]) = arc(&mut stage, "ring", primary);
  assert!((dash - circle * 0.8).abs() < 0.01);
}

#[test]
fn a_whole_ring_goes_back_as_one_dash() {
  let value = Signal::new(1.0_f32);
  let read = value.clone();
  let mut stage = Stage::new(move |_| {
    Column::new()
      .padding(8.0)
      .child(RingProgress::new(read.get()).id("ring"))
      .into()
  });
  let primary = light(PaletteColor::Primary);
  let circle = std::f32::consts::PI * 72.0;
  value.set(0.5);
  stage.pass_after(Duration::ZERO);
  stage.pass_after(Duration::from_millis(60));
  let (_, [dash, gap]) = arc(&mut stage, "ring", primary);
  assert!(dash > circle * 0.5 + 0.5 && dash < circle - 0.5, "on the way: {dash}");
  assert!((dash + gap - circle).abs() < 0.01, "one dash, not many: {dash} {gap}");
}

#[test]
fn reduced_motion_moves_the_arc_at_once() {
  let app = App::new();
  app.theme().set_reduced_motion(Some(true));
  let value = Signal::new(0.2_f32);
  let read = value.clone();
  let mut stage = Stage::in_app(app, move |_| {
    Column::new()
      .padding(8.0)
      .child(RingProgress::new(read.get()).id("ring"))
      .into()
  });
  value.set(0.8);
  stage.pass_after(Duration::ZERO);
  stage.pass_after(Duration::from_millis(60));
  let (_, [dash, _]) = arc(&mut stage, "ring", light(PaletteColor::Primary));
  assert!((dash - std::f32::consts::PI * 72.0 * 0.8).abs() < 0.01, "{dash}");
}

#[test]
fn a_theme_gives_the_progress_another_anatomy() {
  let app = App::new();
  // The value said inside a thick rail: other nodes, the same recipe.
  let renderer: ProgressRenderer = Arc::new(|view: ProgressView| {
    let text = view.value_text().unwrap_or_default();
    let part = view.part(ProgressPart::Value);
    view.root().child(view.track().child(part.text(&text))).into()
  });
  app.theme().set_renderer::<Progress>(renderer);
  let mut stage = Stage::in_app(app, |_| {
    Column::new()
      .padding(8.0)
      .child(
        Progress::new(0.3)
          .style(ProgressPart::Track, |s| s.height(20.0))
          .id("bar"),
      )
      .into()
  });
  assert!(stage.paints("30%"));
  assert_eq!(stage.bounds("bar").3, 20.0, "the rules of the place");
  assert_eq!(
    stage.fill_color("bar"),
    Some(light(PaletteColor::Secondary)),
    "the recipe still styles it"
  );
}
