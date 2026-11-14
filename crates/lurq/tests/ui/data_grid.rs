//! A data grid: the board's Data Grid (its header, its rows, its checkboxes, a sorted column, a selected row, a
//! number, the footer), the focus on a cell and its keys, editing a cell in place (a text, a number, a choice, a
//! box ticked), an edit the app refuses, a grid that is read only, ten thousand rows of fifty columns built in
//! view only, the density, and what tooling reads.

use std::sync::{Arc, Mutex};

use lurq::{
  app::theme::{Density, PaletteColor, ThemePalette},
  components::Column,
  core::Signal,
  layout::{
    Constraints, Size,
    quad::{Quad, QuadContent},
  },
  node::{Element, color::Color},
  ui::{CellEditor, CellValue, DataGrid, TableColumn, TableSort},
};

use super::stage::Stage;

const ARROW_UP: &str = "\u{E007}";

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

#[derive(Clone, Debug, PartialEq)]
struct File {
  name: String,
  status: usize,
  size: f64,
  shared: bool,
}

const STATUSES: [&str; 3] = ["Building", "Passing", "Failed"];

/// The board's files.
fn files() -> Vec<File> {
  [
    ("blur.rs", 0, 12.0, true),
    ("canvas.rs", 1, 48.0, false),
    ("theme.rs", 1, 9.0, false),
  ]
  .into_iter()
  .map(|(name, status, size, shared)| File {
    name: name.to_owned(),
    status,
    size,
    shared,
  })
  .collect()
}

fn columns() -> Vec<TableColumn<File>> {
  vec![
    TableColumn::new("name", "Name")
      .text(|row: &File| row.name.clone())
      .mono(true)
      .sortable(true)
      .editor(CellEditor::text(|row: &File| row.name.clone())),
    TableColumn::new("status", "Status")
      .width(110.0)
      .text(|row: &File| STATUSES[row.status].to_owned())
      .editor(CellEditor::choice(STATUSES, |row: &File| row.status)),
    TableColumn::new("size", "Size")
      .width(70.0)
      .numeric()
      .text(|row: &File| format!("{} KB", row.size))
      .editor(CellEditor::number(|row: &File| row.size)),
    TableColumn::new("shared", "Shared")
      .width(70.0)
      .editor(CellEditor::checkbox(|row: &File| row.shared)),
  ]
}

/// What the grid was told: each edit, as the app took it or not.
type Edits = Arc<Mutex<Vec<(String, String, CellValue)>>>;

/// A grid of `files` the app keeps, 640 wide and as high as its rows, that takes every edit but an empty name.
fn grid(rows: Signal<Vec<File>>, edits: Edits, read_only: bool) -> Stage {
  let mut stage = Stage::new(move |ctx| {
    let (store, told) = (rows.clone(), edits.clone());
    Column::new()
      .size(640.0, 300.0)
      .child(
        DataGrid::new(columns(), rows.get())
          .row_key(|row: &File| row.name.clone())
          .id("g")
          .label("Files")
          .read_only(read_only)
          .on_edit(move |edit| {
            told
              .lock()
              .unwrap()
              .push((edit.row.clone(), edit.column.to_owned(), edit.value.clone()));
            let mut files = store.get();
            let Some(file) = files.iter_mut().find(|file| &file.name == edit.row) else {
              return false;
            };
            match (edit.column, &edit.value) {
              ("name", CellValue::Text(name)) if name.is_empty() => return false,
              ("name", CellValue::Text(name)) => file.name = name.clone(),
              ("status", CellValue::Choice(status)) => file.status = *status,
              ("size", CellValue::Number(size)) => file.size = *size,
              ("shared", CellValue::Checked(shared)) => file.shared = *shared,
              _ => return false,
            }
            store.set(files);
            true
          })
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

fn key(stage: &mut Stage, key: &str) {
  modified(stage, key, false, false);
}

fn modified(stage: &mut Stage, key: &str, shift: bool, ctrl: bool) {
  stage.tree.key_down(key.to_owned(), key.to_owned(), shift, ctrl, false);
  stage.tree.key_up(key.to_owned(), key.to_owned(), shift, ctrl, false);
  stage.pass();
  stage.pass();
}

/// Tab into the grid's cells: past its header, a stop of its own before the rows.
fn into_cells(stage: &mut Stage) {
  key(stage, "Tab");
  if focused(stage).is_some_and(|id| id.starts_with("g-header")) {
    key(stage, "Tab");
  }
}

fn texts(stage: &Stage, content: &str) -> Vec<Quad> {
  stage
    .quads()
    .into_iter()
    .filter(|quad| matches!(&quad.content, QuadContent::Text { text, .. } if text == content))
    .collect()
}

#[test]
fn a_grid_is_the_boards_data_grid_with_its_checkboxes_a_sorted_column_a_selected_row_and_its_footer() {
  let selected = Signal::new(vec!["blur.rs".to_owned()]);
  let scene_selected = selected.clone();
  let mut stage = Stage::new(move |ctx| {
    Column::new()
      .size(640.0, 300.0)
      .child(
        DataGrid::new(columns(), files())
          .row_key(|row: &File| row.name.clone())
          .id("g")
          .selected(scene_selected.clone())
          .sort_controlled(Some(TableSort::ascending("name")), |_| {})
          .total(24)
          .count(true)
          .mount(ctx),
      )
      .into()
  });
  stage
    .tree
    .set_layout_constraints_override(Some(Constraints::loose(Size::new(800.0, 600.0))));
  stage.pass();
  let (x, y, width, _) = stage.bounds("g");
  assert_eq!(width, 640.0);
  for index in 0..3 {
    let (_, row_y, _, height) = stage.bounds(&format!("g-row-{index}"));
    assert_eq!(height, 44.0, "rows of 44");
    assert!(
      (row_y - (y + 40.0 + 44.0 * index as f32)).abs() < 1.01,
      "under a header of 40"
    );
  }
  // The selected row on the highlight, the board's `$accent`; a sorted header in the foreground with its arrow.
  assert_eq!(stage.fill_color("g-row-0"), Some(light(PaletteColor::Highlight)));
  assert_eq!(stage.text("Name").0, light(PaletteColor::Foreground));
  assert!(!texts(&stage, ARROW_UP).is_empty(), "the arrow of the sort");
  assert_eq!(stage.text("Status").0, light(PaletteColor::MutedForeground));
  // The name in the mono type, the size a number of 12 in the muted foreground at its column's end.
  assert_eq!(stage.text("12 KB"), (light(PaletteColor::MutedForeground), 12.0));
  // The checkbox column: 16 at the start, 12 to the name.
  let name = texts(&stage, "blur.rs").pop().unwrap();
  assert!(
    (name.x - (x + 1.0 + 16.0 + 16.0 + 12.0)).abs() < 1.51,
    "the name at 44: {}",
    name.x - x
  );
  assert!(stage.paints("1 of 24 rows selected"), "the footer counts the rows");
}

#[test]
fn the_focus_is_on_a_cell_and_the_arrows_home_end_and_ctrl_move_it_by_cell() {
  let mut stage = grid(Signal::new(files()), Edits::default(), false);
  key(&mut stage, "Tab");
  assert_eq!(
    focused(&stage).as_deref(),
    Some("g-header-name"),
    "the header is a stop before the rows"
  );
  key(&mut stage, "Tab");
  assert_eq!(
    focused(&stage).as_deref(),
    Some("g-cell-0-0"),
    "Tab enters the rows at the first cell"
  );
  key(&mut stage, "ArrowRight");
  assert_eq!(focused(&stage).as_deref(), Some("g-cell-0-1"));
  key(&mut stage, "ArrowDown");
  assert_eq!(
    focused(&stage).as_deref(),
    Some("g-cell-1-1"),
    "the same column of the next row"
  );
  key(&mut stage, "End");
  assert_eq!(focused(&stage).as_deref(), Some("g-cell-1-3"), "End is the row's last");
  key(&mut stage, "Home");
  assert_eq!(focused(&stage).as_deref(), Some("g-cell-1-0"));
  modified(&mut stage, "End", false, true);
  assert_eq!(
    focused(&stage).as_deref(),
    Some("g-cell-2-3"),
    "Ctrl+End, the last cell"
  );
  modified(&mut stage, "Home", false, true);
  assert_eq!(focused(&stage).as_deref(), Some("g-cell-0-0"), "Ctrl+Home, the first");
  key(&mut stage, "ArrowUp");
  assert_eq!(
    focused(&stage).as_deref(),
    Some("g-cell-0-0"),
    "nothing above the first row"
  );
}

#[test]
fn tab_goes_on_through_the_cells_of_a_row_and_out_of_the_grid_from_its_last() {
  let mut stage = grid(Signal::new(files()), Edits::default(), false);
  into_cells(&mut stage);
  key(&mut stage, "Tab");
  assert_eq!(focused(&stage).as_deref(), Some("g-cell-0-1"));
  modified(&mut stage, "Tab", true, false);
  assert_eq!(
    focused(&stage).as_deref(),
    Some("g-cell-0-0"),
    "Shift+Tab, the cell before"
  );
  for _ in 0..3 {
    key(&mut stage, "Tab");
  }
  assert_eq!(focused(&stage).as_deref(), Some("g-cell-0-3"));
  key(&mut stage, "Tab");
  let after = focused(&stage);
  assert!(
    after.as_deref().is_none_or(|id| !id.starts_with("g-cell")),
    "out of the grid: {after:?}"
  );
}

#[test]
fn enter_edits_a_cell_in_place_and_enter_commits_it_tells_the_app_and_goes_to_the_cell_under_it() {
  let (rows, edits) = (Signal::new(files()), Edits::default());
  let mut stage = grid(rows.clone(), edits.clone(), false);
  into_cells(&mut stage);
  key(&mut stage, "Enter");
  assert_eq!(focused(&stage).as_deref(), Some("g-editor"), "the editor has the focus");
  assert!(stage.paints("blur.rs"), "it starts from the cell's text");
  stage.tree.text_input_replace(0..7, "glow.rs");
  stage.pass();
  key(&mut stage, "Enter");
  assert_eq!(
    *edits.lock().unwrap(),
    [(
      "blur.rs".to_owned(),
      "name".to_owned(),
      CellValue::Text("glow.rs".to_owned())
    )]
  );
  assert_eq!(rows.get()[0].name, "glow.rs", "the app took it");
  assert!(stage.paints("glow.rs"), "the grid shows the app's rows");
  assert_eq!(focused(&stage).as_deref(), Some("g-cell-1-0"), "the cell under it");
}

#[test]
fn escape_puts_a_cell_back_and_tells_nothing() {
  let (rows, edits) = (Signal::new(files()), Edits::default());
  let mut stage = grid(rows.clone(), edits.clone(), false);
  into_cells(&mut stage);
  key(&mut stage, "F2");
  assert_eq!(focused(&stage).as_deref(), Some("g-editor"));
  stage.tree.text_input_replace(0..7, "x");
  stage.pass();
  key(&mut stage, "Escape");
  assert!(edits.lock().unwrap().is_empty());
  assert_eq!(rows.get(), files());
  assert_eq!(focused(&stage).as_deref(), Some("g-cell-0-0"), "back on its cell");
  assert!(stage.paints("blur.rs"));
}

#[test]
fn a_letter_typed_on_a_cell_starts_its_edit_and_tab_commits_it_and_goes_on() {
  let (rows, edits) = (Signal::new(files()), Edits::default());
  let mut stage = grid(rows.clone(), edits.clone(), false);
  into_cells(&mut stage);
  key(&mut stage, "ArrowRight");
  key(&mut stage, "ArrowRight");
  key(&mut stage, "7");
  assert_eq!(focused(&stage).as_deref(), Some("g-editor"), "the size is edited");
  key(&mut stage, "Tab");
  assert_eq!(
    edits.lock().unwrap().last().map(|edit| edit.2.clone()),
    Some(CellValue::Number(7.0)),
    "the edit starts from what was typed"
  );
  assert_eq!(rows.get()[0].size, 7.0);
  assert_eq!(focused(&stage).as_deref(), Some("g-cell-0-3"), "Tab goes on");
}

#[test]
fn an_edit_the_app_refuses_leaves_the_editor_open() {
  let (rows, edits) = (Signal::new(files()), Edits::default());
  let mut stage = grid(rows.clone(), edits.clone(), false);
  into_cells(&mut stage);
  key(&mut stage, "Enter");
  stage.tree.text_input_replace(0..7, "");
  stage.pass();
  key(&mut stage, "Enter");
  assert_eq!(edits.lock().unwrap().len(), 1, "the app was told");
  assert_eq!(rows.get(), files(), "and refused it");
  assert_eq!(
    focused(&stage).as_deref(),
    Some("g-editor"),
    "the editor stays, with the focus"
  );
  key(&mut stage, "Escape");
  assert_eq!(focused(&stage).as_deref(), Some("g-cell-0-0"));
  assert!(stage.paints("blur.rs"));
}

#[test]
fn space_ticks_a_box_in_its_cell_and_a_choice_commits_as_it_is_chosen() {
  let (rows, edits) = (Signal::new(files()), Edits::default());
  let mut stage = grid(rows.clone(), edits.clone(), false);
  into_cells(&mut stage);
  key(&mut stage, "End");
  key(&mut stage, " ");
  assert_eq!(
    *edits.lock().unwrap(),
    [("blur.rs".to_owned(), "shared".to_owned(), CellValue::Checked(false))]
  );
  assert!(!rows.get()[0].shared);
  assert_eq!(
    focused(&stage).as_deref(),
    Some("g-cell-0-3"),
    "the box turns in its cell"
  );

  // The status: Enter opens its select; a choice commits.
  key(&mut stage, "Home");
  key(&mut stage, "ArrowRight");
  key(&mut stage, "Enter");
  assert_eq!(focused(&stage).as_deref(), Some("g-editor"));
  key(&mut stage, "Enter");
  key(&mut stage, "ArrowDown");
  key(&mut stage, "ArrowDown");
  key(&mut stage, "Enter");
  assert_eq!(
    edits.lock().unwrap().last().cloned(),
    Some(("blur.rs".to_owned(), "status".to_owned(), CellValue::Choice(2)))
  );
  assert_eq!(rows.get()[0].status, 2);
  assert!(stage.paints("Failed"));
}

#[test]
fn a_read_only_grid_moves_the_focus_and_edits_nothing() {
  let (rows, edits) = (Signal::new(files()), Edits::default());
  let mut stage = grid(rows.clone(), edits.clone(), true);
  into_cells(&mut stage);
  key(&mut stage, "Enter");
  key(&mut stage, "x");
  key(&mut stage, "End");
  key(&mut stage, " ");
  assert!(edits.lock().unwrap().is_empty());
  assert_ne!(focused(&stage).as_deref(), Some("g-editor"));
  assert_eq!(focused(&stage).as_deref(), Some("g-cell-0-3"));
}

#[test]
fn a_double_press_of_a_cell_edits_it() {
  let mut stage = grid(Signal::new(files()), Edits::default(), false);
  let (x, y) = {
    let quad = texts(&stage, "canvas.rs").pop().unwrap();
    (quad.x + 4.0, quad.y + quad.height / 2.0)
  };
  for _ in 0..2 {
    stage.tree.mouse_move(x, y);
    stage.tree.mouse_down(x, y, lurq::app::events::MouseButton::Left);
    stage.tree.mouse_up(x, y, lurq::app::events::MouseButton::Left);
    stage.pass();
  }
  stage.pass();
  assert_eq!(focused(&stage).as_deref(), Some("g-editor"));
}

#[test]
fn ten_thousand_rows_of_fifty_columns_build_those_in_view_and_page_down_keeps_the_column() {
  let rows: Arc<Vec<File>> = Arc::new(
    (0..10_000)
      .map(|index| File {
        name: format!("file {index}"),
        status: index % 3,
        size: index as f64,
        shared: false,
      })
      .collect(),
  );
  let mut stage = Stage::new(move |ctx| {
    let mut columns = vec![
      TableColumn::new("name", "Name")
        .width(140.0)
        .text(|row: &File| row.name.clone()),
    ];
    columns.extend((1..50).map(|index| {
      TableColumn::new(&format!("c{index}"), &format!("C{index}"))
        .width(100.0)
        .text(move |row: &File| format!("{}.{index}", row.size))
    }));
    Column::new()
      .size(640.0, 300.0)
      .child(
        DataGrid::new_shared(columns, rows.clone())
          .id("g")
          .height(300.0)
          .mount(ctx),
      )
      .into()
  });
  stage
    .tree
    .set_layout_constraints_override(Some(Constraints::loose(Size::new(800.0, 600.0))));
  stage.pass();
  stage.pass();
  let cells = stage
    .quads()
    .iter()
    .filter(|quad| matches!(&quad.content, QuadContent::Text { text, .. } if text.contains('.')))
    .count();
  assert!(cells < 200, "about a screen of cells, not 500 000: {cells}");
  into_cells(&mut stage);
  key(&mut stage, "ArrowRight");
  key(&mut stage, "ArrowRight");
  key(&mut stage, "PageDown");
  let at = focused(&stage).unwrap();
  assert!(at.starts_with("g-cell-") && at.ends_with("-2"), "the same column: {at}");
  modified(&mut stage, "End", false, true);
  assert_eq!(
    focused(&stage).as_deref(),
    Some("g-cell-9999-49"),
    "the last cell of the last row"
  );
  assert!(stage.paints("9999.49"));
}

#[test]
fn a_compact_grid_has_rows_of_36() {
  let mut stage = Stage::new(|ctx| {
    Column::new()
      .size(640.0, 300.0)
      .child(
        DataGrid::new(columns(), files())
          .id("g")
          .density(Density::Compact)
          .mount(ctx),
      )
      .into()
  });
  stage
    .tree
    .set_layout_constraints_override(Some(Constraints::loose(Size::new(800.0, 600.0))));
  stage.pass();
  assert_eq!(stage.bounds("g-row-0").3, 36.0);
}

#[test]
fn a_name_longer_than_its_column_is_cut_with_an_ellipsis_in_its_header() {
  let mut stage = Stage::new(|ctx| {
    let columns = vec![
      TableColumn::new("name", "A title much longer than the column it is the header of")
        .width(120.0)
        .text(|row: &File| row.name.clone()),
      TableColumn::new("size", "Size").text(|row: &File| format!("{} KB", row.size)),
    ];
    Column::new()
      .size(640.0, 300.0)
      .child(DataGrid::new(columns, files()).id("g").mount(ctx))
      .into()
  });
  stage
    .tree
    .set_layout_constraints_override(Some(Constraints::loose(Size::new(800.0, 600.0))));
  stage.pass();
  let size = texts(&stage, "Size").pop().unwrap();
  let (x, ..) = stage.bounds("g");
  assert!(
    (size.x - (x + 1.0 + 16.0 + 120.0 + 16.0)).abs() < 1.5,
    "the next header where its track is"
  );
  let cut = stage.cut_text_quad("A title much longer than the column it is the header of");

  assert!(
    cut.is_some_and(|quad| quad.x + quad.width <= size.x),
    "cut before the next column"
  );
}

#[cfg(any(feature = "mcp", feature = "devtools"))]
#[test]
fn tooling_reads_a_grid_of_rows_of_gridcells_with_their_places_and_whether_they_are_read_only() {
  let mut stage = grid(Signal::new(files()), Edits::default(), false);
  let mut said = |id: &str, name: &str| {
    stage
      .tree
      .get_element_by_id_mut(id)
      .and_then(|element| element.description(name))
  };
  assert_eq!(said("g", "a11y_role").as_deref(), Some("grid"));
  assert_eq!(said("g", "a11y_name").as_deref(), Some("Files"));
  assert_eq!(said("g-row-0", "a11y_role").as_deref(), Some("row"));
  assert_eq!(said("g-row-0", "a11y_rowindex").as_deref(), Some("2"));
  assert_eq!(said("g-cell-0-1", "a11y_role").as_deref(), Some("gridcell"));
  assert_eq!(said("g-cell-0-1", "a11y_colindex").as_deref(), Some("2"));
  assert_eq!(said("g-cell-0-1", "a11y_readonly").as_deref(), Some("false"));
  let mut stage = grid(Signal::new(files()), Edits::default(), true);
  let said = stage
    .tree
    .get_element_by_id_mut("g-cell-0-0")
    .and_then(|element| element.description("a11y_readonly"));
  assert_eq!(said.as_deref(), Some("true"));
}
