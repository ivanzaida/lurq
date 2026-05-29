use lurq::{
  app::{Tree, events::MouseButton},
  core::Signal,
};

#[test]
fn arrow_keys_update_focused_slider_within_range() {
  let value = Signal::new(5.0_f32);
  let mut runtime = Tree::new();

  runtime.set_root(
    lurq::components::Slider::new(value.clone())
      .range(0.0, 10.0)
      .width(100.0),
  );
  let rect = runtime.find_element(|_| true).unwrap().bounds();
  let (x, y) = rect.center();

  runtime.click(x, y, MouseButton::Left);
  runtime.key_down("ArrowRight".to_owned(), "ArrowRight".to_owned(), false, false, false);
  assert_eq!(value.get(), 6.0);

  runtime.key_down("ArrowLeft".to_owned(), "ArrowLeft".to_owned(), false, false, false);
  assert_eq!(value.get(), 5.0);
}
