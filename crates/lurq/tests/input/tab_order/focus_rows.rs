//! A focus group of rows a list builds only in part (`FocusRows`): its keys step over the rows, built or not,
//! the row stepped to is asked for when it is not built and focused once a layout has it, keys pressed meanwhile
//! go on from it, and typeahead, PageUp and PageDown and the keys of a nested group reach any row.

use std::sync::{Arc, Mutex};

use lurq::{
  app::{App, Tree, component::Component, ctx::Ctx},
  components::{Button, Column},
  core::Signal,
  node::{Element, FocusGroup, FocusRows},
};

use super::{focused_id, tab};

/// How many rows the list builds: from its start, five.
const WINDOW: usize = 5;

/// A hundred rows, of which the list builds five from `start`. Row 50 is disabled, row 73 is called "Zeta" and the
/// rest "Row n". In the nested form row 0 holds rows 1 to 60, and row 61 is closed.
struct Rows {
  start: Signal<usize>,
  nested: bool,
  asked: Mutex<Vec<(usize, bool)>>,
}

impl Rows {
  fn new(nested: bool) -> Arc<Self> {
    Arc::new(Self {
      start: Signal::new(0),
      nested,
      asked: Mutex::new(Vec::new()),
    })
  }
}

impl FocusRows for Rows {
  fn count(&self) -> usize {
    100
  }

  fn label(&self, row: usize) -> Option<String> {
    Some(if row == 73 {
      "Zeta".to_owned()
    } else {
      format!("Row {row}")
    })
  }

  fn enabled(&self, row: usize) -> bool {
    row != 50
  }

  fn level(&self, row: usize) -> u32 {
    if self.nested && (1..=60).contains(&row) { 2 } else { 1 }
  }

  fn expanded(&self, row: usize) -> Option<bool> {
    match row {
      0 if self.nested => Some(true),
      61 if self.nested => Some(false),
      _ => None,
    }
  }

  fn set_expanded(&self, row: usize, open: bool) {
    self.asked.lock().unwrap().push((row, open));
  }

  fn page(&self) -> usize {
    10
  }

  fn reveal(&self, row: usize) {
    self.start.set(row.saturating_sub(2).min(self.count() - WINDOW));
  }
}

#[derive(Clone, lurq::DevtoolsInspectable)]
struct WindowProps(#[devtools_ignore] Arc<Rows>);

impl PartialEq for WindowProps {
  fn eq(&self, other: &Self) -> bool {
    Arc::ptr_eq(&self.0, &other.0)
  }
}

/// The list: the five rows from its start, each saying which row it is.
struct Window;

impl Component for Window {
  type Props = WindowProps;

  fn create(_: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let rows = ctx.props::<WindowProps>().0.clone();
    let start = rows.start.get();
    let group: Arc<dyn FocusRows> = rows.clone();
    Column::new()
      .focus_group(
        FocusGroup::vertical()
          .nested(true)
          .one_stop(true)
          .typeahead(true)
          .wrap(false),
      )
      .focus_rows(group)
      .with_children((start..start + WINDOW).map(|row| {
        Button::new(&rows.label(row).unwrap())
          .id(format!("row-{row}"))
          .tab_index(0)
          .focus_row(row)
          .disabled(!rows.enabled(row))
      }))
  }
}

fn mount(rows: &Arc<Rows>) -> (Tree, App) {
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.mount_root::<Window>(&mut app, WindowProps(rows.clone()));
  tree.pass_headless(&mut app);
  (tree, app)
}

fn key(tree: &mut Tree, key: &str) {
  tree.key_down(key.to_owned(), key.to_owned(), false, false, false);
  tree.key_up(key.to_owned(), key.to_owned(), false, false, false);
}

/// A key, then the passes that build the row it asked for.
fn step(tree: &mut Tree, app: &mut App, name: &str) {
  key(tree, name);
  tree.pass_headless(app);
  tree.pass_headless(app);
}

#[test]
fn the_keys_reach_rows_the_list_has_not_built_and_focus_them_once_it_has() {
  let rows = Rows::new(false);
  let (mut tree, mut app) = mount(&rows);
  tab(&mut tree);
  assert_eq!(focused_id(&tree).as_deref(), Some("row-0"));

  // End: row 99 is not built; the list is asked for it, and it has the focus once it is.
  step(&mut tree, &mut app, "End");
  assert_eq!(rows.start.get(), 95);
  assert_eq!(focused_id(&tree).as_deref(), Some("row-99"));
  step(&mut tree, &mut app, "Home");
  assert_eq!(focused_id(&tree).as_deref(), Some("row-0"));

  // Down to a row that is built: focused at once.
  key(&mut tree, "ArrowDown");
  assert_eq!(focused_id(&tree).as_deref(), Some("row-1"));

  // PageDown goes a page of rows, past what is built; to the nearest that takes the focus.
  step(&mut tree, &mut app, "PageDown");
  assert_eq!(focused_id(&tree).as_deref(), Some("row-11"));
  for _ in 0..3 {
    step(&mut tree, &mut app, "PageDown");
  }
  assert_eq!(focused_id(&tree).as_deref(), Some("row-41"));
  step(&mut tree, &mut app, "PageDown");
  assert_eq!(focused_id(&tree).as_deref(), Some("row-51"));
  step(&mut tree, &mut app, "ArrowUp");
  assert_eq!(focused_id(&tree).as_deref(), Some("row-49"), "past the disabled row 50");
  step(&mut tree, &mut app, "PageUp");
  assert_eq!(focused_id(&tree).as_deref(), Some("row-39"));

  // A letter: the next row whose label starts with it, wherever it is.
  step(&mut tree, &mut app, "z");
  assert_eq!(focused_id(&tree).as_deref(), Some("row-73"));
}

#[test]
fn keys_pressed_while_the_list_builds_a_row_go_on_from_that_row() {
  let rows = Rows::new(false);
  let (mut tree, mut app) = mount(&rows);
  tab(&mut tree);
  key(&mut tree, "End");
  // Not built yet: the next key goes on from row 99, not from the row that had the focus.
  key(&mut tree, "ArrowUp");
  key(&mut tree, "ArrowUp");
  tree.pass_headless(&mut app);
  tree.pass_headless(&mut app);
  assert_eq!(focused_id(&tree).as_deref(), Some("row-97"));
}

#[test]
fn the_keys_of_a_nested_group_read_the_levels_of_rows_that_are_not_built() {
  let rows = Rows::new(true);
  let (mut tree, mut app) = mount(&rows);
  tab(&mut tree);
  // Right on an open row: into it.
  step(&mut tree, &mut app, "ArrowRight");
  assert_eq!(focused_id(&tree).as_deref(), Some("row-1"));
  // Out of row 60 to row 0, which is not built.
  for _ in 0..5 {
    step(&mut tree, &mut app, "PageDown");
  }
  for _ in 0..9 {
    step(&mut tree, &mut app, "ArrowDown");
  }
  assert_eq!(focused_id(&tree).as_deref(), Some("row-60"));
  step(&mut tree, &mut app, "ArrowLeft");
  assert_eq!(focused_id(&tree).as_deref(), Some("row-0"));
  // `*` at the top: row 61, the closed one beside row 0, is asked to open; Left on row 0 to close it.
  step(&mut tree, &mut app, "*");
  step(&mut tree, &mut app, "ArrowLeft");
  assert_eq!(*rows.asked.lock().unwrap(), [(61, true), (0, false)]);
}
