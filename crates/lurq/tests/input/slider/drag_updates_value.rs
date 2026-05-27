use lurq::{
  app::{Runtime, events::MouseButton},
  core::Signal,
  node::Element,
};

#[test]
fn dragging_slider_updates_signal_from_pointer_position() {
  let value = Signal::new(0.0);
  let mut runtime = Runtime::new();

  runtime.set_root(Element::slider(value.clone()).range(0.0, 10.0).width(100.0));
  let rect = runtime.find_element(|_| true).unwrap().rect;
  let y = rect.y + rect.height / 2.0;

  runtime.mouse_down(rect.x, y, MouseButton::Left);
  runtime.mouse_move(rect.x + 75.0, y);
  runtime.mouse_up(rect.x + 75.0, y, MouseButton::Left);

  assert!((value.get() - 7.5).abs() < f32::EPSILON);
}
