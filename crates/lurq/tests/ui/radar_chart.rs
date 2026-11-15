//! A radar chart: the board's Radar Chart (its frame of `Sm`, a square plot of 150 in the middle, the axes' names round
//! it, the legend under it), the points as tooling reads them, the tooltip of the axis under the pointer, the arrows
//! round the axes and between the series, a series hidden through the legend, missing values and no data.

use std::sync::{Arc, Mutex};

use lurq::{
  app::{
    events::MouseButton,
    theme::{PaletteColor, ThemePalette},
  },
  canvas::{CanvasItem, CanvasItemShape},
  components::Column,
  core::Signal,
  layout::{Constraints, Size},
  node::color::Color,
  ui::{ChartPoint, ChartSeries, RadarChart},
};

use super::stage::Stage;

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

const AREAS: [&str; 6] = ["Draw", "Text", "Layout", "Memory", "Startup", "Input"];

fn renderers() -> [ChartSeries; 2] {
  [
    ChartSeries::new("wgpu", [92.0, 70.0, 64.0, 58.0, 75.0, 66.0]),
    ChartSeries::new("DX12", [68.0, 76.0, 70.0, 80.0, 62.0, 86.0]),
  ]
}

fn stage(chart: impl Fn() -> RadarChart + Send + Sync + 'static) -> Stage {
  let mut stage = Stage::new(move |ctx| Column::new().size(270.0, 420.0).child(chart().mount(ctx)).into());
  stage
    .tree
    .set_layout_constraints_override(Some(Constraints::loose(Size::new(800.0, 700.0))));
  stage.pass();
  stage
}

fn board(selected: Arc<Mutex<Vec<ChartPoint>>>) -> Stage {
  stage(move || {
    let selected = selected.clone();
    RadarChart::new(AREAS, renderers())
      .title("Renderer profile")
      .description("Relative score per area")
      .on_select(move |point| selected.lock().unwrap().push(point))
      .id("c")
  })
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
    _ => panic!("a point"),
  }
}

#[test]
fn the_boards_radar_is_a_card_of_sm_a_square_of_150_in_the_middle_with_the_names_round_it() {
  let mut stage = board(Arc::default());
  let (x, y, width, _) = stage.bounds("c");
  assert_eq!(width, 270.0);
  let title = stage.text_quad("Renderer profile").unwrap();
  assert!(
    (title.x - (x + 20.0)).abs() < 1.0 && (title.y - (y + 20.0)).abs() < 4.0,
    "padded 20"
  );
  assert_eq!(stage.text("Renderer profile").1, 15.0);
  let (plot_x, plot_y, plot_width, plot_height) = stage.bounds("c-plot");
  assert_eq!((plot_width, plot_height), (150.0, 150.0), "a square of 150");
  assert!((plot_x + 75.0 - (x + 135.0)).abs() < 0.5, "in the middle of the frame");

  // Each name outside the outer ring of 60: the first over the top, the fourth under the bottom.
  assert_eq!(stage.text("Draw"), (light(PaletteColor::MutedForeground), 11.0));
  let draw = stage.text_quad("Draw").unwrap();
  assert!(
    draw.y + draw.height <= plot_y + 15.0 - 5.0,
    "over the top: {} + {}",
    draw.y,
    draw.height
  );
  assert!((draw.x + draw.width / 2.0 - (plot_x + 75.0)).abs() < 1.0);
  let memory = stage.text_quad("Memory").unwrap();
  assert!(memory.y >= plot_y + 135.0 + 5.0, "under the bottom");
  let text = stage.text_quad("Text").unwrap();
  assert!(text.x >= plot_x + 75.0 + 52.0, "to the right of its vertex");

  // The legend under the plot names the two series.
  assert!(stage.text_quad("wgpu").is_some() && stage.text_quad("DX12").is_some());

  // The points as tooling reads them: twelve, the first at the top on a scale to 100.
  let read = points(&mut stage);
  assert_eq!(read.len(), 12);
  assert_eq!(read[0].label.as_deref(), Some("Draw, wgpu"));
  assert_eq!(read[0].value.as_deref(), Some("92"));
  let (px, py) = place(&read[0]);
  assert!(
    (px - 75.0).abs() < 0.01 && (py - (75.0 - 60.0 * 0.92)).abs() < 0.01,
    "{px}, {py}"
  );
  let (mx, _) = place(&read[3]);
  assert!((mx - 75.0).abs() < 0.01, "Memory straight down");
}

#[test]
fn the_axis_under_the_pointer_has_its_tooltip_and_a_press_selects_the_nearest_series() {
  let selected: Arc<Mutex<Vec<ChartPoint>>> = Arc::default();
  let mut stage = board(selected.clone());
  let (plot_x, plot_y, ..) = stage.bounds("c-plot");
  // Toward Memory, near DX12's value of 80 there.
  let (x, y) = (plot_x + 75.0, plot_y + 75.0 + 48.0);
  stage.tree.mouse_move(x, y);
  stage.pass();
  assert!(stage.text_quad("Memory").is_some());
  assert!(
    stage.text_quad("58").is_some() && stage.text_quad("80").is_some(),
    "both series' values"
  );
  stage.tree.mouse_down(x, y, MouseButton::Left);
  stage.tree.mouse_up(x, y, MouseButton::Left);
  stage.pass();
  assert_eq!(*selected.lock().unwrap(), [ChartPoint::new(1, 3)]);
}

#[test]
fn the_arrows_go_round_the_axes_and_between_the_series() {
  let selected: Arc<Mutex<Vec<ChartPoint>>> = Arc::default();
  let mut stage = board(selected.clone());
  stage.key("Tab");
  stage.key("ArrowRight");
  stage.key("ArrowDown");
  stage.key("Enter");
  assert_eq!(*selected.lock().unwrap(), [ChartPoint::new(1, 1)]);
  stage.key("End");
  stage.key("ArrowUp");
  stage.key("Enter");
  assert_eq!(selected.lock().unwrap()[1], ChartPoint::new(0, 5));
  #[cfg(any(feature = "mcp", feature = "devtools"))]
  {
    stage.pass();
    let said = stage
      .tree
      .get_element_by_id_mut("c-plot")
      .and_then(|element| element.description("a11y_value"));
    assert_eq!(said.as_deref(), Some("Input, wgpu: 66, DX12: 86"));
  }
}

#[test]
fn a_hidden_series_is_left_out_and_the_scale_follows_the_rest() {
  let hidden = Signal::new(vec!["wgpu".to_owned()]);
  let bound = hidden.clone();
  let mut stage = stage(move || RadarChart::new(AREAS, renderers()).hidden(bound.clone()).id("c"));
  assert_eq!(points(&mut stage).len(), 6, "DX12's alone");
  stage.click("c-legend-item-0");
  stage.pass();
  assert!(hidden.get().is_empty());
  assert_eq!(points(&mut stage).len(), 12);
}

#[test]
fn a_missing_value_is_not_read_the_rings_numbers_are_written_where_told_and_no_data_says_so() {
  let mut gaps = stage(|| {
    RadarChart::new(["a", "b", "c"], [ChartSeries::new("v", [4.0, f64::NAN, 2.0])])
      .value_axis(true)
      .id("c")
  });
  assert_eq!(points(&mut gaps).len(), 2, "the missing value has no point");
  assert!(gaps.text_quad("4").is_some(), "the outer ring's number");
  assert_eq!(gaps.text("4").0, light(PaletteColor::MutedForeground));

  let empty = stage(|| {
    RadarChart::new(Vec::<String>::new(), [])
      .empty_text("No runs yet")
      .id("c")
  });
  assert!(empty.text_quad("No runs yet").is_some());
}
