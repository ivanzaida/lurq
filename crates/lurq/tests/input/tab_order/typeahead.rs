//! Typeahead in a focus group: a letter moves the focus to the element whose
//! label starts with it, letters typed quickly are one prefix, and a pause
//! starts again.

use std::{
  sync::{Arc, Mutex},
  time::{Duration, Instant},
};

use lurq::{
  app::Tree,
  components::{Button, Column, Row, Text, TextInput},
  core::Signal,
  node::FocusGroup,
};

use super::{focused_id, headless, tab};

/// Presses `key` at `at` on the tree's clock.
fn key_at(tree: &mut Tree, key: &str, at: Instant) {
  tree.set_animation_clock(Some(at));
  tree.key_down(key.to_owned(), key.to_owned(), false, false, false);
  tree.key_up(key.to_owned(), key.to_owned(), false, false, false);
}

/// A button whose text is its label, with the id `id`.
fn item(id: &str, text: &str) -> Button {
  Button::new(text).id(id.to_owned()).tab_index(0)
}

/// A list of commands with typeahead: Cut, Copy, Paste, Select all, Share (disabled), Save as.
fn commands(group: FocusGroup) -> Column {
  Column::new().focus_group(group).with_children([
    item("cut", "Cut"),
    item("copy", "Copy"),
    item("paste", "Paste"),
    item("select", "Select all"),
    item("share", "Share").disabled(true),
    item("save", "Save as"),
  ])
}

/// A pause longer than the typeahead's window, which is a second.
const PAUSE: Duration = Duration::from_millis(1100);
/// Quicker than the window: the next letter of a word typed at speed.
const QUICK: Duration = Duration::from_millis(150);

#[test]
fn a_letter_goes_to_the_element_that_starts_with_it_and_again_to_the_next() {
  let mut tree = headless(commands(FocusGroup::vertical().typeahead(true)));
  tab(&mut tree);
  assert_eq!(focused_id(&tree).as_deref(), Some("cut"));
  let mut now = Instant::now();
  key_at(&mut tree, "p", now);
  assert_eq!(focused_id(&tree).as_deref(), Some("paste"));
  now += PAUSE;
  key_at(&mut tree, "S", now);
  assert_eq!(focused_id(&tree).as_deref(), Some("select"), "a capital matches too");
  // The same letter again, quickly, goes on to the next that starts with it: the disabled one is passed by.
  now += QUICK;
  key_at(&mut tree, "s", now);
  assert_eq!(focused_id(&tree).as_deref(), Some("save"));
  now += QUICK;
  key_at(&mut tree, "s", now);
  assert_eq!(focused_id(&tree).as_deref(), Some("select"), "and round");
  // After a pause, from the one after the focused, round the group.
  now += PAUSE;
  key_at(&mut tree, "c", now);
  assert_eq!(focused_id(&tree).as_deref(), Some("cut"));
  now += PAUSE;
  key_at(&mut tree, "c", now);
  assert_eq!(focused_id(&tree).as_deref(), Some("copy"));
  // A letter nothing starts with leaves the focus where it is.
  now += PAUSE;
  key_at(&mut tree, "x", now);
  assert_eq!(focused_id(&tree).as_deref(), Some("copy"));
}

#[test]
fn letters_typed_quickly_are_one_prefix_and_a_pause_starts_again() {
  let mut tree = headless(commands(FocusGroup::vertical().typeahead(true)));
  tab(&mut tree);
  let mut now = Instant::now();
  // "s", "a": past Select all to Save as.
  key_at(&mut tree, "s", now);
  assert_eq!(focused_id(&tree).as_deref(), Some("select"));
  now += QUICK;
  key_at(&mut tree, "a", now);
  assert_eq!(focused_id(&tree).as_deref(), Some("save"));
  // "s", "e" from Save as: the prefix is looked for from the focused one on, so "s" goes on to Select all and
  // "se" stays there.
  now += PAUSE;
  key_at(&mut tree, "s", now);
  now += QUICK;
  key_at(&mut tree, "e", now);
  assert_eq!(focused_id(&tree).as_deref(), Some("select"));
  // After a pause "a" is a word of its own: nothing starts with it.
  now += PAUSE;
  key_at(&mut tree, "a", now);
  assert_eq!(focused_id(&tree).as_deref(), Some("select"));
}

#[test]
fn a_focus_label_is_matched_before_the_text_and_a_button_with_two_texts_has_none() {
  let mut tree = headless(
    Column::new()
      .focus_group(FocusGroup::vertical().typeahead(true))
      .child(item("first", "First"))
      // A label of its own: its text has the shortcut in it, which is not what it is called.
      .child(item("rename", "F2  Rename").focus_label("Rename"))
      // Two texts and no label: typing does not reach it.
      .child(
        Button::empty()
          .id("two")
          .tab_index(0)
          .child(Text::new("Zoom"))
          .child(Text::new("100%")),
      ),
  );
  tab(&mut tree);
  let now = Instant::now();
  key_at(&mut tree, "r", now);
  assert_eq!(focused_id(&tree).as_deref(), Some("rename"));
  key_at(&mut tree, "f", now + PAUSE);
  assert_eq!(focused_id(&tree).as_deref(), Some("first"), "its text is not its label");
  key_at(&mut tree, "z", now + PAUSE * 2);
  assert_eq!(focused_id(&tree).as_deref(), Some("first"));
}

#[test]
fn a_group_without_typeahead_and_a_text_input_in_one_keep_the_letters() {
  let mut tree = headless(commands(FocusGroup::vertical()));
  tab(&mut tree);
  key_at(&mut tree, "p", Instant::now());
  assert_eq!(focused_id(&tree).as_deref(), Some("cut"), "no typeahead");

  let text = Signal::new(String::new());
  let mut tree = headless(
    Row::new()
      .focus_group(FocusGroup::horizontal().typeahead(true))
      .child(TextInput::new(text).id("field").tab_index(0))
      .child(item("paste", "Paste")),
  );
  tab(&mut tree);
  assert_eq!(focused_id(&tree).as_deref(), Some("field"));
  key_at(&mut tree, "p", Instant::now());
  assert_eq!(
    focused_id(&tree).as_deref(),
    Some("field"),
    "the input takes the letter"
  );
}

#[test]
fn space_still_presses_what_has_the_focus() {
  let pressed = Arc::new(Mutex::new(0));
  let presses = pressed.clone();
  let mut tree = headless(
    Column::new()
      .focus_group(FocusGroup::vertical().typeahead(true))
      .child(item("cut", "Cut").on_click(move |_| *presses.lock().unwrap() += 1))
      .child(item("space", " Space")),
  );
  tab(&mut tree);
  key_at(&mut tree, " ", Instant::now());
  assert_eq!(focused_id(&tree).as_deref(), Some("cut"));
  assert_eq!(*pressed.lock().unwrap(), 1);
}
