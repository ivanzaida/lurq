use lurq::{
  app::{Tree, events::MouseButton},
  core::Signal,
};

use crate::support::{render_pass, run_pass};

#[test]
fn renders_caret_after_text_input_is_focused() {
  let value = Signal::new("A".to_owned());
  let mut runtime = Tree::new();

  runtime.set_root(lurq::components::TextInput::new(value));
  run_pass(&mut runtime);
  let rect = runtime.find_element(|_| true).unwrap().bounds();
  let (x, y) = rect.center();

  runtime.click(x, y, MouseButton::Left);
  let snapshot = render_pass(&mut runtime);

  assert!(snapshot.rects.iter().any(|rect| rect.width == 1.0 && rect.height > 0.0));
}
