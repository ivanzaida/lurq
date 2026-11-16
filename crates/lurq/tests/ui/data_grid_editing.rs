//! Editing a data grid's cells, each kind of editor through its whole flow: by the keys and by the pointer, the
//! editor in its cell's room at any width, one edit told for one change, the cell showing it after, the focus back on
//! the cell, a box in a cell turned by one press, and a page of several grids that does not move as one is edited.

use std::sync::{Arc, Mutex};

use lurq::{
  app::events::MouseButton,
  components::{Column, ScrollVertical},
  core::Signal,
  layout::{Constraints, Size},
  ui::{CellEditor, CellValue, DataGrid, TableColumn},
};

use super::stage::Stage;

const STATUSES: [&str; 3] = ["Building", "Passing", "Failed"];

#[derive(Clone, Debug, PartialEq)]
struct File {
  name: String,
  status: usize,
  size: f64,
  shared: bool,
}

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

type Edits = Arc<Mutex<Vec<(String, String, CellValue)>>>;

/// The columns, each as wide as `width`.
fn columns(width: f32) -> Vec<TableColumn<File>> {
  vec![
    TableColumn::new("name", "Name")
      .width(width)
      .text(|row: &File| row.name.clone())
      .editor(CellEditor::text(|row: &File| row.name.clone())),
    TableColumn::new("status", "Status")
      .width(width)
      .text(|row: &File| STATUSES[row.status].to_owned())
      .editor(CellEditor::choice(STATUSES, |row: &File| row.status)),
    TableColumn::new("size", "Size")
      .width(width)
      .numeric()
      .text(|row: &File| format!("{} KB", row.size))
      .editor(CellEditor::number(|row: &File| row.size)),
    TableColumn::new("shared", "Shared")
      .width(width)
      .editor(CellEditor::checkbox(|row: &File| row.shared)),
  ]
}

/// A grid of the files whose edits are taken into its rows and told to `edits`.
fn grid(rows: &Signal<Vec<File>>, edits: &Edits, width: f32, id: Option<&'static str>) -> DataGrid<File, String> {
  let (store, told) = (rows.clone(), edits.clone());
  let mut grid = DataGrid::new(columns(width), rows.get())
    .row_key(|row: &File| row.name.clone())
    .label("Files")
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
        ("name", CellValue::Text(name)) => file.name = name.clone(),
        ("status", CellValue::Choice(status)) => file.status = *status,
        ("size", CellValue::Number(size)) => file.size = *size,
        ("shared", CellValue::Checked(shared)) => file.shared = *shared,
        _ => return false,
      }
      store.set(files);
      true
    });
  if let Some(id) = id {
    grid = grid.id(id);
  }
  grid
}

fn stage_of(rows: &Signal<Vec<File>>, edits: &Edits, width: f32) -> Stage {
  let (rows, edits) = (rows.clone(), edits.clone());
  let mut stage = Stage::new(move |ctx| {
    Column::new()
      .size(1100.0, 400.0)
      .child(grid(&rows, &edits, width, Some("g")).mount(ctx))
      .into()
  });
  stage
    .tree
    .set_layout_constraints_override(Some(Constraints::loose(Size::new(1200.0, 700.0))));
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
  stage.tree.key_down(key.to_owned(), key.to_owned(), false, false, false);
  stage.tree.key_up(key.to_owned(), key.to_owned(), false, false, false);
  stage.pass();
}

fn press_at(stage: &mut Stage, (x, y): (f32, f32)) {
  stage.tree.mouse_move(x, y);
  stage.tree.mouse_down(x, y, MouseButton::Left);
  stage.tree.mouse_up(x, y, MouseButton::Left);
  stage.pass();
}

/// Whether `inner` lies within `outer`, to a half pixel.
fn within(inner: (f32, f32, f32, f32), outer: (f32, f32, f32, f32)) -> bool {
  inner.0 >= outer.0 - 0.5
    && inner.1 >= outer.1 - 0.5
    && inner.0 + inner.2 <= outer.0 + outer.2 + 0.5
    && inner.1 + inner.3 <= outer.1 + outer.3 + 0.5
}

#[test]
fn each_editor_is_laid_out_in_its_cells_room_at_any_width() {
  for width in [64.0, 120.0, 260.0] {
    for (column, opens) in [(0, "Enter"), (1, "Enter"), (2, "Enter")] {
      let (rows, edits) = (Signal::new(files()), Edits::default());
      let mut stage = stage_of(&rows, &edits, width);
      let cell = format!("g-cell-0-{column}");
      stage.click(&cell);
      key(&mut stage, opens);
      assert_eq!(
        focused(&stage).as_deref(),
        Some("g-editor"),
        "column {column} at {width}"
      );
      let (editor, room) = (stage.bounds("g-editor"), stage.bounds(&cell));
      assert!(
        within(editor, room),
        "the editor of column {column} at {width} wide, {editor:?}, is in its cell, {room:?}"
      );
      key(&mut stage, "Escape");
      assert_eq!(
        focused(&stage).as_deref(),
        Some(cell.as_str()),
        "Escape gives the focus back"
      );
      assert!(edits.lock().unwrap().is_empty(), "Escape tells nothing");
    }
  }
}

#[test]
fn a_choice_is_told_once_with_the_value_chosen_by_the_keys_and_by_the_pointer() {
  // By the keys: Enter edits, Enter opens the list, the arrows go down two, Enter takes it.
  let (rows, edits) = (Signal::new(files()), Edits::default());
  let mut stage = stage_of(&rows, &edits, 140.0);
  stage.click("g-cell-0-1");
  key(&mut stage, "Enter");
  key(&mut stage, "Enter");
  key(&mut stage, "ArrowDown");
  key(&mut stage, "ArrowDown");
  key(&mut stage, "Enter");
  assert_eq!(
    *edits.lock().unwrap(),
    [("blur.rs".to_owned(), "status".to_owned(), CellValue::Choice(2))],
    "one edit, of the value chosen"
  );
  assert_ne!(focused(&stage).as_deref(), Some("g-editor"), "the editor is gone");
  assert_eq!(
    focused(&stage).as_deref(),
    Some("g-cell-0-1"),
    "the focus is back on the cell"
  );
  assert!(stage.paints("Failed"), "the cell shows the choice");

  // By the pointer: a double press edits, a press opens the list, a press of an option takes it.
  let (rows, edits) = (Signal::new(files()), Edits::default());
  let mut stage = stage_of(&rows, &edits, 140.0);
  let at = stage.center("g-cell-1-1");
  press_at(&mut stage, at);
  press_at(&mut stage, at);
  assert_eq!(focused(&stage).as_deref(), Some("g-editor"), "a double press edits");
  let trigger = stage.center("g-editor");
  press_at(&mut stage, trigger);
  // The option is under the editor, in the list: not the cell of the row above, which says it too.
  let (_, editor_y, ..) = stage.bounds("g-editor");
  let option = stage
    .quads()
    .into_iter()
    .filter(|quad| matches!(&quad.content, lurq::layout::quad::QuadContent::Text { text, .. } if text == "Building"))
    .find(|quad| quad.y > editor_y + 10.0)
    .expect("the list is open, its options painted");
  press_at(
    &mut stage,
    (option.x + option.width / 2.0, option.y + option.height / 2.0),
  );
  assert_eq!(
    *edits.lock().unwrap(),
    [("canvas.rs".to_owned(), "status".to_owned(), CellValue::Choice(0))]
  );
  assert_eq!(focused(&stage).as_deref(), Some("g-cell-1-1"));
}

#[test]
fn a_text_and_a_number_are_told_once_and_shown_after() {
  let (rows, edits) = (Signal::new(files()), Edits::default());
  let mut stage = stage_of(&rows, &edits, 140.0);
  stage.click("g-cell-0-0");
  key(&mut stage, "Enter");
  stage.tree.text_input_replace(0..7, "glow.rs");
  stage.pass();
  key(&mut stage, "Enter");
  stage.click("g-cell-0-2");
  key(&mut stage, "Enter");
  stage.tree.text_input_replace(0..2, "30");
  stage.pass();
  key(&mut stage, "Tab");
  assert_eq!(
    *edits.lock().unwrap(),
    [
      (
        "blur.rs".to_owned(),
        "name".to_owned(),
        CellValue::Text("glow.rs".to_owned())
      ),
      ("glow.rs".to_owned(), "size".to_owned(), CellValue::Number(30.0)),
    ]
  );
  assert!(stage.paints("glow.rs") && stage.paints("30 KB"));
  assert_eq!(
    focused(&stage).as_deref(),
    Some("g-cell-0-3"),
    "Tab goes on to the next cell"
  );
}

#[test]
fn a_box_in_a_cell_turns_by_one_press_focused_or_not_and_once_by_a_double_press() {
  let (rows, edits) = (Signal::new(files()), Edits::default());
  let mut stage = stage_of(&rows, &edits, 120.0);
  let edits_of = |edits: &Edits| edits.lock().unwrap().len();

  // A cell that has not the focus: one press.
  let at = stage.center("g-cell-1-3");
  press_at(&mut stage, at);
  assert_eq!(edits_of(&edits), 1, "one press, one edit");
  assert_eq!(edits.lock().unwrap()[0].2, CellValue::Checked(true));
  assert!(rows.get()[1].shared);

  // The same cell, focused now, pressed again after a double press's while: one press again.
  std::thread::sleep(std::time::Duration::from_millis(600));
  press_at(&mut stage, at);
  assert_eq!(edits_of(&edits), 2);
  assert!(!rows.get()[1].shared);

  // A double press turns it once: its second press is not another turn, nor an edit of the box.
  std::thread::sleep(std::time::Duration::from_millis(600));
  let other = stage.center("g-cell-2-3");
  press_at(&mut stage, other);
  press_at(&mut stage, other);
  assert_eq!(edits_of(&edits), 3);
  assert!(rows.get()[2].shared);
  assert_ne!(focused(&stage).as_deref(), Some("g-editor"));

  // Space on the focused cell turns it once.
  key(&mut stage, " ");
  assert_eq!(edits_of(&edits), 4);
  assert!(!rows.get()[2].shared);
}

#[test]
fn a_double_press_in_a_grid_of_a_page_of_grids_moves_no_scroll_and_edits_that_grid() {
  let (rows, edits) = (Signal::new(files()), Edits::default());
  let (held, told) = (rows.clone(), edits.clone());
  let page = lurq::layout::layout_kind::ScrollState::new();
  let scroll = page.clone();
  // Three grids of the same rows, none given an id, the middle one in view.
  let mut stage = Stage::new(move |ctx| {
    let grids = (0..3).map(|place| {
      Column::new()
        .height(500.0)
        .key(format!("grid-{place}"))
        .child(grid(&held, &told, 140.0, None).mount(ctx))
    });
    ScrollVertical::new(Column::new().width(1100.0).with_children(grids))
      .with_scroll_state(scroll.clone())
      .size(1100.0, 600.0)
      .into()
  });
  stage
    .tree
    .set_layout_constraints_override(Some(Constraints::loose(Size::new(1200.0, 700.0))));
  stage.pass();
  page.set_scroll(0.0, 480.0);
  stage.pass();
  let before = page.scroll_y();
  let name = stage
    .quads()
    .into_iter()
    .filter(|quad| matches!(&quad.content, lurq::layout::quad::QuadContent::Text { text, .. } if text == "canvas.rs"))
    .find(|quad| quad.y > 40.0 && quad.y < 560.0)
    .expect("the middle grid's row is in view");
  let at = (name.x + 4.0, name.y + name.height / 2.0);
  press_at(&mut stage, at);
  press_at(&mut stage, at);
  assert_eq!(page.scroll_y(), before, "the page does not move");
  let editor = focused(&stage).expect("an editor has the focus");
  let (_, y, ..) = stage.bounds(&editor);
  assert!(y > 40.0 && y < 560.0, "the editor is in the grid pressed, in view: {y}");
  assert!(
    editor.starts_with("grid-") && editor.ends_with("-editor"),
    "a grid given no id makes one of its own for what it names: {editor}"
  );
}

#[test]
fn a_double_press_in_a_grid_under_the_pages_board_grid_moves_no_scroll() {
  let (rows, edits) = (Signal::new(files()), Edits::default());
  let (held, told) = (rows.clone(), edits.clone());
  let selected = Signal::new(vec!["blur.rs".to_owned()]);
  let sort = Signal::new(vec![lurq::ui::TableSort::ascending("name")]);
  let page = lurq::layout::layout_kind::ScrollState::new();
  let scroll = page.clone();
  // The page's arrangement: a grid with a selection, a sort, a total and a pager, then the grid that is edited,
  // their rows keyed by the same names.
  let mut stage = Stage::new(move |ctx| {
    let board = DataGrid::new(columns(140.0), held.get())
      .row_key(|row: &File| row.name.clone())
      .label("Files")
      .sort(sort.clone())
      .selected(selected.clone())
      .total(24)
      .count(true)
      .key("board")
      .mount(ctx);
    let editing = grid(&held, &told, 140.0, None).key("editing").mount(ctx);
    let page = Column::new()
      .width(1100.0)
      .child(Column::new().height(700.0).child(board))
      .child(Column::new().height(900.0).child(editing))
      .child(Column::new().height(900.0));
    ScrollVertical::new(page)
      .with_scroll_state(scroll.clone())
      .size(1100.0, 600.0)
      .into()
  });
  stage
    .tree
    .set_layout_constraints_override(Some(Constraints::loose(Size::new(1200.0, 700.0))));
  stage.pass();
  page.set_scroll(0.0, 680.0);
  stage.pass();
  let before = page.scroll_y();
  let building: Vec<_> = stage
    .quads()
    .into_iter()
    .filter(|quad| matches!(&quad.content, lurq::layout::quad::QuadContent::Text { text, .. } if text == "Building"))
    .filter(|quad| quad.y > 40.0 && quad.y < 560.0)
    .collect();
  let cell = building.first().expect("the edited grid's status in view");
  let at = (cell.x + 4.0, cell.y + cell.height / 2.0);
  press_at(&mut stage, at);
  assert_eq!(page.scroll_y(), before, "a press moves no scroll");
  press_at(&mut stage, at);
  assert_eq!(page.scroll_y(), before, "nor does the second");
}

#[test]
fn a_press_in_a_grid_beside_a_list_that_sticks_to_the_top_of_the_page_moves_no_scroll() {
  let (rows, edits) = (Signal::new(files()), Edits::default());
  let (held, told) = (rows.clone(), edits.clone());
  let page = lurq::layout::layout_kind::ScrollState::new();
  let scroll = page.clone();
  // The gallery's page: its sections in a column, and beside them the list of its sections, which sticks to the
  // top and is taller than the band a grid's row is in.
  let mut stage = Stage::new(move |ctx| {
    let sections = Column::new()
      .width(800.0)
      .child(
        Column::new()
          .height(700.0)
          .child(grid(&held, &told, 140.0, None).key("first").mount(ctx)),
      )
      .child(
        Column::new()
          .height(700.0)
          .child(grid(&held, &told, 140.0, None).key("second").mount(ctx)),
      )
      .child(Column::new().height(900.0));
    let contents = Column::new()
      .width(200.0)
      .height(520.0)
      .sticky(lurq::layout::layout_kind::Sticky::top(0.0));
    let page = lurq::components::Row::new()
      .align_items(lurq::layout::Alignment::Start)
      .child(sections)
      .child(contents);
    ScrollVertical::new(page)
      .with_scroll_state(scroll.clone())
      .size(1000.0, 600.0)
      .into()
  });
  stage
    .tree
    .set_layout_constraints_override(Some(Constraints::loose(Size::new(1200.0, 700.0))));
  stage.pass();
  // The second grid's first rows near the top of the window, under the band the list covers at its side.
  page.set_scroll(0.0, 650.0);
  stage.pass();
  let before = page.scroll_y();
  let cell = stage
    .quads()
    .into_iter()
    .filter(|quad| matches!(&quad.content, lurq::layout::quad::QuadContent::Text { text, .. } if text == "Building"))
    .find(|quad| quad.y > 40.0 && quad.y < 300.0)
    .expect("the second grid's status near the top");
  let at = (cell.x + 4.0, cell.y + cell.height / 2.0);
  press_at(&mut stage, at);
  assert_eq!(page.scroll_y(), before, "a press of a cell in view moves no scroll");
  press_at(&mut stage, at);
  assert_eq!(page.scroll_y(), before, "nor does the second of a double press");
  assert!(
    focused(&stage).is_some_and(|id| id.ends_with("-editor")),
    "the double press edits the cell pressed: {:?}",
    focused(&stage)
  );
}
