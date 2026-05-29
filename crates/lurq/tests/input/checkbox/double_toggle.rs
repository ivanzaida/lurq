use lurq::{
  app::{Tree, events::MouseButton},
  core::Signal,
};

#[test]
fn double_click_returns_checkbox_signal_to_initial_state() {
  let checked = Signal::new(false);
  let mut runtime = Tree::new();

  runtime.set_root(lurq::components::Checkbox::new(checked.clone()));
  let rect = runtime
    .find_element(|_| true)
    .expect("checkbox should be layoutable")
    .bounds();
  let (x, y) = rect.center();

  runtime.click(x, y, MouseButton::Left);
  runtime.click(x, y, MouseButton::Left);

  assert!(!checked.get());
}
