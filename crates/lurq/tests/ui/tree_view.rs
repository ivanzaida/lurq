//! A tree view: its rows against the board, what a press of a row, of its chevron and a double press do, the
//! keys of a tree, selecting many with Ctrl and Shift, a disabled row, children that come later, the guides,
//! and a theme's other anatomy.

use std::sync::{Arc, Mutex};

use lurq::{
  app::{
    App,
    events::MouseButton,
    theme::{PaletteColor, ThemePalette},
  },
  components::{Column, Row},
  core::Signal,
  layout::{
    quad::{Quad, QuadContent},
    text_style::FontWeight,
  },
  node::color::Color,
  ui::{TreeItem, TreeView, TreeViewPart, TreeViewRenderer, TreeViewView},
};

use super::stage::Stage;

const CHEVRON_RIGHT: &str = "\u{E01B}";
const CHEVRON_DOWN: &str = "\u{E019}";
const FILE: &str = "\u{E02E}";
const FOLDER: &str = "\u{E031}";
const FOLDER_OPEN: &str = "\u{E034}";

type Id = &'static str;

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

fn folder(id: Id) -> TreeItem<Id> {
  TreeItem::folder(id, id).element_id(id)
}

fn file(id: Id) -> TreeItem<Id> {
  TreeItem::file(id, id).element_id(id)
}

/// The board's tree: crates, with lurq (lib.rs, theme.rs) and demo (main.rs) in it; Cargo.toml.
fn files() -> Vec<TreeItem<Id>> {
  vec![
    folder("crates").children([
      folder("lurq").children([file("lib.rs"), file("theme.rs")]),
      folder("demo").child(file("main.rs")),
    ]),
    file("Cargo.toml"),
  ]
}

/// What a tree was asked and told.
#[derive(Clone)]
struct Held {
  open: Signal<Vec<Id>>,
  selected: Signal<Option<Id>>,
  activated: Arc<Mutex<Vec<Id>>>,
  expanded: Arc<Mutex<Vec<Id>>>,
}

impl Held {
  fn new(open: &[Id], selected: Option<Id>) -> Self {
    Self {
      open: Signal::new(open.to_vec()),
      selected: Signal::new(selected),
      activated: Arc::default(),
      expanded: Arc::default(),
    }
  }

  fn tree(&self, items: Vec<TreeItem<Id>>) -> TreeView<Id> {
    let (activated, expanded) = (self.activated.clone(), self.expanded.clone());
    TreeView::new(items)
      .label("Files")
      .expanded(self.open.clone())
      .selected(self.selected.clone())
      .on_activate(move |id| activated.lock().unwrap().push(id))
      .on_expand(move |id| expanded.lock().unwrap().push(id))
      .style(TreeViewPart::Root, |s| s.width(220.0))
      .id("tree")
  }
}

/// A stage of the board's tree, with `open` open and `selected` selected.
fn board(open: &[Id], selected: Option<Id>) -> (Stage, Held) {
  let held = Held::new(open, selected);
  let scene = held.clone();
  let stage = Stage::new(move |_| Column::new().padding(8.0).child(scene.tree(files())).into());
  (stage, held)
}

/// The quads of the text `content`.
fn texts(stage: &Stage, content: &str) -> Vec<Quad> {
  stage
    .quads()
    .into_iter()
    .filter(|quad| matches!(&quad.content, QuadContent::Text { text, .. } if text == content))
    .collect()
}

/// The text `content` painted in the row `id`.
fn in_row(stage: &mut Stage, id: &str, content: &str) -> Option<Quad> {
  let (_, y, _, height) = stage.bounds(id);
  texts(stage, content)
    .into_iter()
    .find(|quad| quad.y >= y - 0.01 && quad.y + quad.height <= y + height + 0.01)
}

fn color_of(quad: &Quad) -> Color {
  match &quad.content {
    QuadContent::Text { style, .. } => style.color,
    _ => panic!("a text"),
  }
}

fn focused(stage: &Stage) -> Option<String> {
  stage
    .tree
    .focused_element()
    .and_then(|element| element.id().map(str::to_owned))
}

/// A key with Shift, Ctrl or both.
fn key_with(stage: &mut Stage, key: &str, shift: bool, ctrl: bool) {
  stage.tree.key_down(key.to_owned(), key.to_owned(), shift, ctrl, false);
  stage.tree.key_up(key.to_owned(), key.to_owned(), shift, ctrl, false);
  stage.pass();
}

/// A press of the row `id` with Shift, Ctrl or both.
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

/// A press of the chevron of the row `id`.
fn press_chevron(stage: &mut Stage, id: &str) {
  let quad = [CHEVRON_RIGHT, CHEVRON_DOWN]
    .into_iter()
    .find_map(|chevron| in_row(stage, id, chevron))
    .unwrap_or_else(|| panic!("#{id} should have a chevron"));
  let (x, y) = (quad.x + quad.width / 2.0, quad.y + quad.height / 2.0);
  stage.tree.mouse_move(x, y);
  stage.tree.mouse_down(x, y, MouseButton::Left);
  stage.tree.mouse_up(x, y, MouseButton::Left);
  stage.pass();
}

#[test]
fn the_rows_are_28_high_2_apart_and_each_level_moves_a_row_in_by_16() {
  let (mut stage, _) = board(&["crates", "lurq"], Some("lib.rs"));
  let rows = ["crates", "lurq", "lib.rs", "theme.rs", "demo", "Cargo.toml"];
  let (x, top, ..) = stage.bounds("crates");
  for (index, id) in rows.into_iter().enumerate() {
    let (row_x, y, width, height) = stage.bounds(id);
    assert_eq!((row_x, width, height), (x, 220.0, 28.0), "{id}");
    assert!((y - (top + 30.0 * index as f32)).abs() < 0.01, "{id} at {y}");
  }
  // What a closed row holds is not there.
  assert!(!stage.paints("main.rs"));

  // The chevron is 8 in at the top and 16 more at each level; a row that does not open has its room, empty.
  for (id, level) in [("crates", 0.0), ("lurq", 1.0), ("demo", 1.0)] {
    let chevron = [CHEVRON_RIGHT, CHEVRON_DOWN]
      .into_iter()
      .find_map(|chevron| in_row(&mut stage, id, chevron))
      .unwrap();
    assert!(
      (chevron.x - (x + 8.0 + 16.0 * level)).abs() < 0.5,
      "{id}: {}",
      chevron.x - x
    );
  }
  // A file's icon after the chevron's room and 6, its label 16 and 6 after it: 8 + 32 + 14 + 6 + 16 + 6.
  let label = in_row(&mut stage, "lib.rs", "lib.rs").unwrap();
  assert!((label.x - (x + 82.0)).abs() < 0.5, "{}", label.x - x);
  let icon = in_row(&mut stage, "lib.rs", FILE).unwrap();
  assert!((icon.x - (x + 60.0)).abs() < 0.5, "{}", icon.x - x);
}

#[test]
fn an_open_row_has_its_chevron_down_and_its_open_icon_and_the_selected_one_is_highlighted() {
  let (mut stage, _) = board(&["crates", "lurq"], Some("lib.rs"));
  assert!(in_row(&mut stage, "crates", CHEVRON_DOWN).is_some());
  assert!(in_row(&mut stage, "crates", FOLDER_OPEN).is_some());
  assert!(in_row(&mut stage, "demo", CHEVRON_RIGHT).is_some());
  assert!(in_row(&mut stage, "demo", FOLDER).is_some());
  assert!(in_row(&mut stage, "Cargo.toml", FILE).is_some());
  assert!(
    [CHEVRON_RIGHT, CHEVRON_DOWN]
      .iter()
      .all(|chevron| in_row(&mut stage, "Cargo.toml", chevron).is_none()),
    "a file has no chevron"
  );

  // The label in the body's type at 13, in the foreground; the icons in the muted foreground.
  assert_eq!(stage.text("theme.rs"), (light(PaletteColor::Foreground), 13.0));
  let icon = in_row(&mut stage, "theme.rs", FILE).unwrap();
  assert_eq!(color_of(&icon), light(PaletteColor::MutedForeground));
  let chevron = in_row(&mut stage, "demo", CHEVRON_RIGHT).unwrap();
  assert_eq!(color_of(&chevron), light(PaletteColor::MutedForeground));

  // The selected row: the highlight, its label at 500 and its icon in the foreground. The others are not filled.
  assert_eq!(stage.fill_color("lib.rs"), Some(light(PaletteColor::Highlight)));
  assert_eq!(stage.fill_color("theme.rs"), None);
  let label = in_row(&mut stage, "lib.rs", "lib.rs").unwrap();
  let QuadContent::Text { style, .. } = &label.content else {
    unreachable!()
  };
  assert_eq!(style.weight, FontWeight::Medium);
  let icon = in_row(&mut stage, "lib.rs", FILE).unwrap();
  assert_eq!(color_of(&icon), light(PaletteColor::Foreground));
}

#[test]
fn a_press_selects_a_row_its_chevron_opens_it_and_a_double_press_activates_it() {
  let (mut stage, held) = board(&["crates"], None);
  stage.click("Cargo.toml");
  assert_eq!(held.selected.get(), Some("Cargo.toml"));
  assert!(held.activated.lock().unwrap().is_empty(), "one press only selects");

  // The chevron opens and closes the row, and does not select it.
  press_chevron(&mut stage, "lurq");
  assert_eq!(held.open.get(), ["crates", "lurq"]);
  assert_eq!(*held.expanded.lock().unwrap(), ["lurq"]);
  assert!(stage.paints("lib.rs"));
  assert_eq!(held.selected.get(), Some("Cargo.toml"));
  // A press elsewhere between the two, so that the second is not the second of a double press.
  stage.click("Cargo.toml");
  press_chevron(&mut stage, "lurq");
  assert_eq!(held.open.get(), ["crates"]);
  assert!(!stage.paints("lib.rs"));

  // A double press activates: a file is opened, a folder opens or closes as well.
  let (x, y) = stage.center("Cargo.toml");
  for _ in 0..2 {
    stage.tree.mouse_down(x, y, MouseButton::Left);
    stage.tree.mouse_up(x, y, MouseButton::Left);
  }
  stage.pass();
  assert_eq!(*held.activated.lock().unwrap(), ["Cargo.toml"]);
  let (x, y) = stage.center("demo");
  for _ in 0..2 {
    stage.tree.mouse_down(x, y, MouseButton::Left);
    stage.tree.mouse_up(x, y, MouseButton::Left);
  }
  stage.pass();
  assert_eq!(*held.activated.lock().unwrap(), ["Cargo.toml", "demo"]);
  assert!(held.open.get().contains(&"demo"));
  assert!(stage.paints("main.rs"));
}

#[test]
fn tab_enters_at_the_selected_row_and_the_keys_move_open_and_close_as_a_tree_does() {
  let (mut stage, held) = board(&["crates"], Some("demo"));
  stage.key("Tab");
  assert_eq!(
    focused(&stage).as_deref(),
    Some("demo"),
    "one stop, at the selected row"
  );
  assert!(stage.has_ring(light(PaletteColor::Ring)));

  // Right opens a closed row, and goes into an open one.
  stage.key("ArrowRight");
  assert_eq!(held.open.get(), ["crates", "demo"]);
  assert_eq!(focused(&stage).as_deref(), Some("demo"));
  stage.key("ArrowRight");
  assert_eq!(focused(&stage).as_deref(), Some("main.rs"));
  // Left goes out of a leaf, and closes an open row.
  stage.key("ArrowLeft");
  assert_eq!(focused(&stage).as_deref(), Some("demo"));
  stage.key("ArrowLeft");
  assert_eq!(held.open.get(), ["crates"]);
  assert!(!stage.paints("main.rs"));
  stage.key("ArrowLeft");
  assert_eq!(focused(&stage).as_deref(), Some("crates"));

  // Up and Down through what is shown, Home and End to the ends; no wrapping.
  stage.key("ArrowUp");
  assert_eq!(focused(&stage).as_deref(), Some("crates"));
  stage.key("End");
  assert_eq!(focused(&stage).as_deref(), Some("Cargo.toml"));
  stage.key("ArrowUp");
  assert_eq!(focused(&stage).as_deref(), Some("demo"));

  // `*` opens every row at the level beside the focused one.
  stage.key("*");
  assert_eq!(held.open.get(), ["crates", "lurq", "demo"]);

  // A letter goes to the next row that starts with it.
  stage.key("t");
  assert_eq!(focused(&stage).as_deref(), Some("theme.rs"));

  // Space selects, Enter activates.
  stage.key(" ");
  assert_eq!(held.selected.get(), Some("theme.rs"));
  stage.key("Enter");
  assert_eq!(*held.activated.lock().unwrap(), ["theme.rs"]);
  assert_eq!(held.selected.get(), Some("theme.rs"));
}

#[test]
fn with_multiple_ctrl_adds_shift_selects_a_range_and_space_and_ctrl_a_select_too() {
  let open = Signal::new(vec!["crates", "lurq", "demo"]);
  let selected: Signal<Vec<Id>> = Signal::new(Vec::new());
  let (scene_open, scene_selected) = (open.clone(), selected.clone());
  let mut stage = Stage::new(move |_| {
    Column::new()
      .child(
        TreeView::new(files())
          .expanded(scene_open.clone())
          .selected(scene_selected.clone())
          .multiple(true),
      )
      .into()
  });
  stage.click("lurq");
  assert_eq!(selected.get(), ["lurq"]);
  press_with(&mut stage, "theme.rs", false, true);
  assert_eq!(selected.get(), ["lurq", "theme.rs"], "Ctrl adds");
  press_with(&mut stage, "lurq", false, true);
  assert_eq!(selected.get(), ["theme.rs"], "and takes out");
  // Shift: from the last selected to the row pressed, the anchor kept last.
  press_with(&mut stage, "main.rs", true, false);
  assert_eq!(selected.get(), ["demo", "main.rs", "theme.rs"]);
  // A press alone selects that row alone.
  stage.click("lib.rs");
  assert_eq!(selected.get(), ["lib.rs"]);

  // Shift with the arrows moves the focus and selects the rows on the way.
  key_with(&mut stage, "ArrowDown", true, false);
  assert_eq!(focused(&stage).as_deref(), Some("theme.rs"));
  key_with(&mut stage, "ArrowDown", true, false);
  assert_eq!(selected.get(), ["theme.rs", "demo", "lib.rs"]);
  key_with(&mut stage, "ArrowUp", true, false);
  assert_eq!(selected.get(), ["theme.rs", "lib.rs"]);
  // Space adds the focused row or takes it out.
  stage.key(" ");
  assert_eq!(selected.get(), ["lib.rs"]);
  // Ctrl+A: every row shown.
  key_with(&mut stage, "a", false, true);
  assert_eq!(selected.get().len(), 7);
  assert_eq!(selected.get().last(), Some(&"lib.rs"));
}

#[test]
fn a_disabled_row_takes_nothing_and_the_keys_pass_it_by() {
  let held = Held::new(&["crates"], None);
  let scene = held.clone();
  let mut stage = Stage::new(move |_| {
    let items = vec![
      folder("crates").children([folder("lurq").disabled(true).child(file("lib.rs")), folder("demo")]),
      file("Cargo.toml"),
    ];
    Column::new().child(scene.tree(items)).into()
  });
  stage.click("lurq");
  assert_eq!(held.selected.get(), None);
  assert!(in_row(&mut stage, "lurq", CHEVRON_RIGHT).is_some());
  press_chevron(&mut stage, "lurq");
  assert_eq!(held.open.get(), ["crates"]);

  stage.key("Tab");
  assert_eq!(focused(&stage).as_deref(), Some("crates"));
  stage.key("ArrowDown");
  assert_eq!(focused(&stage).as_deref(), Some("demo"), "past the disabled row");

  // A disabled tree: no row takes anything.
  let held = Held::new(&["crates"], None);
  let scene = held.clone();
  let mut stage = Stage::new(move |_| Column::new().child(scene.tree(files()).disabled(true)).into());
  stage.click("crates");
  press_chevron(&mut stage, "crates");
  assert_eq!((held.selected.get(), held.open.get()), (None, vec!["crates"]));
}

#[test]
fn a_lazy_row_opens_with_a_spinner_until_the_app_brings_its_children() {
  let held = Held::new(&[], None);
  let loaded = Signal::new(false);
  let (scene, scene_loaded) = (held.clone(), loaded.clone());
  let mut stage = Stage::new(move |_| {
    let mut remote = TreeItem::folder("origin", "origin").element_id("origin").lazy(true);
    if scene_loaded.get() {
      remote = remote.child(file("main"));
    }
    Column::new().child(scene.tree(vec![remote])).into()
  });
  // Closed, it has a chevron as a folder has.
  assert!(in_row(&mut stage, "origin", CHEVRON_RIGHT).is_some());
  stage.key("Tab");
  stage.key("ArrowRight");
  assert_eq!(*held.expanded.lock().unwrap(), ["origin"], "the app is told it opened");
  // Open with no children: a spinner where the chevron was.
  assert!(
    [CHEVRON_RIGHT, CHEVRON_DOWN]
      .iter()
      .all(|chevron| in_row(&mut stage, "origin", chevron).is_none())
  );
  assert!(!stage.paints("main"));
  loaded.set(true);
  stage.pass();
  assert!(in_row(&mut stage, "origin", CHEVRON_DOWN).is_some());
  assert!(stage.paints("main"));
}

#[test]
fn guides_run_under_the_chevron_of_the_row_a_row_is_in() {
  let held = Held::new(&["crates", "lurq"], None);
  let scene = held.clone();
  let mut stage = Stage::new(move |_| Column::new().child(scene.tree(files()).guides(true)).into());
  let border = light(PaletteColor::Border);
  let (x, ..) = stage.bounds("crates");
  let lines: Vec<Quad> = stage
    .quads()
    .into_iter()
    .filter(|quad| matches!(&quad.content, QuadContent::Rect { color, .. } if *color == border) && quad.width == 1.0)
    .collect();
  // lurq and demo have one each, lib.rs and theme.rs two: six lines, as high as a row and the room under it.
  assert_eq!(lines.len(), 6);
  assert!(lines.iter().all(|line| line.height == 30.0));
  // The first level's under the middle of crates's chevron (8 + 7), the second's under lurq's (24 + 7).
  let mut columns: Vec<f32> = lines.iter().map(|line| line.x - x).collect();
  columns.sort_by(f32::total_cmp);
  columns.dedup_by(|a, b| (*a - *b).abs() < 0.01);
  assert_eq!(columns.len(), 2);
  assert!((columns[0] - 14.5).abs() < 0.5, "{columns:?}");
  assert!((columns[1] - 30.5).abs() < 0.5, "{columns:?}");
}

#[test]
fn a_renderer_sees_the_rows_shown_with_their_levels_and_places() {
  let seen = Arc::new(Mutex::new(Vec::new()));
  let told = seen.clone();
  let renderer: TreeViewRenderer = Arc::new(move |view: TreeViewView| {
    *told.lock().unwrap() = view
      .rows
      .iter()
      .map(|row| {
        (
          row.label.to_string(),
          row.level,
          row.position,
          row.expanded,
          row.selected,
        )
      })
      .collect();
    Row::new().id("other").size(10.0, 10.0).into()
  });
  let app = App::new();
  app.theme().set_renderer::<TreeView<Id>>(renderer);
  let held = Held::new(&["crates", "demo"], Some("main.rs"));
  let scene = held.clone();
  let mut stage = Stage::in_app(app, move |_| Column::new().child(scene.tree(files())).into());
  assert_eq!(stage.bounds("other").2, 10.0);
  assert_eq!(
    *seen.lock().unwrap(),
    [
      ("crates".to_owned(), 1, (1, 2), true, false),
      ("lurq".to_owned(), 2, (1, 2), false, false),
      ("demo".to_owned(), 2, (2, 2), true, false),
      ("main.rs".to_owned(), 3, (1, 1), false, true),
      ("Cargo.toml".to_owned(), 1, (2, 2), false, false),
    ]
  );
}
