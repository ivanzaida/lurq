//! A scatter chart: the board's Scatter Chart (300 wide, a plot of 260 by 140, points of 8 at 0.85, the numbers of
//! the axis across under the plot, the line of the trend), the point nearest the pointer in two dimensions with its
//! ring and its tooltip, the keys, several series and the legend off, negative values, one point, no data, loading.

use std::sync::{Arc, Mutex};

use lurq::{
  app::{
    events::MouseButton,
    theme::{PaletteColor, ThemePalette},
  },
  canvas::{CanvasItem, CanvasItemShape},
  components::Column,
  layout::{Constraints, Size, quad::QuadContent},
  node::color::Color,
  ui::{ChartPoint, ChartSeries, ScatterChart},
};

use super::stage::Stage;

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

/// The board's scenes: nodes and milliseconds, rising together.
fn scenes() -> ChartSeries {
  ChartSeries::points(
    "Scenes",
    [
      (550.0, 3.3),
      (1050.0, 5.1),
      (1600.0, 4.3),
      (2150.0, 6.6),
      (2700.0, 6.1),
      (3200.0, 8.4),
      (3900.0, 7.6),
      (4300.0, 9.9),
      (5000.0, 11.0),
      (5500.0, 10.4),
      (6000.0, 12.7),
      (6600.0, 13.4),
      (7300.0, 15.5),
    ],
  )
}

fn outliers() -> ChartSeries {
  ChartSeries::points(
    "Outlier",
    [(1850.0, 7.6), (3700.0, 10.3), (6150.0, 14.2), (7100.0, 16.3)],
  )
}

fn thousands(value: f64) -> String {
  if value == 0.0 {
    "0".to_owned()
  } else {
    format!("{}k", lurq::ui::default_value_text(value / 1000.0))
  }
}

/// The board's scatter chart, 300 wide, mounted, its canvas drawn on the CPU.
fn board(selected: Arc<Mutex<Vec<ChartPoint>>>, trend: bool) -> Stage {
  stage(move || {
    let selected = selected.clone();
    ScatterChart::new([scenes(), outliers()])
      .title("Nodes vs frame time")
      .description("One dot per scene")
      .trend(trend)
      .x_text(thousands)
      .value_text(|ms| format!("{ms} ms"))
      .on_select(move |point| selected.lock().unwrap().push(point))
      .id("c")
  })
}

fn stage(chart: impl Fn() -> ScatterChart + Send + Sync + 'static) -> Stage {
  let mut stage = Stage::software(move |ctx| Column::new().size(300.0, 600.0).child(chart().mount(ctx)).into());
  stage
    .tree
    .set_layout_constraints_override(Some(Constraints::loose(Size::new(800.0, 700.0))));
  stage.pass();
  stage
}

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
    _ => panic!("a point is a point"),
  }
}

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
    .expect("drawn on the CPU")
    .expect("a snapshot");
  let (px, py) = ((x * scale) as u32, (y * scale) as u32);
  let at = ((py.min(shot.height - 1) * shot.width + px.min(shot.width - 1)) * 4) as usize;
  Color::new(shot.rgba[at], shot.rgba[at + 1], shot.rgba[at + 2], shot.rgba[at + 3])
}

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

#[test]
fn the_boards_scatter_chart_is_a_plot_of_260_by_140_its_points_of_8_and_the_numbers_across_under_it() {
  let mut stage = board(Arc::default(), false);
  let (x, _, width, _) = stage.bounds("c");
  assert_eq!(width, 300.0);
  assert_eq!(
    stage.text("Nodes vs frame time"),
    (light(PaletteColor::CardForeground), 15.0)
  );
  let (plot_x, _, plot_width, plot_height) = stage.bounds("c-plot");
  assert_eq!((plot_width, plot_height), (260.0, 140.0));
  assert!((plot_x - (x + 20.0)).abs() < 0.5);
  // The numbers of the axis across, as the app writes them, from the plot's start to its end.
  for text in ["0", "2k", "4k", "6k", "8k"] {
    assert_eq!(texts(&stage, text), 1, "{text}");
  }
  assert!(stage.text_quad("0").unwrap().x - plot_x < 1.0);
  let drawn = points(&mut stage);
  assert_eq!(drawn.len(), 17, "13 scenes and 4 outliers");
  // 4.3k across of 8k is at 4300 / 8000 of the plot's width.
  let at = drawn
    .iter()
    .find(|item| item.label.as_deref() == Some("Scenes, 4.3k"))
    .unwrap();
  let (px, py) = place(at);
  assert!((px - 260.0 * 4300.0 / 8000.0).abs() < 0.01);
  assert_eq!(at.value.as_deref(), Some("9.9 ms"));
  // A dot of 8 in the series' color at 0.85, nothing past its edge.
  let dot = pixel(&mut stage, px, py);
  let chart = light(PaletteColor::Chart1);
  assert!(
    near(dot, chart) && (dot.a() as i32 - 217).abs() <= 4,
    "{dot:?}: at 0.85"
  );
  assert_eq!(pixel(&mut stage, px + 5.5, py).a(), 0, "a dot of 8");
  // No legend unless told.
  assert_eq!(texts(&stage, "Outlier"), 0);
}

#[test]
fn the_point_nearest_the_pointer_has_its_ring_and_its_tooltip_and_a_press_selects_it() {
  let selected: Arc<Mutex<Vec<ChartPoint>>> = Arc::default();
  let mut stage = board(selected.clone(), false);
  let drawn = points(&mut stage);
  let target = drawn
    .iter()
    .find(|item| item.label.as_deref() == Some("Outlier, 3.7k"))
    .unwrap();
  let (px, py) = place(target);
  let (plot_x, plot_y, ..) = stage.bounds("c-plot");
  // A little off the point, and nearer it than any other in two dimensions.
  let (x, y) = (plot_x + px + 3.0, plot_y + py - 3.0);
  stage.tree.mouse_move(x, y);
  stage.pass();
  assert_eq!(texts(&stage, "10.3 ms"), 1);
  assert!(stage.text_quad("3.7k").is_some(), "its x as the tooltip's title");
  assert!(
    near(pixel(&mut stage, px, py), light(PaletteColor::Chart2)),
    "in its own series' color"
  );
  assert!(
    near(pixel(&mut stage, px + 4.0, py - 0.5), light(PaletteColor::Card)),
    "ringed"
  );
  // Far from every point, nothing.
  stage.tree.mouse_move(plot_x + 250.0, plot_y + 130.0);
  stage.pass();
  assert_eq!(texts(&stage, "10.3 ms"), 0);
  stage.tree.mouse_move(x, y);
  stage.tree.mouse_down(x, y, MouseButton::Left);
  stage.tree.mouse_up(x, y, MouseButton::Left);
  stage.pass();
  assert_eq!(*selected.lock().unwrap(), [ChartPoint::new(1, 1)]);
}

#[test]
fn the_trend_is_the_line_that_fits_the_first_series_over_the_points() {
  let mut stage = board(Arc::default(), true);
  let drawn = points(&mut stage);
  let scenes: Vec<(f32, f32)> = drawn
    .iter()
    .filter(|item| item.label.as_deref().is_some_and(|label| label.starts_with("Scenes")))
    .map(place)
    .collect();
  // The least-squares line through the scenes, sampled between two of them where no dot is.
  let count = scenes.len() as f32;
  let (mean_x, mean_y) = (
    scenes.iter().map(|(x, _)| x).sum::<f32>() / count,
    scenes.iter().map(|(_, y)| y).sum::<f32>() / count,
  );
  let slope = scenes.iter().map(|(x, y)| (x - mean_x) * (y - mean_y)).sum::<f32>()
    / scenes.iter().map(|(x, _)| (x - mean_x).powi(2)).sum::<f32>();
  let x = (scenes[3].0 + scenes[4].0) / 2.0;
  let y = mean_y + slope * (x - mean_x);
  let line = pixel(&mut stage, x, y);
  assert!(line.a() > 0, "the trend is drawn there");
  assert!(
    near(line, light(PaletteColor::MutedForeground)),
    "in the muted foreground: {line:?}"
  );
  let without = board(Arc::default(), false);
  let mut without = without;
  assert_eq!(pixel(&mut without, x, y).a(), 0, "and nothing there without it");
}

#[test]
fn the_plot_is_one_stop_of_tab_whose_arrows_go_along_a_series_and_between_series() {
  let selected: Arc<Mutex<Vec<ChartPoint>>> = Arc::default();
  let mut stage = board(selected.clone(), false);
  stage.key("Tab");
  stage.pass();
  assert_eq!(texts(&stage, "3.3 ms"), 1, "the first point, the least x");
  stage.key("ArrowRight");
  stage.pass();
  assert_eq!(texts(&stage, "5.1 ms"), 1);
  stage.key("ArrowDown");
  stage.pass();
  assert_eq!(texts(&stage, "10.3 ms"), 1, "the outliers' second");
  stage.key("Enter");
  assert_eq!(*selected.lock().unwrap(), [ChartPoint::new(1, 1)]);
}

#[test]
fn a_legend_when_told_negative_values_one_point_and_no_data() {
  let legend = stage(|| ScatterChart::new([scenes(), outliers()]).legend(true).id("c"));
  assert_eq!(texts(&legend, "Outlier"), 1);
  let mut drift = stage_of(vec![ChartSeries::points("Drift", [(-4.0, -2.0), (3.0, 5.0)])]);
  let drawn = points(&mut drift);
  let (low, high) = (place(&drawn[0]), place(&drawn[1]));
  assert!(low.0 < high.0 && low.1 > high.1);
  assert!(
    low.0 >= 0.0 && low.1 <= 140.0,
    "inside the plot, at its corner: the scales start on them"
  );
  let mut one = stage_of(vec![ChartSeries::points("One", [(5.0, 5.0)])]);
  assert_eq!(points(&mut one).len(), 1);
  let empty = stage_of(Vec::new());
  assert!(empty.text_quad("No data").is_some());
  let loading = stage(|| ScatterChart::new([scenes()]).loading(true).id("c"));
  assert!(loading.text_quad("No data").is_none());
}

fn stage_of(series: Vec<ChartSeries>) -> Stage {
  stage(move || ScatterChart::new(series.clone()).id("c"))
}
