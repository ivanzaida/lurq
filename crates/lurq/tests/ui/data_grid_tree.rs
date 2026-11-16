//! A data grid's rows as a tree, each under the row its parent key names, opened and closed from the chevron in the
//! first column and from Right and Left there; and a row's detail under it, opened the same way, measured as it is
//! laid out, under the grid's windowing.

use std::sync::Arc;

use lurq::{
  components::{Column, Text},
  core::Signal,
  layout::{Constraints, Size},
  ui::{ColumnFilter, DataGrid, TableColumn, TableFilters, TextMatch},
};

use super::stage::Stage;

/// A file or a folder, under the folder `parent` names.
#[derive(Clone)]
struct Entry {
  name: &'static str,
  parent: Option<&'static str>,
  kind: &'static str,
}

fn entries() -> Vec<Entry> {
  [
    ("src", None, "Folder"),
    ("render", Some("src"), "Folder"),
    ("blur.rs", Some("render"), "Rust"),
    ("layers.rs", Some("render"), "Rust"),
    ("lib.rs", Some("src"), "Rust"),
    ("tests", None, "Folder"),
    ("grid.rs", Some("tests"), "Rust"),
  ]
  .into_iter()
  .map(|(name, parent, kind)| Entry { name, parent, kind })
  .collect()
}

fn entry_columns() -> Vec<TableColumn<Entry>> {
  vec![
    TableColumn::new("name", "Name")
      .filter_text(true)
      .text(|row: &Entry| row.name.to_owned()),
    TableColumn::new("kind", "Kind")
      .width(90.0)
      .text(|row: &Entry| row.kind.to_owned()),
  ]
}

fn tree(open: Signal<Vec<String>>, filters: Signal<TableFilters>) -> Stage {
  let mut stage = Stage::new(move |ctx| {
    Column::new()
      .size(600.0, 400.0)
      .child(
        DataGrid::new(entry_columns(), entries())
          .row_key(|row: &Entry| row.name.to_owned())
          .id("t")
          .label("Files")
          .tree(|row: &Entry| row.parent.map(str::to_owned))
          .expanded(open.clone())
          .filters(filters.clone())
          .mount(ctx),
      )
      .into()
  });
  stage
    .tree
    .set_layout_constraints_override(Some(Constraints::loose(Size::new(800.0, 600.0))));
  stage.pass();
  stage
}

fn focused(stage: &Stage) -> Option<String> {
  stage
    .tree
    .focused_element()
    .and_then(|element| element.id().map(str::to_owned))
}

/// What the row at `index` says of itself to what reads the tree.
#[cfg(any(feature = "mcp", feature = "devtools"))]
fn row(stage: &mut Stage, index: usize, what: &str) -> Option<String> {
  stage
    .tree
    .get_element_by_id_mut(&format!("t-row-{index}"))
    .and_then(|element| element.description(what))
}

fn x_of(stage: &Stage, text: &str) -> f32 {
  stage.text_quad(text).unwrap_or_else(|| panic!("{text:?} is painted")).x
}

#[test]
fn a_tree_shows_the_rows_under_an_open_row_indented_a_level_and_opens_from_the_keys() {
  let open: Signal<Vec<String>> = Signal::new(Vec::new());
  let mut stage = tree(open.clone(), Signal::new(TableFilters::new()));
  // Closed, the rows at the top alone.
  assert!(stage.paints("src") && stage.paints("tests"));
  assert!(!stage.paints("render") && !stage.paints("grid.rs"));

  #[cfg(any(feature = "mcp", feature = "devtools"))]
  {
    assert_eq!(row(&mut stage, 0, "a11y_level").as_deref(), Some("1"));
    assert_eq!(row(&mut stage, 0, "a11y_expanded").as_deref(), Some("false"));
  }

  // Right on the first cell opens the row: the rows under it come after it, indented 18.
  stage.click("t-cell-0-0");
  stage.key("ArrowRight");
  assert_eq!(open.get(), ["src"]);
  assert!(stage.paints("render") && stage.paints("lib.rs"));
  assert!(!stage.paints("blur.rs"), "the rows under a closed row are not built");
  assert_eq!(x_of(&stage, "render") - x_of(&stage, "src"), 18.0);
  #[cfg(any(feature = "mcp", feature = "devtools"))]
  {
    assert_eq!(row(&mut stage, 0, "a11y_expanded").as_deref(), Some("true"));
    assert_eq!(row(&mut stage, 1, "a11y_level").as_deref(), Some("2"));
  }
  let (_, src_y, ..) = stage.bounds("t-row-0");
  let (_, render_y, ..) = stage.bounds("t-row-1");
  let (_, tests_y, ..) = stage.bounds("t-row-5");
  assert!(
    src_y < render_y && render_y < tests_y,
    "each row after the row it is under"
  );

  // The focus stays on the cell: Right again goes to the next cell, as on a row that is open.
  stage.key("ArrowRight");
  assert_eq!(open.get(), ["src"]);

  // Left on a row that is closed goes to the row it is under; Left on an open row closes it.
  stage.click("t-cell-1-0");
  stage.key("ArrowLeft");
  assert_eq!(
    focused(&stage).as_deref(),
    Some("t-cell-0-0"),
    "Left went to the row it is under"
  );
  stage.key("ArrowLeft");
  assert!(open.get().is_empty());
  assert!(!stage.paints("render"));
}

#[test]
fn a_chevron_opens_its_row_and_a_filter_keeps_the_rows_a_kept_row_is_under() {
  let open: Signal<Vec<String>> = Signal::new(vec!["src".to_owned(), "render".to_owned()]);
  let filters = Signal::new(TableFilters::new());
  let mut stage = tree(open.clone(), filters.clone());
  // Two levels open: the files of render are 36 in.
  assert_eq!(x_of(&stage, "blur.rs") - x_of(&stage, "src"), 36.0);
  // A leaf has the chevron's room: its name lines up with a folder's at its level.
  assert_eq!(x_of(&stage, "lib.rs"), x_of(&stage, "render"));

  // A filter that keeps blur.rs keeps src and render, which it is under, and nothing else.
  filters.set(TableFilters::new().with(
    "name",
    ColumnFilter::Text {
      how: TextMatch::Contains,
      text: "blur".to_owned(),
    },
  ));
  stage.pass();
  assert!(stage.paints("src") && stage.paints("render") && stage.paints("blur.rs"));
  assert!(!stage.paints("layers.rs") && !stage.paints("lib.rs") && !stage.paints("tests"));

  // A press of the chevron of src closes it, and does not select or move anything else.
  filters.set(TableFilters::new());
  stage.pass();
  let (x, y, _, height) = stage.bounds("t-cell-0-0");
  // The chevron is the first thing in the cell, after the row's padding.
  let chevron = stage
    .quads()
    .into_iter()
    .filter(|quad| quad.x >= x && quad.x < x + 40.0 && quad.y >= y && quad.y + quad.height <= y + height)
    .min_by(|a, b| a.x.total_cmp(&b.x))
    .expect("the chevron is painted");
  let (cx, cy) = (chevron.x + chevron.width / 2.0, chevron.y + chevron.height / 2.0);
  stage.tree.mouse_move(cx, cy);
  stage.tree.mouse_down(cx, cy, lurq::app::events::MouseButton::Left);
  stage.tree.mouse_up(cx, cy, lurq::app::events::MouseButton::Left);
  stage.pass();
  assert_eq!(open.get(), ["render"]);
  assert!(!stage.paints("render"));
}

#[test]
fn a_detail_opens_under_its_row_as_high_as_it_is_and_closes_from_the_keys() {
  let open: Signal<Vec<usize>> = Signal::new(Vec::new());
  let shown = open.clone();
  let rows: Arc<Vec<usize>> = Arc::new((0..500).collect());
  let mut stage = Stage::new(move |ctx| {
    Column::new()
      .size(600.0, 400.0)
      .child(
        DataGrid::new_shared(
          vec![
            TableColumn::new("n", "Number").text(|row: &usize| format!("Job {row}")),
            TableColumn::new("state", "State")
              .width(120.0)
              .text(|row: &usize| if row % 2 == 0 { "Passed" } else { "Failed" }.to_owned()),
          ],
          rows.clone(),
        )
        .id("t")
        .label("Jobs")
        .height(360.0)
        .detail(|row: &usize| {
          Column::new()
            .child(Text::new(&format!("The log of job {row}")))
            .child(Text::new("Built in 12 s"))
            .child(Text::new("Tested in 30 s"))
            .into()
        })
        .details(shown.clone())
        .mount(ctx),
      )
      .into()
  });
  stage
    .tree
    .set_layout_constraints_override(Some(Constraints::loose(Size::new(800.0, 600.0))));
  stage.pass();
  assert!(!stage.paints("The log of job 0"));
  let (_, row_y, _, row_height) = stage.bounds("t-row-0");
  let (_, next_y, ..) = stage.bounds("t-row-1");
  assert_eq!(next_y, row_y + row_height);

  // Right on the first cell opens the detail under the row; the next row comes after it, as high as it is.
  stage.click("t-cell-0-0");
  stage.key("ArrowRight");
  stage.pass();
  assert_eq!(open.get(), [0]);
  let (_, detail_y, detail_width, detail_height) = stage.bounds("t-detail-0");
  assert_eq!(detail_y, row_y + row_height, "under its row");
  assert!(detail_height > 50.0, "as high as its three lines: {detail_height}");
  let (grid_x, _, grid_width, _) = stage.bounds("t");
  assert!(
    detail_width <= grid_width && detail_width > grid_width - 20.0,
    "as wide as the view: {detail_width}"
  );
  let _ = grid_x;
  let (_, next_y, ..) = stage.bounds("t-row-1");
  assert_eq!(next_y, detail_y + detail_height, "the next row after it");
  // Down goes to the next row, past the detail.
  stage.key("ArrowDown");
  assert_eq!(focused(&stage).as_deref(), Some("t-cell-1-0"));

  // Scrolled to the end and back over the rows built as they come, the detail's line keeps its height.
  stage
    .tree
    .scroll(300.0, 200.0, 0.0, -100_000.0, lurq::app::events::ScrollPhase::Scroll);
  stage.pass();
  assert!(stage.paints("Job 499"));
  stage
    .tree
    .scroll(300.0, 200.0, 0.0, 100_000.0, lurq::app::events::ScrollPhase::Scroll);
  stage.pass();
  assert!(stage.paints("The log of job 0"));

  // Left on the first cell closes it.
  stage.click("t-cell-0-0");
  stage.key("ArrowLeft");
  stage.pass();
  assert!(open.get().is_empty());
  assert!(!stage.paints("The log of job 0"));
}
