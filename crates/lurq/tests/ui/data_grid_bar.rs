//! The bar over a data grid, as the board's Table Toolbar draws it: its search field the grid's search, a tag for each
//! filter set that takes it off, Filter opening a column's filter, the density the grid takes, the columns shown and
//! Export.

use std::sync::{Arc, Mutex};

use lurq::{
  app::events::MouseButton,
  components::Column,
  core::Signal,
  layout::{Constraints, Size},
  ui::{ColumnFilter, ColumnLayout, DataGrid, GridExport, TableColumn, TableFilters},
};

use super::stage::Stage;

#[derive(Clone)]
struct File {
  name: &'static str,
  status: &'static str,
  size: u32,
}

fn files() -> Vec<File> {
  [
    ("blur.rs", "Building", 12),
    ("canvas.rs", "Passing", 48),
    ("theme.rs", "Passing", 9),
  ]
  .into_iter()
  .map(|(name, status, size)| File { name, status, size })
  .collect()
}

fn columns() -> Vec<TableColumn<File>> {
  vec![
    TableColumn::new("name", "Name")
      .filter_text(true)
      .text(|row: &File| row.name.to_owned()),
    TableColumn::new("status", "Status")
      .width(110.0)
      .filter_set(true)
      .text(|row: &File| row.status.to_owned()),
    TableColumn::new("size", "Size")
      .width(80.0)
      .numeric()
      .text(|row: &File| format!("{} KB", row.size)),
  ]
}

/// What the grid told the app.
#[derive(Default)]
struct Told {
  searched: Mutex<Vec<String>>,
  exported: Mutex<Vec<GridExport>>,
}

fn grid(filters: Signal<TableFilters>, layout: Signal<ColumnLayout>, told: Arc<Told>) -> Stage {
  let mut stage = Stage::new(move |ctx| {
    let (searched, exported) = (told.clone(), told.clone());
    Column::new()
      .size(900.0, 500.0)
      .child(
        DataGrid::new(columns(), files())
          .row_key(|row: &File| row.name.to_owned())
          .id("g")
          .label("Files")
          .toolbar(true)
          .filters(filters.clone())
          .column_layout(layout.clone())
          .on_search_change(move |query| searched.searched.lock().unwrap().push(query.to_owned()))
          .on_export(move |export| exported.exported.lock().unwrap().push(export.clone()))
          .mount(ctx),
      )
      .into()
  });
  stage
    .tree
    .set_layout_constraints_override(Some(Constraints::loose(Size::new(1000.0, 700.0))));
  stage.pass();
  stage
}

fn press(stage: &mut Stage, x: f32, y: f32) {
  stage.tree.mouse_move(x, y);
  stage.pass();
  stage.tree.mouse_down(x, y, MouseButton::Left);
  stage.tree.mouse_up(x, y, MouseButton::Left);
  stage.pass();
  stage.pass();
}

/// A press of the menu item that says `label`: the last text that says it, the menu being over the rest.
fn choose(stage: &mut Stage, label: &str) {
  let quad = stage
    .quads()
    .into_iter()
    .filter(|quad| matches!(&quad.content, lurq::layout::quad::QuadContent::Text { text, .. } if text == label))
    .last()
    .unwrap_or_else(|| panic!("the menu shows {label:?}"));
  press(stage, quad.x + quad.width / 2.0, quad.y + quad.height / 2.0);
}

#[test]
fn the_bar_stands_over_the_grid_as_the_board_draws_it_and_its_field_searches_the_grid() {
  let told = Arc::new(Told::default());
  let mut stage = grid(
    Signal::new(TableFilters::new()),
    Signal::new(ColumnLayout::default()),
    told.clone(),
  );
  let (bar_x, bar_y, bar_width, bar_height) = stage.bounds("g-bar");
  let (grid_x, grid_y, grid_width, _) = stage.bounds("g");
  assert_eq!(bar_height, 52.0, "the board's bar is 52 high");
  assert!(bar_y + bar_height <= grid_y, "over the grid");
  assert_eq!((bar_x, bar_width), (grid_x, grid_width), "as wide as the grid");
  let (_, _, search_width, _) = stage.bounds("g-bar-search");
  assert!(
    (search_width - 220.0).abs() < 1.0,
    "a search field of 220: {search_width}"
  );
  for words in ["Filter", "Columns", "Export"] {
    assert!(stage.paints(words), "the bar has {words}");
  }

  // Typed into, the field is the grid's search: the app hears it, and the grid marks what matches.
  stage.click("g-bar-search");
  stage.tree.text_input_replace(0..0, "canvas");
  stage.pass();
  assert_eq!(told.searched.lock().unwrap().last().map(String::as_str), Some("canvas"));
  stage.pass();
  let (_, _, field_width, _) = stage.bounds("g-bar-search");
  assert!(field_width > 0.0);
  assert!(stage.paints("canvas.rs"));
}

#[test]
fn a_tag_says_each_filter_set_and_takes_it_off_and_filter_opens_a_columns_filter() {
  let filters = Signal::new(
    TableFilters::new()
      .with("status", ColumnFilter::one_of(["Passing"]))
      .with("name", ColumnFilter::contains("rs")),
  );
  let mut stage = grid(filters.clone(), Signal::new(ColumnLayout::default()), Arc::default());
  assert!(stage.paints("Status: Passing"));
  assert!(stage.paints("Name: rs"));
  // The tag's × takes its filter off, the other stays.
  stage.click("g-bar-filter-status-remove");
  assert!(filters.get().get("status").is_none());
  assert_eq!(filters.get().get("name"), Some(&ColumnFilter::contains("rs")));
  assert!(!stage.paints("Status: Passing"));

  // Filter lists the columns that filter; one chosen opens its filter's panel.
  stage.click("g-bar-filter");
  stage.pass();
  choose(&mut stage, "Status");
  assert!(stage.paints("Select all"), "the status's panel of values is open");
}

#[test]
fn the_density_the_bar_chooses_is_the_grids_and_its_columns_and_export_work_as_the_menus_do() {
  let layout = Signal::new(ColumnLayout::default());
  let told = Arc::new(Told::default());
  let mut stage = grid(Signal::new(TableFilters::new()), layout.clone(), told.clone());
  let (_, _, _, roomy) = stage.bounds("g-row-0");
  // The third of the density's cells of 30: compact.
  let (x, y, _, height) = stage.bounds("g-bar-density");
  press(&mut stage, x + 75.0, y + height / 2.0);
  let (_, _, _, compact) = stage.bounds("g-row-0");
  assert!(compact < roomy, "compact rows are lower: {compact} from {roomy}");

  // Columns: a menu of every column, checked where it is shown; one chosen is hidden.
  stage.click("g-bar-columns");
  stage.pass();
  choose(&mut stage, "Size");
  assert!(layout.get().place("size").is_some_and(|place| place.hidden));

  // Export: the rows shown, as CSV.
  stage.click("g-bar-export");
  stage.pass();
  choose(&mut stage, "Shown rows as CSV");
  let exported = told.exported.lock().unwrap().last().cloned().expect("the app is told");
  assert_eq!(
    exported.text,
    "Name,Status\r\nblur.rs,Building\r\ncanvas.rs,Passing\r\ntheme.rs,Passing\r\n"
  );
}
