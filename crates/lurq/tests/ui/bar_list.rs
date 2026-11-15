//! A bar list: the board's card and rows (the name, the track, the bar, the value), the scale rounded up from the
//! largest value, the values written with the decimals they need, a value under 0 and one over the scale, a long
//! name, no rows, rows that are pressed as one stop of Tab, what tooling reads, and a theme's rules and renderer.

use std::sync::{Arc, Mutex};

use lurq::{
  app::theme::{PaletteColor, ThemePalette},
  components::{Column, Text},
  layout::{
    quad::{Quad, QuadContent},
    text_style::FontWeight,
  },
  node::color::Color,
  ui::{BarList, BarListItem, BarListPart, BarListRenderer, BarListView, Button},
};

use super::stage::Stage;

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

/// The filled rectangles of `color`.
fn rects(stage: &Stage, color: Color) -> Vec<Quad> {
  stage
    .quads()
    .into_iter()
    .filter(|quad| matches!(&quad.content, QuadContent::Rect { color: fill, .. } if *fill == color))
    .collect()
}

fn focused(stage: &Stage) -> Option<String> {
  stage
    .tree
    .focused_element()
    .and_then(|element| element.id().map(str::to_owned))
}

/// The board's list: the slowest tests.
fn board() -> BarList {
  BarList::new([
    BarListItem::new("text_pipeline", 9.2).id("a"),
    BarListItem::new("canvas_mesh", 7.0).id("b"),
    BarListItem::new("layout_flex", 4.7).id("c"),
    BarListItem::new("theme_roles", 2.5).id("d"),
  ])
  .title("Slowest tests")
  .description("Seconds")
  .id("list")
}

/// A stage of `list` in a column of the board's 300.
fn stage_of(list: impl Fn() -> BarList + Send + Sync + 'static) -> Stage {
  Stage::new(move |_| Column::new().width(300.0).child(list()).into())
}

#[test]
fn the_card_holds_rows_of_a_name_a_bar_and_a_value() {
  let mut stage = stage_of(board);
  let (x, y, width, _) = stage.bounds("list");
  assert_eq!(width, 300.0);
  // The card: its fill, 20 in.
  assert_eq!(stage.fill_color("list"), Some(light(PaletteColor::Card)));
  let title = stage.text_quad("Slowest tests").unwrap();
  assert!((title.x - (x + 20.0)).abs() < 0.5 && (title.y - (y + 20.0)).abs() < 2.0);
  assert_eq!(stage.text("Slowest tests").1, 15.0);
  assert_eq!(stage.text("Seconds"), (light(PaletteColor::MutedForeground), 12.0));

  // A row: the name in a column of 92, the bar 10 after it, 14 high in a track of 26, the value after the track.
  let (_, row_y, row_width, row_height) = stage.bounds("a");
  assert_eq!(row_height, 26.0);
  assert!((row_width - 260.0).abs() < 0.5);
  let name = stage.text_quad("text_pipeline").unwrap();
  assert!((name.x - (x + 20.0)).abs() < 0.5);
  assert_eq!(stage.text("text_pipeline").1, 11.0);
  assert!(stage.font_of("text_pipeline").to_lowercase().contains("mono"));
  let bars = rects(&stage, light(PaletteColor::Chart1));
  assert_eq!(bars.len(), 4);
  assert!((bars[0].x - (x + 20.0 + 92.0 + 10.0)).abs() < 0.5, "{}", bars[0].x - x);
  assert_eq!(bars[0].height, 14.0);
  assert!((bars[0].y - (row_y + 6.0)).abs() < 0.5, "in the middle of its track");
  // Rows 16 apart.
  let (_, b_y, ..) = stage.bounds("b");
  assert!((b_y - (row_y + 26.0 + 16.0)).abs() < 0.5);

  // One scale, from 0 to 10, the largest value rounded up: 9.2 fills 92% of its track, 2.5 a quarter.
  let value = stage.text_quad("9.2").unwrap();
  let track = value.x - 10.0 - bars[0].x;
  assert!(
    (bars[0].width - track * 0.92).abs() < 0.5,
    "{} of {track}",
    bars[0].width
  );
  let quarter = stage.text_quad("2.5").unwrap();
  let quarter_track = quarter.x - 10.0 - bars[3].x;
  assert!((bars[3].width - quarter_track * 0.25).abs() < 0.5);
  assert_eq!(stage.text("9.2"), (light(PaletteColor::MutedForeground), 11.0));
  // Written with the decimals the most precise value needs: 7 is 7.0.
  assert!(stage.paints("7.0"));
}

#[test]
fn the_scale_values_and_names_that_do_not_fit() {
  // Under 0 no bar, over the scale a full track, the app's scale and its words.
  let mut stage = stage_of(|| {
    BarList::new([
      BarListItem::new("a_test_with_a_name_much_longer_than_its_column", 12.0).id("long"),
      BarListItem::new("negative", -3.0),
      BarListItem::new("half", 5.0),
    ])
    .max(10.0)
    .value_text(|seconds| format!("{seconds} s"))
  });
  let bars = rects(&stage, light(PaletteColor::Chart1));
  assert_eq!(bars.len(), 2, "the negative value has no bar");
  let full = stage.text_quad("12 s").unwrap();
  assert!(
    (bars[0].x + bars[0].width - (full.x - 10.0)).abs() < 0.5,
    "over the scale: the whole track"
  );
  let cut = stage
    .cut_text_quad("a_test_with_a_name_much_longer_than_its_column")
    .unwrap();
  assert!(cut.width <= 92.5, "the name keeps to its column: {}", cut.width);
  assert!(stage.paints("-3 s"), "the value says what it is");
  let (_, _, width, _) = stage.bounds("long");
  assert!((width - 260.0).abs() < 0.5, "the row keeps its width");

  // A row's own color, and the integers written as they are.
  let stage = stage_of(|| {
    BarList::new([
      BarListItem::new("one", 3).color(PaletteColor::Chart3),
      BarListItem::new("two", 4),
    ])
  });
  assert_eq!(rects(&stage, light(PaletteColor::Chart3)).len(), 1);
  assert!(stage.paints("3") && stage.paints("4"));

  // No rows: what an empty list says.
  let stage = stage_of(|| BarList::new(Vec::<BarListItem>::new()).empty_text("Nothing ran"));
  assert!(stage.paints("Nothing ran"));
  let stage = stage_of(|| BarList::new(Vec::<BarListItem>::new()).framed(false));
  assert!(stage.paints("No data"));
}

/// The list's `on_select` makes every row pressed and is told the row's key or name, after the row's own handler;
/// the rows are one stop of Tab, the arrows between them past a disabled one.
#[test]
fn rows_that_are_pressed_are_one_stop_and_tell_which() {
  let told: Arc<Mutex<Vec<String>>> = Arc::default();
  let heard = told.clone();
  let mut stage = Stage::new(move |_| {
    let (own, list) = (heard.clone(), heard.clone());
    Column::new()
      .width(300.0)
      .child(Button::new("Before").id("before"))
      .child(
        BarList::new([
          BarListItem::new("text_pipeline", 9.2)
            .on_select(move || own.lock().unwrap().push("own".to_owned()))
            .id("a"),
          BarListItem::new("canvas_mesh", 7.0).key("mesh").id("b"),
          BarListItem::new("layout_flex", 4.7).disabled(true).id("c"),
          BarListItem::new("theme_roles", 2.5).id("d"),
        ])
        .on_select(move |key| list.lock().unwrap().push(key.to_owned())),
      )
      .child(Button::new("After").id("after"))
      .into()
  });
  stage.click("a");
  assert_eq!(*told.lock().unwrap(), ["own", "text_pipeline"]);
  stage.click("b");
  assert_eq!(told.lock().unwrap().last().map(String::as_str), Some("mesh"), "its key");
  stage.click("c");
  assert_eq!(told.lock().unwrap().len(), 3, "a disabled row takes nothing");

  stage.click("before");
  stage.key("Tab");
  assert_eq!(focused(&stage).as_deref(), Some("a"), "into the rows at the first");
  stage.key("Home");
  assert_eq!(focused(&stage).as_deref(), Some("a"));
  stage.key("ArrowDown");
  stage.key("ArrowDown");
  assert_eq!(focused(&stage).as_deref(), Some("d"), "past the disabled one");
  stage.key("Enter");
  assert_eq!(told.lock().unwrap().last().map(String::as_str), Some("theme_roles"));
  stage.key("Tab");
  assert_eq!(focused(&stage).as_deref(), Some("after"), "one stop");
}

#[test]
fn a_rule_and_a_renderer_of_the_theme_change_it() {
  let stage = stage_of(|| board().style(BarListPart::Bar, |s| s.height(6.0)));
  assert!(
    rects(&stage, light(PaletteColor::Chart1))
      .iter()
      .all(|bar| bar.height == 6.0)
  );
  let stage = stage_of(|| board().color(PaletteColor::Chart2));
  assert_eq!(rects(&stage, light(PaletteColor::Chart2)).len(), 4);

  let app = lurq::app::App::new();
  let renderer: BarListRenderer = Arc::new(|view: BarListView| {
    let rows: Vec<lurq::node::Element> = view
      .rows
      .iter()
      .map(|row| Text::new(&format!("{} = {}", row.name, row.value_text)).into())
      .collect();
    Column::new().with_children(rows).into()
  });
  app.theme().set_renderer::<BarList>(renderer);
  let stage = Stage::in_app(app, |_| board().into());
  assert!(stage.paints("text_pipeline = 9.2"));
  let _ = FontWeight::Normal;
}

/// A bar list is a figure called by its title, holding a list whose rows say their name and value.
#[cfg(any(feature = "mcp", feature = "devtools"))]
#[test]
fn a_bar_list_tells_tooling_its_rows() {
  let mut stage = stage_of(board);
  let mut said = |id: &str, name: &str| {
    stage
      .tree
      .get_element_by_id_mut(id)
      .and_then(|element| element.description(name))
  };
  assert_eq!(said("list", "a11y_role").as_deref(), Some("figure"));
  assert_eq!(said("list", "a11y_name").as_deref(), Some("Slowest tests"));
  assert_eq!(said("list", "a11y_description").as_deref(), Some("Seconds"));
  assert_eq!(said("a", "a11y_role").as_deref(), Some("listitem"));
  assert_eq!(said("a", "a11y_name").as_deref(), Some("text_pipeline"));
  assert_eq!(said("a", "a11y_valuetext").as_deref(), Some("9.2"));
}

/// A row follows the theme's density, its bar keeping its 14.
#[test]
fn a_row_follows_the_density() {
  for (density, height) in [
    (lurq::app::theme::Density::Compact, 18.0),
    (lurq::app::theme::Density::Comfortable, 34.0),
    (lurq::app::theme::Density::Touch, 38.0),
  ] {
    let app = lurq::app::App::new();
    app.theme().set_density(density);
    let mut stage = Stage::in_app(app, |_| Column::new().width(300.0).child(board()).into());
    assert_eq!(stage.bounds("a").3, height, "{density:?}");
    assert!(
      rects(&stage, light(PaletteColor::Chart1))
        .iter()
        .all(|bar| bar.height == 14.0)
    );
  }
}
