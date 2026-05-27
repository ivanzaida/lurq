use lurq::{app::Runtime, core::Signal, node::Element};

#[test]
fn empty_value_without_placeholder_is_still_layoutable() {
  let value = Signal::new(String::new());
  let mut runtime = Runtime::new();

  runtime.set_root(Element::text_input(value));

  let root = runtime
    .find_element(|_| true)
    .expect("text input root should be layoutable");
  assert!(root.rect.width >= 0.0);
  assert!(root.rect.height >= 0.0);
}
