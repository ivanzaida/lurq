//! A tree view: its rows against the board, what a press of a row, of its chevron and a double press do, the
//! keys of a tree, selecting many with Ctrl and Shift, a disabled row, children that come later, the guides,
//! a theme's other anatomy, a tree of ten thousand rows in a virtual list that the keys reach every row of, and
//! what stands at a row's end shown only while the row is pointed at or focused, and a row opening and closing
//! with its height in either form.

use std::{
  sync::{Arc, Mutex},
  time::Duration,
};

use lurq::{
  app::{
    App,
    ctx::Ctx,
    events::MouseButton,
    theme::{Density, PaletteColor, Theme, ThemePalette},
  },
  components::{Column, Row, Text},
  core::Signal,
  layout::{
    quad::{Quad, QuadContent},
    text_style::FontWeight,
  },
  node::{Element, color::Color},
  ui::{Case, Opacity, Recipe, Themed, TreeItem, TreeView, TreeViewPart, TreeViewRenderer, TreeViewView},
};

use super::stage::Stage;

/// The chevron: down while its row is open, turned to the end while it is closed.
const CHEVRON: &str = "\u{E019}";
const FILE: &str = "\u{E02E}";
const FOLDER: &str = "\u{E031}";
const FOLDER_OPEN: &str = "\u{E034}";

type Id = &'static str;

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

fn folder(id: Id) -> TreeItem<Id> {
  TreeItem::folder(id, id).id(id)
}

fn file(id: Id) -> TreeItem<Id> {
  TreeItem::file(id, id).id(id)
}

/// The board's tree: crates, with lurq (lib.rs, theme.rs) and demo (main.rs) in it; Cargo.toml.
fn files() -> Vec<TreeItem<Id>> {
  vec![
    folder("crates").items([
      folder("lurq").items([file("lib.rs"), file("theme.rs")]),
      folder("demo").item(file("main.rs")),
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
    TreeView::new(self.selected.clone(), items)
      .label("Files")
      .open(self.open.clone())
      .on_activate(move |id| activated.lock().unwrap().push(id))
      .on_open_change(move |id, open| {
        if open {
          expanded.lock().unwrap().push(id)
        }
      })
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

/// Where `quad` is painted, turned or not: its left, its top, its right and its bottom.
fn painted(quad: &Quad) -> (f32, f32, f32, f32) {
  let [ox, oy] = quad.transform_origin.unwrap_or([quad.width / 2.0, quad.height / 2.0]);
  let t = &quad.transform;
  let corners = [
    (0.0, 0.0),
    (quad.width, 0.0),
    (0.0, quad.height),
    (quad.width, quad.height),
  ]
  .map(|(u, v)| {
    (
      quad.x + ox + t.a * (u - ox) + t.c * (v - oy) + t.tx,
      quad.y + oy + t.b * (u - ox) + t.d * (v - oy) + t.ty,
    )
  });
  let (xs, ys) = (corners.map(|(x, _)| x), corners.map(|(_, y)| y));
  let low = |values: [f32; 4]| values.into_iter().fold(f32::MAX, f32::min);
  let high = |values: [f32; 4]| values.into_iter().fold(f32::MIN, f32::max);
  (low(xs), low(ys), high(xs), high(ys))
}

/// The text `content` painted in the row `id`.
fn in_row(stage: &mut Stage, id: &str, content: &str) -> Option<Quad> {
  let (_, y, _, height) = stage.bounds(id);
  texts(stage, content).into_iter().find(|quad| {
    let (_, top, _, bottom) = painted(quad);
    top >= y - 0.01 && bottom <= y + height + 0.01
  })
}

/// Whether the chevron `quad` is turned to the end: its row is closed.
fn turned(quad: &Quad) -> bool {
  quad.transform.b.abs() > 0.99
}

/// The chevron of the row `id`, down while it is open.
fn open_chevron(stage: &mut Stage, id: &str) -> Option<Quad> {
  in_row(stage, id, CHEVRON).filter(|quad| !turned(quad))
}

/// The chevron of the row `id`, turned to the end while it is closed.
fn closed_chevron(stage: &mut Stage, id: &str) -> Option<Quad> {
  in_row(stage, id, CHEVRON).filter(turned)
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
  let quad = in_row(stage, id, CHEVRON).unwrap_or_else(|| panic!("#{id} should have a chevron"));
  let (left, top, right, bottom) = painted(&quad);
  let (x, y) = ((left + right) / 2.0, (top + bottom) / 2.0);
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
    let chevron = in_row(&mut stage, id, CHEVRON).unwrap();
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
  assert!(open_chevron(&mut stage, "crates").is_some());
  assert!(in_row(&mut stage, "crates", FOLDER_OPEN).is_some());
  assert!(closed_chevron(&mut stage, "demo").is_some());
  assert!(in_row(&mut stage, "demo", FOLDER).is_some());
  assert!(in_row(&mut stage, "Cargo.toml", FILE).is_some());
  assert!(
    in_row(&mut stage, "Cargo.toml", CHEVRON).is_none(),
    "a file has no chevron"
  );

  // The label in the body's type at 13, in the foreground; the icons in the muted foreground.
  assert_eq!(stage.text("theme.rs"), (light(PaletteColor::Foreground), 13.0));
  let icon = in_row(&mut stage, "theme.rs", FILE).unwrap();
  assert_eq!(color_of(&icon), light(PaletteColor::MutedForeground));
  let chevron = closed_chevron(&mut stage, "demo").unwrap();
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
  // Every press at one time: two of a row in a row are a double press however slow the machine.
  stage.presses_at(Duration::ZERO);
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
fn a_press_dragged_off_the_row_selects_nothing() {
  let (mut stage, held) = board(&["crates"], None);
  let (x, y) = stage.center("Cargo.toml");
  let (off_x, off_y) = stage.center("crates");
  stage.tree.mouse_move(x, y);
  stage.tree.mouse_down(x, y, MouseButton::Left);
  stage.pass();
  assert_eq!(held.selected.get(), None, "the press alone selects nothing");
  stage.tree.mouse_move(off_x, off_y);
  stage.tree.mouse_up(off_x, off_y, MouseButton::Left);
  stage.pass();
  assert_eq!(held.selected.get(), None, "released off the row it went down on");

  // Released where it went down, it selects.
  stage.tree.mouse_move(x, y);
  stage.tree.mouse_down(x, y, MouseButton::Left);
  stage.tree.mouse_up(x, y, MouseButton::Left);
  stage.pass();
  assert_eq!(held.selected.get(), Some("Cargo.toml"));
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
      .child(TreeView::multiple(scene_selected.clone(), files()).open(scene_open.clone()))
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
  stage.shortcut("a");
  assert_eq!(selected.get().len(), 7);
  assert_eq!(selected.get().last(), Some(&"lib.rs"));
}

#[test]
fn a_disabled_row_takes_nothing_and_the_keys_pass_it_by() {
  let held = Held::new(&["crates"], None);
  let scene = held.clone();
  let mut stage = Stage::new(move |_| {
    let items = vec![
      folder("crates").items([folder("lurq").disabled(true).item(file("lib.rs")), folder("demo")]),
      file("Cargo.toml"),
    ];
    Column::new().child(scene.tree(items)).into()
  });
  stage.click("lurq");
  assert_eq!(held.selected.get(), None);
  assert!(closed_chevron(&mut stage, "lurq").is_some());
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
    let mut remote = TreeItem::folder("origin", "origin").id("origin").lazy(true);
    if scene_loaded.get() {
      remote = remote.item(file("main"));
    }
    Column::new().child(scene.tree(vec![remote])).into()
  });
  // Closed, it has a chevron as a folder has.
  assert!(closed_chevron(&mut stage, "origin").is_some());
  stage.key("Tab");
  stage.key("ArrowRight");
  assert_eq!(*held.expanded.lock().unwrap(), ["origin"], "the app is told it opened");
  // Open with no children: a spinner where the chevron was.
  assert!(in_row(&mut stage, "origin", CHEVRON).is_none());
  assert!(!stage.paints("main"));
  loaded.set(true);
  stage.pass();
  assert!(open_chevron(&mut stage, "origin").is_some());
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
      .map(|row| (row.label.to_string(), row.level, row.position, row.open, row.selected))
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

/// A hundred folders of a hundred files: folder `i` is `i * 1000`, its file `j` is `i * 1000 + j + 1`; folder 87
/// is called "Zeta". Rows are `r{i}` and `r{i}-{j}`.
fn big() -> Vec<TreeItem<u32>> {
  (0..100u32)
    .map(|i| {
      let label = if i == 87 {
        "Zeta".to_owned()
      } else {
        format!("Folder {i}")
      };
      TreeItem::folder(i * 1000, &label).id(format!("r{i}")).items(
        (0..100u32).map(move |j| TreeItem::file(i * 1000 + j + 1, &format!("file {i}.{j}")).id(format!("r{i}-{j}"))),
      )
    })
    .collect()
}

/// The big tree in a virtual list 240 high, `open` open, any number selected.
fn big_stage(open: Vec<u32>) -> (Stage, Signal<Vec<u32>>, Signal<Vec<u32>>) {
  let (open, selected) = (Signal::new(open), Signal::new(Vec::new()));
  let (scene_open, scene_selected) = (open.clone(), selected.clone());
  let stage = Stage::new(move |ctx| {
    Column::new()
      .child(
        TreeView::multiple(scene_selected.clone(), big())
          .label("Files")
          .open(scene_open.clone())
          .style(TreeViewPart::Root, |s| s.width(220.0).height(240.0))
          .id("tree")
          .mount(ctx),
      )
      .into()
  });
  (stage, open, selected)
}

/// A key, then the passes in which the list builds the row it went to and the focus goes there.
fn settle(stage: &mut Stage, key: &str) {
  stage.key(key);
  stage.pass();
  stage.pass();
}

#[test]
fn a_tree_in_a_virtual_list_builds_the_rows_in_view_and_the_keys_reach_every_row() {
  let every: Vec<u32> = (0..100).map(|i| i * 1000).collect();
  let (mut stage, open, _) = big_stage(every);
  // 10100 rows are shown; a few are built, each as wide as the tree.
  let built = stage
    .quads()
    .iter()
    .filter(|quad| matches!(&quad.content, QuadContent::Text { text, .. } if text.starts_with("file ")))
    .count();
  assert!(built > 0 && built < 60, "{built} rows built");
  assert_eq!(stage.bounds("r0").2, 220.0);

  stage.key("Tab");
  assert_eq!(focused(&stage).as_deref(), Some("r0"));
  settle(&mut stage, "End");
  assert_eq!(focused(&stage).as_deref(), Some("r99-99"));
  settle(&mut stage, "Home");
  assert_eq!(focused(&stage).as_deref(), Some("r0"));
  // A page is as many rows as the view holds: 240 over rows of 28 and 2 apart.
  settle(&mut stage, "PageDown");
  assert_eq!(focused(&stage).as_deref(), Some("r0-7"));
  settle(&mut stage, "PageUp");
  assert_eq!(focused(&stage).as_deref(), Some("r0"));

  // A letter: Zeta, 8787 rows down.
  settle(&mut stage, "z");
  assert_eq!(focused(&stage).as_deref(), Some("r87"));
  // Left closes it, Right opens it again and goes in.
  settle(&mut stage, "ArrowLeft");
  assert!(!open.get().contains(&87000));
  settle(&mut stage, "ArrowRight");
  assert!(open.get().contains(&87000));
  settle(&mut stage, "ArrowRight");
  assert_eq!(focused(&stage).as_deref(), Some("r87-0"));
  // From the last file of a folder, Left goes out to the folder, a hundred rows up, not built.
  settle(&mut stage, "End");
  settle(&mut stage, "ArrowLeft");
  assert_eq!(focused(&stage).as_deref(), Some("r99"));
}

#[test]
fn shift_with_the_keys_and_ctrl_a_select_rows_that_are_not_built() {
  let every: Vec<u32> = (0..100).map(|i| i * 1000).collect();
  let (mut stage, _, selected) = big_stage(every);
  stage.key("Tab");
  stage.key(" ");
  assert_eq!(selected.get(), [0]);
  // Shift+End: from the anchor to the last row, and the focus goes there.
  stage
    .tree
    .key_down("End".to_owned(), "End".to_owned(), true, false, false);
  stage.pass();
  stage.pass();
  assert_eq!(selected.get().len(), 10100);
  assert_eq!(selected.get().last(), Some(&0), "the anchor stays last");
  assert_eq!(focused(&stage).as_deref(), Some("r99-99"));
  // A press of the last row alone, then Shift+PageUp: a page of rows up.
  stage.click("r99-99");
  stage
    .tree
    .key_down("PageUp".to_owned(), "PageUp".to_owned(), true, false, false);
  stage.pass();
  stage.pass();
  assert_eq!(selected.get().len(), 9);
  // Ctrl+A: every row shown.
  stage.shortcut("a");
  assert_eq!(selected.get().len(), 10100);
}

#[test]
fn star_opens_every_folder_at_the_top_of_a_tree_in_a_virtual_list() {
  let (mut stage, open, _) = big_stage(Vec::new());
  stage.key("Tab");
  settle(&mut stage, "*");
  assert_eq!(open.get().len(), 100);
  settle(&mut stage, "End");
  assert_eq!(focused(&stage).as_deref(), Some("r99-99"));
}

#[test]
fn what_an_item_marks_shows_at_the_row_end_only_while_the_row_is_pointed_at_or_focused() {
  // The design: hidden at rest, shown in the group's light, and always under a finger.
  let theme = Theme::new();
  let recipe = TreeView::<u32>::recipe();
  let rest = recipe.resolve(&Case::new(), &theme);
  assert_eq!(
    rest.part(TreeViewPart::Reveal).styles().rest.opacity,
    Some(Opacity::Value(0.0))
  );
  assert_eq!(
    rest.part(TreeViewPart::Reveal).styles().within.opacity,
    Some(Opacity::Value(1.0))
  );
  let touch = recipe.resolve(&Case::new().density(Density::Touch), &theme);
  assert_eq!(
    touch.part(TreeViewPart::Reveal).rest().opacity,
    Some(Opacity::Value(1.0))
  );

  // Live: the part is lit with the row. A rule of the test's paints it while it is.
  let lit = light(PaletteColor::Primary);
  let mut stage = Stage::new(move |_| {
    Column::new()
      .child(
        TreeView::new(
          Signal::new(None),
          [
            TreeItem::file("a", "a.rs")
              .id("a")
              .trailing(Text::new("new"))
              .trailing_on_hover(true),
            TreeItem::file("b", "b.rs").id("b").trailing(Text::new("2")),
          ],
        )
        .rules(&Recipe::new().within(TreeViewPart::Reveal, |s| s.background(PaletteColor::Primary)))
        .style(TreeViewPart::Root, |s| s.width(220.0)),
      )
      .into()
  });
  let shown = |stage: &Stage| {
    stage
      .quads()
      .iter()
      .any(|quad| matches!(&quad.content, QuadContent::Rect { color, .. } if *color == lit))
  };
  assert!(!shown(&stage), "at rest");
  stage.hover("a");
  assert!(shown(&stage), "the row under the pointer");
  stage.hover("b");
  assert!(!shown(&stage), "another row under the pointer");
  stage.key("Tab");
  assert_eq!(focused(&stage).as_deref(), Some("a"));
  assert!(shown(&stage), "the row with the focus");
}

#[test]
fn a_rows_height_follows_the_density() {
  for (density, height) in [
    (Density::Compact, 20.0),
    (Density::Default, 28.0),
    (Density::Comfortable, 36.0),
    (Density::Touch, 40.0),
  ] {
    let app = App::new();
    app.theme().set_density(density);
    let held = Held::new(&["crates"], None);
    let mut stage = Stage::in_app(app, move |_| {
      Column::new().padding(8.0).child(held.tree(files())).into()
    });
    let (_, top, _, row) = stage.bounds("crates");
    let (_, next, ..) = stage.bounds("lurq");
    assert_eq!(row, height, "{density:?}");
    assert!((next - top - (height + 2.0)).abs() < 0.01, "{density:?}: 2 apart");
  }
}

#[test]
fn on_change_is_told_the_new_selection_a_person_makes_and_not_what_is_so_or_what_the_app_sets() {
  let told = Arc::new(Mutex::new(Vec::<Id>::new()));
  let second = Arc::new(Mutex::new(0));
  let held = Held::new(&["crates"], None);
  let (scene, tell, also) = (held.clone(), told.clone(), second.clone());
  let mut stage = Stage::new(move |_| {
    let (tell, also) = (tell.clone(), also.clone());
    Column::new()
      .padding(8.0)
      .child(
        scene
          .tree(files())
          .on_change(move |chosen: &Id| tell.lock().unwrap().push(*chosen))
          .on_change(move |_| *also.lock().unwrap() += 1),
      )
      .into()
  });
  stage.click("Cargo.toml");
  assert_eq!(*told.lock().unwrap(), ["Cargo.toml"], "the row a tree of one selects");
  assert_eq!(*second.lock().unwrap(), 1, "a second handler is called after the first");
  assert_eq!(focused(&stage).as_deref(), Some("Cargo.toml"));
  // Space on the row that is selected already changes nothing, and tells nothing.
  stage.key(" ");
  assert_eq!(told.lock().unwrap().len(), 1);
  // Nor does the app setting the signal.
  held.selected.set(Some("crates"));
  stage.pass();
  assert_eq!(told.lock().unwrap().len(), 1);
  assert_eq!(stage.fill_color("crates"), Some(light(PaletteColor::Highlight)));
}

#[test]
fn a_controlled_tree_shows_what_the_app_answers_with() {
  // The app is told each change and answers with what it wants: it keeps Cargo.toml selected whatever is asked.
  let answer = Signal::new(vec!["Cargo.toml"]);
  let asked = Arc::new(Mutex::new(Vec::<Vec<Id>>::new()));
  let (scene, tell) = (answer.clone(), asked.clone());
  let mut stage = Stage::new(move |_| {
    let (answer, tell) = (scene.clone(), tell.clone());
    let held = answer.get();
    Column::new()
      .padding(8.0)
      .child(
        TreeView::controlled_multiple(held, files(), move |chosen| {
          tell.lock().unwrap().push(chosen.to_vec());
          answer.set(vec!["Cargo.toml"]);
        })
        .open(Signal::new(vec!["crates"])),
      )
      .into()
  });
  assert_eq!(stage.fill_color("Cargo.toml"), Some(light(PaletteColor::Highlight)));
  stage.click("crates");
  assert_eq!(*asked.lock().unwrap(), [vec!["crates"]], "told what a person asked for");
  assert_eq!(
    stage.fill_color("Cargo.toml"),
    Some(light(PaletteColor::Highlight)),
    "the app's answer is shown"
  );
  assert_eq!(stage.fill_color("crates"), None);
}

#[test]
fn on_open_change_is_told_each_row_opened_and_closed() {
  let told = Arc::new(Mutex::new(Vec::new()));
  let held = Held::new(&["crates"], None);
  let (scene, tell) = (held.clone(), told.clone());
  let mut stage = Stage::new(move |_| {
    let tell = tell.clone();
    Column::new()
      .padding(8.0)
      .child(
        scene
          .tree(files())
          .on_open_change(move |row, open| tell.lock().unwrap().push((*row, open))),
      )
      .into()
  });
  press_chevron(&mut stage, "lurq");
  press_chevron(&mut stage, "crates");
  assert_eq!(*told.lock().unwrap(), [("lurq", true), ("crates", false)]);
  assert_eq!(held.open.get(), ["lurq"]);
}

#[test]
fn an_item_given_no_icon_has_none_in_place_of_its_folders() {
  let stage = Stage::new(|_| {
    Column::new()
      .padding(8.0)
      .child(TreeView::new(
        Signal::new(None),
        [folder("plain").icon(None::<lurq::ui::Name>), folder("dressed")],
      ))
      .into()
  });
  let mut stage = stage;
  assert!(in_row(&mut stage, "plain", FOLDER).is_none());
  assert!(in_row(&mut stage, "dressed", FOLDER).is_some());
}

#[test]
fn a_row_with_a_key_keeps_its_focus_in_a_virtual_list_as_a_row_is_put_in_before_it() {
  let first = Signal::new(false);
  let selected: Signal<Option<Id>> = Signal::new(None);
  let (scene, chosen) = (first.clone(), selected.clone());
  let mut stage = Stage::new(move |ctx| {
    let mut items = Vec::new();
    if scene.get() {
      items.push(TreeItem::file("new", "new.rs").key("new"));
    }
    items.push(TreeItem::file("a", "a.rs").key("a"));
    items.push(TreeItem::file("b", "b.rs").key("b"));
    Column::new()
      .padding(8.0)
      .child(
        TreeView::new(chosen.clone(), items)
          .style(TreeViewPart::Root, |s| s.width(220.0).height(200.0))
          .mount(ctx),
      )
      .into()
  });
  settle(&mut stage, "Tab");
  settle(&mut stage, "ArrowDown");
  first.set(true);
  stage.pass();
  stage.pass();
  // Space selects the row that has the focus: b.rs, which a row was put in before.
  stage.key(" ");
  assert_eq!(selected.get(), Some("b"));
}

#[test]
fn a_controlled_tree_of_one_is_told_the_row_and_shows_the_answer() {
  let answer: Signal<Option<Id>> = Signal::new(Some("Cargo.toml"));
  let asked = Arc::new(Mutex::new(Vec::<Id>::new()));
  let (scene, tell) = (answer.clone(), asked.clone());
  let mut stage = Stage::new(move |_| {
    let (answer, tell) = (scene.clone(), tell.clone());
    Column::new()
      .padding(8.0)
      .child(
        TreeView::controlled(answer.get(), files(), move |row: &Id| {
          tell.lock().unwrap().push(*row);
          // The app takes the row asked for.
          answer.set(Some(*row));
        })
        .open(Signal::new(vec!["crates"])),
      )
      .into()
  });
  stage.click("crates");
  assert_eq!(*asked.lock().unwrap(), ["crates"]);
  assert_eq!(stage.fill_color("crates"), Some(light(PaletteColor::Highlight)));
  assert_eq!(stage.fill_color("Cargo.toml"), None);
}

#[test]
fn a_tree_of_any_number_is_told_the_whole_new_set() {
  let selected: Signal<Vec<Id>> = Signal::new(vec!["Cargo.toml"]);
  let told = Arc::new(Mutex::new(Vec::<Vec<Id>>::new()));
  let (scene, tell) = (selected.clone(), told.clone());
  let mut stage = Stage::new(move |_| {
    let tell = tell.clone();
    Column::new()
      .padding(8.0)
      .child(
        TreeView::multiple(scene.clone(), files())
          .open(Signal::new(vec!["crates"]))
          .on_change(move |rows: &[Id]| tell.lock().unwrap().push(rows.to_vec())),
      )
      .into()
  });
  press_with(&mut stage, "crates", false, true);
  assert_eq!(*told.lock().unwrap(), [vec!["Cargo.toml", "crates"]]);
  assert_eq!(selected.get(), ["Cargo.toml", "crates"]);
}

/// Presses a key with no pass after it: what it starts is caught on its way. The clock is set to the present first:
/// what the key starts runs from it, and not from the second the stage's last pass went on to.
fn press_key(stage: &mut Stage, key: &str) {
  stage.pass_after(Duration::ZERO);
  stage.tree.key_down(key.to_owned(), key.to_owned(), false, false, false);
  stage.tree.key_up(key.to_owned(), key.to_owned(), false, false, false);
}

/// Runs the timers of the motion of a tree in a virtual list that are due `after` from now, and a pass.
fn tick(stage: &mut Stage, after: Duration) {
  stage.tree.tick_timers_at(lurq::time::Instant::now() + after);
  stage.pass_after(Duration::ZERO);
}

fn top(stage: &mut Stage, id: &str) -> f32 {
  stage.bounds(id).1
}

/// The board's tree with crates and lurq open, one that builds every row or, `windowed`, the rows in view of a
/// virtual list 300 high; in `app` where it is given.
fn moving(windowed: bool, app: Option<App>) -> (Stage, Held) {
  let held = Held::new(&["crates", "lurq"], None);
  let scene = held.clone();
  let build = move |ctx: &mut Ctx| -> Element {
    let tree = scene.tree(files());
    let tree: Element = if windowed {
      tree.style(TreeViewPart::Root, |s| s.height(300.0)).mount(ctx)
    } else {
      tree.into()
    };
    Column::new().padding(8.0).child(tree).into()
  };
  let stage = match app {
    Some(app) => Stage::in_app(app, build),
    None => Stage::new(build),
  };
  (stage, held)
}

/// The motion of a row in a tree of either form: it closes and opens with its height, what is below moving through
/// every place between, and turns round when it is pressed again on the way.
fn a_row_opens_and_closes_with_its_height(windowed: bool) {
  let (mut stage, held) = moving(windowed, None);
  let opened = top(&mut stage, "Cargo.toml");
  stage.key("Tab");
  assert_eq!(focused(&stage).as_deref(), Some("crates"));

  // Left closes it: at the start what is below has not moved, on the way it is between, at the end it is up by
  // the four rows it held.
  press_key(&mut stage, "ArrowLeft");
  stage.pass_after(Duration::ZERO);
  let start = top(&mut stage, "Cargo.toml");
  assert!(
    (start - opened).abs() < 1.0,
    "nothing jumps at the start: {opened} -> {start}"
  );
  stage.pass_after(Duration::from_millis(60));
  let on_the_way = top(&mut stage, "Cargo.toml");
  tick(&mut stage, Duration::from_secs(2));
  stage.pass();
  let closed = top(&mut stage, "Cargo.toml");
  assert!(
    (opened - closed - 120.0).abs() < 1.0,
    "four rows of 30 less: {opened} -> {closed}"
  );
  assert!(
    on_the_way < opened - 0.5 && on_the_way > closed + 0.5,
    "on the way: {closed} < {on_the_way} < {opened}"
  );
  assert!(!stage.paints("lib.rs"), "closed, its rows are gone");
  assert_eq!(held.open.get(), ["lurq"]);
  // Its rows are not reached once it has closed: Down goes on to the next row.
  stage.key("ArrowDown");
  assert_eq!(focused(&stage).as_deref(), Some("Cargo.toml"));
  stage.key("ArrowUp");

  // Right opens it, and Left on the way turns it round from where it is. Rows built for it in a virtual list are
  // given their height on the turn of the loop after the one that laid them out at none: its timers are ticked.
  press_key(&mut stage, "ArrowRight");
  stage.pass_after(Duration::ZERO);
  tick(&mut stage, Duration::from_millis(1));
  stage.pass_after(Duration::from_millis(60));
  let opening = top(&mut stage, "Cargo.toml");
  assert!(
    opening > closed + 0.5 && opening < opened - 0.5,
    "opening: {closed} < {opening} < {opened}"
  );
  press_key(&mut stage, "ArrowLeft");
  stage.pass_after(Duration::from_millis(60));
  let turned = top(&mut stage, "Cargo.toml");
  assert!(
    turned <= opening + 0.5,
    "turned round, it goes back up from where it was: {opening} -> {turned}"
  );
  tick(&mut stage, Duration::from_secs(2));
  stage.pass();
  assert!((top(&mut stage, "Cargo.toml") - closed).abs() < 1.0, "closed again");
  assert_eq!(
    focused(&stage).as_deref(),
    Some("crates"),
    "the focus stayed on the row through it all"
  );

  // Opened to the end, its rows are there and reached again, the open row in it open still.
  stage.key("ArrowRight");
  tick(&mut stage, Duration::from_millis(1));
  stage.pass();
  tick(&mut stage, Duration::from_secs(2));
  stage.pass();
  assert!((top(&mut stage, "Cargo.toml") - opened).abs() < 1.0, "open again");
  assert!(stage.paints("lib.rs"));
  stage.key("ArrowDown");
  assert_eq!(focused(&stage).as_deref(), Some("lurq"));
}

#[test]
fn a_row_of_the_tree_that_builds_every_row_opens_and_closes_with_its_height() {
  a_row_opens_and_closes_with_its_height(false);
}

#[test]
fn a_row_of_the_tree_that_builds_the_rows_in_view_opens_and_closes_with_its_height() {
  a_row_opens_and_closes_with_its_height(true);
}

#[test]
fn while_a_row_closes_the_arrows_and_typeahead_pass_its_rows_by_and_right_turns_it_round() {
  for windowed in [false, true] {
    let (mut stage, held) = moving(windowed, None);
    let opened = top(&mut stage, "Cargo.toml");
    stage.key("Tab");
    press_key(&mut stage, "ArrowLeft");
    stage.pass_after(Duration::ZERO);
    stage.pass_after(Duration::from_millis(40));
    // Down goes past the rows that are going, to the row after them.
    press_key(&mut stage, "ArrowDown");
    stage.pass_after(Duration::ZERO);
    stage.pass_after(Duration::ZERO);
    assert_eq!(focused(&stage).as_deref(), Some("Cargo.toml"), "windowed {windowed}");
    press_key(&mut stage, "ArrowUp");
    stage.pass_after(Duration::ZERO);
    stage.pass_after(Duration::ZERO);
    assert_eq!(focused(&stage).as_deref(), Some("crates"), "windowed {windowed}");
    // A letter of a row that is going finds nothing: the focus stays.
    press_key(&mut stage, "t");
    stage.pass_after(Duration::ZERO);
    stage.pass_after(Duration::ZERO);
    assert_eq!(focused(&stage).as_deref(), Some("crates"), "windowed {windowed}");
    // Right on the way turns it round: it opens again from where it is, and its rows are reached.
    press_key(&mut stage, "ArrowRight");
    stage.pass_after(Duration::from_millis(20));
    assert_eq!(held.open.get(), ["lurq", "crates"], "windowed {windowed}");
    tick(&mut stage, Duration::from_millis(1));
    tick(&mut stage, Duration::from_secs(2));
    stage.pass();
    assert!(
      (top(&mut stage, "Cargo.toml") - opened).abs() < 1.0,
      "windowed {windowed}: open again"
    );
    stage.key("ArrowDown");
    assert_eq!(focused(&stage).as_deref(), Some("lurq"), "windowed {windowed}");
  }
}

#[test]
fn with_motion_reduced_a_row_opens_and_closes_at_once_in_either_form() {
  for windowed in [false, true] {
    let app = App::new();
    app.theme().set_reduced_motion(Some(true));
    let (mut stage, _) = moving(windowed, Some(app));
    let opened = top(&mut stage, "Cargo.toml");
    stage.key("Tab");
    press_key(&mut stage, "ArrowLeft");
    stage.pass_after(Duration::ZERO);
    stage.pass_after(Duration::from_millis(10));
    let closed = top(&mut stage, "Cargo.toml");
    assert!(
      (opened - closed - 120.0).abs() < 1.0,
      "windowed {windowed}: at once, {opened} -> {closed}"
    );
    assert!(!stage.paints("lib.rs"), "windowed {windowed}: its rows gone at once");
    press_key(&mut stage, "ArrowRight");
    stage.pass_after(Duration::ZERO);
    stage.pass_after(Duration::from_millis(10));
    assert!(
      (top(&mut stage, "Cargo.toml") - opened).abs() < 1.0,
      "windowed {windowed}: open at once"
    );
  }
}

#[test]
fn the_chevron_turns_as_the_row_closes() {
  let (mut stage, _) = moving(false, None);
  let chevron = |stage: &mut Stage| in_row(stage, "crates", CHEVRON).map(|quad| format!("{:?}", quad.transform));
  let open = chevron(&mut stage);
  stage.key("Tab");
  press_key(&mut stage, "ArrowLeft");
  stage.pass_after(Duration::ZERO);
  stage.pass_after(Duration::from_millis(60));
  let turning = texts(&stage, CHEVRON)
    .into_iter()
    .min_by(|a, b| a.y.total_cmp(&b.y))
    .map(|quad| format!("{:?}", quad.transform));
  stage.pass();
  let closed = chevron(&mut stage);
  assert!(open.is_some() && closed.is_some());
  assert_ne!(open, closed, "turned to the end");
  assert_ne!(turning, open);
  assert_ne!(turning, closed, "on its way round");
}

#[test]
fn a_row_of_thousands_in_a_virtual_list_opens_and_closes_building_the_rows_in_view() {
  let open = Signal::new(Vec::<u32>::new());
  let scene = open.clone();
  let mut stage = Stage::new(move |ctx| {
    let items = vec![
      TreeItem::folder(0u32, "Many")
        .id("many")
        .items((1..=5000u32).map(|i| TreeItem::file(i, &format!("file {i}")).id(format!("f{i}")))),
      TreeItem::file(9999, "Tail").id("tail"),
    ];
    Column::new()
      .child(
        TreeView::new(Signal::new(None), items)
          .open(scene.clone())
          .style(TreeViewPart::Root, |s| s.width(220.0).height(240.0))
          .mount(ctx),
      )
      .into()
  });
  let built = |stage: &Stage| {
    stage
      .quads()
      .iter()
      .filter(|quad| matches!(&quad.content, QuadContent::Text { text, .. } if text.starts_with("file ")))
      .count()
  };
  let tail = top(&mut stage, "tail");
  stage.key("Tab");
  press_key(&mut stage, "ArrowRight");
  stage.pass_after(Duration::ZERO);
  tick(&mut stage, Duration::from_millis(1));
  stage.pass_after(Duration::from_millis(60));
  let opening = built(&stage);
  assert!(opening > 0 && opening < 60, "{opening} rows built on the way");
  // The first rows open in turn, the first first: on the way the first is higher than the one after the next.
  let height = |stage: &mut Stage, id: &str| {
    let (_, y, ..) = stage.bounds(id);
    y
  };
  let (first, third) = (height(&mut stage, "f1"), height(&mut stage, "f3"));
  assert!(
    third - first < 59.5,
    "on the way the rows are not all at their height: {first} {third}"
  );
  tick(&mut stage, Duration::from_secs(2));
  stage.pass();
  assert!(built(&stage) < 60, "open, the rows in view are built");
  assert!(!stage.paints("Tail"), "what is below is past the view");

  // Closed, the rows go as they came, and what was below is back.
  press_key(&mut stage, "ArrowLeft");
  stage.pass_after(Duration::ZERO);
  stage.pass_after(Duration::from_millis(60));
  assert!(built(&stage) < 60, "closing, the rows in view are built");
  tick(&mut stage, Duration::from_secs(2));
  stage.pass();
  assert_eq!(built(&stage), 0, "closed, none");
  assert!((top(&mut stage, "tail") - tail).abs() < 1.0);
  assert_eq!(focused(&stage).as_deref(), Some("many"));
}

#[test]
fn in_the_touch_form_a_tap_opens_a_row_at_once() {
  let (mut stage, held) = board(&["crates"], None);
  stage.tree.set_pointer(lurq::app::Pointer::Coarse);
  stage.pass();
  stage.tap("Cargo.toml");
  assert_eq!(held.selected.get(), Some("Cargo.toml"));
  assert_eq!(
    *held.activated.lock().unwrap(),
    ["Cargo.toml"],
    "a tap does what a double press does, held back for nothing"
  );
  // A mouse's single press only selects.
  stage.click("demo");
  assert_eq!(held.activated.lock().unwrap().len(), 1);
}

/// Whether `content` is painted whole: a text of it and nothing cut from it.
fn whole(stage: &Stage, content: &str) -> bool {
  !texts(stage, content).is_empty()
}

#[test]
fn a_cut_label_ends_in_an_ellipsis_and_is_said_whole_in_a_tooltip_while_it_is_cut() {
  const TITLE: &str = "2026-10 one queue for every project and its runs";
  let selected = Signal::new(None::<Id>);
  let mut stage = Stage::new(move |_| {
    let tree = |id: Id, label: &str, tooltip: bool| {
      TreeView::new(selected.clone(), [TreeItem::file(id, label).id(id)])
        .label_tooltip_when_cut(tooltip)
        .style(TreeViewPart::Root, |s| s.width(160.0))
    };
    Column::new()
      .padding(8.0)
      .spacing(8.0)
      .child(tree("cut", TITLE, true))
      .child(tree("whole", "Short", true))
      .child(tree("off", &TITLE.replace("queue", "board"), false))
      .into()
  });
  let (x, _, width, _) = stage.bounds("cut");
  assert!(!whole(&stage, TITLE), "cut at rest");
  let cut = stage.cut_text_quad(TITLE).expect("it is cut with an ellipsis");
  assert!(cut.x + cut.width <= x + width + 0.5, "the cut text stays in its row");
  let label = stage.bounds("cut-label");
  assert!(label.0 + label.2 <= x + width + 0.01, "the label stays in its row");
  stage.own_timers();
  stage.hover("whole-label");
  stage.tick_timers(Duration::from_secs(2));
  stage.pass();
  assert_eq!(texts(&stage, "Short").len(), 1, "no tooltip over a label that fits");
  stage.hover("off-label");
  stage.tick_timers(Duration::from_secs(2));
  stage.pass();
  assert!(!whole(&stage, &TITLE.replace("queue", "board")), "none unless told");
  stage.hover("cut-label");
  stage.tick_timers(Duration::from_secs(4));
  stage.pass();
  assert!(whole(&stage, TITLE), "its tooltip says it whole");
}
