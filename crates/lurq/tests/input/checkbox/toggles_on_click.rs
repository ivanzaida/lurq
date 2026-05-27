use lurq::{
  app::{Runtime, events::MouseButton},
  core::Signal,
};

#[test]
fn click_toggles_checkbox_signal() {
  let checked = Signal::new(false);
  let mut runtime = Runtime::new();

  runtime.set_root(lurq::components::Checkbox::new(checked.clone()));
  let rect = runtime
    .find_element(|_| true)
    .expect("checkbox should be layoutable")
    .bounds();
  let (x, y) = rect.center();

  runtime.click(x, y, MouseButton::Left);

  assert!(checked.get());
}
