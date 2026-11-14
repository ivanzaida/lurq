//! A table: its header and rows against the board, columns that line up whatever is in a cell, sorting,
//! selecting with presses, checkboxes and keys, a header that stays while the rows scroll, the rows of a
//! mounted table built where they are in view, the empty and the loading forms, and a theme's other anatomy.

use std::sync::{Arc, Mutex};

use lurq::{
  app::{
    App,
    events::{MouseButton, ScrollPhase},
    theme::{Density, PaletteColor, ThemePalette},
  },
  components::{Column, Row},
  core::Signal,
  layout::{
    Constraints, Size,
    quad::{Quad, QuadContent},
  },
  node::{Element, color::Color},
  ui::{Button, Table, TableColumn, TablePart, TableRenderer, TableSort, TableView},
};

use super::stage::Stage;

const ARROW_UP: &str = "\u{E007}";
const ARROW_DOWN: &str = "\u{E004}";
const CHEVRONS_VERTICAL: &str = "\u{E01D}";
const CHECK: &str = "\u{E017}";
const MINUS: &str = "\u{E050}";
const INBOX: &str = "\u{E041}";

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

#[derive(Clone, Debug, PartialEq)]
struct Crate {
  name: &'static str,
  status: &'static str,
  updated: &'static str,
  size: u32,
}

/// The board's crates.
fn crates() -> Vec<Crate> {
  vec![
    Crate {
      name: "crates/lurq",
      status: "Passing",
      updated: "2 min ago",
      size: 48,
    },
    Crate {
      name: "crates/lurq_macros",
      status: "Building",
      updated: "just now",
      size: 12,
    },
    Crate {
      name: "crates/mobile_demo",
      status: "Failed",
      updated: "1 h ago",
      size: 9,
    },
  ]
}

fn columns() -> Vec<TableColumn<Crate>> {
  vec![
    TableColumn::new("name", "Name")
      .text(|row: &Crate| row.name.to_owned())
      .mono(true)
      .sortable(true),
    TableColumn::new("status", "Status")
      .width(120.0)
      .text(|row: &Crate| row.status.to_owned()),
    TableColumn::new("updated", "Updated")
      .width(120.0)
      .muted(true)
      .text(|row: &Crate| row.updated.to_owned()),
  ]
}

/// What a table was told.
#[derive(Clone, Default)]
struct Told {
  clicked: Arc<Mutex<Vec<&'static str>>>,
  activated: Arc<Mutex<Vec<&'static str>>>,
}

fn table(rows: Vec<Crate>) -> Table<Crate, &'static str> {
  Table::new(columns(), rows).row_key(|row: &Crate| row.name).id("t")
}

/// A window of 800 by 600, wider than the stage's, for a table of the board's 640.
fn window(mut stage: Stage) -> Stage {
  stage
    .tree
    .set_layout_constraints_override(Some(Constraints::loose(Size::new(800.0, 600.0))));
  stage.pass();
  stage
}

/// A stage of `scene` in a box 640 wide, as the board's table is.
fn wide(scene: impl Fn() -> Element + Send + Sync + 'static) -> Stage {
  window(Stage::new(move |_| {
    Column::new().size(640.0, 300.0).child(scene()).into()
  }))
}

/// The quads of the text `content`.
fn texts(stage: &Stage, content: &str) -> Vec<Quad> {
  stage
    .quads()
    .into_iter()
    .filter(|quad| matches!(&quad.content, QuadContent::Text { text, .. } if text == content))
    .collect()
}

/// The text `content` painted in the element `id`.
fn within(stage: &mut Stage, id: &str, content: &str) -> Option<Quad> {
  let (x, y, width, height) = stage.bounds(id);
  texts(stage, content).into_iter().find(|quad| {
    quad.y >= y - 0.01 && quad.y + quad.height <= y + height + 0.01 && quad.x >= x - 0.01 && quad.x < x + width
  })
}

fn focused(stage: &Stage) -> Option<String> {
  stage
    .tree
    .focused_element()
    .and_then(|element| element.id().map(str::to_owned))
}

fn press_with(stage: &mut Stage, id: &str, shift: bool, ctrl: bool) {
  let (x, y) = stage.center(id);
  stage.tree.mouse_move(x, y);
  stage
    .tree
    .mouse_down_with_modifiers(x, y, MouseButton::Left, shift, ctrl, false);
  stage
    .tree
    .mouse_up_with_modifiers(x, y, MouseButton::Left, shift, ctrl, false);
  stage.pass();
}

fn press_at(stage: &mut Stage, x: f32, y: f32, shift: bool) {
  stage.tree.mouse_move(x, y);
  stage
    .tree
    .mouse_down_with_modifiers(x, y, MouseButton::Left, shift, false, false);
  stage
    .tree
    .mouse_up_with_modifiers(x, y, MouseButton::Left, shift, false, false);
  stage.pass();
}

fn key_with(stage: &mut Stage, key: &str, shift: bool, ctrl: bool) {
  stage.tree.key_down(key.to_owned(), key.to_owned(), shift, ctrl, false);
  stage.tree.key_up(key.to_owned(), key.to_owned(), shift, ctrl, false);
  stage.pass();
}

#[test]
fn the_header_is_40_high_on_the_muted_fill_and_the_rows_44_their_text_as_the_board_has_it() {
  let mut stage = wide(|| table(crates()).into());
  let (x, y, width, _) = stage.bounds("t");
  assert_eq!(width, 640.0, "a table is as wide as what holds it");

  // The rows, 44 high one under the other under a header of 40.
  for index in 0..3 {
    let (row_x, row_y, row_width, height) = stage.bounds(&format!("t-row-{index}"));
    assert!((row_x - (x + 1.0)).abs() < 1.01, "row {index} at the box's inner edge");
    assert!(
      (row_width - (width - 2.0)).abs() < 2.01,
      "row {index} as wide as the box"
    );
    assert_eq!(height, 44.0);
    assert!(
      (row_y - (y + 40.0 + 44.0 * index as f32)).abs() < 1.01,
      "row {index} at {}",
      row_y - y
    );
  }
  let header = texts(&stage, "Name").pop().expect("the header says Name");
  assert!(header.y < y + 40.0, "the header is above the rows");
  assert!(
    stage.quads().iter().any(|quad| matches!(
      &quad.content,
      QuadContent::Rect { color, .. } if *color == light(PaletteColor::Muted)
    ) && (quad.height - 40.0).abs() < 1.01),
    "the header is on the muted fill"
  );

  // The titles 13 in the muted foreground; the cells 13 in the foreground, the muted column muted, the name
  // in the mono type.
  assert_eq!(stage.text("Status"), (light(PaletteColor::MutedForeground), 13.0));
  assert_eq!(stage.text("Passing"), (light(PaletteColor::Foreground), 13.0));
  assert_eq!(stage.text("2 min ago"), (light(PaletteColor::MutedForeground), 13.0));
  assert!(stage.font_of("crates/lurq").to_lowercase().contains("mono"));

  // The text of a cell starts 16 in, as the header's does.
  let name = within(&mut stage, "t-row-0", "crates/lurq").unwrap();
  assert!((name.x - (x + 1.0 + 16.0)).abs() < 1.01, "{}", name.x - x);
  assert!((header.x - name.x).abs() < 0.5);
}

#[test]
fn the_columns_of_the_header_and_of_every_row_line_up_whatever_is_in_a_cell() {
  let mut rows = crates();
  rows[1].status = "Building for a long while";
  let mut stage = wide(move || {
    let columns = vec![
      TableColumn::new("name", "Name").text(|row: &Crate| row.name.to_owned()),
      TableColumn::new("status", "Status")
        .fit()
        .text(|row: &Crate| row.status.to_owned()),
      TableColumn::new("size", "Size")
        .width(70.0)
        .numeric()
        .text(|row: &Crate| format!("{} KB", row.size)),
    ];
    Table::new(columns, rows.clone()).id("t").into()
  });
  // The column sized by its content starts where the header's does in every row, and is as wide as its widest
  // cell: the next column starts at the same place in every row.
  let status = texts(&stage, "Status").pop().unwrap();
  for (index, text) in ["Passing", "Building for a long while", "Failed"]
    .into_iter()
    .enumerate()
  {
    let cell = within(&mut stage, &format!("t-row-{index}"), text).unwrap();
    assert!((cell.x - status.x).abs() < 0.5, "{text}");
  }
  // The numbers are at the column's end, the table's last 16 in: each ends where the header's title ends.
  let size = texts(&stage, "Size").pop().unwrap();
  let (x, _, width, _) = stage.bounds("t");
  assert!(((size.x + size.width) - (x + width - 1.0 - 16.0)).abs() < 1.01);
  for (index, text) in ["48 KB", "12 KB", "9 KB"].into_iter().enumerate() {
    let cell = within(&mut stage, &format!("t-row-{index}"), text).unwrap();
    assert!(((cell.x + cell.width) - (size.x + size.width)).abs() < 0.5, "{text}");
  }
  // A number is mono 12 in the muted foreground.
  assert_eq!(stage.text("48 KB"), (light(PaletteColor::MutedForeground), 12.0));
}

#[test]
fn a_press_of_a_sortable_header_goes_through_ascending_descending_and_none() {
  let sort = Signal::new(None::<TableSort>);
  let asked = Arc::new(Mutex::new(Vec::new()));
  let (scene_sort, told) = (sort.clone(), asked.clone());
  let mut stage = wide(move || {
    let told = told.clone();
    let columns = columns();
    let mut rows = crates();
    if let Some(by) = scene_sort.get() {
      by.sort(&columns, &mut rows);
    }
    Table::new(columns, rows)
      .id("t")
      .sort(scene_sort.clone())
      .on_sort(move |next| told.lock().unwrap().push(next))
      .into()
  });
  // A column that sorts has the mark that it does; one that does not, nothing.
  assert!(within(&mut stage, "t-header-name", CHEVRONS_VERTICAL).is_some());
  assert!(texts(&stage, ARROW_UP).is_empty());

  stage.click("t-header-name");
  assert_eq!(sort.get(), Some(TableSort::ascending("name")));
  assert!(within(&mut stage, "t-header-name", ARROW_UP).is_some());
  // Sorted, the title is in the foreground; the rows come in the order the app gives.
  assert_eq!(stage.text("Name").0, light(PaletteColor::Foreground));
  assert!(within(&mut stage, "t-row-0", "crates/lurq").is_some());

  stage.click("t-header-name");
  assert_eq!(sort.get(), Some(TableSort::descending("name")));
  assert!(within(&mut stage, "t-header-name", ARROW_DOWN).is_some());
  assert!(within(&mut stage, "t-row-0", "crates/mobile_demo").is_some());

  stage.click("t-header-name");
  assert_eq!(sort.get(), None);
  assert_eq!(
    *asked.lock().unwrap(),
    [
      Some(TableSort::ascending("name")),
      Some(TableSort::descending("name")),
      None
    ]
  );
}

#[test]
fn sort_orders_rows_by_a_columns_key_or_its_text_and_keeps_equal_ones_in_order() {
  let columns = vec![
    TableColumn::new("name", "Name").text(|row: &Crate| row.name.to_owned()),
    TableColumn::new("size", "Size").sort_key(|row: &Crate| row.size),
  ];
  let mut rows = crates();
  TableSort::ascending("size").sort(&columns, &mut rows);
  assert_eq!(rows.iter().map(|row| row.size).collect::<Vec<_>>(), [9, 12, 48]);
  TableSort::descending("name").sort(&columns, &mut rows);
  assert_eq!(rows[0].name, "crates/mobile_demo");
  assert_eq!(
    TableSort::after_press(Some(&TableSort::descending("size")), "name"),
    Some(TableSort::ascending("name")),
    "another column starts ascending"
  );
}

/// A table whose rows are selected, with what it was told.
fn selecting(selected: &[&'static str]) -> (Stage, Signal<Vec<&'static str>>, Told) {
  let signal = Signal::new(selected.to_vec());
  let told = Told::default();
  let (scene_signal, scene_told) = (signal.clone(), told.clone());
  let stage = wide(move || {
    let (clicked, activated) = (scene_told.clicked.clone(), scene_told.activated.clone());
    table(crates())
      .selected(scene_signal.clone())
      .on_row_click(move |key| clicked.lock().unwrap().push(*key))
      .on_row_activate(move |key| activated.lock().unwrap().push(*key))
      .into()
  });
  (stage, signal, told)
}

#[test]
fn a_selected_row_is_highlighted_with_its_checkbox_checked_and_the_headers_is_mixed() {
  let (mut stage, ..) = selecting(&["crates/lurq_macros"]);
  assert_eq!(stage.fill_color("t-row-1"), Some(light(PaletteColor::Highlight)));
  assert_eq!(stage.fill_color("t-row-0"), None);
  assert!(within(&mut stage, "t-row-1", CHECK).is_some());
  assert!(within(&mut stage, "t-row-0", CHECK).is_none());
  // The header's checkbox is mixed while some rows are selected.
  assert!(texts(&stage, MINUS).len() == 1);

  // With the checkboxes, the cells start 12 after the checkbox's 16.
  let (x, ..) = stage.bounds("t");
  let name = within(&mut stage, "t-row-0", "crates/lurq").unwrap();
  assert!((name.x - (x + 1.0 + 16.0 + 16.0 + 12.0)).abs() < 1.01, "{}", name.x - x);
}

#[test]
fn a_press_selects_a_row_alone_ctrl_adds_it_and_shift_selects_a_range() {
  let (mut stage, selected, told) = selecting(&[]);
  stage.click("t-row-0");
  assert_eq!(selected.get(), ["crates/lurq"]);
  assert_eq!(*told.clicked.lock().unwrap(), ["crates/lurq"]);

  press_with(&mut stage, "t-row-2", false, true);
  assert_eq!(selected.get(), ["crates/lurq", "crates/mobile_demo"]);
  press_with(&mut stage, "t-row-0", false, true);
  assert_eq!(selected.get(), ["crates/mobile_demo"], "Ctrl takes a selected row out");

  // Shift selects from the last selected row to the pressed one, the anchor last.
  press_with(&mut stage, "t-row-0", true, false);
  let mut range = selected.get();
  assert_eq!(range.last(), Some(&"crates/mobile_demo"));
  range.sort();
  assert_eq!(range, ["crates/lurq", "crates/lurq_macros", "crates/mobile_demo"]);
  assert!(told.activated.lock().unwrap().is_empty(), "a press does not activate");
}

#[test]
fn a_press_of_a_checkbox_adds_its_row_and_the_headers_selects_every_row_or_none() {
  let (mut stage, selected, _) = selecting(&["crates/lurq"]);
  // The checkbox of the third row: at the row's start, 16 in.
  let (x, y, _, height) = stage.bounds("t-row-2");
  press_at(&mut stage, x + 16.0 + 8.0, y + height / 2.0, false);
  assert_eq!(
    selected.get(),
    ["crates/lurq", "crates/mobile_demo"],
    "the checkbox adds, as Ctrl does"
  );

  stage.click("t-select-all");
  let mut all = selected.get();
  all.sort();
  assert_eq!(all, ["crates/lurq", "crates/lurq_macros", "crates/mobile_demo"]);
  assert_eq!(texts(&stage, CHECK).len(), 4, "every row's and the header's");
  stage.click("t-select-all");
  assert!(selected.get().is_empty(), "where all are selected, none");
}

#[test]
fn a_press_selects_and_focuses_at_once_and_a_second_press_soon_after_activates() {
  let (mut stage, selected, told) = selecting(&[]);
  stage.click("t-row-1");
  assert_eq!(
    selected.get(),
    ["crates/lurq_macros"],
    "selected at the press, not after a wait"
  );
  assert_eq!(focused(&stage).as_deref(), Some("t-row-1"));
  assert!(told.activated.lock().unwrap().is_empty());

  stage.click("t-row-1");
  assert_eq!(
    *told.activated.lock().unwrap(),
    ["crates/lurq_macros"],
    "the double press activates"
  );
  assert_eq!(selected.get(), ["crates/lurq_macros"], "and leaves the selection");

  // A third press is the first of another.
  stage.click("t-row-1");
  assert_eq!(told.activated.lock().unwrap().len(), 1);
}

#[test]
fn the_rows_are_one_stop_of_tab_the_arrows_move_space_selects_and_enter_activates() {
  let (mut stage, selected, told) = selecting(&[]);
  // The header's checkbox is a stop, then the rows.
  key_with(&mut stage, "Tab", false, false);
  assert_eq!(focused(&stage).as_deref(), Some("t-select-all"));
  key_with(&mut stage, "Tab", false, false);
  assert_eq!(focused(&stage).as_deref(), Some("t-row-0"));
  assert!(
    stage.has_ring(light(PaletteColor::Ring)),
    "the ring inside the focused row"
  );

  key_with(&mut stage, "ArrowDown", false, false);
  assert_eq!(focused(&stage).as_deref(), Some("t-row-1"));
  key_with(&mut stage, " ", false, false);
  assert_eq!(selected.get(), ["crates/lurq_macros"]);
  key_with(&mut stage, "End", false, false);
  assert_eq!(focused(&stage).as_deref(), Some("t-row-2"));
  key_with(&mut stage, " ", false, false);
  assert_eq!(
    selected.get(),
    ["crates/lurq_macros", "crates/mobile_demo"],
    "Space adds the row"
  );
  key_with(&mut stage, "Enter", false, false);
  assert_eq!(*told.activated.lock().unwrap(), ["crates/mobile_demo"]);
  assert_eq!(
    selected.get(),
    ["crates/lurq_macros", "crates/mobile_demo"],
    "Enter does not select"
  );
  key_with(&mut stage, "a", false, true);
  assert_eq!(selected.get().len(), 3, "Ctrl+A selects them all");

  // Tab leaves the rows at once: they are one stop.
  key_with(&mut stage, "Tab", false, false);
  assert!(focused(&stage).is_none_or(|id| !id.starts_with("t-row")));
}

#[test]
fn tab_goes_from_a_row_into_its_buttons_and_on_out_of_the_table_and_their_presses_are_theirs() {
  let pressed = Arc::new(Mutex::new(Vec::new()));
  let selected = Signal::new(Vec::<&'static str>::new());
  let (told, scene_selected) = (pressed.clone(), selected.clone());
  let mut stage = wide(move || {
    let told = told.clone();
    let mut columns = columns();
    columns.push(TableColumn::actions("actions", move |row: &Crate| {
      let (told, name) = (told.clone(), row.name);
      Button::new("Open")
        .id(format!("open-{name}"))
        .on_click(move |_| told.lock().unwrap().push(name))
        .into()
    }));
    Column::new()
      .child(
        Table::new(columns, crates())
          .row_key(|row: &Crate| row.name)
          .id("t")
          .selected(scene_selected.clone())
          .checkboxes(false),
      )
      .child(Button::new("After").id("after"))
      .into()
  });
  key_with(&mut stage, "Tab", false, false);
  assert_eq!(focused(&stage).as_deref(), Some("t-row-0"));
  key_with(&mut stage, "Tab", false, false);
  assert_eq!(
    focused(&stage).as_deref(),
    Some("open-crates/lurq"),
    "into the row's button"
  );
  key_with(&mut stage, "Tab", true, false);
  assert_eq!(focused(&stage).as_deref(), Some("t-row-0"), "Shift+Tab back to the row");
  // The arrows pass the buttons by: they move between the rows.
  key_with(&mut stage, "ArrowDown", false, false);
  assert_eq!(focused(&stage).as_deref(), Some("t-row-1"));
  key_with(&mut stage, "Tab", false, false);
  assert_eq!(focused(&stage).as_deref(), Some("open-crates/lurq_macros"));
  key_with(&mut stage, "Tab", false, false);
  assert_eq!(focused(&stage).as_deref(), Some("after"), "and on out of the table");

  // A press of the button is the button's: the row is not selected.
  stage.click("open-crates/mobile_demo");
  assert_eq!(*pressed.lock().unwrap(), ["crates/mobile_demo"]);
  assert!(selected.get().is_empty());
}

fn many(count: usize) -> Arc<Vec<Crate>> {
  Arc::new(
    (0..count)
      .map(|index| Crate {
        name: Box::leak(format!("crate {index}").into_boxed_str()),
        status: "Passing",
        updated: "now",
        size: index as u32,
      })
      .collect(),
  )
}

#[test]
fn the_header_stays_while_the_rows_scroll_in_the_tables_height() {
  let rows = many(30);
  let mut stage = wide(move || Table::new_shared(columns(), rows.clone()).id("t").height(240.0).into());
  let (x, y, _, height) = stage.bounds("t");
  assert_eq!(height, 240.0);
  let header_y = texts(&stage, "Name").pop().unwrap().y;

  stage
    .tree
    .scroll(x + 100.0, y + 120.0, 0.0, -300.0, ScrollPhase::Scroll);
  stage.pass();
  let (_, row_y, ..) = stage.bounds("t-row-0");
  assert!(row_y < y, "the rows scrolled up");
  assert!(
    (texts(&stage, "Name").pop().unwrap().y - header_y).abs() < 0.5,
    "the header did not"
  );
}

/// The veils of `color` (inset shadows) whose box holds the point `(x, y)`, narrowest first.
fn veils_at(stage: &Stage, color: Color, x: f32, y: f32) -> Vec<Quad> {
  let mut veils: Vec<Quad> = stage
    .quads()
    .into_iter()
    .filter(|quad| {
      matches!(&quad.content, QuadContent::BoxShadow(shadow) if shadow.inset && shadow.color == color)
        && quad.x <= x
        && x <= quad.x + quad.width
        && quad.y <= y
        && y <= quad.y + quad.height
    })
    .collect();
  veils.sort_by(|a, b| a.width.total_cmp(&b.width));
  veils
}

#[test]
fn a_first_column_that_stays_takes_its_rows_veil_under_the_pointer_and_held_down_and_not_from_the_focus() {
  let mut stage = wide(|| {
    let mut columns = columns();
    let name = columns.remove(0).sticky(true);
    columns.insert(0, name);
    Table::new(columns, crates())
      .row_key(|row: &Crate| row.name)
      .id("t")
      .into()
  });
  let name = within(&mut stage, "t-row-0", "crates/lurq").expect("the first row's name");
  let (name_x, name_y) = (name.x + 2.0, name.y + name.height / 2.0);
  let (_, row_y, row_width, _) = stage.bounds("t-row-0");
  let hover = light(PaletteColor::StateHover);
  let pressed = light(PaletteColor::StatePressed);
  assert!(veils_at(&stage, hover, name_x, name_y).is_empty(), "no veil at rest");

  // The pointer on the row's last cell, away from the cell that stays: the row and that cell are veiled.
  let updated = within(&mut stage, "t-row-0", "2 min ago").expect("the first row's time");
  let (x, y) = (updated.x + 2.0, updated.y + updated.height / 2.0);
  stage.tree.mouse_move(x, y);
  stage.pass();
  let veils = veils_at(&stage, hover, name_x, name_y);
  assert_eq!(veils.len(), 2, "the row's veil and the stuck cell's own over its fill");
  assert!(veils[0].width < row_width - 1.0, "the narrower is the cell's");
  assert!(veils[0].y >= row_y - 0.01);

  // Held down: the cell is veiled as pressed, as the row is.
  stage.tree.mouse_down(x, y, MouseButton::Left);
  stage.pass();
  let veils = veils_at(&stage, pressed, name_x, name_y);
  assert_eq!(veils.len(), 2, "the row and the stuck cell held down");
  stage.tree.mouse_up(x, y, MouseButton::Left);
  stage.pass();

  // The press focused the row; with the pointer gone the row has no veil, and nor has the cell.
  assert_eq!(focused(&stage).as_deref(), Some("t-row-0"));
  stage.tree.mouse_leave_window();
  stage.pass();
  assert!(
    veils_at(&stage, hover, name_x, name_y).is_empty(),
    "the focus alone veils nothing"
  );
}

#[test]
fn a_row_the_keys_reach_is_shown_clear_of_the_header_that_stays_over_it() {
  let rows = many(30);
  let mut stage = wide(move || Table::new_shared(columns(), rows.clone()).id("t").height(240.0).into());
  let (x, y, ..) = stage.bounds("t");
  key_with(&mut stage, "Tab", false, false);
  assert_eq!(focused(&stage).as_deref(), Some("t-row-0"));
  // The rows scroll up under the header, the focused one out of view above it.
  stage
    .tree
    .scroll(x + 100.0, y + 120.0, 0.0, -300.0, ScrollPhase::Scroll);
  stage.pass();
  key_with(&mut stage, "ArrowDown", false, false);
  assert_eq!(focused(&stage).as_deref(), Some("t-row-1"));
  let (_, row_y, ..) = stage.bounds("t-row-1");
  assert!(
    row_y >= y + 40.0 - 0.5,
    "under the header, not behind it: {}",
    row_y - y
  );
  assert!(row_y < y + 240.0, "in view");
}

#[test]
fn in_a_table_that_is_not_mounted_the_page_keys_move_by_what_its_height_shows() {
  let rows = many(30);
  let mut stage = wide(move || Table::new_shared(columns(), rows.clone()).id("t").height(240.0).into());
  key_with(&mut stage, "Tab", false, false);
  assert_eq!(focused(&stage).as_deref(), Some("t-row-0"));
  // Rows of 44 in a table 240 high: a page is four or five rows on, as much as the view shows, not the last row.
  key_with(&mut stage, "PageDown", false, false);
  let paged = focused(&stage).unwrap();
  assert!(paged == "t-row-4" || paged == "t-row-5", "PageDown went to {paged}");
  let (_, row_y, ..) = stage.bounds(&paged);
  let (_, y, ..) = stage.bounds("t");
  assert!(
    row_y >= y + 40.0 - 0.5 && row_y < y + 240.0,
    "the row it went to is in view"
  );
  key_with(&mut stage, "PageUp", false, false);
  assert_eq!(focused(&stage).as_deref(), Some("t-row-0"));
}

#[test]
fn a_mounted_table_of_ten_thousand_rows_builds_those_in_view_and_end_reaches_the_last() {
  let rows = many(10_000);
  let mut stage = window(Stage::new(move |ctx| {
    Column::new()
      .size(640.0, 300.0)
      .child(
        Table::new_shared(columns(), rows.clone())
          .id("t")
          .height(300.0)
          .mount(ctx),
      )
      .into()
  }));
  assert!(stage.paints("crate 0"));
  assert!(
    stage.tree.get_element_by_id_mut("t-row-200").is_none(),
    "far rows are not built"
  );
  let built = (0..200)
    .filter(|index| stage.tree.get_element_by_id_mut(&format!("t-row-{index}")).is_some())
    .count();
  assert!(built < 40, "about a screen of rows and a few around: {built}");
  // The rows that are not built still take their room: the content is as tall as every row.
  let (_, y, ..) = stage.bounds("t");
  key_with(&mut stage, "Tab", false, false);
  assert_eq!(focused(&stage).as_deref(), Some("t-row-0"));
  key_with(&mut stage, "End", false, false);
  stage.pass();
  assert_eq!(focused(&stage).as_deref(), Some("t-row-9999"));
  assert!(stage.paints("crate 9999"));
  let (_, last_y, _, last_height) = stage.bounds("t-row-9999");
  assert!(last_y + last_height <= y + 300.0 + 0.5, "the last row is in view");
  key_with(&mut stage, "PageUp", false, false);
  stage.pass();
  let row = focused(&stage).unwrap();
  let at: usize = row.trim_start_matches("t-row-").parse().unwrap();
  assert!((9990..9999).contains(&at), "a page up: {at}");
  key_with(&mut stage, "Home", false, false);
  stage.pass();
  assert_eq!(focused(&stage).as_deref(), Some("t-row-0"));
}

#[test]
fn with_no_rows_an_empty_state_stands_under_the_header_and_loading_rows_of_skeletons() {
  let stage = wide(|| table(Vec::new()).into());
  assert!(stage.paints("Name"), "the header stays");
  assert!(stage.paints("No rows"));
  assert!(stage.paints(INBOX));

  let stage = wide(|| {
    table(Vec::new())
      .empty(Row::new().child(lurq::components::Text::new("Nothing matches")))
      .into()
  });
  assert!(stage.paints("Nothing matches"));
  assert!(!stage.paints("No rows"));

  let stage = wide(|| table(crates()).loading(true).into());
  assert!(stage.paints("Name"), "the header stays while loading");
  assert!(!stage.paints("crates/lurq"), "skeletons stand where the rows will be");
  let muted = light(PaletteColor::Muted);
  let bones = stage
    .quads()
    .into_iter()
    .filter(|quad| matches!(&quad.content, QuadContent::Rect { color, .. } if *color == muted) && quad.height < 13.0)
    .count();
  assert_eq!(bones, 12, "a line in each of the three cells of four rows");
}

#[test]
fn the_footer_counts_the_rows_and_what_is_selected() {
  let selected = Signal::new(vec!["crates/lurq"]);
  let scene_selected = selected.clone();
  let stage = wide(move || {
    table(crates())
      .selected(scene_selected.clone())
      .count(true)
      .total(24)
      .into()
  });
  assert!(stage.paints("1 of 24 rows selected"));
  assert_eq!(
    stage.text("1 of 24 rows selected"),
    (light(PaletteColor::MutedForeground), 13.0)
  );
  // With a footer, the last row has its line under it.
  let stage = wide(|| table(crates()).count(true).into());
  assert!(stage.paints("3 rows"));
}

#[test]
fn a_compact_table_has_rows_of_36() {
  let mut stage = wide(|| table(crates()).density(Density::Compact).into());
  assert_eq!(stage.bounds("t-row-0").3, 36.0);
  assert_eq!(stage.bounds("t-row-1").1 - stage.bounds("t-row-0").1, 36.0);
}

#[cfg(feature = "mcp")]
#[test]
fn a_table_tells_tooling_its_rows_its_sort_and_what_is_selected() {
  let selected = Signal::new(vec!["crates/lurq"]);
  let scene_selected = selected.clone();
  let mut stage = wide(move || {
    table(crates())
      .label("Crates")
      .selected(scene_selected.clone())
      .sorted_by(Some(TableSort::ascending("name")), |_| {})
      .into()
  });
  let said = |stage: &mut Stage, id: &str, name: &str| {
    stage
      .tree
      .get_element_by_id_mut(id)
      .and_then(|element| element.description(name))
  };
  assert_eq!(said(&mut stage, "t", "a11y_role").as_deref(), Some("grid"));
  assert_eq!(said(&mut stage, "t", "a11y_rowcount").as_deref(), Some("4"));
  assert_eq!(said(&mut stage, "t", "a11y_name").as_deref(), Some("Crates"));
  assert_eq!(said(&mut stage, "t-row-0", "a11y_role").as_deref(), Some("row"));
  assert_eq!(said(&mut stage, "t-row-0", "a11y_selected").as_deref(), Some("true"));
  assert_eq!(said(&mut stage, "t-row-1", "a11y_selected").as_deref(), Some("false"));
  assert_eq!(said(&mut stage, "t-row-1", "a11y_rowindex").as_deref(), Some("3"));
  assert_eq!(
    said(&mut stage, "t-select-all", "a11y_checked").as_deref(),
    Some("mixed")
  );
}

#[test]
fn a_theme_gives_the_table_another_anatomy_and_its_rows_still_select() {
  let app = App::new();
  // A table that draws no header: the rows alone, with a word of how many there are.
  let renderer: TableRenderer = Arc::new(|mut view: TableView| {
    let count = format!("{} crates", view.total);
    let body = view.body();
    view
      .root()
      .child(view.part(TablePart::Count).text(&count))
      .child(body)
      .into()
  });
  app.theme().set_renderer::<Table<Crate, &'static str>>(renderer);
  let selected = Signal::new(Vec::new());
  let scene_selected = selected.clone();
  let mut stage = window(Stage::in_app(app, move |_| {
    Column::new()
      .size(640.0, 300.0)
      .child(table(crates()).selected(scene_selected.clone()))
      .into()
  }));
  assert!(stage.paints("3 crates"));
  assert!(!stage.paints("Status"));
  stage.click("t-row-2");
  assert_eq!(selected.get(), ["crates/mobile_demo"]);
}
