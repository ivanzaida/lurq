//! A section of a menu that scrolls (`Menu::scrolling`): its items stay the menu's, the arrows and typing reach them
//! and scroll them into view, and in a menu that keeps the focus in the app's field the keys move through them and
//! the active one is scrolled into view.

use std::sync::{Arc, Mutex};

use lurq::{
  app::theme::{PaletteColor, ThemePalette},
  components::{Column, TextInput},
  core::{ElementRef, Signal},
  node::{Element, color::Color},
  ui::{Menu, MenuEntry, MenuItem, MenuKeys},
};

use super::stage::Stage;

const NAMES: [&str; 12] = [
  "Alder", "Birch", "Cedar", "Dogwood", "Elm", "Fir", "Ginkgo", "Hazel", "Ivy", "Juniper", "Kapok", "Larch",
];

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

fn highlight() -> Color {
  ThemePalette::light().get(PaletteColor::Highlight)
}

/// The trees of `names`, as items `tree-<n>` that say they were chosen.
fn trees(names: &[&'static str], log: &Log) -> Vec<MenuEntry> {
  names
    .iter()
    .copied()
    .enumerate()
    .map(|(index, name)| {
      let log = log.clone();
      let name: &'static str = name;
      MenuItem::new(name)
        .id(format!("tree-{index}"))
        .on_select(move || log.lock().unwrap().push(name.to_owned()))
        .into()
    })
    .collect()
}

/// Whether the element `id` is wholly inside the box of `view`.
fn shown_in(stage: &mut Stage, id: &str, view: &str) -> bool {
  let (_, y, _, height) = stage.bounds(id);
  let (_, top, _, room) = stage.bounds(view);
  y >= top - 0.5 && y + height <= top + room + 0.5
}

fn page(anchor: ElementRef, menu: Menu) -> Element {
  Column::new()
    .size(400.0, 500.0)
    .padding(20.0)
    .child(Column::new().size(120.0, 30.0).id("anchor").ref_element(anchor))
    .child(menu)
    .into()
}

#[test]
fn a_section_that_scrolls_is_as_high_as_it_is_told_and_its_items_are_the_menus() {
  let (open, log) = (Signal::new(true), Log::default());
  let anchor = ElementRef::new();
  let (held, told) = (open.clone(), log.clone());
  let mut stage = Stage::new(move |_| {
    page(
      anchor.clone(),
      Menu::new(held.clone(), anchor.clone())
        .id("menu")
        .item(MenuItem::new("New tree").id("new"))
        .separator()
        .scrolling(trees(&NAMES, &told), 128.0)
        .item(MenuItem::new("Unnamed")),
    )
  });
  // Four rows of 32 show; the rest scroll. The panel holds the section and the items around it.
  let (_, _, _, height) = stage.bounds("menu-scroll");
  assert_eq!(height, 128.0);
  let (_, panel_y, _, panel_height) = stage.bounds("menu");
  assert!(
    panel_height < 128.0 + 3.0 * 32.0 + 20.0,
    "the panel is as high as what it shows"
  );
  assert!(panel_y > 0.0);
  assert!(shown_in(&mut stage, "tree-0", "menu-scroll"));
  assert!(!shown_in(&mut stage, "tree-6", "menu-scroll"));
  // An item after the section with no id of its own is named after the menu, after those in the section.
  assert!(stage.ids().iter().any(|id| id == "menu-item"), "{:?}", stage.ids());

  // The arrows reach the items of the section, and the one the focus goes to is scrolled into view.
  assert_eq!(focused(&stage).as_deref(), Some("new"));
  for _ in 0..7 {
    stage.key("ArrowDown");
  }
  assert_eq!(focused(&stage).as_deref(), Some("tree-6"));
  assert!(
    shown_in(&mut stage, "tree-6", "menu-scroll"),
    "the focused item is in view"
  );

  // Typing finds an item of the section by its label.
  stage.key("k");
  assert_eq!(focused(&stage).as_deref(), Some("tree-10"));
  assert!(shown_in(&mut stage, "tree-10", "menu-scroll"));

  // Choosing one closes the menu and calls its handler.
  stage.key("Enter");
  assert_eq!(said(&log), ["Kapok"]);
  assert!(!open.get());
}

#[test]
fn a_press_of_an_item_in_the_section_chooses_it() {
  let (open, log) = (Signal::new(true), Log::default());
  let anchor = ElementRef::new();
  let (held, told) = (open.clone(), log.clone());
  let mut stage = Stage::new(move |_| {
    page(
      anchor.clone(),
      Menu::new(held.clone(), anchor.clone())
        .id("menu")
        .scrolling(trees(&NAMES[..3], &told), 200.0),
    )
  });
  // Shorter than its most height, the section is as high as its items.
  let (_, _, _, height) = stage.bounds("menu-scroll");
  assert_eq!(height, 3.0 * 32.0);
  stage.click("tree-1");
  assert_eq!(said(&log), ["Birch"]);
  assert!(!open.get());
}

/// A field, and a menu under it that keeps the focus in it, holding a command and the trees `shown` says in a
/// section 128 high.
fn field_with_trees(open: &Signal<bool>, shown: &Signal<usize>, log: &Log) -> (Stage, MenuKeys) {
  let field = ElementRef::new();
  let keys = MenuKeys::new(field.clone());
  let (open, shown, log, held) = (open.clone(), shown.clone(), log.clone(), keys.clone());
  let stage = Stage::new(move |_| {
    let menu = Menu::new(open.clone(), field.clone())
      .id("menu")
      .keeps_focus(&held)
      .item(MenuItem::new("Plant").id("plant"))
      .scrolling(trees(&NAMES[..shown.get()], &log), 128.0);
    Column::new()
      .size(400.0, 500.0)
      .padding(20.0)
      .child(
        TextInput::new(Signal::new(String::new()))
          .id("field")
          .width(200.0)
          .ref_element(field.clone()),
      )
      .child(menu)
      .into()
  });
  (stage, keys)
}

#[test]
fn the_keys_of_a_menu_that_keeps_the_focus_move_through_the_section_and_scroll_the_active_item_into_view() {
  let (open, shown, log) = (Signal::new(false), Signal::new(NAMES.len()), Log::default());
  let (mut stage, keys) = field_with_trees(&open, &shown, &log);
  stage.click("field");
  open.set(true);
  stage.pass();
  assert_eq!(focused(&stage).as_deref(), Some("field"));
  assert_eq!(keys.active(), Some(0), "the command first");

  for _ in 0..8 {
    stage.key("ArrowDown");
  }
  assert_eq!(keys.active(), Some(8), "the eighth tree, Hazel");
  assert_eq!(stage.fill_color("tree-7"), Some(highlight()));
  assert!(
    shown_in(&mut stage, "tree-7", "menu-scroll"),
    "the active item is in view"
  );
  assert!(
    !shown_in(&mut stage, "tree-0", "menu-scroll"),
    "the section scrolled to it"
  );

  // Up past the section's top brings it back the other way.
  for _ in 0..7 {
    stage.key("ArrowUp");
  }
  assert_eq!(keys.active(), Some(1));
  assert!(shown_in(&mut stage, "tree-0", "menu-scroll"));

  // Round from the first to the last item.
  stage.key("ArrowUp");
  stage.key("ArrowUp");
  assert_eq!(keys.active(), Some(NAMES.len()));
  assert!(shown_in(
    &mut stage,
    &format!("tree-{}", NAMES.len() - 1),
    "menu-scroll"
  ));

  // Enter chooses the active item of the section.
  stage.key("Enter");
  assert_eq!(said(&log), ["Larch"]);
  assert!(!open.get());
  assert_eq!(focused(&stage).as_deref(), Some("field"));
}

#[test]
fn the_section_goes_back_to_its_top_when_what_it_holds_changes() {
  let (open, shown, log) = (Signal::new(false), Signal::new(NAMES.len()), Log::default());
  let (mut stage, keys) = field_with_trees(&open, &shown, &log);
  stage.click("field");
  open.set(true);
  stage.pass();
  for _ in 0..10 {
    stage.key("ArrowDown");
  }
  assert!(!shown_in(&mut stage, "tree-0", "menu-scroll"));

  // What was typed holds fewer trees: the first item is the active one, and the section is at its top.
  shown.set(9);
  stage.pass();
  assert_eq!(keys.active(), Some(0));
  assert!(shown_in(&mut stage, "tree-0", "menu-scroll"));
}

#[test]
fn the_entries_of_a_section_that_scrolls_are_as_far_apart_as_its_part_says() {
  for gap in [None, Some(1.0)] {
    let open = Signal::new(true);
    let anchor = ElementRef::new();
    let held = open.clone();
    let mut stage = Stage::new(move |_| {
      let mut rules = lurq::ui::Recipe::new();
      if let Some(gap) = gap {
        rules = rules.base(lurq::ui::MenuPart::Scroll, |s| s.gap(gap));
      }
      page(
        anchor.clone(),
        Menu::new(held.clone(), anchor.clone())
          .id("menu")
          .rules(&rules)
          .scrolling(trees(&NAMES, &Log::default()), 128.0),
      )
    });
    let (_, first, _, height) = stage.bounds("tree-0");
    let (_, second, ..) = stage.bounds("tree-1");
    assert!(
      (second - (first + height + gap.unwrap_or(0.0))).abs() < 0.01,
      "{gap:?}: {first} {height} {second}"
    );
  }
}

/// Ten accounts, keyed `account-<n>` where `keyed`, and under the one `opened` its three models, whose labels are
/// the same under every account.
fn accounts(opened: Option<usize>, keyed: bool) -> Vec<MenuEntry> {
  let mut entries = Vec::new();
  for account in 0..10 {
    let mut item = MenuItem::new(&format!("Account {account}")).id(format!("account-{account}"));
    if keyed {
      item = item.key(format!("account-{account}"));
    }
    entries.push(item.into());
    if opened == Some(account) {
      for model in ["Small", "Medium", "Large"] {
        let mut item = MenuItem::new(model);
        if keyed {
          item = item.key(format!("account-{account}-{model}"));
        }
        entries.push(item.into());
      }
    }
  }
  entries
}

fn account_picker(opened: &Signal<Option<usize>>, keyed: bool) -> (Stage, MenuKeys, Signal<bool>) {
  let (field, open) = (ElementRef::new(), Signal::new(false));
  let keys = MenuKeys::new(field.clone());
  let (held, shown, opened) = (keys.clone(), open.clone(), opened.clone());
  let mut stage = Stage::new(move |_| {
    let menu = Menu::new(shown.clone(), field.clone())
      .id("menu")
      .keeps_focus(&held)
      .scrolling(accounts(opened.get(), keyed), 128.0);
    Column::new()
      .size(400.0, 500.0)
      .padding(20.0)
      .child(
        TextInput::new(Signal::new(String::new()))
          .id("field")
          .width(200.0)
          .ref_element(field.clone()),
      )
      .child(menu)
      .into()
  });
  stage.click("field");
  open.set(true);
  stage.pass();
  (stage, keys, open)
}

#[test]
fn the_active_item_stays_by_its_key_when_what_the_menu_holds_changes() {
  let opened = Signal::new(None);
  let (mut stage, keys, open) = account_picker(&opened, true);
  for _ in 0..7 {
    stage.key("ArrowDown");
  }
  assert_eq!(keys.active(), Some(7));
  assert!(!shown_in(&mut stage, "account-0", "menu-scroll"));

  // The account opens its models under it: it stays the active one, and the section where it is scrolled to.
  opened.set(Some(7));
  stage.pass();
  assert!(open.get());
  assert_eq!(keys.active(), Some(7), "Account 7 still");
  assert!(shown_in(&mut stage, "account-7", "menu-scroll"));
  assert!(!shown_in(&mut stage, "account-0", "menu-scroll"), "not back at the top");

  // Models open above it: it is further down the items, and still the active one.
  opened.set(Some(2));
  stage.pass();
  assert_eq!(keys.active(), Some(10), "Account 7, after three models");
  stage.key("Enter");
  assert!(!open.get(), "Enter chose it");
}

#[test]
fn an_item_with_no_key_gives_way_to_the_first_when_what_the_menu_holds_changes() {
  let opened = Signal::new(None);
  let (mut stage, keys, _) = account_picker(&opened, false);
  for _ in 0..7 {
    stage.key("ArrowDown");
  }
  opened.set(Some(7));
  stage.pass();
  assert_eq!(keys.active(), Some(0));
  assert!(shown_in(&mut stage, "account-0", "menu-scroll"));
}
