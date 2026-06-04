use lurq::{
  app::{Tree, events::MouseButton},
  core::Signal,
};

use crate::support::{pointer_click, run_pass};

#[test]
fn space_toggles_focused_checkbox() {
  let checked = Signal::new(false);
  let mut runtime = Tree::new();

  runtime.set_root(lurq::components::Checkbox::new(checked.clone()));
  run_pass(&mut runtime);
  let rect = runtime.find_element(|_| true).unwrap().bounds();
  let (x, y) = rect.center();

  pointer_click(&mut runtime, x, y, MouseButton::Left);
  assert!(checked.get());

  runtime.key_down(" ".to_owned(), "Space".to_owned(), false, false, false);
  assert!(!checked.get());
}
