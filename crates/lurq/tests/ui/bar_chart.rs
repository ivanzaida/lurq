//! A bar chart: the board's Bar Chart (its frame, its plot of 160, six bars 12 apart, the current one), the bars as
//! tooling reads them, the tooltip of the bar under the pointer, the one stop of Tab and its arrows, a press told to
//! `on_select`, several series side by side and stacked with their legend, negative values, no data, loading, and a
//! drawing made again only when what it shows changes.

use std::sync::{Arc, Mutex};

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
  ui::{BarChart, ChartPoint, ChartSeries},
};

use super::stage::Stage;

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

const RELEASES: [&str; 6] = ["0.55", "0.56", "0.57", "0.58", "0.59", "0.60"];

fn frame_time() -> ChartSeries {
  ChartSeries::new("Frame time", [11.8, 13.2, 9.6, 8.4, 7.0, 5.2])
}

/// The board's bar chart, 420 wide, mounted.
fn board(selected: Arc<Mutex<Vec<ChartPoint>>>) -> Stage {
  stage(move || {
    let selected = selected.clone();
    BarChart::new(RELEASES, [frame_time()])
      .title("Frame time")
      .description("Median per build, last six releases")
      .current(5)
      .value_text(|ms| format!("{ms} ms"))
      .on_select(move |point| selected.lock().unwrap().push(point))
      .id("c")
  })
}

fn stage(chart: impl Fn() -> BarChart + Send + Sync + 'static) -> Stage {
  let mut stage = Stage::new(move |ctx| Column::new().size(420.0, 600.0).child(chart().mount(ctx)).into());
  stage
    .tree
    .set_layout_constraints_override(Some(Constraints::loose(Size::new(800.0, 700.0))));
  stage.pass();
  stage
}

/// The items the plot says it holds: its bars, as tooling reads them.
fn bars(stage: &mut Stage) -> Vec<CanvasItem> {
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
    _ => panic!("a bar is a rect"),
  }
}

fn texts(stage: &Stage, content: &str) -> usize {
  stage
    .quads()
    .iter()
    .filter(|quad| matches!(&quad.content, QuadContent::Text { text, .. } if text == content))
    .count()
}

#[test]
fn the_boards_bar_chart_is_a_card_padded_24_a_plot_of_160_and_six_bars_12_apart() {
  let mut stage = board(Arc::default());
  let (x, y, width, _) = stage.bounds("c");
  assert_eq!(width, 420.0);
  assert_eq!(stage.fill_color("c"), Some(light(PaletteColor::Card)));
  let (title_color, title_size) = stage.text("Frame time");
  assert_eq!((title_color, title_size), (light(PaletteColor::CardForeground), 16.0));
  assert_eq!(
    stage.text("Median per build, last six releases"),
    (light(PaletteColor::MutedForeground), 13.0)
  );
  let title = stage.text_quad("Frame time").unwrap();
  assert!(
    (title.x - (x + 24.0)).abs() < 1.0 && (title.y - (y + 24.0)).abs() < 4.0,
    "padded 24"
  );
  let (plot_x, plot_y, plot_width, plot_height) = stage.bounds("c-plot");
  assert_eq!((plot_width, plot_height), (372.0, 160.0));
  assert!((plot_x - (x + 24.0)).abs() < 0.5);
  let description = stage.text_quad("Median per build, last six releases").unwrap();
  assert!(
    (plot_y - (description.y + description.height + 20.0)).abs() < 4.0,
    "20 under the header"
  );

  let drawn = bars(&mut stage);
  assert_eq!(drawn.len(), 6);
  for (place, bar) in drawn.iter().enumerate() {
    let (bx, _, bw, bh) = rect(bar);
    assert!((bw - 52.0).abs() < 0.01, "(372 - 5 × 12) / 6: {bw}");
    assert!((bx - place as f32 * 64.0).abs() < 0.01, "12 apart");
    assert!(bh > 0.0 && bh <= 160.0);
  }
  assert_eq!(drawn[1].label.as_deref(), Some("0.56"));
  assert_eq!(
    drawn[1].value.as_deref(),
    Some("13.2 ms"),
    "its number as the app writes it"
  );
  assert!(
    rect(&drawn[1]).3 > rect(&drawn[5]).3,
    "a taller bar for a greater value"
  );

  // The names under the bars, mono 11 in the muted foreground, each under its own.
  assert_eq!(stage.text("0.59"), (light(PaletteColor::MutedForeground), 11.0));
  let name = stage.text_quad("0.56").unwrap();
  assert!(
    (name.x + name.width / 2.0 - (plot_x + 64.0 + 26.0)).abs() < 1.0,
    "centred under its bar"
  );
}

#[test]
fn the_bar_under_the_pointer_has_its_tooltip_and_a_press_selects_it() {
  let selected: Arc<Mutex<Vec<ChartPoint>>> = Arc::default();
  let mut stage = board(selected.clone());
  assert!(stage.text_quad("13.2 ms").is_none(), "no tooltip at rest");
  let (plot_x, plot_y, ..) = stage.bounds("c-plot");
  let (x, y) = (plot_x + 64.0 + 26.0, plot_y + 150.0);
  stage.tree.mouse_move(x, y);
  stage.pass();
  assert_eq!(texts(&stage, "13.2 ms"), 1, "the tooltip's value");
  assert_eq!(stage.text("13.2 ms").0, light(PaletteColor::PopoverForeground));
  assert!(stage.text_quad("Frame time").is_some());
  let (_, tip_y, _, tip_height) = stage.bounds("c-tooltip");
  assert!(tip_y + tip_height <= plot_y + 160.0, "over the bar");
  stage.tree.mouse_down(x, y, MouseButton::Left);
  stage.tree.mouse_up(x, y, MouseButton::Left);
  stage.pass();
  assert_eq!(*selected.lock().unwrap(), [ChartPoint::new(0, 1)]);
}

#[test]
fn the_plot_is_one_stop_of_tab_whose_arrows_go_between_the_bars() {
  let selected: Arc<Mutex<Vec<ChartPoint>>> = Arc::default();
  let mut stage = board(selected.clone());
  stage.key("Tab");
  stage.pass();
  assert_eq!(texts(&stage, "11.8 ms"), 1, "the first bar's tooltip with the focus");
  stage.key("ArrowRight");
  stage.key("ArrowRight");
  stage.pass();
  assert_eq!(texts(&stage, "9.6 ms"), 1);
  stage.key("End");
  stage.pass();
  assert_eq!(texts(&stage, "5.2 ms"), 1);
  stage.key("Enter");
  assert_eq!(*selected.lock().unwrap(), [ChartPoint::new(0, 5)]);
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
      said(&mut stage, "c", "a11y_description").as_deref(),
      Some("Median per build, last six releases")
    );
    assert_eq!(
      said(&mut stage, "c-plot", "a11y_value").as_deref(),
      Some("0.60, Frame time: 5.2 ms")
    );
  }
}

#[test]
fn several_series_stand_side_by_side_and_stacked_one_on_another_with_their_legend() {
  let phases = || {
    [
      ChartSeries::new("Layout", [4.0, 3.4, 3.0]),
      ChartSeries::new("Text", [3.0, 2.8, 2.2]),
      ChartSeries::new("Paint", [2.4, 2.0, 1.8]),
    ]
  };
  let mut side = stage(move || BarChart::new(["a", "b", "c"], phases()).id("c"));
  let grouped = bars(&mut side);
  assert_eq!(grouped.len(), 9);
  let (first, second) = (rect(&grouped[0]), rect(&grouped[1]));
  assert!(
    (second.0 - (first.0 + first.2 + 4.0)).abs() < 0.01,
    "4 between a category's bars"
  );
  assert!(
    side.text_quad("Layout").is_some() && side.text_quad("Paint").is_some(),
    "the legend"
  );
  assert_eq!(
    side.text("Layout").0,
    light(PaletteColor::MutedForeground),
    "its names muted, as the board's"
  );

  let mut stacked = stage(move || BarChart::new(["a", "b", "c"], phases()).stacked(true).id("c"));
  let stack = bars(&mut stacked);
  let (bottom, middle) = (rect(&stack[0]), rect(&stack[1]));
  assert_eq!(bottom.0, middle.0, "one on another");
  assert!(
    (middle.1 + middle.3 + 1.0 - bottom.1).abs() < 0.01,
    "a pixel between two segments"
  );
}

#[test]
fn a_hidden_series_is_left_out_and_its_item_in_the_legend_shows_it_again() {
  let hidden = Signal::new(vec!["Text".to_owned()]);
  let bound = hidden.clone();
  let mut stage = stage(move || {
    BarChart::new(
      ["a", "b"],
      [
        ChartSeries::new("Layout", [4.0, 3.0]),
        ChartSeries::new("Text", [2.0, 1.0]),
      ],
    )
    .hidden(bound.clone())
    .id("c")
  });
  assert_eq!(bars(&mut stage).len(), 2, "the bars of Layout alone");
  stage.click("c-legend-item-1");
  stage.pass();
  assert!(hidden.get().is_empty());
  assert_eq!(bars(&mut stage).len(), 4);
}

#[test]
fn negative_values_go_down_from_zero_on_a_line_across_the_plot() {
  let mut stage = stage(|| {
    BarChart::new(["q1", "q2", "q3"], [ChartSeries::new("Change", [12.0, -8.0, 5.0])])
      .grid(true)
      .value_axis(true)
      .id("c")
  });
  let drawn = bars(&mut stage);
  let (up, down) = (rect(&drawn[0]), rect(&drawn[1]));
  assert!(
    (down.1 - (up.1 + up.3)).abs() < 1.5,
    "the negative bar starts where the positive ends: zero"
  );
  assert!(down.1 > 20.0 && down.1 < 140.0, "zero is inside the plot");
  assert!(
    stage.text_quad("-10").is_some() && stage.text_quad("0").is_some(),
    "the ticks' numbers"
  );
}

#[test]
fn no_data_says_so_and_loading_is_a_skeleton() {
  let mut empty = stage(|| {
    BarChart::new(Vec::<String>::new(), [])
      .empty_text("No builds yet")
      .id("c")
  });
  assert_eq!(empty.text("No builds yet").0, light(PaletteColor::MutedForeground));
  let loading = stage(|| BarChart::new(RELEASES, [frame_time()]).loading(true).id("c"));
  assert_eq!(texts(&loading, "0.55"), 0, "no axis while loading");
}

#[test]
fn a_long_category_name_ends_in_an_ellipsis_under_its_bar() {
  let stage = stage(|| {
    BarChart::new(
      ["a category name much longer than its bar", "b"],
      [ChartSeries::new("v", [1.0, 2.0])],
    )
    .id("c")
  });
  let cut = stage.cut_text_quad("a category name much longer than its bar");
  assert!(cut.is_some_and(|quad| quad.width < 190.0), "cut to its band");
}

#[test]
fn the_plot_is_drawn_again_only_when_what_it_shows_changes() {
  let values = Signal::new(vec![1.0, 2.0]);
  let bound = values.clone();
  let mut stage = stage(move || BarChart::new(["a", "b"], [ChartSeries::new("v", bound.get())]).id("c"));
  let canvas = stage
    .tree
    .get_element_by_id_mut("c-plot")
    .and_then(|element| element.canvas())
    .unwrap();
  let before = canvas.status().content_revision;
  stage.pass();
  stage.pass();
  assert_eq!(
    canvas.status().content_revision,
    before,
    "a pass that changes nothing draws nothing"
  );
  values.set(vec![1.0, 3.0]);
  stage.pass();
  assert!(
    canvas.status().content_revision > before,
    "a change of the data draws it again"
  );
}

#[test]
fn the_legend_and_the_tooltip_stand_on_their_own_as_the_board_draws_them() {
  use lurq::ui::{ChartLegend, ChartLegendItem, ChartTooltip, ChartTooltipRow};
  let mut stage = Stage::new(|_| {
    Column::new()
      .size(400.0, 300.0)
      .child(
        ChartLegend::new([
          ChartLegendItem::new("Layout", PaletteColor::Chart1),
          ChartLegendItem::new("Text", PaletteColor::Chart2),
        ])
        .id("legend"),
      )
      .child(
        ChartTooltip::new("Build 0.59")
          .row(ChartTooltipRow::new(PaletteColor::Chart1, "wgpu", "6.9 ms"))
          .row(ChartTooltipRow::new(
            PaletteColor::Chart2,
            "a backend name much longer than its room",
            "9.4 ms",
          ))
          .id("tip"),
      )
      .into()
  });
  stage.pass();
  // The legend: keys of 10 in the series' colors, the names 12 in the foreground, 16 between items.
  assert_eq!(stage.text("Layout"), (light(PaletteColor::Foreground), 12.0));
  let (first_x, _, first_width, _) = stage.bounds("legend-item-0");
  let (second_x, ..) = stage.bounds("legend-item-1");
  assert!(
    (second_x - (first_x + first_width + 16.0)).abs() < 0.5,
    "16 between items"
  );
  let key = stage
    .quads()
    .into_iter()
    .find(|quad| quad.width == 10.0 && quad.height == 10.0)
    .expect("a key of 10");
  assert!(matches!(key.content, QuadContent::Rect { color, .. } if color == light(PaletteColor::Chart1)));

  // The tooltip: at least 140 on the popover, its title 12 and its values mono 11, a long name cut.
  let (_, _, width, _) = stage.bounds("tip");
  assert!(width >= 140.0);
  assert_eq!(stage.fill_color("tip"), Some(light(PaletteColor::Popover)));
  assert_eq!(stage.text("Build 0.59"), (light(PaletteColor::PopoverForeground), 12.0));
  assert_eq!(stage.text("6.9 ms"), (light(PaletteColor::PopoverForeground), 11.0));
  assert_eq!(stage.text("wgpu"), (light(PaletteColor::MutedForeground), 11.0));
  assert!(
    stage.text_quad("9.4 ms").is_some(),
    "the value keeps its room beside a long name"
  );
}

#[test]
fn bars_lying_across_run_from_the_start_with_the_names_at_the_start_and_the_values_under() {
  use lurq::ui::Orientation;
  let mut stage = stage(|| {
    BarChart::new(
      ["text_pipeline", "canvas_mesh", "layout_flex"],
      [ChartSeries::new("Seconds", [9.2, 7.0, 4.7])],
    )
    .orientation(Orientation::Horizontal)
    .value_axis(true)
    .id("c")
  });
  let (plot_x, plot_y, plot_width, plot_height) = stage.bounds("c-plot");
  let drawn = bars(&mut stage);
  let (first, second) = (rect(&drawn[0]), rect(&drawn[1]));
  assert!(first.0.abs() < 0.5, "from the plot's start: {first:?}");
  assert!(first.2 > second.2, "a longer bar for a greater value");
  assert!(
    (first.3 - second.3).abs() < 0.01 && first.3 > 0.0,
    "bands of one height"
  );
  assert!(second.1 > first.1, "one under another");
  assert!(first.2 <= plot_width && first.1 + first.3 <= plot_height);
  // The names at the plot's start, each at its band's height, ending before the plot.
  let name = stage.text_quad("canvas_mesh").expect("the name");
  assert!(name.x + name.width <= plot_x, "before the plot");
  let middle = plot_y + second.1 + second.3 / 2.0;
  assert!(
    (name.y + name.height / 2.0 - middle).abs() < 3.0,
    "at its band's height"
  );
  // The values under the plot.
  let zero = stage.text_quad("0").expect("a tick's number");
  assert!(zero.y >= plot_y + plot_height, "under the plot");
}
