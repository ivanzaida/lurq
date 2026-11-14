//! A context menu: opened at the pointer by a secondary press, at the finger by a long press, at its element by the
//! Menu key and Shift+F10; kept inside the window; what choosing does; one open at a time; the bar of the board's
//! edit menu; and a theme's rules over it.

use std::sync::{Arc, Mutex};

use lurq::{
  app::{
    App,
    ctx::Ctx,
    events::MouseButton,
    theme::{PaletteColor, ThemePalette},
  },
  components::{Column, Rect, Stack},
  core::Signal,
  node::{Element, color::Color},
  ui::{Button, ContextMenu, ContextMenuPart, MenuItem, icons},
};

use super::stage::Stage;

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

/// A window of 400 by 300 with `content` in it: the menu's layer is painted only inside a root of the window's size.
fn stage(scene: impl Fn(&mut Ctx) -> Element + Send + Sync + 'static) -> Stage {
  stage_in(App::new(), scene)
}

fn stage_in(app: App, scene: impl Fn(&mut Ctx) -> Element + Send + Sync + 'static) -> Stage {
  let mut stage = Stage::in_app(app, move |ctx| Stack::new().size(400.0, 300.0).child(scene(ctx)).into());
  stage.tree.resize(400, 300);
  stage.pass();
  stage
}

/// What was chosen, in order, and a handler for each item that says so.
fn log() -> (
  Arc<Mutex<Vec<&'static str>>>,
  impl Fn(&'static str) -> Box<dyn Fn() + Send + Sync> + Clone + Send + Sync + 'static,
) {
  let chosen = Arc::new(Mutex::new(Vec::new()));
  let told = chosen.clone();
  (chosen, move |name| {
    let told = told.clone();
    Box::new(move || told.lock().unwrap().push(name))
  })
}

/// The board's context menu on an area of 380 by 280 at 10, 10.
fn area(open: Signal<bool>, tell: impl Fn(&'static str) -> Box<dyn Fn() + Send + Sync>) -> ContextMenu {
  ContextMenu::new(Rect::new(380.0, 280.0).id("area"), open)
    .id("menu")
    .item(
      MenuItem::new("Back")
        .id("back")
        .shortcut("Alt+←")
        .on_select(tell("back")),
    )
    .item(
      MenuItem::new("Reload")
        .id("reload")
        .shortcut("Ctrl+R")
        .on_select(tell("reload")),
    )
    .separator()
    .item(
      MenuItem::new("Delete")
        .id("delete")
        .icon(icons::TRASH)
        .destructive(true)
        .on_select(tell("delete")),
    )
}

fn placed(content: impl Into<Element>) -> Element {
  Column::new().padding(10.0).child(content).into()
}

/// A press of the secondary button at a point of the window.
fn secondary_press(stage: &mut Stage, x: f32, y: f32) {
  stage.tree.mouse_move(x, y);
  stage.tree.mouse_down(x, y, MouseButton::Right);
  stage.tree.mouse_up(x, y, MouseButton::Right);
  stage.pass();
}

#[test]
fn a_secondary_press_opens_the_menu_at_the_pointer() {
  let open = Signal::new(false);
  let (_, tell) = log();
  let held = open.clone();
  let mut stage = stage(move |_| placed(area(held.clone(), tell.clone())));
  assert!(!stage.paints("Back"), "closed until it is asked for");
  secondary_press(&mut stage, 100.0, 60.0);
  assert!(open.get());
  // Its corner at the pointer, at least 220 wide as the board's, in the popover's color.
  let (x, y, width, _) = stage.bounds("menu");
  assert_eq!((x, y), (100.0, 60.0));
  assert_eq!(width, 220.0);
  assert_eq!(stage.fill_color("menu"), Some(light(PaletteColor::Popover)));
  // Its items are a menu's: 32 tall, the shortcut in the mono type at 11, the destructive one in its color.
  assert_eq!(stage.bounds("back").3, 32.0);
  assert_eq!(stage.text("Ctrl+R"), (light(PaletteColor::MutedForeground), 11.0));
  assert_eq!(stage.text("Delete").0, light(PaletteColor::Destructive));
}

#[test]
fn a_left_press_does_not_open_it() {
  let open = Signal::new(false);
  let (_, tell) = log();
  let held = open.clone();
  let mut stage = stage(move |_| placed(area(held.clone(), tell.clone())));
  stage.click("area");
  assert!(!open.get());
}

#[test]
fn where_the_window_has_no_room_it_opens_up_and_back_from_the_pointer() {
  let open = Signal::new(false);
  let (_, tell) = log();
  let held = open.clone();
  let mut stage = stage(move |_| placed(area(held.clone(), tell.clone())));
  secondary_press(&mut stage, 380.0, 280.0);
  let (x, y, width, height) = stage.bounds("menu");
  assert_eq!((x + width, y + height), (380.0, 280.0));
}

#[test]
fn a_long_press_opens_it_at_the_finger() {
  let open = Signal::new(false);
  let (_, tell) = log();
  let held = open.clone();
  let mut stage = stage(move |_| placed(area(held.clone(), tell.clone())));
  assert!(
    stage.tree.touch_long_press(50.0, 40.0),
    "the element takes the long press"
  );
  stage.pass();
  assert!(open.get());
  let (x, y, ..) = stage.bounds("menu");
  assert_eq!((x, y), (50.0, 40.0));
}

#[test]
fn shift_f10_opens_it_under_the_focused_element_and_the_focus_comes_back() {
  let open = Signal::new(false);
  let (chosen, tell) = log();
  let held = open.clone();
  let mut stage = stage(move |_| {
    placed(
      ContextMenu::new(Button::new("report.pdf").id("file"), held.clone())
        .id("menu")
        .item(MenuItem::new("Open").id("open").on_select(tell("open")))
        .item(MenuItem::new("Rename").id("rename").on_select(tell("rename"))),
    )
  });
  stage.key("Tab");
  stage
    .tree
    .key_down("F10".to_owned(), "F10".to_owned(), true, false, false);
  stage
    .tree
    .key_up("F10".to_owned(), "F10".to_owned(), true, false, false);
  stage.pass();
  assert!(open.get());
  // Under the element from its start, as a menu hangs from its button, with the focus on its first item.
  let (fx, fy, _, fh) = stage.bounds("file");
  let (x, y, ..) = stage.bounds("menu");
  assert_eq!((x, y), (fx, fy + fh + 4.0));
  assert_eq!(stage.fill_color("open"), Some(light(PaletteColor::Highlight)));
  stage.key("ArrowDown");
  stage.key("Enter");
  assert_eq!(*chosen.lock().unwrap(), ["rename"]);
  assert!(!open.get());
  // The focus is back on the element: the Menu key opens the menu again.
  stage.key("ContextMenu");
  assert!(open.get(), "the element had the focus back");
}

#[test]
fn choosing_an_item_calls_its_handler_and_closes_the_menu_and_escape_closes_it() {
  let open = Signal::new(false);
  let (chosen, tell) = log();
  let held = open.clone();
  let mut stage = stage(move |_| placed(area(held.clone(), tell.clone())));
  secondary_press(&mut stage, 100.0, 60.0);
  stage.click("reload");
  assert_eq!(*chosen.lock().unwrap(), ["reload"]);
  assert!(!open.get());
  secondary_press(&mut stage, 100.0, 60.0);
  assert!(open.get());
  stage.key("Escape");
  assert!(!open.get());
}

#[test]
fn a_second_secondary_press_moves_it_and_one_context_menu_is_open_at_a_time() {
  let (first, second) = (Signal::new(false), Signal::new(false));
  let (_, tell) = log();
  let (one, two) = (first.clone(), second.clone());
  let mut stage = stage(move |_| {
    Column::new()
      .padding(10.0)
      .spacing(20.0)
      .child(
        ContextMenu::new(Rect::new(380.0, 100.0).id("one"), one.clone())
          .id("first")
          .item(MenuItem::new("Copy").on_select(tell("copy"))),
      )
      .child(
        ContextMenu::new(Rect::new(380.0, 100.0).id("two"), two.clone())
          .id("second")
          .item(MenuItem::new("Paste").on_select(tell("paste"))),
      )
      .into()
  });
  secondary_press(&mut stage, 50.0, 30.0);
  assert_eq!(stage.bounds("first").0, 50.0);
  // Asked for again on its element, it stands at the new point.
  secondary_press(&mut stage, 120.0, 80.0);
  assert!(first.get());
  assert_eq!((stage.bounds("first").0, stage.bounds("first").1), (120.0, 80.0));
  // Asked for on another element, that one's opens and the first closes.
  secondary_press(&mut stage, 60.0, 180.0);
  assert!(second.get());
  assert!(!first.get());
  assert!(!stage.paints("Copy"));
}

#[test]
fn a_disabled_context_menu_opens_nothing() {
  let open = Signal::new(false);
  let (_, tell) = log();
  let held = open.clone();
  let mut stage = stage(move |_| placed(area(held.clone(), tell.clone()).disabled(true)));
  secondary_press(&mut stage, 100.0, 60.0);
  assert!(!open.get());
  assert!(!stage.paints("Back"));
}

#[test]
fn the_bar_lays_the_commands_in_a_row_as_the_boards_edit_menu() {
  let open = Signal::new(false);
  let (chosen, tell) = log();
  let held = open.clone();
  let mut stage = stage(move |_| {
    placed(
      ContextMenu::new(Rect::new(380.0, 280.0).id("area"), held.clone())
        .bar(true)
        .id("bar")
        .items(["cut", "copy", "paste"].map(|id| MenuItem::new(&id.to_uppercase()).id(id).on_select(tell(id)))),
    )
  });
  secondary_press(&mut stage, 100.0, 60.0);
  let (bx, by, ..) = stage.bounds("bar");
  assert_eq!((bx, by), (100.0, 60.0));
  let (cut, copy, paste) = (stage.bounds("cut"), stage.bounds("copy"), stage.bounds("paste"));
  // In a row, 3 in from the bar, each 28 tall, their labels 13.
  assert_eq!((cut.0 - bx, cut.1 - by), (3.0, 3.0));
  assert_eq!((cut.1, copy.1, paste.1), (cut.1, cut.1, cut.1));
  assert!(cut.0 < copy.0 && copy.0 < paste.0);
  assert_eq!(cut.3, 28.0);
  assert_eq!(stage.text("COPY").1, 13.0);
  // Right moves the focus along it; Enter chooses and closes it.
  stage.key("ArrowRight");
  stage.key("Enter");
  assert_eq!(*chosen.lock().unwrap(), ["copy"]);
  assert!(!open.get());
}

#[test]
fn a_themes_rules_change_it() {
  let app = App::new();
  app
    .theme()
    .patch_recipe::<ContextMenu>(|recipe| recipe.base(ContextMenuPart::Panel, |s| s.min_width(300.0)));
  let open = Signal::new(false);
  let (_, tell) = log();
  let held = open.clone();
  let mut stage = stage_in(app, move |_| placed(area(held.clone(), tell.clone())));
  secondary_press(&mut stage, 20.0, 20.0);
  assert_eq!(stage.bounds("menu").2, 300.0);
}
