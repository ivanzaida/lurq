use lurq::{
  app::{Tree, events::MouseButton},
  core::Signal,
};

use crate::support::run_pass;

#[test]
fn typing_into_focused_text_input_appends_to_existing_value() {
  let value = Signal::new("A".to_owned());
  let mut runtime = Tree::new();

  runtime.set_root(lurq::components::TextInput::new(value.clone()));
  run_pass(&mut runtime);
  let rect = runtime.find_element(|_| true).unwrap().bounds();
  let (x, y) = rect.center();

  runtime.click(x, y, MouseButton::Left);
  runtime.key_down("B".to_owned(), "KeyB".to_owned(), false, false, false);

  assert_eq!(value.get(), "AB");
}

#[test]
fn backspace_removes_character_before_caret() {
  let value = Signal::new("AB".to_owned());
  let mut runtime = Tree::new();

  runtime.set_root(lurq::components::TextInput::new(value.clone()));
  run_pass(&mut runtime);
  let rect = runtime.find_element(|_| true).unwrap().bounds();
  let (x, y) = rect.center();

  runtime.click(x, y, MouseButton::Left);
  runtime.key_down("Backspace".to_owned(), "Backspace".to_owned(), false, false, false);

  assert_eq!(value.get(), "A");
}

#[test]
fn arrow_left_moves_caret_before_inserted_text() {
  let value = Signal::new("AB".to_owned());
  let mut runtime = Tree::new();

  runtime.set_root(lurq::components::TextInput::new(value.clone()));
  run_pass(&mut runtime);
  let rect = runtime.find_element(|_| true).unwrap().bounds();
  let (x, y) = rect.center();

  runtime.click(x, y, MouseButton::Left);
  runtime.key_down("ArrowLeft".to_owned(), "ArrowLeft".to_owned(), false, false, false);
  runtime.key_down("C".to_owned(), "KeyC".to_owned(), false, false, false);

  assert_eq!(value.get(), "ACB");
}

#[test]
fn backspace_removes_previous_unicode_character() {
  let value = Signal::new("Aé".to_owned());
  let mut runtime = Tree::new();

  runtime.set_root(lurq::components::TextInput::new(value.clone()));
  run_pass(&mut runtime);
  let rect = runtime.find_element(|_| true).unwrap().bounds();
  let (x, y) = rect.center();

  runtime.click(x, y, MouseButton::Left);
  runtime.key_down("Backspace".to_owned(), "Backspace".to_owned(), false, false, false);

  assert_eq!(value.get(), "A");
}
