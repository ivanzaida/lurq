//! A line chart and an area chart: the board's Line Chart (its frame of `Sm`, its plot of 140 with the lines across
//! it, two lines from the plot's start to its end, the legend), the place under the pointer with its cursor, its marks
//! and its tooltip beside it, the keys, gaps, a smooth curve, ten thousand values, negative values, one value, a
//! hidden series, no data and loading; the area under a line as the board fills it, and stacked areas.

use std::{
  sync::{Arc, Mutex},
  time::Instant,
};

use lurq::{
  app::{
    events::MouseButton,
    theme::{PaletteColor, ThemePalette},
  },
  canvas::{CanvasItem, CanvasItemShape},
  components::Column,
  core::Signal,
  layout::{Constraints, Size, quad::QuadContent},
  node::color::Color,
  ui::{AreaChart, ChartPoint, ChartSeries, LineChart, LineCurve},
};

use super::stage::Stage;

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

const BUILDS: [&str; 7] = ["0.54", "0.55", "0.56", "0.57", "0.58", "0.59", "0.60"];

fn wgpu() -> ChartSeries {
  ChartSeries::new("wgpu", [10.4, 9.6, 9.9, 8.2, 8.6, 6.9, 6.3])
}

fn directx() -> ChartSeries {
  ChartSeries::new("DirectX 12", [11.6, 11.2, 10.3, 10.6, 9.4, 9.4, 8.6])
}

fn ms(value: f64) -> String {
  format!("{} ms", lurq::ui::default_value_text(value))
}

/// The board's line chart, 420 wide, mounted.
fn board(selected: Arc<Mutex<Vec<ChartPoint>>>) -> Stage {
  stage(move || {
    let selected = selected.clone();
    LineChart::new(BUILDS, [wgpu(), directx()])
      .title("Frame time")
      .description("Milliseconds per frame, last 7 builds")
      .value_text(ms)
      .on_select(move |point| selected.lock().unwrap().push(point))
      .id("c")
  })
}

fn stage<C: Into<lurq::node::Element>>(chart: impl Fn() -> C + Send + Sync + 'static) -> Stage
where
  C: Mountable,
{
  let mut stage = Stage::software(move |ctx| Column::new().size(420.0, 600.0).child(chart().mount_in(ctx)).into());
  stage
    .tree
    .set_layout_constraints_override(Some(Constraints::loose(Size::new(800.0, 700.0))));
  stage.pass();
  stage
}

/// A chart the stage mounts.
trait Mountable {
  fn mount_in(self, ctx: &mut lurq::app::ctx::Ctx) -> lurq::node::Element;
}

impl Mountable for LineChart {
  fn mount_in(self, ctx: &mut lurq::app::ctx::Ctx) -> lurq::node::Element {
    self.mount(ctx)
  }
}

impl Mountable for AreaChart {
  fn mount_in(self, ctx: &mut lurq::app::ctx::Ctx) -> lurq::node::Element {
    self.mount(ctx)
  }
}

/// The points the plot says it holds, as tooling reads them.
fn points(stage: &mut Stage) -> Vec<CanvasItem> {
  let canvas = stage
    .tree
    .get_element_by_id_mut("c-plot")
    .and_then(|element| element.canvas())
    .expect("the plot is a canvas");
  canvas.items().iter().cloned().collect()
}

fn place(item: &CanvasItem) -> (f32, f32) {
  match item.shape {
    CanvasItemShape::Point { x, y, .. } => (x, y),
    _ => panic!("a value is a point"),
  }
}

/// The color the plot drew at `(x, y)` of its own box.
fn pixel(stage: &mut Stage, x: f32, y: f32) -> Color {
  let canvas = stage
    .tree
    .get_element_by_id_mut("c-plot")
    .and_then(|element| element.canvas())
    .expect("the plot is a canvas");
  let scale = canvas.scale_factor().max(1.0);
  let shot = canvas
    .snapshot()
    .try_take()
    .expect("drawn in software")
    .expect("a snapshot");
  let (px, py) = ((x * scale) as u32, (y * scale) as u32);
  let at = ((py.min(shot.height - 1) * shot.width + px.min(shot.width - 1)) * 4) as usize;
  Color::new(shot.rgba[at], shot.rgba[at + 1], shot.rgba[at + 2], shot.rgba[at + 3])
}

/// Whether `a` and `b` are the same color within a few of each channel.
fn near(a: Color, b: Color) -> bool {
  [(a.r(), b.r()), (a.g(), b.g()), (a.b(), b.b())]
    .into_iter()
    .all(|(a, b)| (a as i32 - b as i32).abs() <= 12)
}

fn texts(stage: &Stage, content: &str) -> usize {
  stage
    .quads()
    .iter()
    .filter(|quad| matches!(&quad.content, QuadContent::Text { text, .. } if text == content))
    .count()
}

fn hover(stage: &mut Stage, index: usize, count: usize, y: f32) {
  let (plot_x, plot_y, width, _) = stage.bounds("c-plot");
  let x = plot_x + width * index as f32 / (count - 1) as f32;
  stage.tree.mouse_move(x, plot_y + y);
  stage.pass();
}

#[test]
fn the_boards_line_chart_is_a_card_of_sm_a_plot_of_140_and_two_lines_from_its_start_to_its_end() {
  let mut stage = board(Arc::default());
  let (x, y, width, _) = stage.bounds("c");
  assert_eq!(width, 420.0);
  assert_eq!(stage.fill_color("c"), Some(light(PaletteColor::Card)));
  assert_eq!(stage.text("Frame time"), (light(PaletteColor::CardForeground), 15.0));
  assert_eq!(
    stage.text("Milliseconds per frame, last 7 builds"),
    (light(PaletteColor::MutedForeground), 12.0)
  );
  let title = stage.text_quad("Frame time").unwrap();
  assert!(
    (title.x - (x + 20.0)).abs() < 1.0 && (title.y - (y + 20.0)).abs() < 4.0,
    "padded 20"
  );
  let (plot_x, _, plot_width, plot_height) = stage.bounds("c-plot");
  assert_eq!((plot_width, plot_height), (380.0, 140.0));
  assert!(
    (plot_x - (x + 20.0)).abs() < 0.5,
    "no numbers beside the lines across it"
  );

  let drawn = points(&mut stage);
  assert_eq!(drawn.len(), 14, "a value of each series in each build");
  let first: Vec<(f32, f32)> = drawn.iter().take(7).map(place).collect();
  for (index, (px, _)) in first.iter().enumerate() {
    assert!(
      (px - 380.0 * index as f32 / 6.0).abs() < 0.01,
      "from the plot's start to its end"
    );
  }
  assert!(first[6].1 > first[0].1, "a value that falls is lower on the plot");
  assert_eq!(drawn[5].label.as_deref(), Some("0.59, wgpu"));
  assert_eq!(drawn[5].value.as_deref(), Some("6.9 ms"));
  // The lines are drawn in their series' colors, the first over the second.
  let (wx, wy) = place(&drawn[3]);
  assert!(near(pixel(&mut stage, wx, wy), light(PaletteColor::Chart1)));
  let (dx, dy) = place(&drawn[7 + 3]);
  assert!(near(pixel(&mut stage, dx, dy), light(PaletteColor::Chart2)));
  // The lines across the plot at its ticks, and the names of the builds under it, the first at its start.
  assert!(stage.text_quad("0.54").unwrap().x - plot_x < 1.0);
  let last = stage.text_quad("0.60").unwrap();
  assert!(
    (last.x + last.width - (plot_x + 380.0)).abs() < 1.0,
    "the last at its end"
  );
  // The legend: a dot and a name for each series.
  assert_eq!(texts(&stage, "wgpu"), 1);
  assert_eq!(stage.text("DirectX 12").0, light(PaletteColor::MutedForeground));
}

#[test]
fn the_place_under_the_pointer_has_its_cursor_its_marks_and_a_tooltip_beside_it() {
  let selected: Arc<Mutex<Vec<ChartPoint>>> = Arc::default();
  let mut stage = board(selected.clone());
  assert!(stage.text_quad("9.4 ms").is_none(), "no tooltip at rest");
  hover(&mut stage, 5, 7, 70.0);
  // Every series' value there.
  assert_eq!(texts(&stage, "6.9 ms"), 1);
  assert_eq!(texts(&stage, "9.4 ms"), 1);
  assert!(stage.text_quad("0.59").is_some());
  let (plot_x, plot_y, ..) = stage.bounds("c-plot");
  let cursor = 380.0 * 5.0 / 6.0;
  let (tip_x, tip_y, tip_width, _) = stage.bounds("c-tooltip");
  assert!(
    (tip_x + tip_width - (plot_x + cursor - 10.0)).abs() < 1.0,
    "10 before the cursor"
  );
  assert!((tip_y - (plot_y + 4.0)).abs() < 1.0, "4 from the plot's top");
  // The cursor across the plot, and a mark of 10 on each line ringed in the card's color.
  assert!(near(pixel(&mut stage, cursor, 135.0), light(PaletteColor::Input)));
  let drawn = points(&mut stage);
  let (mx, my) = place(&drawn[5]);
  assert!(near(pixel(&mut stage, mx, my), light(PaletteColor::Chart1)));
  assert!(
    near(pixel(&mut stage, mx + 4.0, my - 0.5), light(PaletteColor::Card)),
    "the ring"
  );
  // A press tells the series nearest the pointer there.
  let (_, wy) = place(&drawn[5]);
  let (x, y) = (plot_x + cursor, plot_y + wy);
  stage.tree.mouse_move(x, y);
  stage.tree.mouse_down(x, y, MouseButton::Left);
  stage.tree.mouse_up(x, y, MouseButton::Left);
  stage.pass();
  assert_eq!(*selected.lock().unwrap(), [ChartPoint::new(0, 5)]);
}

#[test]
fn a_tooltip_with_no_room_before_the_cursor_stands_after_it() {
  let mut stage = board(Arc::default());
  hover(&mut stage, 1, 7, 70.0);
  let (plot_x, ..) = stage.bounds("c-plot");
  let (tip_x, ..) = stage.bounds("c-tooltip");
  assert!(
    (tip_x - (plot_x + 380.0 / 6.0 + 10.0)).abs() < 1.0,
    "10 after the cursor"
  );
}

#[test]
fn the_plot_is_one_stop_of_tab_whose_arrows_go_between_builds_and_series() {
  let selected: Arc<Mutex<Vec<ChartPoint>>> = Arc::default();
  let mut stage = board(selected.clone());
  stage.key("Tab");
  stage.pass();
  assert_eq!(texts(&stage, "10.4 ms"), 1, "the first build's tooltip with the focus");
  stage.key("ArrowRight");
  stage.key("ArrowDown");
  stage.pass();
  assert_eq!(texts(&stage, "11.2 ms"), 1);
  stage.key("End");
  stage.key("Enter");
  assert_eq!(*selected.lock().unwrap(), [ChartPoint::new(1, 6)]);
  #[cfg(any(feature = "mcp", feature = "devtools"))]
  {
    let said = |stage: &mut Stage, id: &str, name: &str| {
      stage
        .tree
        .get_element_by_id_mut(id)
        .and_then(|element| element.description(name))
    };
    assert_eq!(said(&mut stage, "c", "a11y_role").as_deref(), Some("figure"));
    assert_eq!(said(&mut stage, "c", "a11y_name").as_deref(), Some("Frame time"));
    assert_eq!(
      said(&mut stage, "c-plot", "a11y_value").as_deref(),
      Some("0.60, wgpu: 6.3 ms, DirectX 12: 8.6 ms")
    );
  }
}

#[test]
fn a_missing_value_leaves_a_gap_in_its_line() {
  let mut stage = stage(|| {
    LineChart::new(
      ["a", "b", "c", "d", "e"],
      [ChartSeries::new("Load", [4.0, 5.0, f64::NAN, 5.0, 4.0])],
    )
    .id("c")
  });
  let drawn = points(&mut stage);
  assert_eq!(drawn.len(), 4, "no point where there is no value");
  // Between b and d nothing is drawn; between a and b the line is.
  let (bx, by) = place(&drawn[1]);
  let (ax, ay) = place(&drawn[0]);
  assert!(near(
    pixel(&mut stage, (ax + bx) / 2.0, (ay + by) / 2.0),
    light(PaletteColor::Chart1)
  ));
  assert_eq!(pixel(&mut stage, 190.0, by).a(), 0, "the gap");
}

#[test]
fn a_smooth_line_goes_through_every_value() {
  let values = [3.0, 8.0, 5.0, 9.0, 2.0];
  let mut stage = stage(move || {
    LineChart::new(["a", "b", "c", "d", "e"], [ChartSeries::new("Load", values)])
      .curve(LineCurve::Smooth)
      .id("c")
  });
  for item in points(&mut stage) {
    let (x, y) = place(&item);
    assert!(
      near(pixel(&mut stage, x, y), light(PaletteColor::Chart1)),
      "through {:?}",
      item.label
    );
  }
  // Between b and c the curve is not the straight line: it leaves the middle of the chord.
  let drawn = points(&mut stage);
  let ((bx, by), (cx, cy)) = (place(&drawn[1]), place(&drawn[2]));
  let (mx, my) = ((bx + cx) / 2.0, (by + cy) / 2.0);
  let straight = stage_line(values, LineCurve::Straight, mx, my);
  assert!(
    near(straight, light(PaletteColor::Chart1)),
    "a straight line is at the chord's middle"
  );
}

/// The color a line of `values` drawn with `curve` has at `(x, y)`.
fn stage_line(values: [f64; 5], curve: LineCurve, x: f32, y: f32) -> Color {
  let mut stage = stage(move || {
    LineChart::new(["a", "b", "c", "d", "e"], [ChartSeries::new("Load", values)])
      .curve(curve)
      .id("c")
  });
  pixel(&mut stage, x, y)
}

#[test]
fn a_line_of_ten_thousand_values_has_no_mark_of_its_own_and_the_pointer_finds_its_place_at_once() {
  let names: Vec<String> = (0..10_000).map(|index| index.to_string()).collect();
  let values: Vec<f64> = (0..10_000)
    .map(|index| ((index as f64) / 300.0).sin() * 50.0 + 60.0)
    .collect();
  let mut stage = stage(move || {
    LineChart::new(names.clone(), [ChartSeries::new("Signal", values.clone())])
      .title("Signal")
      .id("c")
  });
  assert_eq!(points(&mut stage).len(), 10_000);
  // The axis writes eight of the names, the first and the last among them.
  assert!(stage.text_quad("0").is_some() && stage.text_quad("9999").is_some());
  let (plot_x, plot_y, width, _) = stage.bounds("c-plot");
  let started = Instant::now();
  stage.tree.mouse_move(plot_x + width / 2.0, plot_y + 70.0);
  let found = started.elapsed();
  stage.pass();
  assert!(found.as_millis() < 50, "{found:?}");
  assert!(
    stage.text_quad("5000").is_some() || stage.text_quad("4999").is_some(),
    "the middle value's name"
  );
}

#[test]
fn negative_values_go_under_the_line_of_zero() {
  let mut stage = stage(|| {
    LineChart::new(
      ["Q1", "Q2", "Q3", "Q4"],
      [ChartSeries::new("Change", [12.0, -8.5, 5.0, -2.5])],
    )
    .id("c")
  });
  let drawn = points(&mut stage);
  let (_, high) = place(&drawn[0]);
  let (_, low) = place(&drawn[1]);
  assert!(low > high && low < 140.0 && high > 0.0);
}

#[test]
fn one_value_is_a_dot_in_the_plots_middle() {
  let mut stage = stage(|| LineChart::new(["0.60"], [ChartSeries::new("Frame time", [5.2])]).id("c"));
  let drawn = points(&mut stage);
  assert_eq!(drawn.len(), 1);
  let (x, y) = place(&drawn[0]);
  assert!((x - 190.0).abs() < 0.01);
  assert!(near(pixel(&mut stage, x, y), light(PaletteColor::Chart1)));
}

#[test]
fn a_hidden_series_is_left_out_and_its_item_in_the_legend_shows_it_again() {
  let hidden = Signal::new(vec!["wgpu".to_owned()]);
  let kept = hidden.clone();
  let mut stage = stage(move || LineChart::new(BUILDS, [wgpu(), directx()]).hidden(kept.clone()).id("c"));
  let drawn = points(&mut stage);
  assert_eq!(drawn.len(), 7);
  assert!(
    drawn
      .iter()
      .all(|item| item.label.as_deref().is_some_and(|label| label.ends_with("DirectX 12")))
  );
  assert_eq!(texts(&stage, "wgpu"), 1, "still in the legend");
}

#[test]
fn no_data_says_so_and_loading_is_a_skeleton() {
  let stage = stage(|| {
    LineChart::new(Vec::<String>::new(), Vec::new())
      .empty_text("Nothing yet")
      .id("c")
  });
  assert!(stage.text_quad("Nothing yet").is_some());
  let stage = stage_loading();
  assert!(stage.text_quad("No data").is_none());
}

fn stage_loading() -> Stage {
  stage(|| LineChart::new(BUILDS, [wgpu()]).loading(true).id("c"))
}

#[test]
fn an_area_is_the_fill_under_its_line_in_its_color_at_0_16() {
  let mut stage = stage(|| {
    AreaChart::new(
      ["0", "5", "10", "15", "20", "25", "30", "35"],
      [ChartSeries::new(
        "Resident",
        [380.0, 520.0, 470.0, 720.0, 670.0, 920.0, 870.0, 1060.0],
      )],
    )
    .title("Memory")
    .description("Megabytes resident during a session")
    .axis_labels(["0 min", "10", "20", "30", "40"])
    .id("c")
  });
  assert!(stage.text_quad("0 min").is_some() && stage.text_quad("40").is_some());
  assert!(
    stage.text_quad("35").is_none(),
    "its own axis in place of the categories"
  );
  let drawn = points(&mut stage);
  let (x, y) = place(&drawn[3]);
  let card = light(PaletteColor::Card);
  let chart = light(PaletteColor::Chart1);
  let fill = pixel(&mut stage, x, (y + 140.0) / 2.0);
  let mix = |c: u8, d: u8| (d as f32 * 0.16 + c as f32 * 0.84).round() as u8;
  let expected = Color::new(
    mix(card.r(), chart.r()),
    mix(card.g(), chart.g()),
    mix(card.b(), chart.b()),
    255,
  );
  // The canvas is clear where nothing is drawn: the fill alone, over nothing, is the color at 0.16.
  let drawn_fill = fill;
  assert!(
    near(over(card, drawn_fill), expected),
    "{drawn_fill:?} over the card is {expected:?}"
  );
  assert!(near(pixel(&mut stage, x, y), chart), "the line over it");
}

/// `top` over `under`, as a canvas that is clear where nothing is drawn shows over the card.
fn over(under: Color, top: Color) -> Color {
  let alpha = top.a() as f32 / 255.0;
  let mix = |u: u8, t: u8| (t as f32 * alpha + u as f32 * (1.0 - alpha)).round() as u8;
  Color::new(
    mix(under.r(), top.r()),
    mix(under.g(), top.g()),
    mix(under.b(), top.b()),
    255,
  )
}

#[test]
fn stacked_areas_stand_one_on_another() {
  let mut stage = stage(|| {
    AreaChart::new(
      ["a", "b", "c"],
      [
        ChartSeries::new("Layout", [2.0, 3.0, 2.0]),
        ChartSeries::new("Paint", [2.0, 2.0, 3.0]),
      ],
    )
    .stacked(true)
    .id("c")
  });
  let drawn = points(&mut stage);
  let (_, under) = place(&drawn[0]);
  let (_, on) = place(&drawn[3]);
  assert!(on < under, "the second series stands on the first: 4 over 2");
  // Its tooltip says its own value, not the stack's.
  hover(&mut stage, 0, 3, on);
  assert_eq!(texts(&stage, "2"), 2, "both series are 2 there");
}

#[test]
fn a_smooth_line_never_goes_past_the_values_on_either_side() {
  // A rise stays a rise: between 1 and 5 the curve is nowhere over 5.
  let mut stage = stage(|| {
    LineChart::new(["a", "b", "c", "d"], [ChartSeries::new("Load", [1.0, 5.0, 5.0, 5.0])])
      .curve(LineCurve::Smooth)
      .id("c")
  });
  let drawn = points(&mut stage);
  let (_, top) = place(&drawn[1]);
  let (bx, _) = place(&drawn[1]);
  let (cx, _) = place(&drawn[2]);
  for step in 1..8 {
    let x = bx + (cx - bx) * step as f32 / 8.0;
    assert_eq!(pixel(&mut stage, x, top - 4.0).a(), 0, "nothing over the flat at {x}");
  }
}
