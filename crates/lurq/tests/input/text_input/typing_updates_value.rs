use lurq::{
  app::{Tree, events::MouseButton},
  core::Signal,
};

use crate::support::{pointer_click, run_pass};

#[test]
fn focused_text_input_appends_key_down_text_to_signal() {
  let value = Signal::new(String::new());
  let mut runtime = Tree::new();

  runtime.set_root(lurq::components::TextInput::new(value.clone()).placeholder("Name"));
  run_pass(&mut runtime);
  let rect = runtime
    .find_element(|_| true)
    .expect("text input should be layoutable")
    .bounds();
  let (x, y) = rect.center();

  pointer_click(&mut runtime, x, y, MouseButton::Left);
  runtime.key_down("A".to_owned(), "KeyA".to_owned(), false, false, false);

  assert_eq!(value.get(), "A");
}

#[test]
fn key_down_capture_can_prevent_text_input_editing() {
  let value = Signal::new(String::new());
  let mut runtime = Tree::new();

  runtime.set_root(
    lurq::components::TextInput::new(value.clone())
      .placeholder("Name")
      .on_key_down_capture(|event| event.key == "A"),
  );
  run_pass(&mut runtime);
  let rect = runtime
    .find_element(|_| true)
    .expect("text input should be layoutable")
    .bounds();
  let (x, y) = rect.center();

  pointer_click(&mut runtime, x, y, MouseButton::Left);
  runtime.key_down("A".to_owned(), "KeyA".to_owned(), false, false, false);
  runtime.key_down("B".to_owned(), "KeyB".to_owned(), false, false, false);

  assert_eq!(value.get(), "B");
}

#[test]
fn key_down_prevent_default_blocks_text_input_editing() {
  let value = Signal::new(String::new());
  let mut runtime = Tree::new();

  runtime.set_root(
    lurq::components::TextInput::new(value.clone())
      .placeholder("Name")
      .on_key_down(|event| {
        if event.key == "A" {
          event.prevent_default();
        }
      }),
  );
  run_pass(&mut runtime);
  let rect = runtime
    .find_element(|_| true)
    .expect("text input should be layoutable")
    .bounds();
  let (x, y) = rect.center();

  pointer_click(&mut runtime, x, y, MouseButton::Left);
  runtime.key_down("A".to_owned(), "KeyA".to_owned(), false, false, false);
  runtime.key_down("B".to_owned(), "KeyB".to_owned(), false, false, false);

  assert_eq!(value.get(), "B");
}

#[test]
fn external_value_change_keeps_end_caret_at_new_end() {
  let value = Signal::new("/pla".to_owned());
  let fill = value.clone();
  let mut runtime = Tree::new();

  runtime.set_root(
    lurq::components::TextInput::new(value.clone())
      .placeholder("Name")
      .on_key_down_capture(move |event| {
        if event.key == "Tab" {
          fill.set("/play ".to_owned());
          return true;
        }
        false
      }),
  );
  run_pass(&mut runtime);
  let rect = runtime
    .find_element(|_| true)
    .expect("text input should be layoutable")
    .bounds();
  let (x, y) = rect.center();

  pointer_click(&mut runtime, x, y, MouseButton::Left);
  runtime.key_down("Tab".to_owned(), "Tab".to_owned(), false, false, false);
  run_pass(&mut runtime);
  runtime.key_down("1".to_owned(), "Digit1".to_owned(), false, false, false);

  assert_eq!(value.get(), "/play 1");
}

#[test]
fn displayed_text_updates_after_typing() {
  let value = Signal::new(String::new());
  let mut runtime = Tree::new();

  runtime.set_root(lurq::components::TextInput::new(value.clone()).placeholder("Name"));
  run_pass(&mut runtime);
  let rect = runtime.find_element(|_| true).unwrap().bounds();
  let (x, y) = rect.center();

  pointer_click(&mut runtime, x, y, MouseButton::Left);
  runtime.key_down("A".to_owned(), "KeyA".to_owned(), false, false, false);
  run_pass(&mut runtime);

  assert!(runtime.find_element(|el| el.text_content() == Some("A")).is_some());
}
