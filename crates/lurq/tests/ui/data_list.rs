//! A data list: its terms and values against the board, the terms as wide as the widest, the lines under each item
//! but the last, a value that is an element, one over the other, and a theme's other anatomy.

use std::sync::{Arc, Mutex};

use lurq::{
  app::{
    App,
    theme::{PaletteColor, ThemePalette},
  },
  components::{Column, Row},
  layout::quad::{Quad, QuadContent},
  node::color::Color,
  ui::{Badge, DataList, DataListPart, DataListRenderer, DataListState, DataListView, Orientation, Recipe},
};

use super::stage::Stage;

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

/// The lines of 1 in the border's color.
fn lines(stage: &Stage) -> Vec<Quad> {
  let border = light(PaletteColor::Border);
  stage
    .quads()
    .into_iter()
    .filter(|quad| {
      quad.border.and_then(|border| border.bottom).map(|side| side.color) == Some(border)
        || (matches!(&quad.content, QuadContent::Rect { color, .. } if *color == border) && quad.height == 1.0)
    })
    .collect()
}

/// The board's data list, 300 wide.
fn board() -> DataList {
  DataList::new()
    .text("Version", "0.60.0")
    .text("Renderer", "wgpu")
    .text("License", "MIT")
    .item("Status", Badge::new("Stable").variant("success"))
    .id("list")
    .style(DataListPart::Root, |s| s.width(300.0))
}

#[test]
fn the_terms_are_muted_as_wide_as_the_widest_and_the_values_12_after_them() {
  let mut stage = Stage::new(|_| Column::new().padding(8.0).child(board()).into());
  let (x, y, width, _) = stage.bounds("list");
  assert_eq!(width, 300.0);
  assert_eq!(stage.text("Version"), (light(PaletteColor::MutedForeground), 13.0));
  assert_eq!(stage.text("0.60.0"), (light(PaletteColor::Foreground), 13.0));
  let widest = ["Version", "Renderer", "License", "Status"]
    .into_iter()
    .map(|term| stage.text_quad(term).unwrap().width)
    .fold(0.0, f32::max);
  // Every value starts at one place: 12 after the widest term.
  for value in ["0.60.0", "wgpu", "MIT"] {
    let quad = stage.text_quad(value).unwrap();
    assert!((quad.x - (x + widest + 12.0)).abs() < 1.0, "{value}: {}", quad.x - x);
  }
  assert!(stage.paints("Stable"), "a value that is an element");
  // Each term 10 under the line over it.
  let version = stage.text_quad("Version").unwrap();
  assert!(version.y >= y + 9.0 && version.y <= y + 14.0, "{}", version.y - y);
}

#[test]
fn a_line_runs_under_each_item_but_the_last() {
  let mut stage = Stage::new(|_| Column::new().padding(8.0).child(board()).into());
  let (_, y, _, height) = stage.bounds("list");
  let lines = lines(&stage);
  assert!(!lines.is_empty());
  let bottom = y + height;
  assert!(
    lines.iter().all(|line| line.y + line.height < bottom - 1.0),
    "no line under the last item"
  );
  // The lines of a term and of its value meet: they are at one height, and there are three.
  let mut heights: Vec<i32> = lines.iter().map(|line| (line.y + line.height).round() as i32).collect();
  heights.sort();
  heights.dedup();
  assert_eq!(heights.len(), 3, "{heights:?}");
}

#[test]
fn one_over_the_other_each_term_is_over_its_value() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .padding(8.0)
      .child(
        DataList::new()
          .orientation(Orientation::Vertical)
          .text("Version", "0.60.0")
          .text("Renderer", "wgpu")
          .id("list")
          .style(DataListPart::Root, |s| s.width(200.0)),
      )
      .into()
  });
  let (x, ..) = stage.bounds("list");
  let term = stage.text_quad("Version").unwrap();
  let value = stage.text_quad("0.60.0").unwrap();
  assert!((term.x - x).abs() < 0.5 && (value.x - x).abs() < 0.5);
  assert!(value.y >= term.y + term.height + 1.5, "2 under the term");
  assert_eq!(stage.text("Version").1, 12.0);
  let wgpu = stage.text_quad("wgpu").unwrap();
  assert!(wgpu.y > value.y + 20.0, "the next item under a line and 10 more");
}

#[test]
fn a_renderer_sees_the_items() {
  let seen = Arc::new(Mutex::new(Vec::new()));
  let told = seen.clone();
  let renderer: DataListRenderer = Arc::new(move |view: DataListView| {
    *told.lock().unwrap() = view
      .items
      .iter()
      .map(|item| {
        (
          item.term.to_string(),
          item.value.as_ref().map(|value| value.to_string()),
          item.element.is_some(),
        )
      })
      .collect();
    Row::new().id("other").size(10.0, 10.0).into()
  });
  let app = App::new();
  app.theme().set_renderer::<DataList>(renderer);
  let mut stage = Stage::in_app(app, |_| Column::new().child(board()).into());
  assert_eq!(stage.bounds("other").2, 10.0);
  assert_eq!(seen.lock().unwrap()[3], ("Status".to_owned(), None, true),);
  assert_eq!(
    seen.lock().unwrap()[0],
    ("Version".to_owned(), Some("0.60.0".to_owned()), false)
  );
}

/// One of the states `Horizontal` and `Vertical` is on, by the orientation: a recipe styles either.
#[test]
fn the_orientation_is_the_horizontal_or_the_vertical_state() {
  let rules = Recipe::new()
    .state(DataListState::Horizontal, DataListPart::Term, |s| {
      s.foreground(PaletteColor::Primary)
    })
    .state(DataListState::Vertical, DataListPart::Term, |s| {
      s.foreground(PaletteColor::Destructive)
    });
  let stage = Stage::new(move |_| {
    Column::new()
      .child(DataList::new().rules(&rules).text("Side", "by side"))
      .child(
        DataList::new()
          .orientation(Orientation::Vertical)
          .rules(&rules)
          .text("Over", "the other"),
      )
      .into()
  });
  assert_eq!(stage.text("Side").0, light(PaletteColor::Primary));
  assert_eq!(stage.text("Over").0, light(PaletteColor::Destructive));
}

/// `texts` and `items` add several at once, as `text` and `item` add one.
#[test]
fn texts_and_items_add_several_at_once() {
  let seen = Arc::new(Mutex::new(Vec::new()));
  let told = seen.clone();
  let renderer: DataListRenderer = Arc::new(move |view: DataListView| {
    *told.lock().unwrap() = view.items.iter().map(|item| item.term.to_string()).collect::<Vec<_>>();
    view.render()
  });
  let app = App::new();
  app.theme().set_renderer::<DataList>(renderer);
  let _stage = Stage::in_app(app, |_| {
    Column::new()
      .child(
        DataList::new()
          .texts([("Version", "0.60.0"), ("Renderer", "wgpu")])
          .items([("Status", Badge::new("Stable"))]),
      )
      .into()
  });
  assert_eq!(*seen.lock().unwrap(), ["Version", "Renderer", "Status"]);
}
