//! A heatmap: the board's Heatmap (its frame of `Sm`, cells of 14 four apart, the rows' names at the start, the scale
//! under the grid from "Less" to "More"), a cell's color the scale's at its step, the cells as tooling reads them,
//! the tooltip of the cell under the pointer, the arrows from cell to cell, a cell with no value, the columns' names,
//! and no data.

use std::sync::{Arc, Mutex};

use lurq::{
  app::{
    events::MouseButton,
    theme::{PaletteColor, ThemePalette},
  },
  canvas::{CanvasItem, CanvasItemShape},
  components::Column,
  layout::{Constraints, Size},
  node::color::Color,
  ui::{ChartPoint, ChartSeries, Heatmap},
};

use super::stage::Stage;

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

const DAYS: [&str; 7] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];

fn weeks() -> Vec<String> {
  (1..=18).map(|week| format!("Week {week}")).collect()
}

/// Commits per day: 0 to 8, a value of every step.
fn commits() -> Vec<ChartSeries> {
  DAYS
    .iter()
    .enumerate()
    .map(|(day, name)| ChartSeries::new(name, (0..18).map(move |week| ((week * 7 + day) * 37 % 9) as f64)))
    .collect()
}

fn stage(chart: impl Fn() -> Heatmap + Send + Sync + 'static) -> Stage {
  let mut stage = Stage::software(move |ctx| Column::new().size(420.0, 400.0).child(chart().mount(ctx)).into());
  stage
    .tree
    .set_layout_constraints_override(Some(Constraints::loose(Size::new(800.0, 700.0))));
  stage.pass();
  stage
}

fn board(selected: Arc<Mutex<Vec<ChartPoint>>>) -> Stage {
  stage(move || {
    let selected = selected.clone();
    Heatmap::new(weeks(), commits())
      .title("Commits")
      .description("Per day, last 18 weeks")
      .on_select(move |point| selected.lock().unwrap().push(point))
      .id("c")
  })
}

fn cells(stage: &mut Stage) -> Vec<CanvasItem> {
  let canvas = stage
    .tree
    .get_element_by_id_mut("c-plot")
    .and_then(|element| element.canvas())
    .expect("the plot is a canvas");
  canvas.items().iter().cloned().collect()
}

fn rect(item: &CanvasItem) -> (f32, f32, f32, f32) {
  match item.shape {
    CanvasItemShape::Rect { x, y, width, height } => (x, y, width, height),
    _ => panic!("a cell is a rect"),
  }
}

/// The color the plot drew at `(x, y)` of its box, as it is on the canvas: a translucent step is translucent.
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
  let at = ((py * shot.width + px) * 4) as usize;
  Color::new(shot.rgba[at], shot.rgba[at + 1], shot.rgba[at + 2], shot.rgba[at + 3])
}

fn near(a: Color, b: Color) -> bool {
  [(a.r(), b.r()), (a.g(), b.g()), (a.b(), b.b()), (a.a(), b.a())]
    .into_iter()
    .all(|(a, b)| (a as i32 - b as i32).abs() <= 2)
}

#[test]
fn the_boards_heatmap_is_a_card_of_sm_cells_of_14_four_apart_and_the_scale_under_them() {
  let mut stage = board(Arc::default());
  let (x, y, width, _) = stage.bounds("c");
  assert_eq!(width, 420.0);
  let title = stage.text_quad("Commits").unwrap();
  assert!(
    (title.x - (x + 20.0)).abs() < 1.0 && (title.y - (y + 20.0)).abs() < 4.0,
    "padded 20"
  );
  let (plot_x, plot_y, _, plot_height) = stage.bounds("c-plot");
  assert_eq!(plot_height, 7.0 * 18.0 - 4.0, "seven rows of 14, 4 apart");

  // The rows' names at the start, mono 10 muted, every other one where the rows are as close as the board's.
  assert_eq!(stage.text("Mon"), (light(PaletteColor::MutedForeground), 10.0));
  assert!(stage.text_quad("Wed").is_some() && stage.text_quad("Fri").is_some());
  assert!(stage.text_quad("Tue").is_none());
  let mon = stage.text_quad("Mon").unwrap();
  assert!((mon.x - plot_x).abs() < 1.0);
  assert!(
    (mon.y + mon.height / 2.0 - (plot_y + 7.0)).abs() < 2.0,
    "through the middle of its row"
  );

  // The cells: 14 on a side, 4 apart, after the names' room of 26.
  let read = cells(&mut stage);
  assert_eq!(read.len(), 7 * 18);
  let (first_x, first_y, side, _) = rect(&read[0]);
  assert_eq!((first_x, first_y, side), (26.0, 0.0, 14.0));
  let (second_x, ..) = rect(&read[1]);
  assert_eq!(second_x, 26.0 + 18.0);
  let (_, row_y, ..) = rect(&read[18]);
  assert_eq!(row_y, 18.0, "the next row 4 under");
  assert_eq!(read[1].label.as_deref(), Some("Mon, Week 2"));

  // The scale under the grid, at the frame's end: Less, five steps of 12, More.
  assert_eq!(stage.text("Less"), (light(PaletteColor::MutedForeground), 11.0));
  let (_, _, step_width, step_height) = stage.bounds("c-scale-2");
  assert_eq!((step_width, step_height), (12.0, 12.0));
  let more = stage.text_quad("More").unwrap();
  assert!((more.x + more.width - (x + 400.0)).abs() < 1.0, "at the frame's end");
  let (_, scale_y, ..) = stage.bounds("c-scale");
  assert!(
    (scale_y - (plot_y + plot_height + 16.0)).abs() < 1.0,
    "16 under the grid"
  );
}

#[test]
fn a_cell_is_the_color_of_its_step_on_the_scale() {
  let mut stage = board(Arc::default());
  let read = cells(&mut stage);
  let steps: Vec<Color> = (0..5)
    .map(|step| stage.fill_color(&format!("c-scale-{step}")).expect("a step"))
    .collect();
  assert_eq!(steps[0], light(PaletteColor::Muted), "the lowest step muted");
  assert_eq!(
    steps[4],
    light(PaletteColor::Chart1),
    "the highest the first chart color"
  );
  assert!((steps[1].a() as i32 - 64).abs() <= 1, "a quarter of its opacity");
  // 0 to 8 on five steps: 0 the lowest, 1 and 2 the second, ..., 7 and 8 the highest.
  for item in &read {
    let value: f64 = item.value.as_deref().unwrap().parse().unwrap();
    let step = if value == 0.0 { 0 } else { (value / 2.0).ceil() as usize };
    let (x, y, side, _) = rect(item);
    let drawn = pixel(&mut stage, x + side / 2.0, y + side / 2.0);
    assert!(
      near(drawn, steps[step]),
      "{:?} at {value}: {drawn:?} against {:?}",
      item.label,
      steps[step]
    );
  }
}

#[test]
fn the_cell_under_the_pointer_has_its_tooltip_and_the_arrows_go_from_cell_to_cell() {
  let selected: Arc<Mutex<Vec<ChartPoint>>> = Arc::default();
  let mut stage = board(selected.clone());
  let (plot_x, plot_y, ..) = stage.bounds("c-plot");
  // Wednesday of the third week.
  let (x, y) = (plot_x + 26.0 + 2.0 * 18.0 + 7.0, plot_y + 2.0 * 18.0 + 7.0);
  stage.tree.mouse_move(x, y);
  stage.pass();
  assert!(stage.text_quad("Week 3").is_some(), "the tooltip titled by its column");
  stage.tree.mouse_down(x, y, MouseButton::Left);
  stage.tree.mouse_up(x, y, MouseButton::Left);
  stage.pass();
  assert_eq!(*selected.lock().unwrap(), [ChartPoint::new(2, 2)]);
  stage.tree.mouse_move(plot_x - 100.0, plot_y - 100.0);
  stage.pass();

  stage.key("ArrowRight");
  stage.key("ArrowDown");
  stage.key("ArrowDown");
  stage.key("Enter");
  assert_eq!(
    selected.lock().unwrap()[1],
    ChartPoint::new(4, 3),
    "along the row, then down the column"
  );
  stage.key("End");
  stage.key("Enter");
  assert_eq!(selected.lock().unwrap()[2], ChartPoint::new(4, 17));
  #[cfg(any(feature = "mcp", feature = "devtools"))]
  {
    stage.pass();
    let said = stage
      .tree
      .get_element_by_id_mut("c-plot")
      .and_then(|element| element.description("a11y_value"));
    let value = (17 * 7 + 4) * 37 % 9;
    assert_eq!(said.as_deref(), Some(format!("Week 18, Fri: {value}").as_str()));
  }
}

#[test]
fn a_cell_with_no_value_is_an_edge_and_says_so() {
  let mut stage = stage(|| {
    Heatmap::new(["a", "b"], [ChartSeries::new("row", [3.0, f64::NAN])])
      .missing_text("No commits counted")
      .id("c")
  });
  let read = cells(&mut stage);
  assert_eq!(read[1].value.as_deref(), Some("No commits counted"));
  let (x, y, side, _) = rect(&read[1]);
  assert_eq!(
    pixel(&mut stage, x + side / 2.0, y + side / 2.0).a(),
    0,
    "nothing inside"
  );
  assert!(pixel(&mut stage, x + side / 2.0, y + 0.5).a() > 0, "its edge");
}

#[test]
fn the_columns_names_are_written_where_told_and_no_data_says_so() {
  let named = stage(|| Heatmap::new(weeks(), commits()).column_axis(true).id("c"));
  assert!(named.text_quad("Week 1").is_some(), "the first column's name");
  assert!(named.text_quad("Week 2").is_none(), "the next has no room beside it");

  let empty = stage(|| {
    Heatmap::new(Vec::<String>::new(), [])
      .empty_text("No commits yet")
      .id("c")
  });
  assert!(empty.text_quad("No commits yet").is_some());
  assert!(empty.text_quad("Less").is_some(), "the scale stays");
}

#[test]
fn the_heatmap_moves_as_every_chart() {
  super::chart_motion::moves_as_every_chart(|ctx, values, moves| {
    Heatmap::new(["a", "b", "c"], [ChartSeries::new("v", values)])
      .motion(moves)
      .id("c")
      .mount(ctx)
  });
}

#[test]
fn a_cell_goes_from_its_old_steps_color_to_its_new_one() {
  use super::chart_motion::{near, pixel, software};
  let value = lurq::core::Signal::new(0.0);
  let bound = value.clone();
  let mut stage = software(move |ctx| {
    Heatmap::new(["a", "b"], [ChartSeries::new("row", [bound.get(), 8.0])])
      .id("c")
      .mount(ctx)
  });
  let middle = |stage: &mut Stage| {
    let read = cells(stage);
    let (x, y, side, _) = rect(&read[0]);
    pixel(stage, x + side / 2.0, y + side / 2.0)
  };
  let (low, high) = (light(PaletteColor::Muted), light(PaletteColor::Chart1));
  assert!(near(middle(&mut stage), low), "the lowest step");
  value.set(8.0);
  stage.pass_after(std::time::Duration::ZERO);
  stage.pass_after(std::time::Duration::from_millis(60));
  let half = middle(&mut stage);
  assert!(!near(half, low) && !near(half, high), "between the two: {half:?}");
  stage.pass();
  assert!(near(middle(&mut stage), high), "at rest the highest");
}

#[test]
fn the_grid_is_as_high_as_its_cells_are_drawn_at_every_width() {
  // 52 columns: cells of 14 need 26 + 52 × 18 - 4 = 958, more than the frame has at the first two widths.
  for width in [300.0_f32, 600.0, 1100.0] {
    let mut stage = Stage::new(move |ctx| {
      Column::new()
        .size(width, 600.0)
        .child(
          Heatmap::new((1..=52).map(|week| format!("W{week}")), commits_for(52))
            .id("c")
            .mount(ctx),
        )
        .into()
    });
    stage
      .tree
      .set_layout_constraints_override(Some(Constraints::loose(Size::new(1200.0, 700.0))));
    stage.pass();
    stage.pass();
    let read = cells(&mut stage);
    let (_, _, plot_width, plot_height) = stage.bounds("c-plot");
    let (_, last_y, side, _) = rect(read.last().unwrap());
    assert!(
      (plot_height - (last_y + side)).abs() < 0.5,
      "at {width}: the plot ends with its last row ({plot_height} against {})",
      last_y + side
    );
    let (last_x, ..) = rect(&read[51]);
    assert!(
      last_x + side <= plot_width + 0.5,
      "at {width}: every column in the plot"
    );
    let wide = width - 40.0 >= 958.0;
    assert_eq!(side == 14.0, wide, "at {width}: cells of {side}");
  }
}

/// Commits for seven days by `count` weeks.
fn commits_for(count: usize) -> Vec<ChartSeries> {
  DAYS
    .iter()
    .enumerate()
    .map(|(day, name)| ChartSeries::new(name, (0..count).map(move |week| ((week * 7 + day) * 37 % 9) as f64)))
    .collect()
}
