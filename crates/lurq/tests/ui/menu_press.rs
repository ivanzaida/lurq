//! What a press does in a menu: an item that leaves the menu open (`MenuItem::keeps_open`), submenus that open on a
//! press only (`Menu::submenus(SubmenuOpen::Press)`), and an anchor that is no trigger of a dropdown
//! (`Menu::switch_on_anchor_press(false)`).

use std::sync::{Arc, Mutex};

use lurq::{
  components::{Column, Rect, TextInput},
  core::{ElementRef, Signal},
  node::Element,
  ui::{Button, Menu, MenuItem, MenuKeys, SubmenuOpen},
};

use super::stage::Stage;

type Log = Arc<Mutex<Vec<String>>>;

fn said(log: &Log) -> Vec<String> {
  log.lock().unwrap().clone()
}

fn focused(stage: &Stage) -> Option<String> {
  stage
    .tree
    .focused_element()
    .and_then(|element| element.id().map(str::to_owned))
}

fn page(anchor: ElementRef, menu: Menu) -> Element {
  Column::new()
    .size(400.0, 400.0)
    .padding(20.0)
    .child(Rect::new(120.0, 30.0).id("anchor").ref_element(anchor))
    .child(menu)
    .into()
}

/// A filter's options that choose several: each one turns itself on or off in `chosen` and leaves the menu open.
fn options(chosen: &Signal<Vec<&'static str>>) -> Vec<MenuItem> {
  ["Running", "Failed", "Done"]
    .into_iter()
    .map(|name| {
      let chosen = chosen.clone();
      MenuItem::new(name)
        .id(name.to_lowercase())
        .checked(chosen.get().contains(&name))
        .keeps_open(true)
        .on_select(move || {
          chosen.update(|held| match held.iter().position(|at| *at == name) {
            Some(at) => {
              held.remove(at);
            }
            None => held.push(name),
          });
        })
    })
    .collect()
}

#[test]
fn an_item_that_keeps_the_menu_open_is_chosen_and_the_menu_stays() {
  let (open, chosen) = (Signal::new(true), Signal::new(Vec::new()));
  let anchor = ElementRef::new();
  let (held, picked) = (open.clone(), chosen.clone());
  let mut stage = Stage::new(move |_| {
    page(
      anchor.clone(),
      Menu::new(held.clone(), anchor.clone())
        .id("menu")
        .items(options(&picked))
        .separator()
        .item(MenuItem::new("Clear").id("clear").on_select({
          let picked = picked.clone();
          move || picked.set(Vec::new())
        })),
    )
  });
  stage.click("running");
  stage.click("done");
  assert_eq!(chosen.get(), ["Running", "Done"]);
  assert!(open.get(), "the menu stays open");

  // From the keyboard: Enter and Space on the focused option choose it, and the focus stays on it. The press of
  // Done left it there.
  assert_eq!(focused(&stage).as_deref(), Some("done"));
  stage.key("ArrowUp");
  assert_eq!(focused(&stage).as_deref(), Some("failed"));
  stage.key("Enter");
  assert_eq!(chosen.get(), ["Running", "Done", "Failed"]);
  assert!(open.get());
  assert_eq!(
    focused(&stage).as_deref(),
    Some("failed"),
    "the focus stays on the option"
  );
  stage.key(" ");
  assert_eq!(chosen.get(), ["Running", "Done"]);

  // An item without it still closes the menu.
  stage.click("clear");
  assert!(chosen.get().is_empty());
  assert!(!open.get());
}

#[test]
fn an_item_that_keeps_the_menu_open_does_so_where_the_menu_keeps_the_focus_in_a_field() {
  let (open, chosen) = (Signal::new(false), Signal::new(Vec::new()));
  let field = ElementRef::new();
  let keys = MenuKeys::new(field.clone());
  let (held, picked, menu_keys) = (open.clone(), chosen.clone(), keys.clone());
  let mut stage = Stage::new(move |_| {
    Column::new()
      .size(400.0, 400.0)
      .padding(20.0)
      .child(
        TextInput::new(Signal::new(String::new()))
          .id("field")
          .width(200.0)
          .ref_element(field.clone()),
      )
      .child(
        Menu::new(held.clone(), field.clone())
          .keeps_focus(&menu_keys)
          .items(options(&picked)),
      )
      .into()
  });
  stage.click("field");
  open.set(true);
  stage.pass();
  stage.key("ArrowDown");
  stage.key("Enter");
  assert_eq!(chosen.get(), ["Failed"]);
  assert!(open.get(), "the menu stays open");
  assert_eq!(keys.active(), Some(1), "and the active item where it was");
  assert_eq!(focused(&stage).as_deref(), Some("field"));
  stage.click("done");
  assert_eq!(chosen.get(), ["Failed", "Done"]);
  assert!(open.get());
}

/// A menu with an item, and two items with submenus that open on a press: Move to and Priority.
fn card_menu(open: &Signal<bool>, moving: &Signal<bool>, ranking: &Signal<bool>, log: &Log) -> Stage {
  let (open, moving, ranking, log) = (open.clone(), moving.clone(), ranking.clone(), log.clone());
  let (anchor, move_to, priority) = (ElementRef::new(), ElementRef::new(), ElementRef::new());
  Stage::new(move |_| {
    let told = |what: &'static str| {
      let log = log.clone();
      move || log.lock().unwrap().push(what.to_owned())
    };
    page(
      anchor.clone(),
      Menu::new(open.clone(), anchor.clone())
        .id("menu")
        .submenus(SubmenuOpen::Press)
        .style(lurq::ui::MenuPart::Panel, |s| s.min_width(160.0))
        .item(MenuItem::new("Open").id("open").on_select(told("open")))
        .item(
          MenuItem::new("Move to").id("move").submenu(
            Menu::new(moving.clone(), move_to.clone())
              .id("columns")
              .item(MenuItem::new("Done").id("done").on_select(told("done"))),
          ),
        )
        .item(
          MenuItem::new("Priority").id("priority").submenu(
            Menu::new(ranking.clone(), priority.clone())
              .id("priorities")
              .item(MenuItem::new("High").id("high")),
          ),
        ),
    )
  })
}

#[test]
fn submenus_that_open_on_a_press_do_not_open_or_close_under_the_pointer() {
  let (open, moving, ranking, log) = (
    Signal::new(true),
    Signal::new(false),
    Signal::new(false),
    Log::default(),
  );
  let mut stage = card_menu(&open, &moving, &ranking, &log);
  stage.hover("move");
  assert!(!moving.get(), "the pointer opens nothing");

  stage.click("move");
  assert!(moving.get(), "a press opens it");
  stage.hover("open");
  stage.hover("priority");
  assert!(
    moving.get() && !ranking.get(),
    "the pointer on other items closes nothing and opens nothing"
  );

  // A press of another item with a submenu opens its own and closes the other.
  stage.click("priority");
  assert!(ranking.get() && !moving.get());
  // A press of the item whose submenu is open closes it.
  stage.click("priority");
  assert!(!ranking.get() && open.get());

  // Choosing in a submenu closes it and the menu.
  stage.click("move");
  stage.click("done");
  assert_eq!(said(&log), ["done"]);
  assert!(!moving.get() && !open.get());
}

#[test]
fn the_keyboard_opens_a_submenu_of_a_menu_that_opens_them_on_a_press() {
  let (open, moving, ranking, log) = (
    Signal::new(true),
    Signal::new(false),
    Signal::new(false),
    Log::default(),
  );
  let mut stage = card_menu(&open, &moving, &ranking, &log);
  assert_eq!(focused(&stage).as_deref(), Some("open"));
  stage.key("ArrowDown");
  stage.key("ArrowRight");
  assert!(moving.get(), "Right opens it");
  assert_eq!(focused(&stage).as_deref(), Some("done"), "at its first item");
  stage.key("ArrowLeft");
  assert!(!moving.get());
  assert_eq!(focused(&stage).as_deref(), Some("move"));
  stage.key("Enter");
  assert!(moving.get(), "Enter opens it too");
  stage.key("Enter");
  assert_eq!(said(&log), ["done"]);
  assert!(!open.get());
}

/// A menu at a point over a row, opened by the app, and a menu under a button that a press opens: the row is the
/// first menu's anchor, and says when it is pressed.
fn row_and_button(row_open: &Signal<bool>, button_open: &Signal<bool>, switch: bool, log: &Log) -> Stage {
  let (row_open, button_open, log) = (row_open.clone(), button_open.clone(), log.clone());
  let (row, button) = (ElementRef::new(), ElementRef::new());
  Stage::new(move |_| {
    let pressed = log.clone();
    let toggle = button_open.clone();
    Column::new()
      .size(400.0, 400.0)
      .padding(20.0)
      .spacing(200.0)
      .child(
        Button::new("Actions")
          .id("actions")
          .ref_element(button.clone())
          .on_click(move |_| toggle.update(|open| *open = !*open)),
      )
      .child(
        Menu::new(button_open.clone(), button.clone())
          .id("actions-menu")
          .item(MenuItem::new("Refresh")),
      )
      .child(
        Button::new("Row")
          .id("row")
          .ref_element(row.clone())
          .on_click(move |_| pressed.lock().unwrap().push("row".to_owned())),
      )
      .child(
        Menu::new(row_open.clone(), row.clone())
          .id("row-menu")
          .at(Some((60.0, 40.0)))
          .switch_on_anchor_press(switch)
          .item(MenuItem::new("Rename")),
      )
      .into()
  })
}

#[test]
fn a_menu_whose_anchor_is_no_trigger_does_not_let_a_press_of_it_through_another_open_menu() {
  // The anchor a trigger, as by default: with another menu open, a press of the row closes that one and reaches the
  // row.
  let (row_open, button_open, log) = (Signal::new(false), Signal::new(false), Log::default());
  let mut stage = row_and_button(&row_open, &button_open, true, &log);
  stage.click("actions");
  assert!(button_open.get());
  stage.click("row");
  assert!(!button_open.get());
  assert_eq!(said(&log), ["row"], "the press went on to the row");

  // No trigger: the press only closes the open menu, as a press anywhere else does.
  let (row_open, button_open, log) = (Signal::new(false), Signal::new(false), Log::default());
  let mut stage = row_and_button(&row_open, &button_open, false, &log);
  stage.click("actions");
  assert!(button_open.get());
  stage.click("row");
  assert!(!button_open.get());
  assert!(said(&log).is_empty(), "the press only closed the menu");
}
