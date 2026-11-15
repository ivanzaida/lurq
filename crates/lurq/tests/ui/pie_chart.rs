//! A pie chart: the board's Pie Chart (its frame of `Sm`, a pie of 96, the legend beside it with the shares) and its
//! Donut Chart (`Md`, a ring of 0.66, the total in its hole where the app writes it), the slices as tooling reads
//! them, the slice under the pointer picked out with its tooltip, the arrows round the slices, a slice hidden through
//! the legend, and the edges: a slice too small to see, one slice, all zero.

use std::sync::{Arc, Mutex};

use lurq::{
  app::theme::{PaletteColor, ThemePalette},
  canvas::{CanvasItem, CanvasItemShape},
  components::Column,
  core::Signal,
  layout::{Constraints, Size},
  node::color::Color,
  ui::{ChartPoint, ChartSeries, PieChart},
};

use super::stage::Stage;

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

const CRATES: [&str; 4] = ["render", "layout", "text", "other"];

fn sizes() -> ChartSeries {
  ChartSeries::new("Size", [42.0, 28.0, 19.0, 11.0])
}

fn stage(chart: impl Fn() -> PieChart + Send + Sync + 'static) -> Stage {
  let mut stage = Stage::new(move |ctx| Column::new().size(280.0, 400.0).child(chart().mount(ctx)).into());
  stage
    .tree
    .set_layout_constraints_override(Some(Constraints::loose(Size::new(800.0, 700.0))));
  stage.pass();
  stage
}

fn board(selected: Arc<Mutex<Vec<ChartPoint>>>) -> Stage {
  stage(move || {
    let selected = selected.clone();
    PieChart::new(CRATES, sizes())
      .title("Crates by size")
      .description("Share of compiled code")
      .value_text(|mb| format!("{mb} MB"))
      .on_select(move |point| selected.lock().unwrap().push(point))
      .id("c")
  })
}

fn slices(stage: &mut Stage) -> Vec<CanvasItem> {
  let canvas = stage
    .tree
    .get_element_by_id_mut("c-plot")
    .and_then(|element| element.canvas())
    .expect("the plot is a canvas");
  canvas.items().iter().cloned().collect()
}

fn center(item: &CanvasItem) -> (f32, f32) {
  match item.shape {
    CanvasItemShape::Point { x, y, .. } => (x, y),
    _ => panic!("a slice is found at its middle"),
  }
}

#[test]
fn the_boards_pie_is_a_card_of_sm_a_pie_of_96_and_its_legend_beside_it_with_the_shares() {
  let mut stage = board(Arc::default());
  let (x, y, width, _) = stage.bounds("c");
  assert_eq!(width, 280.0);
  let title = stage.text_quad("Crates by size").unwrap();
  assert!(
    (title.x - (x + 20.0)).abs() < 1.0 && (title.y - (y + 20.0)).abs() < 4.0,
    "padded 20, as the board's Sm"
  );
  assert_eq!(stage.text("Crates by size").1, 15.0);
  assert_eq!(stage.text("Share of compiled code").1, 12.0);
  let (plot_x, _, plot_width, plot_height) = stage.bounds("c-plot");
  assert_eq!((plot_width, plot_height), (96.0, 96.0), "a pie of 96");
  assert!((plot_x - (x + 20.0)).abs() < 0.5);

  // The legend beside it, 18 away: the names 12 in the card's foreground, the shares mono 11 muted at the end.
  let (legend_x, _, legend_width, _) = stage.bounds("c-legend");
  assert!((legend_x - (plot_x + 96.0 + 18.0)).abs() < 0.5, "18 beside the pie");
  assert_eq!(stage.text("render"), (light(PaletteColor::CardForeground), 12.0));
  assert_eq!(stage.text("42%"), (light(PaletteColor::MutedForeground), 11.0));
  let share = stage.text_quad("42%").unwrap();
  assert!(
    (share.x + share.width - (legend_x + legend_width)).abs() < 1.0,
    "the share at the item's end"
  );
  let (_, first_y, _, first_height) = stage.bounds("c-legend-item-0");
  let (_, second_y, ..) = stage.bounds("c-legend-item-1");
  assert!(
    (second_y - (first_y + first_height + 7.0)).abs() < 0.5,
    "7 between items in Sm"
  );

  // The slices from the top and clockwise: the first on the right half, the last on the left near the top.
  let drawn = slices(&mut stage);
  assert_eq!(drawn.len(), 4);
  assert_eq!(drawn[0].label.as_deref(), Some("render"));
  assert_eq!(drawn[0].value.as_deref(), Some("42 MB, 42%"));
  let (first, last) = (center(&drawn[0]), center(&drawn[3]));
  assert!(first.0 > 48.0, "render on the right: {first:?}");
  assert!(last.0 < 48.0 && last.1 < 48.0, "other at the top left: {last:?}");
}

#[test]
fn the_slice_under_the_pointer_is_picked_out_with_its_tooltip_and_a_press_selects_it() {
  let selected: Arc<Mutex<Vec<ChartPoint>>> = Arc::default();
  let mut stage = board(selected.clone());
  let drawn = slices(&mut stage);
  let (sx, sy) = center(&drawn[1]);
  let (plot_x, plot_y, ..) = stage.bounds("c-plot");
  stage.tree.mouse_move(plot_x + sx, plot_y + sy);
  stage.pass();
  assert!(stage.text_quad("28 MB").is_some(), "the tooltip of layout");
  assert!(stage.text_quad("Size").is_some(), "titled with the series");
  stage
    .tree
    .mouse_down(plot_x + sx, plot_y + sy, lurq::app::events::MouseButton::Left);
  stage
    .tree
    .mouse_up(plot_x + sx, plot_y + sy, lurq::app::events::MouseButton::Left);
  stage.pass();
  assert_eq!(*selected.lock().unwrap(), [ChartPoint::new(0, 1)]);
}

#[test]
fn the_arrows_go_round_the_slices() {
  let selected: Arc<Mutex<Vec<ChartPoint>>> = Arc::default();
  let mut stage = board(selected.clone());
  stage.key("Tab");
  stage.pass();
  assert!(stage.text_quad("42 MB").is_some(), "the first slice with the focus");
  stage.key("ArrowRight");
  stage.key("ArrowRight");
  stage.pass();
  assert!(stage.text_quad("19 MB").is_some());
  stage.key("End");
  stage.key("Enter");
  assert_eq!(*selected.lock().unwrap(), [ChartPoint::new(0, 3)]);
  #[cfg(any(feature = "mcp", feature = "devtools"))]
  {
    let said = stage
      .tree
      .get_element_by_id_mut("c-plot")
      .and_then(|element| element.description("a11y_value"));
    assert_eq!(said.as_deref(), Some("Size, other: 11 MB"));
  }
}

#[test]
fn the_donut_is_md_a_ring_of_066_and_writes_its_total_where_it_is_told() {
  let mut stage = stage(|| {
    PieChart::new(["Layout", "Text", "Paint"], ChartSeries::new("Frame", [7.2, 4.8, 4.0]))
      .donut(true)
      .title("Frame budget")
      .description("Share of a 16 ms frame")
      .total_text("per frame")
      .value_text(|ms| format!("{ms} ms"))
      .id("c")
  });
  let (x, y, ..) = stage.bounds("c");
  let title = stage.text_quad("Frame budget").unwrap();
  assert!(
    (title.x - (x + 24.0)).abs() < 1.0 && (title.y - (y + 24.0)).abs() < 4.0,
    "padded 24, Md"
  );
  assert_eq!(stage.text("Layout"), (light(PaletteColor::CardForeground), 13.0));
  assert!(stage.text_quad("45%").is_some() && stage.text_quad("25%").is_some());
  let (plot_x, _, plot_width, _) = stage.bounds("c-plot");
  assert_eq!(plot_width, 96.0);
  let (legend_x, ..) = stage.bounds("c-legend");
  assert!((legend_x - (plot_x + 96.0 + 20.0)).abs() < 0.5, "20 beside the donut");
  // The ring is the slices', the hole the card's: the pointer in it is on none.
  let (_, plot_y, ..) = stage.bounds("c-plot");
  stage.tree.mouse_move(plot_x + 60.0, plot_y + 4.0);
  stage.pass();
  assert!(stage.text_quad("7.2 ms").is_some(), "on the ring, Layout's tooltip");
  stage.tree.mouse_move(plot_x + 48.0, plot_y + 34.0);
  stage.pass();
  assert!(stage.text_quad("7.2 ms").is_none(), "in the hole, none");
  // The total in the hole, over the app's words.
  let total = stage.text_quad("16 ms").expect("the total");
  let words = stage.text_quad("per frame").expect("its words");
  assert!(total.y < words.y);
  assert!(
    (total.x + total.width / 2.0 - (plot_x + 48.0)).abs() < 1.0,
    "centred in the hole"
  );
}

#[test]
fn a_slice_hidden_through_the_legend_leaves_the_whole_to_the_others() {
  let hidden = Signal::new(Vec::<String>::new());
  let bound = hidden.clone();
  let mut stage = stage(move || PieChart::new(CRATES, sizes()).hidden(bound.clone()).id("c"));
  stage.click("c-legend-item-0");
  stage.pass();
  assert_eq!(hidden.get(), ["render"]);
  assert_eq!(slices(&mut stage).len(), 3, "render is not drawn");
  assert!(stage.text_quad("48%").is_some(), "layout's share of the rest: 28 of 58");
}

#[test]
fn a_slice_too_small_to_see_is_still_read_one_slice_is_the_whole_and_all_zero_says_so() {
  let mut tiny = stage(|| PieChart::new(["most", "a sliver"], ChartSeries::new("Share", [999.0, 1.0])).id("c"));
  let drawn = slices(&mut tiny);
  assert_eq!(drawn.len(), 2);
  assert_eq!(drawn[1].value.as_deref(), Some("1, <1%"), "the sliver to tooling");
  assert!(tiny.text_quad("<1%").is_some(), "and in the legend");

  let mut one = stage(|| PieChart::new(["all"], ChartSeries::new("Share", [5.0])).id("c"));
  let whole = slices(&mut one);
  assert_eq!(whole.len(), 1);
  assert_eq!(center(&whole[0]), (48.0, 48.0), "the whole disc, found at its center");
  assert!(one.text_quad("100%").is_some());

  let zero = stage(|| {
    PieChart::new(["a", "b"], ChartSeries::new("Share", [0.0, 0.0]))
      .empty_text("Nothing compiled yet")
      .id("c")
  });
  assert_eq!(
    zero.text("Nothing compiled yet").0,
    light(PaletteColor::MutedForeground)
  );
}
