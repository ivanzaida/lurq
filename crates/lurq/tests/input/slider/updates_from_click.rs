use lurq::{
  app::{Runtime, events::MouseButton},
  core::Signal,
  node::Element,
};

#[test]
fn click_updates_slider_signal_from_track_position() {
  let value = Signal::new(0.0_f32);
  let mut runtime = Runtime::new();

  runtime.set_root(Element::slider(value.clone()).range(0.0, 10.0).width(100.0));
  let rect = runtime
    .find_element(|_| true)
    .expect("slider should be layoutable")
    .bounds();

  runtime.click(rect.x + rect.width, rect.y + rect.height / 2.0, MouseButton::Left);

  assert_eq!(value.get(), 10.0);
}
