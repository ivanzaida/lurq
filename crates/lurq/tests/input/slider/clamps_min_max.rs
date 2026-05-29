use lurq::{
  app::{Tree, events::MouseButton},
  core::Signal,
};

#[test]
fn clicks_outside_slider_track_clamp_to_min_and_max() {
  let value = Signal::new(5.0_f32);
  let mut runtime = Tree::new();

  runtime.set_root(
    lurq::components::Slider::new(value.clone())
      .range(0.0, 10.0)
      .width(100.0),
  );
  let rect = runtime
    .find_element(|_| true)
    .expect("slider should be layoutable")
    .bounds();

  runtime.click(rect.x - 20.0, rect.y + rect.height / 2.0, MouseButton::Left);
  assert_eq!(value.get(), 0.0);

  runtime.click(
    rect.x + rect.width + 20.0,
    rect.y + rect.height / 2.0,
    MouseButton::Left,
  );
  assert_eq!(value.get(), 10.0);
}
