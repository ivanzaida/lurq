//! The keys a listbox needs of a focus group: PageUp and PageDown move by what the scroll the group is in shows
//! (`FocusGroup::pages`), a move with Shift presses what it moves to with Shift (`FocusGroup::shift_extends`), and
//! in a group whose choice follows the focus a letter chooses as well as it moves (`activate` with `typeahead`).

use std::sync::{Arc, Mutex};

use lurq::{
  app::{App, Tree, events::MouseEvent},
  components::{Button, Column, ScrollVertical},
  node::FocusGroup,
};

use super::{focused_id, headless, tab};

fn key(tree: &mut Tree, key: &str, shift: bool) {
  tree.key_down(key.to_owned(), key.to_owned(), shift, false, false);
  tree.key_up(key.to_owned(), key.to_owned(), shift, false, false);
  tree.pass_headless(&mut App::new());
}

type Pressed = Arc<Mutex<Vec<(String, bool)>>>;

/// Twenty rows of 20, `row-0` to `row-19`, each telling `pressed` its id and whether Shift was held.
fn rows(group: FocusGroup, pressed: &Pressed) -> Column {
  Column::new().focus_group(group).with_children((0..20).map(|index| {
    let (id, pressed) = (format!("row-{index}"), pressed.clone());
    Button::new(&format!("Row {index}"))
      .id(id.clone())
      .tab_index(0)
      .height(20.0)
      .on_click(move |event: MouseEvent| pressed.lock().unwrap().push((id.clone(), event.shift)))
  }))
}

fn paged() -> FocusGroup {
  FocusGroup::vertical().one_stop(true).wrap(false).pages(true)
}

#[test]
fn page_down_and_page_up_move_by_what_the_scroll_shows() {
  let pressed = Pressed::default();
  // 100 high: five rows of 20 are shown.
  let mut tree = headless(ScrollVertical::new(rows(paged(), &pressed)).height(100.0));
  tab(&mut tree);
  assert_eq!(focused_id(&tree).as_deref(), Some("row-0"));
  key(&mut tree, "PageDown", false);
  assert_eq!(focused_id(&tree).as_deref(), Some("row-5"), "a page on: five rows");
  key(&mut tree, "PageDown", false);
  assert_eq!(focused_id(&tree).as_deref(), Some("row-10"));
  key(&mut tree, "PageUp", false);
  assert_eq!(focused_id(&tree).as_deref(), Some("row-5"), "a page back");
  key(&mut tree, "End", false);
  key(&mut tree, "PageDown", false);
  assert_eq!(focused_id(&tree).as_deref(), Some("row-19"), "nothing past the last");
  assert!(pressed.lock().unwrap().is_empty(), "moving presses nothing");
}

#[test]
fn a_group_that_does_not_scroll_is_one_page() {
  let pressed = Pressed::default();
  let mut tree = headless(rows(paged(), &pressed));
  tab(&mut tree);
  key(&mut tree, "PageDown", false);
  assert_eq!(focused_id(&tree).as_deref(), Some("row-19"));
  key(&mut tree, "PageUp", false);
  assert_eq!(focused_id(&tree).as_deref(), Some("row-0"));
}

#[test]
fn a_group_without_pages_leaves_the_page_keys_alone() {
  let pressed = Pressed::default();
  let mut tree = headless(rows(FocusGroup::vertical().one_stop(true), &pressed));
  tab(&mut tree);
  key(&mut tree, "PageDown", false);
  assert_eq!(focused_id(&tree).as_deref(), Some("row-0"));
}

#[test]
fn a_move_with_shift_presses_what_it_moves_to_with_shift() {
  let pressed = Pressed::default();
  let mut tree = headless(rows(paged().shift_extends(true), &pressed));
  tab(&mut tree);
  key(&mut tree, "ArrowDown", false);
  assert!(
    pressed.lock().unwrap().is_empty(),
    "without Shift the focus moves alone"
  );
  key(&mut tree, "ArrowDown", true);
  key(&mut tree, "End", true);
  assert_eq!(
    *pressed.lock().unwrap(),
    [("row-2".to_owned(), true), ("row-19".to_owned(), true)]
  );
}

#[test]
fn where_the_choice_follows_the_focus_a_letter_chooses_too() {
  let pressed = Pressed::default();
  let group = FocusGroup::vertical().one_stop(true).typeahead(true).activate(true);
  let mut tree = headless(rows(group, &pressed));
  tab(&mut tree);
  key(&mut tree, "ArrowDown", false);
  assert_eq!(*pressed.lock().unwrap(), [("row-1".to_owned(), false)]);
  // "Row 1" is the focused one's: the next that starts with "r".
  key(&mut tree, "r", false);
  assert_eq!(focused_id(&tree).as_deref(), Some("row-2"));
  assert_eq!(pressed.lock().unwrap().last(), Some(&("row-2".to_owned(), false)));
}
