use lurq::{
  app::{Tree, events::MouseButton},
  core::Signal,
};

use crate::support::{pointer_click, run_pass};

#[test]
fn clicks_outside_slider_track_clamp_to_min_and_max() {
  let value = Signal::new(5);
  let mut runtime = Tree::new();

  runtime.set_root(lurq::components::Slider::new(value.clone()).range(0, 10).width(100.0));
  run_pass(&mut runtime);
  let rect = runtime
    .find_element(|_| true)
    .expect("slider should be layoutable")
    .bounds();

  pointer_click(
    &mut runtime,
    rect.x - 20.0,
    rect.y + rect.height / 2.0,
    MouseButton::Left,
  );
  assert_eq!(value.get(), 0);

  pointer_click(
    &mut runtime,
    rect.x + rect.width + 20.0,
    rect.y + rect.height / 2.0,
    MouseButton::Left,
  );
  assert_eq!(value.get(), 10);
}
