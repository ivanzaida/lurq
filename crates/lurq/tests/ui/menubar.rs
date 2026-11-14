//! A menubar: its bar and titles as the board draws them, the menu a title opens, the keys and the pointer that
//! move between the titles and hand an open menu to its neighbour, Alt and the titles' letters, a disabled title
//! and bar, and a theme's rules and anatomy.

use std::sync::{Arc, Mutex};

use lurq::{
  app::{
    App,
    events::MouseButton,
    theme::{PaletteColor, ThemePalette},
  },
  components::{Button, Column, Row},
  core::{ElementRef, Signal},
  node::{Element, color::Color},
  ui::{Menu, MenuEntry, MenuItem, Menubar, MenubarPart, MenubarRenderer, MenubarView},
};

use super::stage::Stage;

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

/// What was chosen, in order, and a handler for each item that says so.
type Log = Arc<Mutex<Vec<&'static str>>>;

fn tell(log: &Log, name: &'static str) -> impl Fn() + Send + Sync + 'static {
  let log = log.clone();
  move || log.lock().unwrap().push(name)
}

/// The signals a scene's bar is built with: which menu is open, and whether File's Export as is.
struct Held {
  open: Signal<Option<usize>>,
  exporting: Signal<bool>,
  chosen: Log,
}

/// A page of 400 by 300 with a button before the bar (`doc`) and the bar at 20, 20: File (New, Open, a separator,
/// Export as with PNG and SVG, Quit), Edit (Undo, Cut, Copy, Paste disabled), View (Show grid) and Help, disabled.
fn scene(disabled: bool) -> (Stage, Held) {
  let held = Held {
    open: Signal::new(None),
    exporting: Signal::new(false),
    chosen: Arc::new(Mutex::new(Vec::new())),
  };
  let (open, exporting, chosen, export) = (
    held.open.clone(),
    held.exporting.clone(),
    held.chosen.clone(),
    ElementRef::new(),
  );
  let stage = Stage::new(move |_| {
    let bar = Menubar::new(open.clone())
      .id("bar")
      .disabled(disabled)
      .menu(
        "File",
        [
          MenuItem::new("New")
            .shortcut("Ctrl+N")
            .id("new")
            .on_select(tell(&chosen, "new"))
            .into(),
          MenuItem::new("Open").id("open").into(),
          MenuEntry::Separator,
          MenuItem::new("Export as")
            .id("export")
            .submenu(
              Menu::new(export.clone(), exporting.clone())
                .item(MenuItem::new("PNG").id("png").on_select(tell(&chosen, "png")))
                .item(MenuItem::new("SVG").id("svg")),
            )
            .into(),
          MenuItem::new("Quit").id("quit").into(),
        ],
      )
      .menu(
        "Edit",
        [
          MenuItem::new("Undo").id("undo"),
          MenuItem::new("Cut").id("cut"),
          MenuItem::new("Copy").id("copy"),
          MenuItem::new("Paste").id("paste").disabled(true),
        ],
      )
      .menu("View", [MenuItem::new("Show grid").checked(true).id("grid")])
      .menu("Help", [MenuItem::new("About").id("about")])
      .title_disabled("Help", true);
    Column::new()
      .size(400.0, 300.0)
      .padding(20.0)
      .spacing(8.0)
      .child(Row::new().child(Button::new("Document").id("doc").tab_index(0)))
      .child(Row::new().child(bar))
      .into()
  });
  (stage, held)
}

fn lit(stage: &mut Stage, id: &str) -> bool {
  stage.fill_color(id) == Some(light(PaletteColor::Highlight))
}

fn focused(stage: &Stage) -> Option<String> {
  stage
    .tree
    .focused_element()
    .and_then(|element| element.id().map(str::to_owned))
}

/// Presses Alt and lets it go, with `then` pressed while it is down if it is given.
fn alt(stage: &mut Stage, then: Option<&str>) {
  stage
    .tree
    .key_down("Alt".to_owned(), "AltLeft".to_owned(), false, false, true);
  if let Some(key) = then {
    stage.tree.key_down(key.to_owned(), key.to_owned(), false, false, true);
    stage.tree.key_up(key.to_owned(), key.to_owned(), false, false, true);
  }
  stage
    .tree
    .key_up("Alt".to_owned(), "AltLeft".to_owned(), false, false, false);
  stage.pass();
}

#[test]
fn at_rest_the_bar_and_its_titles_are_as_the_board_draws_them() {
  let (mut stage, _) = scene(false);
  // The bar: 36 tall, of the background's color, with an edge of 1 in the border's color.
  let (bx, by, _, bh) = stage.bounds("bar");
  assert_eq!(bh, 36.0);
  assert_eq!(stage.fill_color("bar"), Some(light(PaletteColor::Background)));
  let edge = stage
    .border("bar")
    .and_then(|quad| quad.border)
    .and_then(|border| border.top);
  assert_eq!(
    edge.map(|side| (side.color, side.width)),
    Some((light(PaletteColor::Border), 1.0))
  );
  // A title is 28 tall, 4 in from the bar's edges, and the next is 2 after it.
  let (x0, y0, w0, h0) = stage.bounds("bar-0");
  assert_eq!((x0 - bx, y0 - by, h0), (4.0, 4.0, 28.0));
  let (x1, ..) = stage.bounds("bar-1");
  assert!((x1 - (x0 + w0) - 2.0).abs() < 0.01, "{x1} {x0} {w0}");
  // Its text is 13 in the foreground, 10 in from its sides.
  assert_eq!(stage.text("File"), (light(PaletteColor::Foreground), 13.0));
  let label = stage.text_quad("File").unwrap();
  assert!((label.x - (x0 + 10.0)).abs() < 0.5);
  assert!((x0 + w0 - 10.0 - (label.x + label.width)).abs() < 0.5);
  // At rest no title is filled, and no menu is painted.
  for id in ["bar-0", "bar-1", "bar-2"] {
    assert_eq!(stage.fill_color(id), None, "{id}");
  }
  assert!(!stage.paints("New"));
}

#[test]
fn a_press_on_a_title_opens_its_menu_under_it_and_a_press_again_closes_it() {
  let (mut stage, held) = scene(false);
  stage.click("bar-0");
  assert_eq!(held.open.get(), Some(0));
  assert!(stage.paints("New"));
  // The title of the open menu is filled with the highlight: the board's accent.
  assert!(lit(&mut stage, "bar-0"));
  // The menu stands under its title from its start, 4 below it, at the menu's width.
  let (tx, ty, _, th) = stage.bounds("bar-0");
  let (mx, my, mw, _) = stage.bounds("bar-0-menu");
  assert_eq!((mx, my, mw), (tx, ty + th + 4.0, 230.0));
  // Opened by the pointer, it shows no focus until a key.
  assert_eq!(stage.fill_color("new"), None);
  stage.click("bar-0");
  assert_eq!(held.open.get(), None);
  assert!(!stage.paints("New"));
}

#[test]
fn choosing_an_item_calls_its_handler_and_closes_the_menu() {
  let (mut stage, held) = scene(false);
  stage.click("bar-0");
  stage.click("new");
  assert_eq!(*held.chosen.lock().unwrap(), ["new"]);
  assert_eq!(held.open.get(), None);
  assert!(!stage.paints("New"));
}

#[test]
fn escape_and_a_press_outside_close_the_menu() {
  let (mut stage, held) = scene(false);
  stage.click("bar-1");
  assert!(stage.paints("Undo"));
  stage.key("Escape");
  assert_eq!(held.open.get(), None);
  // The focus is on the title the menu hung from.
  assert_eq!(focused(&stage).as_deref(), Some("bar-1"));

  stage.click("bar-1");
  assert_eq!(held.open.get(), Some(1));
  stage.tree.mouse_move(390.0, 290.0);
  stage.tree.mouse_down(390.0, 290.0, MouseButton::Left);
  stage.tree.mouse_up(390.0, 290.0, MouseButton::Left);
  stage.pass();
  assert_eq!(held.open.get(), None, "a press outside");
}

#[test]
fn the_app_opens_a_menu_by_its_signal() {
  let (mut stage, held) = scene(false);
  held.open.set(Some(2));
  stage.pass();
  assert!(stage.paints("Show grid"));
  assert!(lit(&mut stage, "bar-2"));
  held.open.set(None);
  stage.pass();
  assert!(!stage.paints("Show grid"));
}

#[test]
fn with_a_menu_open_the_pointer_on_another_title_opens_that_one() {
  let (mut stage, held) = scene(false);
  // With every menu closed, the pointer on a title opens nothing: it is lit, as a title under the pointer is.
  stage.hover("bar-1");
  assert_eq!(held.open.get(), None);
  assert!(lit(&mut stage, "bar-1"));
  stage.click("bar-0");
  stage.hover("bar-1");
  assert_eq!(held.open.get(), Some(1));
  assert!(stage.paints("Undo") && !stage.paints("New"));
  stage.hover("bar-2");
  assert_eq!(held.open.get(), Some(2));
  // A disabled title opens nothing.
  stage.hover("bar-3");
  assert_eq!(held.open.get(), Some(2));
}

#[test]
fn tab_reaches_the_bar_once_and_the_arrows_and_letters_move_between_its_titles() {
  let (mut stage, held) = scene(false);
  stage.key("Tab");
  assert_eq!(focused(&stage).as_deref(), Some("doc"));
  // The bar is one stop, at its first title, which the keyboard's focus lights.
  stage.key("Tab");
  assert_eq!(focused(&stage).as_deref(), Some("bar-0"));
  assert!(lit(&mut stage, "bar-0"));
  stage.key("ArrowRight");
  assert_eq!(focused(&stage).as_deref(), Some("bar-1"));
  assert!(lit(&mut stage, "bar-1") && !lit(&mut stage, "bar-0"));
  // Help is disabled: Right from View goes round to File.
  stage.key("ArrowRight");
  stage.key("ArrowRight");
  assert_eq!(focused(&stage).as_deref(), Some("bar-0"));
  stage.key("ArrowLeft");
  assert_eq!(focused(&stage).as_deref(), Some("bar-2"));
  // A letter goes to the title that starts with it.
  stage.key("e");
  assert_eq!(focused(&stage).as_deref(), Some("bar-1"));
  // Moving opens nothing; Tab leaves the bar.
  assert_eq!(held.open.get(), None);
  stage.key("Tab");
  assert_ne!(focused(&stage).as_deref().map(|id| id.starts_with("bar-")), Some(true));
}

#[test]
fn down_enter_and_space_open_at_the_first_item_and_up_at_the_last() {
  let (mut stage, held) = scene(false);
  stage.key("Tab");
  stage.key("Tab");
  stage.key("ArrowRight");
  stage.key("ArrowDown");
  assert_eq!(held.open.get(), Some(1));
  assert!(lit(&mut stage, "undo"));
  stage.key("Escape");
  assert_eq!(held.open.get(), None);
  assert_eq!(focused(&stage).as_deref(), Some("bar-1"));
  // Up opens at the last item that can be chosen: Paste is disabled.
  stage.key("ArrowUp");
  assert_eq!(held.open.get(), Some(1));
  assert!(lit(&mut stage, "copy"));
  stage.key("Escape");
  stage.key("Enter");
  assert!(lit(&mut stage, "undo"), "Enter at the first");
  stage.key("Escape");
  stage.key(" ");
  assert_eq!(held.open.get(), Some(1));
  assert!(lit(&mut stage, "undo"), "Space at the first");
  // Enter chooses an item, and the focus goes back to the title.
  stage.key("ArrowDown");
  stage.key("Enter");
  assert_eq!(held.open.get(), None);
  assert_eq!(focused(&stage).as_deref(), Some("bar-1"));
}

#[test]
fn left_and_right_in_an_open_menu_open_the_menu_beside_it() {
  let (mut stage, held) = scene(false);
  stage.key("Tab");
  stage.key("Tab");
  stage.key("Enter");
  assert_eq!(held.open.get(), Some(0));
  assert!(lit(&mut stage, "new"));
  // Right closes File and opens Edit, at its first item; Left goes back.
  stage.key("ArrowRight");
  assert_eq!(held.open.get(), Some(1));
  assert!(stage.paints("Undo") && !stage.paints("New"));
  assert!(lit(&mut stage, "undo"));
  stage.key("ArrowLeft");
  assert_eq!(held.open.get(), Some(0));
  assert!(lit(&mut stage, "new"));
  // From the first, Left goes round to the last that is not disabled: View.
  stage.key("ArrowLeft");
  assert_eq!(held.open.get(), Some(2));
  assert!(lit(&mut stage, "grid"));
  // Opened by the pointer, the keys hand it over all the same.
  stage.key("Escape");
  stage.click("bar-0");
  stage.key("ArrowRight");
  assert_eq!(held.open.get(), Some(1));
  assert!(lit(&mut stage, "undo"));
}

#[test]
fn in_a_submenu_left_closes_it_first_and_right_goes_to_the_next_title() {
  let (mut stage, held) = scene(false);
  stage.key("Tab");
  stage.key("Tab");
  stage.key("Enter");
  stage.key("ArrowDown");
  stage.key("ArrowDown");
  assert!(lit(&mut stage, "export"));
  // Right on an item with a submenu opens it.
  stage.key("ArrowRight");
  assert!(held.exporting.get());
  assert_eq!(held.open.get(), Some(0));
  assert!(lit(&mut stage, "png"));
  // Left closes the submenu alone.
  stage.key("ArrowLeft");
  assert!(!held.exporting.get());
  assert_eq!(held.open.get(), Some(0));
  assert!(lit(&mut stage, "export"));
  // Right on an item that has none closes both and opens Edit.
  stage.key("ArrowRight");
  assert!(held.exporting.get());
  stage.key("ArrowRight");
  assert!(!held.exporting.get(), "the submenu closed with its menu");
  assert_eq!(held.open.get(), Some(1));
  assert!(lit(&mut stage, "undo"));
  // Opened again, File has no submenu open.
  stage.key("ArrowLeft");
  assert_eq!(held.open.get(), Some(0));
  assert!(!stage.paints("PNG"));
}

#[test]
fn choosing_in_a_submenu_closes_it_and_the_menu() {
  let (mut stage, held) = scene(false);
  stage.click("bar-0");
  stage.click("export");
  assert!(held.exporting.get());
  stage.click("png");
  assert_eq!(*held.chosen.lock().unwrap(), ["png"]);
  assert!(!held.exporting.get());
  assert_eq!(held.open.get(), None);
}

#[cfg(not(target_os = "macos"))]
#[test]
fn alt_alone_puts_the_focus_on_the_bar_with_the_letters_and_again_gives_it_back() {
  let (mut stage, held) = scene(false);
  stage.key("Tab");
  assert_eq!(focused(&stage).as_deref(), Some("doc"));
  assert!(stage.paints("File"));
  alt(&mut stage, None);
  assert_eq!(focused(&stage).as_deref(), Some("bar-0"));
  assert!(lit(&mut stage, "bar-0"));
  // The first letter of each title is underlined: it is a text of its own.
  assert!(stage.paints("F") && stage.paints("ile") && !stage.paints("File"));
  assert!(stage.paints("E") && stage.paints("dit"));
  // Down opens the menu as it would from Tab.
  stage.key("ArrowDown");
  assert_eq!(held.open.get(), Some(0));
  stage.key("Escape");
  // Alt again: the focus goes back to what had it, and the letters go.
  alt(&mut stage, None);
  assert_eq!(focused(&stage).as_deref(), Some("doc"));
  assert!(stage.paints("File"));
}

#[cfg(not(target_os = "macos"))]
#[test]
fn alt_with_a_titles_letter_opens_its_menu_from_anywhere() {
  let (mut stage, held) = scene(false);
  stage.key("Tab");
  alt(&mut stage, Some("e"));
  assert_eq!(held.open.get(), Some(1));
  assert!(lit(&mut stage, "undo"));
  // Its title has the focus under it: Escape leaves it there.
  stage.key("Escape");
  assert_eq!(focused(&stage).as_deref(), Some("bar-1"));
  // A pressed letter that starts no title, or a disabled one, does nothing.
  alt(&mut stage, Some("h"));
  assert_eq!(held.open.get(), None);
  // Alt with a letter while a menu is open opens the other.
  alt(&mut stage, Some("v"));
  assert_eq!(held.open.get(), Some(2));
  alt(&mut stage, Some("f"));
  assert_eq!(held.open.get(), Some(0));
  assert!(stage.paints("New") && !stage.paints("Show grid"));
}

#[test]
fn a_disabled_bar_takes_nothing() {
  let (mut stage, held) = scene(true);
  stage.click("bar-0");
  assert_eq!(held.open.get(), None);
  assert!(!stage.paints("New"));
  stage.key("Tab");
  stage.key("Tab");
  assert_ne!(focused(&stage).as_deref(), Some("bar-0"));
  alt(&mut stage, Some("f"));
  assert_eq!(held.open.get(), None);
}

#[test]
fn a_disabled_title_is_passed_by_and_opens_nothing() {
  let (mut stage, held) = scene(false);
  stage.click("bar-3");
  assert_eq!(held.open.get(), None);
  assert!(!stage.paints("About"));
}

#[test]
fn a_rule_restyles_a_part_and_a_theme_gives_the_bar_another_anatomy() {
  let open = Signal::new(None);
  let held = open.clone();
  let mut stage = Stage::new(move |_| {
    Column::new()
      .size(400.0, 300.0)
      .padding(20.0)
      .child(
        Row::new().child(
          Menubar::new(held.clone())
            .id("bar")
            .style(MenubarPart::Title, |s| s.padding_x(16.0))
            .menu("File", [MenuItem::new("New")]),
        ),
      )
      .into()
  });
  let (x, _, width, _) = stage.bounds("bar-0");
  let label = stage.text_quad("File").unwrap();
  assert!((label.x - (x + 16.0)).abs() < 0.5);
  assert!((x + width - 16.0 - (label.x + label.width)).abs() < 0.5);

  // A theme whose bar draws its titles last first: other nodes, the same behavior.
  let app = App::new();
  let renderer: MenubarRenderer = Arc::new(|mut view: MenubarView| {
    let mut bar = view.bar();
    for index in (0..view.menus.len()).rev() {
      let title = view.title(index).child(view.label(index));
      let menu = view.menu(index);
      bar = bar.child(Column::new().child(title).child(menu));
    }
    bar.into()
  });
  app.theme().set_renderer::<Menubar>(renderer);
  let open = Signal::new(None);
  let held = open.clone();
  let mut stage = Stage::in_app(app, move |_| -> Element {
    Column::new()
      .size(400.0, 300.0)
      .padding(20.0)
      .child(
        Row::new().child(
          Menubar::new(held.clone())
            .id("bar")
            .menu("File", [MenuItem::new("New")])
            .menu("Edit", [MenuItem::new("Undo")]),
        ),
      )
      .into()
  });
  assert!(stage.bounds("bar-1").0 < stage.bounds("bar-0").0);
  stage.click("bar-1");
  assert_eq!(open.get(), Some(1));
  assert!(stage.paints("Undo"));
}
