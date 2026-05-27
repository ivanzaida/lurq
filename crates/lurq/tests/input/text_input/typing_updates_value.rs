use lurq::{
  app::{Runtime, events::MouseButton},
  core::Signal,
  node::Element,
};

#[test]
fn focused_text_input_appends_key_down_text_to_signal() {
  let value = Signal::new(String::new());
  let mut runtime = Runtime::new();

  runtime.set_root(Element::text_input(value.clone()).placeholder("Name"));
  let rect = runtime
    .find_element(|_| true)
    .expect("text input should be layoutable")
    .bounds();
  let (x, y) = rect.center();

  runtime.click(x, y, MouseButton::Left);
  runtime.key_down("A".to_owned(), "KeyA".to_owned(), false, false, false);

  assert_eq!(value.get(), "A");
}

#[test]
fn displayed_text_updates_after_typing() {
  let value = Signal::new(String::new());
  let mut runtime = Runtime::new();

  runtime.set_root(Element::text_input(value.clone()).placeholder("Name"));
  let rect = runtime.find_element(|_| true).unwrap().bounds();
  let (x, y) = rect.center();

  runtime.click(x, y, MouseButton::Left);
  runtime.key_down("A".to_owned(), "KeyA".to_owned(), false, false, false);

  assert!(runtime.find_element(|el| el.text_content() == Some("A")).is_some());
}
