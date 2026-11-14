//! A row of menus (`FocusGroup::menu_row`): Left and Right that an open menu leaves hand it to the menu beside it,
//! Alt alone puts the focus on the row and gives it back, and Alt with a letter opens the menu of the element that
//! starts with it.

use std::sync::Arc;

use lurq::{
  app::{App, Tree, component::Component, ctx::Ctx, events::KeyboardEvent},
  components::{Button, Column, Popup, Row},
  core::{ElementRef, Signal},
  node::{Element, FocusGroup},
};

use super::{focused_id, tab};

const TITLES: [&str; 3] = ["File", "Edit", "View"];

/// The open signal and the anchor of each menu of the row.
#[derive(Clone, lurq::DevtoolsInspectable)]
struct Menus(#[devtools_ignore] Arc<Vec<(Signal<bool>, ElementRef)>>);

impl PartialEq for Menus {
  fn eq(&self, other: &Self) -> bool {
    Arc::ptr_eq(&self.0, &other.0)
  }
}

impl std::fmt::Debug for Menus {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.write_str("Menus")
  }
}

/// A button before the row, and the row of File, Edit and View: a press of each opens its menu and closes the
/// others. Each menu holds two items, `file-1` and `file-2` for File; Edit's second, `edit-2`, keeps Right for
/// itself, as an item with a submenu does.
struct MenuRow {
  menus: Menus,
}

impl Component for MenuRow {
  type Props = Menus;

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      menus: ctx.props::<Menus>().clone(),
    }
  }

  fn render(&self, _ctx: &mut Ctx) -> impl Into<Element> {
    let all: Vec<Signal<bool>> = self.menus.0.iter().map(|(open, _)| open.clone()).collect();
    let mut row = Row::new().focus_group(FocusGroup::horizontal().one_stop(true).menu_row(true));
    for (index, (open, anchor)) in self.menus.0.iter().enumerate() {
      let name = TITLES[index].to_lowercase();
      let (all, mine) = (all.clone(), open.clone());
      let title = Button::new(TITLES[index])
        .id(name.clone())
        .tab_index(0)
        .ref_element(anchor.clone())
        .on_click(move |_| {
          let opening = !mine.get_untracked();
          for other in &all {
            other.set(false);
          }
          mine.set(opening);
        });
      let mut second = Button::new("Second").id(format!("{name}-2")).tab_index(0);
      if index == 1 {
        second = second.on_key_down(|event: KeyboardEvent| {
          if event.focus_within && event.key == "ArrowRight" {
            event.prevent_default();
          }
        });
      }
      let menu = Column::new()
        .focus_group(FocusGroup::vertical())
        .child(Button::new("First").id(format!("{name}-1")).tab_index(0))
        .child(second);
      row = row.child(
        Column::new()
          .child(title)
          .child(Popup::new(anchor.clone(), menu).open(open.clone()).takes_focus(true)),
      );
    }
    Column::new()
      .size(600.0, 400.0)
      .child(Button::new("Document").id("doc").tab_index(0))
      .child(row)
  }
}

fn mount() -> (Tree, App, Vec<Signal<bool>>) {
  let menus: Vec<(Signal<bool>, ElementRef)> = TITLES.iter().map(|_| (Signal::new(false), ElementRef::new())).collect();
  let open = menus.iter().map(|(open, _)| open.clone()).collect();
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.mount_root::<MenuRow>(&mut app, Menus(Arc::new(menus)));
  tree.pass_headless(&mut app);
  (tree, app, open)
}

fn key(tree: &mut Tree, app: &mut App, key: &str) {
  tree.key_down(key.to_owned(), key.to_owned(), false, false, false);
  tree.key_up(key.to_owned(), key.to_owned(), false, false, false);
  tree.pass_headless(app);
}

/// Alt pressed and let go, with `then` pressed while it is down if it is given.
fn alt(tree: &mut Tree, app: &mut App, then: Option<&str>) {
  tree.key_down("Alt".to_owned(), "AltLeft".to_owned(), false, false, true);
  if let Some(key) = then {
    tree.key_down(key.to_owned(), key.to_owned(), false, false, true);
    tree.key_up(key.to_owned(), key.to_owned(), false, false, true);
  }
  tree.key_up("Alt".to_owned(), "AltLeft".to_owned(), false, false, false);
  tree.pass_headless(app);
}

fn opened(open: &[Signal<bool>]) -> Vec<bool> {
  open.iter().map(Signal::get_untracked).collect()
}

#[test]
fn left_and_right_in_an_open_menu_open_the_one_beside_it() {
  let (mut tree, mut app, open) = mount();
  tab(&mut tree);
  tab(&mut tree);
  assert_eq!(focused_id(&tree).as_deref(), Some("file"));
  key(&mut tree, &mut app, "Enter");
  assert_eq!(opened(&open), [true, false, false]);
  assert_eq!(focused_id(&tree).as_deref(), Some("file-1"));
  // Right in File's menu closes it and presses Edit, whose menu takes the focus.
  key(&mut tree, &mut app, "ArrowRight");
  assert_eq!(opened(&open), [false, true, false]);
  assert_eq!(focused_id(&tree).as_deref(), Some("edit-1"));
  key(&mut tree, &mut app, "ArrowLeft");
  assert_eq!(opened(&open), [true, false, false]);
  assert_eq!(focused_id(&tree).as_deref(), Some("file-1"));
  // From the first, round to the last.
  key(&mut tree, &mut app, "ArrowLeft");
  assert_eq!(opened(&open), [false, false, true]);
  assert_eq!(focused_id(&tree).as_deref(), Some("view-1"));
  // Up and Down are the menu's.
  key(&mut tree, &mut app, "ArrowDown");
  assert_eq!(focused_id(&tree).as_deref(), Some("view-2"));
  assert_eq!(opened(&open), [false, false, true]);
}

#[test]
fn a_key_a_handler_in_the_menu_took_is_not_the_rows() {
  let (mut tree, mut app, open) = mount();
  tab(&mut tree);
  tab(&mut tree);
  key(&mut tree, &mut app, "ArrowRight");
  key(&mut tree, &mut app, "Enter");
  assert_eq!(opened(&open), [false, true, false]);
  key(&mut tree, &mut app, "ArrowDown");
  assert_eq!(focused_id(&tree).as_deref(), Some("edit-2"));
  key(&mut tree, &mut app, "ArrowRight");
  assert_eq!(opened(&open), [false, true, false], "edit-2 keeps Right");
  assert_eq!(focused_id(&tree).as_deref(), Some("edit-2"));
}

#[test]
fn on_the_row_itself_the_arrows_only_move_the_focus() {
  let (mut tree, mut app, open) = mount();
  tab(&mut tree);
  tab(&mut tree);
  key(&mut tree, &mut app, "ArrowRight");
  assert_eq!(focused_id(&tree).as_deref(), Some("edit"));
  assert_eq!(opened(&open), [false, false, false]);
}

#[cfg(not(target_os = "macos"))]
#[test]
fn alt_alone_puts_the_focus_on_the_row_and_again_gives_it_back() {
  let (mut tree, mut app, open) = mount();
  tab(&mut tree);
  assert_eq!(focused_id(&tree).as_deref(), Some("doc"));
  alt(&mut tree, &mut app, None);
  assert_eq!(focused_id(&tree).as_deref(), Some("file"));
  alt(&mut tree, &mut app, None);
  assert_eq!(focused_id(&tree).as_deref(), Some("doc"));
  // From a menu of the row, Alt closes it and gives the focus back too.
  alt(&mut tree, &mut app, None);
  key(&mut tree, &mut app, "Enter");
  assert_eq!(opened(&open), [true, false, false]);
  alt(&mut tree, &mut app, None);
  assert_eq!(opened(&open), [false, false, false]);
  assert_eq!(focused_id(&tree).as_deref(), Some("doc"));
  // Alt with a key between is no Alt alone.
  tree.key_down("Alt".to_owned(), "AltLeft".to_owned(), false, false, true);
  tree.key_down("Shift".to_owned(), "ShiftLeft".to_owned(), true, false, true);
  tree.key_up("Alt".to_owned(), "AltLeft".to_owned(), true, false, false);
  tree.pass_headless(&mut app);
  assert_eq!(focused_id(&tree).as_deref(), Some("doc"));
}

#[cfg(not(target_os = "macos"))]
#[test]
fn alt_with_a_letter_opens_the_menu_of_the_element_that_starts_with_it() {
  let (mut tree, mut app, open) = mount();
  tab(&mut tree);
  alt(&mut tree, &mut app, Some("e"));
  assert_eq!(opened(&open), [false, true, false]);
  assert_eq!(focused_id(&tree).as_deref(), Some("edit-1"));
  // With a menu open, another letter opens the other menu, and one that starts nothing does nothing.
  alt(&mut tree, &mut app, Some("v"));
  assert_eq!(opened(&open), [false, false, true]);
  alt(&mut tree, &mut app, Some("q"));
  assert_eq!(opened(&open), [false, false, true]);
}
