use lurq::{
  app::{Tree, events::MouseButton},
  core::Signal,
};

use crate::support::{pointer_click, run_pass};

#[test]
fn click_updates_slider_signal_from_track_position() {
  let value = Signal::new(0);
  let mut runtime = Tree::new();

  runtime.set_root(lurq::components::Slider::new(value.clone()).range(0, 10).width(100.0));
  run_pass(&mut runtime);
  let rect = runtime
    .find_element(|_| true)
    .expect("slider should be layoutable")
    .bounds();

  pointer_click(
    &mut runtime,
    rect.x + rect.width,
    rect.y + rect.height / 2.0,
    MouseButton::Left,
  );

  assert_eq!(value.get(), 10);
}
