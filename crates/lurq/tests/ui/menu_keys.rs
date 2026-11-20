//! A menu that leaves the focus in the app's field (`Menu::keeps_focus`), and an item asked for aside
//! (`MenuItem::on_open_aside`).

use std::sync::{Arc, Mutex};

use lurq::{
  app::{
    events::{KeyboardEvent, MouseButton, MouseEvent},
    theme::{PaletteColor, ThemePalette},
  },
  components::{Column, Rect, TextInput},
  core::{ElementRef, Signal},
  node::{Element, color::Color},
  ui::{Menu, MenuItem, MenuKeys},
};

use super::stage::Stage;

fn highlight() -> Color {
  ThemePalette::light().get(PaletteColor::Highlight)
}

fn focused(stage: &Stage) -> Option<String> {
  stage
    .tree
    .focused_element()
    .and_then(|element| element.id().map(str::to_owned))
}

/// The ids among `ids` drawn on the highlight.
fn highlighted(stage: &mut Stage, ids: &[&'static str]) -> Vec<&'static str> {
  ids
    .iter()
    .copied()
    .filter(|id| stage.fill_color(id) == Some(highlight()))
    .collect()
}

type Log = Arc<Mutex<Vec<String>>>;

fn told(log: &Log, what: &str) -> impl Fn() + Send + Sync + 'static {
  let (log, what) = (log.clone(), what.to_owned());
  move || log.lock().unwrap().push(what.clone())
}

fn said(log: &Log) -> Vec<String> {
  log.lock().unwrap().clone()
}

/// A field, and a menu of three items under it that keeps the focus in it: Cut, Copy (disabled) and Paste. With
/// `forward`, the field's own Enter sends the text, after the menu's keys.
fn field_with_menu(open: Signal<bool>, text: Signal<String>, log: Log, forward: bool) -> Stage {
  let field = ElementRef::new();
  let keys = MenuKeys::new(field.clone());
  Stage::new(move |_| {
    let menu = Menu::new(open.clone(), field.clone())
      .keeps_focus(&keys)
      .item(MenuItem::new("Cut").id("cut").on_select(told(&log, "cut")))
      .item(MenuItem::new("Copy").id("copy").disabled(true).on_select(told(&log, "copy")))
      .item(
        MenuItem::new("Paste")
          .id("paste")
          .on_select(told(&log, "paste"))
          .on_open_aside(told_aside(&log, "paste aside")),
      );
    let mut input = TextInput::new(text.clone())
      .id("field")
      .width(200.0)
      .ref_element(field.clone());
    if forward {
      let (keys, send) = (keys.clone(), told(&log, "send"));
      input = input.on_key_down(move |event: KeyboardEvent| {
        if keys.key_down(&event) {
          return;
        }
        if event.key == "Enter" {
          event.prevent_default();
          send();
        }
      });
    }
    Column::new()
      .size(400.0, 300.0)
      .padding(20.0)
      .child(input)
      .child(menu)
      .child(Rect::new(10.0, 10.0).id("elsewhere"))
      .into()
  })
}

fn told_aside(log: &Log, what: &str) -> impl Fn(MouseEvent) + Send + Sync + 'static {
  let (log, what) = (log.clone(), what.to_owned());
  move |event: MouseEvent| {
    let shift = if event.shift { " shift" } else { "" };
    log.lock().unwrap().push(format!("{what}{shift}"));
  }
}

const ITEMS: [&str; 3] = ["cut", "copy", "paste"];

#[test]
fn a_menu_that_keeps_the_focus_leaves_it_in_the_field_and_its_keys_move_the_active_item() {
  let (open, text, log) = (Signal::new(false), Signal::new(String::new()), Log::default());
  let mut stage = field_with_menu(open.clone(), text.clone(), log.clone(), false);
  stage.click("field");
  open.set(true);
  stage.pass();
  assert!(stage.paints("Cut"));
  assert_eq!(focused(&stage).as_deref(), Some("field"), "the panel takes no focus");
  assert_eq!(highlighted(&mut stage, &ITEMS), ["cut"], "the first item is the active one");

  stage.key("ArrowDown");
  assert_eq!(highlighted(&mut stage, &ITEMS), ["paste"], "Down passes the disabled item by");
  stage.key("ArrowDown");
  assert_eq!(highlighted(&mut stage, &ITEMS), ["cut"], "and goes round");
  stage.key("ArrowUp");
  assert_eq!(highlighted(&mut stage, &ITEMS), ["paste"]);

  stage.key("p");
  assert_eq!(text.get(), "p", "a letter goes on to the field");
  assert_eq!(focused(&stage).as_deref(), Some("field"));

  stage.key("Enter");
  assert_eq!(said(&log), ["paste"], "Enter chooses the active item");
  assert!(!open.get(), "and the menu closes");
  assert_eq!(focused(&stage).as_deref(), Some("field"));

  open.set(true);
  stage.pass();
  assert_eq!(highlighted(&mut stage, &ITEMS), ["cut"], "it opens again at the first item");
  stage.key("Escape");
  assert!(!open.get(), "Escape closes it");
  assert_eq!(focused(&stage).as_deref(), Some("field"), "and leaves the focus in the field");
}

#[test]
fn the_pointer_makes_an_item_the_active_one_and_a_press_of_it_keeps_the_focus_in_the_field() {
  let (open, text, log) = (Signal::new(false), Signal::new(String::new()), Log::default());
  let mut stage = field_with_menu(open.clone(), text, log.clone(), false);
  stage.click("field");
  open.set(true);
  stage.pass();
  stage.hover("paste");
  stage.tree.mouse_move(390.0, 290.0);
  stage.pass();
  assert_eq!(highlighted(&mut stage, &ITEMS), ["paste"], "the item the pointer came onto stays active");

  stage.click("cut");
  assert_eq!(said(&log), ["cut"]);
  assert!(!open.get());
  assert_eq!(focused(&stage).as_deref(), Some("field"), "the press took no focus");
}

#[test]
fn a_field_that_forwards_its_keys_hears_only_those_the_menu_did_not_take() {
  let (open, text, log) = (Signal::new(false), Signal::new(String::new()), Log::default());
  let mut stage = field_with_menu(open.clone(), text, log.clone(), true);
  stage.click("field");
  stage.key("Enter");
  assert_eq!(said(&log), ["send"], "with the menu closed the field's Enter sends");

  open.set(true);
  stage.pass();
  stage.key("Enter");
  assert_eq!(said(&log), ["send", "cut"], "open, Enter is the menu's, once");
}

#[test]
fn ctrl_enter_asks_for_the_active_item_aside_and_closes_the_menu() {
  let (open, text, log) = (Signal::new(false), Signal::new(String::new()), Log::default());
  let mut stage = field_with_menu(open.clone(), text, log.clone(), false);
  stage.click("field");
  open.set(true);
  stage.pass();
  stage.key("ArrowDown");
  command_enter(&mut stage, true);
  assert_eq!(said(&log), ["paste aside shift"], "its handler alone, told of Shift");
  assert!(!open.get());
}

/// Enter with the platform's key held (Ctrl, Cmd on Apple's platforms), and Shift if `shift`.
fn command_enter(stage: &mut Stage, shift: bool) {
  let mac = cfg!(any(target_os = "macos", target_os = "ios"));
  stage
    .tree
    .key_down_with_meta("Enter".to_owned(), "Enter".to_owned(), shift, !mac, false, mac);
  stage
    .tree
    .key_up_with_meta("Enter".to_owned(), "Enter".to_owned(), shift, !mac, false, mac);
  stage.pass();
}

/// A press with the platform's key held, Ctrl or Cmd on Apple's platforms.
fn command_click(stage: &mut Stage, id: &str) {
  let mac = cfg!(any(target_os = "macos", target_os = "ios"));
  let (x, y) = stage.center(id);
  stage.tree.mouse_move(x, y);
  stage
    .tree
    .mouse_down_with_meta(x, y, MouseButton::Left, false, !mac, false, mac);
  stage
    .tree
    .mouse_up_with_meta(x, y, MouseButton::Left, false, !mac, false, mac);
  stage.pass();
}

fn middle_click(stage: &mut Stage, id: &str) {
  let (x, y) = stage.center(id);
  stage.tree.mouse_move(x, y);
  stage.tree.mouse_down(x, y, MouseButton::Middle);
  stage.tree.mouse_up(x, y, MouseButton::Middle);
  stage.pass();
}

/// A menu that takes the focus, as menus do, with an item that opens a place aside.
fn plain_menu(open: Signal<bool>, log: Log) -> Stage {
  let anchor = ElementRef::new();
  Stage::new(move |_| -> Element {
    Column::new()
      .size(400.0, 300.0)
      .padding(20.0)
      .child(Rect::new(120.0, 30.0).id("anchor").ref_element(anchor.clone()))
      .child(
        Menu::new(open.clone(), anchor.clone()).item(
          MenuItem::new("Report")
            .id("report")
            .on_select(told(&log, "select"))
            .on_open_aside(told_aside(&log, "aside")),
        ),
      )
      .into()
  })
}

#[test]
fn an_item_asked_for_aside_calls_its_handler_alone_and_closes_its_menu() {
  let (open, log) = (Signal::new(true), Log::default());
  let mut stage = plain_menu(open.clone(), log.clone());
  middle_click(&mut stage, "report");
  assert_eq!(said(&log), ["aside"], "the middle button asks for it aside");
  assert!(!open.get(), "and closes the menu");

  open.set(true);
  stage.pass();
  command_click(&mut stage, "report");
  assert_eq!(said(&log), ["aside", "aside"], "so does a press with the key held");
  assert!(!open.get());

  open.set(true);
  stage.pass();
  stage.key("Home");
  command_enter(&mut stage, false);
  assert_eq!(said(&log), ["aside", "aside", "aside"], "and Enter with it on the focused item");
  assert!(!open.get());

  open.set(true);
  stage.pass();
  stage.click("report");
  assert_eq!(said(&log), ["aside", "aside", "aside", "select"], "a plain press chooses it");
}
