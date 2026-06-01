use lurq::{
  app::{Tree, events::MouseButton},
  core::Signal,
  node::color::Color,
};

use crate::support::{render_pass, run_pass};

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

#[test]
fn shift_arrow_selection_is_replaced_by_inserted_text() {
  let value = Signal::new("AB".to_owned());
  let mut runtime = Tree::new();

  runtime.set_root(lurq::components::TextInput::new(value.clone()));
  run_pass(&mut runtime);
  let rect = runtime.find_element(|_| true).unwrap().bounds();
  let (x, y) = rect.center();

  runtime.click(x, y, MouseButton::Left);
  runtime.key_down("ArrowLeft".to_owned(), "ArrowLeft".to_owned(), true, false, false);
  runtime.key_down("C".to_owned(), "KeyC".to_owned(), false, false, false);

  assert_eq!(value.get(), "AC");
}

#[test]
fn ctrl_a_selects_all_text_for_replacement() {
  let value = Signal::new("Hello".to_owned());
  let mut runtime = Tree::new();

  runtime.set_root(lurq::components::TextInput::new(value.clone()));
  run_pass(&mut runtime);
  let rect = runtime.find_element(|_| true).unwrap().bounds();
  let (x, y) = rect.center();

  runtime.click(x, y, MouseButton::Left);
  runtime.key_down("a".to_owned(), "KeyA".to_owned(), false, true, false);
  runtime.key_down("X".to_owned(), "KeyX".to_owned(), false, false, false);

  assert_eq!(value.get(), "X");
}

#[test]
fn enter_is_ignored_for_scroll_overflow_text_input() {
  let value = Signal::new("A".to_owned());
  let mut runtime = Tree::new();

  runtime.set_root(lurq::components::TextInput::new(value.clone()));
  run_pass(&mut runtime);
  let rect = runtime.find_element(|_| true).unwrap().bounds();
  let (x, y) = rect.center();

  runtime.click(x, y, MouseButton::Left);
  runtime.key_down("Enter".to_owned(), "Enter".to_owned(), false, false, false);

  assert_eq!(value.get(), "A");
}

#[test]
fn mouse_drag_selection_renders_and_replaces_selected_text() {
  let value = Signal::new("Hello".to_owned());
  let mut runtime = Tree::new();

  runtime.set_root(lurq::components::TextInput::new(value.clone()));
  run_pass(&mut runtime);
  let rect = runtime.find_element(|_| true).unwrap().bounds();
  let y = rect.y + rect.height / 2.0;

  runtime.mouse_down(rect.x, y, MouseButton::Left);
  runtime.mouse_move(rect.x + rect.width, y);
  runtime.mouse_up(rect.x + rect.width, y, MouseButton::Left);
  runtime.click(rect.x + rect.width, y, MouseButton::Left);

  let snapshot = render_pass(&mut runtime);
  assert!(
    snapshot
      .rects
      .iter()
      .any(|rect| { rect.color == Color::from_hex("#bfdbfe") && rect.width > 1.0 && rect.height > 0.0 })
  );

  runtime.key_down("X".to_owned(), "KeyX".to_owned(), false, false, false);

  assert_eq!(value.get(), "X");
}
