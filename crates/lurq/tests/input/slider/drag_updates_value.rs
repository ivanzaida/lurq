use lurq::{
  app::{Tree, events::MouseButton},
  core::Signal,
};

use crate::support::run_pass;

#[test]
fn dragging_slider_updates_signal_from_pointer_position() {
  let value = Signal::new(0.0);
  let mut runtime = Tree::new();

  runtime.set_root(
    lurq::components::Slider::new(value.clone())
      .range(0.0, 10.0)
      .width(100.0),
  );
  run_pass(&mut runtime);
  let rect = runtime.find_element(|_| true).unwrap().bounds();
  let y = rect.y + rect.height / 2.0;

  runtime.mouse_down(rect.x, y, MouseButton::Left);
  runtime.mouse_move(rect.x + 75.0, y);
  runtime.mouse_up(rect.x + 75.0, y, MouseButton::Left);

  assert!((value.get() - 7.5).abs() < f32::EPSILON);
}
