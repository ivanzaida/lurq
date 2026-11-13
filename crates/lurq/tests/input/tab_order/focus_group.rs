//! A focus group: the arrow keys move the focus between what is in it, and
//! Tab stops in it once if it is told to.

use std::sync::{Arc, Mutex};

use lurq::{
  app::Tree,
  components::{Button, Column, Row, TextInput},
  core::Signal,
  node::FocusGroup,
};

use super::{focused_id, headless, shift_tab, tab};

fn key(tree: &mut Tree, key: &str) {
  tree.key_down(key.to_owned(), key.to_owned(), false, false, false);
  tree.key_up(key.to_owned(), key.to_owned(), false, false, false);
}

fn stop(id: &str) -> Button {
  Button::new(id).id(id.to_owned()).tab_index(0)
}

/// A button before a group of three and a button after it.
fn bar(group: FocusGroup) -> Column {
  Column::new()
    .child(stop("before"))
    .child(
      Row::new()
        .focus_group(group)
        .child(stop("one"))
        .child(stop("two").disabled(true))
        .child(stop("three"))
        .child(stop("four")),
    )
    .child(stop("after"))
}

#[test]
fn the_arrow_keys_move_the_focus_within_the_group_and_pass_a_disabled_one_by() {
  let mut tree = headless(bar(FocusGroup::horizontal()));
  tab(&mut tree);
  tab(&mut tree);
  assert_eq!(focused_id(&tree).as_deref(), Some("one"));
  key(&mut tree, "ArrowRight");
  assert_eq!(
    focused_id(&tree).as_deref(),
    Some("three"),
    "the disabled one is passed by"
  );
  key(&mut tree, "ArrowRight");
  assert_eq!(focused_id(&tree).as_deref(), Some("four"));
  key(&mut tree, "ArrowRight");
  assert_eq!(focused_id(&tree).as_deref(), Some("one"), "from the last to the first");
  key(&mut tree, "ArrowLeft");
  assert_eq!(focused_id(&tree).as_deref(), Some("four"), "and back");
  key(&mut tree, "Home");
  assert_eq!(focused_id(&tree).as_deref(), Some("one"));
  key(&mut tree, "End");
  assert_eq!(focused_id(&tree).as_deref(), Some("four"));

  // The keys of the other axis are not the group's, and outside a group no arrow moves the focus.
  key(&mut tree, "ArrowDown");
  assert_eq!(focused_id(&tree).as_deref(), Some("four"));
  tab(&mut tree);
  assert_eq!(focused_id(&tree).as_deref(), Some("after"));
  key(&mut tree, "ArrowLeft");
  assert_eq!(focused_id(&tree).as_deref(), Some("after"));
}

#[test]
fn a_group_that_does_not_wrap_stops_at_its_ends() {
  let mut tree = headless(bar(FocusGroup::vertical().wrap(false)));
  tab(&mut tree);
  tab(&mut tree);
  key(&mut tree, "ArrowUp");
  assert_eq!(focused_id(&tree).as_deref(), Some("one"));
  key(&mut tree, "ArrowDown");
  key(&mut tree, "ArrowDown");
  key(&mut tree, "ArrowDown");
  assert_eq!(focused_id(&tree).as_deref(), Some("four"));
}

#[test]
fn tab_stops_once_in_a_group_that_is_one_stop() {
  let group = FocusGroup::horizontal().one_stop(true);
  let mut tree = headless(bar(group));
  tab(&mut tree);
  tab(&mut tree);
  assert_eq!(
    focused_id(&tree).as_deref(),
    Some("one"),
    "its first, with no entry marked"
  );
  tab(&mut tree);
  assert_eq!(focused_id(&tree).as_deref(), Some("after"), "Tab leaves the group");
  shift_tab(&mut tree);
  assert_eq!(focused_id(&tree).as_deref(), Some("one"));
  key(&mut tree, "ArrowRight");
  key(&mut tree, "ArrowRight");
  assert_eq!(focused_id(&tree).as_deref(), Some("four"));
  // From inside the group Tab goes on from where the focus is, and back before the group.
  tab(&mut tree);
  assert_eq!(focused_id(&tree).as_deref(), Some("after"));
  shift_tab(&mut tree);
  shift_tab(&mut tree);
  assert_eq!(focused_id(&tree).as_deref(), Some("before"));

  // The entry is where Tab comes in.
  let mut tree = headless(
    Column::new().child(stop("before")).child(
      Row::new()
        .focus_group(group)
        .child(stop("one"))
        .child(stop("two").focus_entry(true))
        .child(stop("three")),
    ),
  );
  tab(&mut tree);
  tab(&mut tree);
  assert_eq!(focused_id(&tree).as_deref(), Some("two"));
}

#[test]
fn a_group_whose_choice_follows_its_focus_presses_what_the_focus_moves_to() {
  let pressed = Arc::new(Mutex::new(Vec::new()));
  let pick = |id: &'static str| {
    let pressed = pressed.clone();
    stop(id).on_click(move |_| pressed.lock().unwrap().push(id))
  };
  let mut tree = headless(
    Row::new()
      .focus_group(FocusGroup::both().activate(true))
      .child(pick("one"))
      .child(pick("two"))
      .child(pick("three")),
  );
  tab(&mut tree);
  assert!(
    pressed.lock().unwrap().is_empty(),
    "Tab moves the focus and presses nothing"
  );
  key(&mut tree, "ArrowDown");
  key(&mut tree, "ArrowRight");
  key(&mut tree, "ArrowUp");
  assert_eq!(*pressed.lock().unwrap(), ["two", "three", "two"]);
}

#[test]
fn a_text_input_in_a_group_keeps_the_arrow_keys() {
  let text = Signal::new("abc".to_owned());
  let mut tree = headless(
    Row::new()
      .focus_group(FocusGroup::horizontal())
      .child(stop("one"))
      .child(TextInput::new(text).id("field").tab_index(0))
      .child(stop("three")),
  );
  tab(&mut tree);
  key(&mut tree, "ArrowRight");
  assert_eq!(focused_id(&tree).as_deref(), Some("field"));
  key(&mut tree, "ArrowRight");
  key(&mut tree, "ArrowLeft");
  assert_eq!(focused_id(&tree).as_deref(), Some("field"), "the arrows move its caret");
}
