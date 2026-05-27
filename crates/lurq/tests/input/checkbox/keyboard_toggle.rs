use lurq::{
  app::{Runtime, events::MouseButton},
  core::Signal,
};

#[test]
fn space_toggles_focused_checkbox() {
  let checked = Signal::new(false);
  let mut runtime = Runtime::new();

  runtime.set_root(lurq::components::Checkbox::new(checked.clone()));
  let rect = runtime.find_element(|_| true).unwrap().bounds();
  let (x, y) = rect.center();

  runtime.click(x, y, MouseButton::Left);
  assert!(checked.get());

  runtime.key_down(" ".to_owned(), "Space".to_owned(), false, false, false);
  assert!(!checked.get());
}
