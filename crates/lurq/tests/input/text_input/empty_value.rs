use lurq::{app::Runtime, core::Signal, node::Element};

#[test]
fn empty_value_without_placeholder_is_still_layoutable() {
  let value = Signal::new(String::new());
  let mut runtime = Runtime::new();

  runtime.set_root(Element::text_input(value));

  let root = runtime
    .find_element(|_| true)
    .expect("text input root should be layoutable");
  let rect = root.bounds();
  assert!(rect.width >= 0.0);
  assert!(rect.height >= 0.0);
}
