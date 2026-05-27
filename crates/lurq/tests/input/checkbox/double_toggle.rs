use lurq::{
  app::{Runtime, events::MouseButton},
  core::Signal,
  node::Element,
};

#[test]
fn double_click_returns_checkbox_signal_to_initial_state() {
  let checked = Signal::new(false);
  let mut runtime = Runtime::new();

  runtime.set_root(Element::checkbox(checked.clone()));
  let rect = runtime
    .find_element(|_| true)
    .expect("checkbox should be layoutable")
    .rect;
  let (x, y) = rect.center();

  runtime.click(x, y, MouseButton::Left);
  runtime.click(x, y, MouseButton::Left);

  assert!(!checked.get());
}
