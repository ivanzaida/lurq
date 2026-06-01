use lurq::{
  app::{Tree, events::MouseButton},
  core::Signal,
};

use crate::support::run_pass;

#[test]
fn arrow_keys_update_focused_slider_within_range() {
  let value = Signal::new(5);
  let mut runtime = Tree::new();

  runtime.set_root(lurq::components::Slider::new(value.clone()).range(0, 10).width(100.0));
  run_pass(&mut runtime);
  let rect = runtime.find_element(|_| true).unwrap().bounds();
  let (x, y) = rect.center();

  runtime.click(x, y, MouseButton::Left);
  runtime.key_down("ArrowRight".to_owned(), "ArrowRight".to_owned(), false, false, false);
  assert_eq!(value.get(), 6);

  runtime.key_down("ArrowLeft".to_owned(), "ArrowLeft".to_owned(), false, false, false);
  assert_eq!(value.get(), 5);
}
