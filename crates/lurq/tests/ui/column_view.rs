//! A column view: its columns against the board, what a press of an item does to the path, the keys along a column
//! and across the columns, activating, a disabled item, children that come later, a label longer than its room,
//! the density, a narrow room that the columns scroll in, tooling, and a theme's other anatomy.

use std::sync::{Arc, Mutex};

use lurq::{
  app::{
    App,
    events::MouseButton,
    theme::{Density, PaletteColor, ThemePalette},
  },
  components::{Column, Row, Text},
  core::Signal,
  layout::{Constraints, Size, quad::QuadContent},
  node::{Element, color::Color},
  ui::{ColumnView, ColumnViewPart, ColumnViewRenderer, ColumnViewView, Interaction, Themed, TreeItem},
};

use super::stage::Stage;

type Id = &'static str;

const CHEVRON: &str = "\u{E01B}";

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

fn folder(id: Id) -> TreeItem<Id> {
  TreeItem::folder(id, id).id(id)
}

fn file(id: Id) -> TreeItem<Id> {
  TreeItem::file(id, id).id(id)
}

/// The board's hierarchy: crates (lurq with src, tests and Cargo.toml; lurq_macros; demo), docs, pencil, scripts.
fn files() -> Vec<TreeItem<Id>> {
  vec![
    folder("crates").items([
      folder("lurq").items([
        folder("src").items([file("lib.rs"), file("theme.rs")]),
        folder("tests").item(file("ui.rs")),
        file("Cargo.toml"),
      ]),
      folder("lurq_macros").item(file("macros.rs")),
      folder("demo").item(file("main.rs")),
    ]),
    folder("docs").item(file("index.md")),
    folder("pencil").item(file("board.pen")),
    folder("scripts").item(file("check.ps1")),
  ]
}

/// What the app was told, in order.
type Told = Arc<Mutex<Vec<String>>>;

fn said(told: &Told) -> Vec<String> {
  std::mem::take(&mut *told.lock().unwrap())
}

/// A window of 640 by 300 that renders `scene`: room for the board's view.
fn wide(scene: impl Fn(&mut lurq::app::ctx::Ctx) -> Element + Send + Sync + 'static) -> Stage {
  let mut stage = Stage::new(scene);
  stage
    .tree
    .set_layout_constraints_override(Some(Constraints::loose(Size::new(640.0, 300.0))));
  stage.pass();
  stage
}

/// The board's view bound to a path, which tells `told` what its handlers hear.
fn board(path: &[Id], mounted: bool) -> (Stage, Signal<Vec<Id>>, Told) {
  let path = Signal::new(path.to_vec());
  let told: Told = Arc::default();
  let (bound, heard) = (path.clone(), told.clone());
  let stage = wide(move |ctx| {
    let (change, activate) = (heard.clone(), heard.clone());
    let seen = bound.clone();
    let view = ColumnView::new(bound.clone(), files())
      .label("Files")
      .id("view")
      .on_change(move |path| {
        change
          .lock()
          .unwrap()
          .push(format!("change {path:?} (the signal has {:?})", seen.get_untracked()))
      })
      .on_activate(move |id| activate.lock().unwrap().push(format!("activate {id}")));
    let view: Element = if mounted { view.mount(ctx) } else { view.into() };
    Column::new()
      .size(640.0, 300.0)
      .padding(8.0)
      .child(view)
      .child(lurq::ui::Button::new("After").id("after"))
      .into()
  });
  (stage, path, told)
}

fn focused(stage: &Stage) -> Option<String> {
  stage
    .tree
    .focused_element()
    .and_then(|element| element.id().map(str::to_owned))
}

fn close_to(a: f32, b: f32) -> bool {
  (a - b).abs() < 0.05
}

#[test]
fn the_view_is_the_boards_three_columns() {
  let (mut stage, ..) = board(&["crates", "lurq"], false);
  assert_eq!(stage.bounds("view"), (8.0, 8.0, 520.0, 176.0));
  assert_eq!(stage.fill_color("view"), Some(light(PaletteColor::Background)));
  let border = stage.border("view").expect("the view's edge").border.unwrap();
  let top = border.top.unwrap();
  assert_eq!((top.width, top.color), (1.0, light(PaletteColor::Border)));

  // Three columns share the view's width, each with a line of 1 at its end but the last.
  let third = 520.0 / 3.0;
  for (column, lined) in [(0, true), (1, true), (2, false)] {
    let (x, y, width, height) = stage.bounds(&format!("view-column-{column}"));
    assert!(close_to(x, 8.0 + third * column as f32), "column {column} at {x}");
    assert!(close_to(width, third), "column {column} is {width} wide");
    assert_eq!((y, height), (8.0, 176.0));
    let line = stage.quads().into_iter().any(|quad| {
      close_to(quad.x, x)
        && close_to(quad.width, width)
        && quad
          .border
          .as_ref()
          .and_then(|border| border.right.as_ref())
          .is_some_and(|side| side.width == 1.0 && side.color == light(PaletteColor::Border))
    });
    assert_eq!(line, lined, "column {column}");
  }

  // An item: 26 high, 4 in from its column, 1 from the next.
  let (x, y, width, height) = stage.bounds("crates");
  assert_eq!((x, y, height), (12.0, 12.0, 26.0));
  assert!(close_to(width, third - 8.0), "{width}");
  let (_, next, ..) = stage.bounds("docs");
  assert_eq!(next, 39.0);
  // Its icon of 14 is 6 in, its label 6 after it at 13, its chevron of 12 is 6 from its end.
  let label = stage.text_quad("crates").unwrap();
  assert_eq!(label.x, 12.0 + 6.0 + 14.0 + 6.0);
  assert_eq!(stage.text("crates").1, 13.0);
  let chevrons: Vec<_> = stage
    .quads()
    .into_iter()
    .filter(|quad| matches!(&quad.content, QuadContent::Text { text, .. } if text == CHEVRON))
    .collect();
  // Every folder shown has one; Cargo.toml has none.
  assert_eq!(chevrons.len(), 4 + 3 + 2);
  let first = &chevrons[0];
  assert_eq!(first.width, 12.0);
  assert!(close_to(first.x + first.width, x + width - 6.0), "{}", first.x);

  // The item on the path is in the highlight; the one it ends at on the info fill, in white.
  assert_eq!(stage.fill_color("crates"), Some(light(PaletteColor::Highlight)));
  assert_eq!(stage.fill_color("lurq"), Some(light(PaletteColor::Info)));
  let white = Color::from_hex("#FFFFFF");
  assert_eq!(stage.text("lurq").0, white);
  assert_eq!(stage.text("crates").0, light(PaletteColor::Foreground));
  assert_eq!(stage.fill_color("docs"), None);
  assert_eq!(stage.fill_color("src"), None);
  // The third column is what lurq holds.
  assert!(stage.paints("src") && stage.paints("tests") && stage.paints("Cargo.toml"));
}

#[test]
fn the_dark_theme_gives_the_view_its_roles() {
  let app = App::new();
  app
    .theme()
    .set_preferred_appearance(Some(lurq::app::window::Appearance::Dark));
  let dark = |role| ThemePalette::dark().get(role);
  let mut stage = Stage::in_app(app, move |_| {
    Column::new()
      .size(640.0, 300.0)
      .child(ColumnView::controlled(["crates", "lurq"], files(), |_| {}).id("view"))
      .into()
  });
  assert_eq!(stage.fill_color("view"), Some(dark(PaletteColor::Background)));
  assert_eq!(stage.fill_color("crates"), Some(dark(PaletteColor::Highlight)));
  assert_eq!(stage.fill_color("lurq"), Some(dark(PaletteColor::Info)));
  assert_eq!(stage.text("lurq").0, Color::from_hex("#FFFFFF"));
}

#[test]
fn a_press_ends_the_path_at_the_item_and_tells_the_new_path_once() {
  for mounted in [false, true] {
    let (mut stage, path, told) = board(&["crates", "lurq"], mounted);
    // Every press at one time: two of an item in a row are a double press however slow the machine.
    stage.presses_at(std::time::Duration::ZERO);
    stage.click("demo");
    assert_eq!(path.get(), ["crates", "demo"], "mounted: {mounted}");
    assert_eq!(
      said(&told),
      [r#"change ["crates", "demo"] (the signal has ["crates", "demo"])"#]
    );
    assert!(stage.paints("main.rs") && !stage.paints("src"));
    // The item chosen already tells of no change: a second press of it activates it.
    stage.click("demo");
    assert_eq!(said(&told), ["activate demo"]);
    // An item of an earlier column cuts the path there.
    stage.click("docs");
    assert_eq!(path.get(), ["docs"]);
    assert!(stage.paints("index.md") && !stage.paints("lurq"));
    said(&told);
    // What the app sets is shown, and told to nobody.
    path.set(vec!["crates", "lurq", "src"]);
    stage.pass();
    assert!(stage.paints("lib.rs"));
    assert!(said(&told).is_empty());
  }
}

#[test]
fn a_controlled_view_shows_what_the_app_answers() {
  let told: Told = Arc::default();
  let heard = told.clone();
  let mut stage = wide(move |_| {
    let heard = heard.clone();
    Column::new()
      .size(640.0, 300.0)
      .child(
        ColumnView::controlled(["crates"], files(), move |path| {
          heard.lock().unwrap().push(format!("{path:?}"))
        })
        .id("view"),
      )
      .into()
  });
  stage.click("docs");
  assert_eq!(said(&told), [r#"["docs"]"#]);
  // The app did not take it: crates is still the one chosen.
  assert_eq!(stage.fill_color("crates"), Some(light(PaletteColor::Info)));
  assert!(stage.paints("lurq"));
}

#[test]
fn the_keys_go_along_a_column_and_across_the_columns_and_the_path_follows() {
  for mounted in [false, true] {
    let (mut stage, path, told) = board(&["crates", "lurq"], mounted);
    // One stop of Tab, at the item the path ends at.
    stage.key("Tab");
    assert_eq!(focused(&stage).as_deref(), Some("lurq"), "mounted: {mounted}");
    stage.key("ArrowDown");
    assert_eq!(focused(&stage).as_deref(), Some("lurq_macros"));
    assert_eq!(path.get(), ["crates", "lurq_macros"]);
    assert_eq!(
      said(&told),
      [r#"change ["crates", "lurq_macros"] (the signal has ["crates", "lurq_macros"])"#]
    );
    stage.key("End");
    assert_eq!(focused(&stage).as_deref(), Some("demo"));
    // Past the last there is none: the focus does not go on into the next column.
    stage.key("ArrowDown");
    assert_eq!(focused(&stage).as_deref(), Some("demo"));
    assert_eq!(path.get(), ["crates", "demo"]);
    stage.key("Home");
    assert_eq!(focused(&stage).as_deref(), Some("lurq"));
    stage.key("ArrowUp");
    assert_eq!(focused(&stage).as_deref(), Some("lurq"));
    // Right goes into what the item holds, to its first item, which is built by the choice.
    stage.key("ArrowRight");
    assert_eq!(focused(&stage).as_deref(), Some("src"));
    assert_eq!(path.get(), ["crates", "lurq", "src"]);
    stage.key("ArrowRight");
    assert_eq!(focused(&stage).as_deref(), Some("lib.rs"));
    assert_eq!(path.get(), ["crates", "lurq", "src", "lib.rs"]);
    // A file holds nothing: Right stays.
    stage.key("ArrowRight");
    assert_eq!(focused(&stage).as_deref(), Some("lib.rs"));
    // Left goes back to the item the column is of, and the path ends there.
    stage.key("ArrowLeft");
    assert_eq!(focused(&stage).as_deref(), Some("src"));
    assert_eq!(path.get(), ["crates", "lurq", "src"]);
    stage.key("ArrowLeft");
    stage.key("ArrowLeft");
    assert_eq!(focused(&stage).as_deref(), Some("crates"));
    assert_eq!(path.get(), ["crates"]);
    stage.key("ArrowLeft");
    assert_eq!(focused(&stage).as_deref(), Some("crates"));
    // Tab leaves the view.
    stage.key("Tab");
    assert_eq!(focused(&stage).as_deref(), Some("after"));
  }
}

#[test]
fn a_letter_goes_to_the_next_item_that_starts_with_it_and_chooses_it() {
  let (mut stage, path, _) = board(&[], false);
  stage.key("Tab");
  assert_eq!(focused(&stage).as_deref(), Some("crates"));
  stage.key("p");
  assert_eq!(focused(&stage).as_deref(), Some("pencil"));
  assert_eq!(path.get(), ["pencil"]);
}

#[test]
fn enter_and_a_double_press_activate_the_item() {
  let (mut stage, path, told) = board(&["crates", "lurq"], false);
  stage.key("Tab");
  stage.key("Enter");
  assert_eq!(said(&told), ["activate lurq"]);
  let (x, y) = stage.center("Cargo.toml");
  stage.tree.mouse_move(x, y);
  // The two presses at one time: a double press however slow the machine.
  stage.presses_at(std::time::Duration::ZERO);
  for _ in 0..2 {
    stage.tree.mouse_down(x, y, MouseButton::Left);
    stage.tree.mouse_up(x, y, MouseButton::Left);
    stage.pass_after(std::time::Duration::from_millis(1));
  }
  assert_eq!(path.get(), ["crates", "lurq", "Cargo.toml"]);
  let said = said(&told);
  assert_eq!(said.len(), 2, "{said:?}");
  assert!(said[0].starts_with("change"));
  assert_eq!(said[1], "activate Cargo.toml");
}

#[test]
fn a_disabled_item_is_passed_by_and_a_disabled_view_takes_nothing() {
  let path = Signal::new(vec!["a"]);
  let bound = path.clone();
  let mut stage = wide(move |_| {
    Column::new()
      .size(640.0, 300.0)
      .child(
        ColumnView::new(
          bound.clone(),
          [
            file("a"),
            file("b").disabled(true),
            folder("c").items([file("d").disabled(true), file("e")]),
          ],
        )
        .id("view"),
      )
      .into()
  });
  stage.click("b");
  assert_eq!(path.get(), ["a"]);
  stage.key("Tab");
  stage.key("ArrowDown");
  assert_eq!(focused(&stage).as_deref(), Some("c"));
  // Right goes to the first item that takes input.
  stage.key("ArrowRight");
  assert_eq!(focused(&stage).as_deref(), Some("e"));
  assert_eq!(path.get(), ["c", "e"]);
  // A disabled item is faded.
  let (_, _, width, height) = stage.bounds("b");
  assert!(width > 0.0 && height == 26.0);

  let path = Signal::new(vec!["a"]);
  let bound = path.clone();
  let mut stage = wide(move |_| {
    Column::new()
      .size(640.0, 300.0)
      .child(
        ColumnView::new(bound.clone(), [file("a"), file("b")])
          .disabled(true)
          .id("view"),
      )
      .into()
  });
  stage.click("b");
  stage.key("Tab");
  stage.key("ArrowDown");
  assert_eq!(path.get(), ["a"]);
  assert_eq!(focused(&stage), None);
}

#[test]
fn a_lazy_item_has_a_column_with_a_spinner_until_its_items_come() {
  let loaded = Signal::new(false);
  let read = loaded.clone();
  let mut stage = wide(move |_| {
    let mut registry = folder("registry").lazy(true);
    if read.get() {
      registry = registry.item(file("wgpu"));
    }
    Column::new()
      .size(640.0, 300.0)
      .child(ColumnView::controlled(["registry"], [registry, file("README.md")], |_| {}).id("view"))
      .into()
  });
  // It has a chevron and a column of its own, with nothing to choose in it yet.
  let (x, _, width, _) = stage.bounds("view-column-1");
  assert!(x > 8.0 && width > 0.0);
  assert!(!stage.paints("wgpu"));
  loaded.set(true);
  stage.pass();
  assert!(stage.paints("wgpu"));
}

#[test]
fn a_label_longer_than_its_room_ends_in_an_ellipsis_and_the_chevron_keeps_its_place() {
  let long = "a-folder-with-a-name-far-longer-than-a-column-is-wide";
  let mut stage = wide(move |_| {
    Column::new()
      .size(640.0, 300.0)
      .child(
        ColumnView::controlled(
          ["crates"],
          [
            TreeItem::folder("long", long).id("long").item(file("x")),
            folder("crates").item(file("y")),
          ],
          |_| {},
        )
        .id("view"),
      )
      .into()
  });
  let (x, _, width, _) = stage.bounds("long");
  let label = stage.cut_text_quad(long).expect("the label, cut");
  assert!(
    label.x + label.width <= x + width - 6.0 - 12.0 - 6.0 + 0.5,
    "{}",
    label.width
  );
  let chevron = stage
    .quads()
    .into_iter()
    .find(|quad| matches!(&quad.content, QuadContent::Text { text, .. } if text == CHEVRON))
    .unwrap();
  assert!(close_to(chevron.x + chevron.width, x + width - 6.0));
}

#[test]
fn an_items_height_follows_the_density() {
  for (density, height) in [
    (Density::Compact, 18.0),
    (Density::Default, 26.0),
    (Density::Comfortable, 34.0),
    (Density::Touch, 38.0),
  ] {
    let app = App::new();
    app.theme().set_density(density);
    let mut stage = Stage::in_app(app, move |_| {
      Column::new()
        .size(640.0, 300.0)
        .child(ColumnView::controlled([], files(), |_| {}).id("view"))
        .into()
    });
    let (_, top, _, item) = stage.bounds("crates");
    let (_, next, ..) = stage.bounds("docs");
    assert_eq!(item, height, "{density:?}");
    assert_eq!(next - top, height + 1.0, "{density:?}: 1 apart");
  }
}

#[test]
fn in_a_narrow_room_the_view_is_as_wide_as_the_room_and_its_columns_scroll() {
  for mounted in [false, true] {
    let path = Signal::new(Vec::new());
    let bound = path.clone();
    // The two stages stand at two places: a press at the place of the last one, soon after it, would be the
    // second of a double press.
    let inset = if mounted { 20.0 } else { 0.0 };
    let mut stage = Stage::new(move |ctx| {
      let view = ColumnView::new(bound.clone(), files()).id("view");
      let view: Element = if mounted { view.mount(ctx) } else { view.into() };
      Column::new()
        .size(400.0, 300.0)
        .padding(inset)
        .child(Row::new().id("room").width(200.0).child(view))
        .into()
    });
    let end = inset + 200.0;
    assert_eq!(stage.bounds("view").2, 200.0, "mounted: {mounted}");
    // The one column of an empty path has the whole view; two keep their least width, and what does not fit is
    // scrolled to.
    assert_eq!(stage.bounds("view-column-0").2, 200.0);
    stage.click("crates");
    assert_eq!(path.get(), ["crates"], "mounted: {mounted}");
    assert_eq!(stage.bounds("view-column-0").2, 160.0);
    assert_eq!(stage.bounds("view-column-1").2, 160.0);
    if mounted {
      // The path grew by a press: the last column is brought into sight.
      let (x, _, width, _) = stage.bounds("view-column-1");
      assert!(close_to(x + width, end), "the last column ends at {}", x + width);
      // The focus is kept in sight over that: Left from the second column shows the item it goes to.
      stage.click("lurq");
      stage.key("ArrowLeft");
      assert_eq!(focused(&stage).as_deref(), Some("crates"));
      let (x, ..) = stage.bounds("crates");
      assert!(x >= inset, "crates is at {x}");
    }
  }
}

#[test]
fn a_mounted_view_in_a_narrow_room_starts_at_the_end_of_its_path_in_its_first_frame() {
  let mut stage = Stage::first_frame(400.0, 300.0, |ctx| {
    Column::new()
      .size(400.0, 300.0)
      .child(
        Row::new().width(200.0).child(
          ColumnView::controlled(["crates", "lurq"], files(), |_| {})
            .id("view")
            .mount(ctx),
        ),
      )
      .into()
  });
  let first = stage.picture();
  let (x, _, width, _) = stage.bounds("view-column-2");
  assert!(close_to(x + width, 200.0), "the last column ends at {}", x + width);
  stage.frame();
  assert_eq!(first, stage.picture());
}

#[test]
fn the_first_frame_of_a_mounted_view_is_the_settled_one() {
  let scene = |ctx: &mut lurq::app::ctx::Ctx| -> Element {
    Column::new()
      .size(640.0, 300.0)
      .child(ColumnView::controlled(["crates", "lurq"], files(), |_| {}).mount(ctx))
      .into()
  };
  let mut stage = Stage::first_frame(640.0, 300.0, scene);
  let first = stage.picture();
  stage.frame();
  assert_eq!(first, stage.picture());
}

#[test]
fn a_forced_focus_draws_the_ring_on_the_item_tab_would_stop_on() {
  let mut stage = wide(|_| {
    Column::new()
      .size(640.0, 300.0)
      .child(
        ColumnView::controlled(["crates"], files(), |_| {})
          .force(Some(Interaction::FocusVisible))
          .id("view"),
      )
      .into()
  });
  assert!(stage.has_ring(light(PaletteColor::Ring)));
  let _ = stage.bounds("crates");
}

#[test]
fn a_theme_gives_the_view_another_anatomy_and_another_rule() {
  let app = App::new();
  let renderer: ColumnViewRenderer = Arc::new(|view: ColumnViewView| {
    Row::new()
      .child(Text::new(&format!("{} columns", view.columns.len())))
      .into()
  });
  app.theme().set_renderer::<ColumnView<Id>>(renderer);
  let stage = Stage::in_app(app, |_| {
    Column::new()
      .size(640.0, 300.0)
      .child(ColumnView::controlled(["crates", "lurq"], files(), |_| {}))
      .into()
  });
  assert!(stage.paints("3 columns"));

  let app = App::new();
  app
    .theme()
    .patch_recipe::<ColumnView<Id>>(|recipe| recipe.base(ColumnViewPart::Item, |s| s.height(40.0)));
  let mut stage = Stage::in_app(app, |_| {
    Column::new()
      .size(640.0, 300.0)
      .child(ColumnView::controlled([], files(), |_| {}))
      .into()
  });
  assert_eq!(stage.bounds("crates").3, 40.0);
  assert_eq!(<ColumnView as Themed>::NAME, "column_view");
}

#[cfg(any(feature = "mcp", feature = "devtools"))]
#[test]
fn tooling_reads_listboxes_of_options_with_the_path_selected() {
  let read = |stage: &mut Stage, id: &str, what: &str| {
    stage
      .tree
      .get_element_by_id_mut(id)
      .and_then(|element| element.description(what))
  };
  let (mut stage, ..) = board(&["crates", "lurq"], false);
  assert_eq!(read(&mut stage, "view", "a11y_role").as_deref(), Some("group"));
  assert_eq!(read(&mut stage, "view", "a11y_name").as_deref(), Some("Files"));
  assert_eq!(
    read(&mut stage, "view-column-1", "a11y_role").as_deref(),
    Some("listbox")
  );
  assert_eq!(
    read(&mut stage, "view-column-1", "a11y_name").as_deref(),
    Some("crates")
  );
  assert_eq!(read(&mut stage, "lurq", "a11y_role").as_deref(), Some("option"));
  assert_eq!(read(&mut stage, "lurq", "a11y_selected").as_deref(), Some("true"));
  assert_eq!(read(&mut stage, "lurq", "a11y_expanded").as_deref(), Some("true"));
  assert_eq!(read(&mut stage, "demo", "a11y_selected").as_deref(), Some("false"));
  assert_eq!(read(&mut stage, "demo", "a11y_level").as_deref(), Some("2"));
  assert_eq!(read(&mut stage, "demo", "a11y_posinset").as_deref(), Some("3"));
  assert_eq!(read(&mut stage, "demo", "a11y_setsize").as_deref(), Some("3"));
  assert_eq!(read(&mut stage, "Cargo.toml", "a11y_expanded"), None);
}
