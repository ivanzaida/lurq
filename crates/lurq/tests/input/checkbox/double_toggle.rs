use lurq::{
  app::{Tree, events::MouseButton},
  core::Signal,
};

use crate::support::run_pass;

#[test]
fn double_click_returns_checkbox_signal_to_initial_state() {
  let checked = Signal::new(false);
  let mut runtime = Tree::new();

  runtime.set_root(lurq::components::Checkbox::new(checked.clone()));
  run_pass(&mut runtime);
  let rect = runtime
    .find_element(|_| true)
    .expect("checkbox should be layoutable")
    .bounds();
  let (x, y) = rect.center();

  runtime.click(x, y, MouseButton::Left);
  runtime.click(x, y, MouseButton::Left);

  assert!(!checked.get());
}
