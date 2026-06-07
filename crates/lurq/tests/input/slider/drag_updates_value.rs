use lurq::{
  app::{Tree, events::MouseButton},
  core::Signal,
  node::color::Color,
};

use crate::support::{render_pass, run_pass};

#[test]
fn dragging_slider_updates_signal_from_pointer_position() {
  let value = Signal::new(0);
  let mut runtime = Tree::new();

  runtime.set_root(lurq::components::Slider::new(value.clone()).range(0, 10).width(100.0));
  run_pass(&mut runtime);
  let rect = runtime.find_element(|_| true).unwrap().bounds();
  let y = rect.y + rect.height / 2.0;

  runtime.mouse_down(rect.x, y, MouseButton::Left);
  runtime.mouse_move(rect.x + 75.0, y);
  runtime.mouse_up(rect.x + 75.0, y, MouseButton::Left);

  assert_eq!(value.get(), 8);
}

#[test]
fn dragging_slider_thumb_tracks_pointer_between_integer_steps() {
  let value = Signal::new(0);
  let mut runtime = Tree::new();

  runtime.set_root(
    lurq::components::Slider::new(value.clone())
      .range(0, 2)
      .width(100.0)
      .height(20.0)
      .track(|style| style.size(100.0, 4.0).background("#64748b"))
      .thumb(|style| style.size(10.0, 10.0).background("#f97316")),
  );
  let snapshot = render_pass(&mut runtime);
  let thumb = snapshot
    .rects
    .iter()
    .find(|rect| rect.color == Color::from_hex("#f97316"))
    .expect("thumb should render");
  let y = thumb.y + thumb.height / 2.0;
  let start_x = thumb.x + thumb.width / 2.0;
  let pointer_x = start_x + 22.5;

  runtime.mouse_down(start_x, y, MouseButton::Left);
  runtime.mouse_move(pointer_x, y);

  assert_eq!(value.get(), 1);
  let snapshot = render_pass(&mut runtime);
  let thumb = snapshot
    .rects
    .iter()
    .find(|rect| rect.color == Color::from_hex("#f97316"))
    .expect("thumb should render while dragging");
  let thumb_center = thumb.x + thumb.width / 2.0;
  assert!(
    (thumb_center - pointer_x).abs() <= 0.5,
    "dragging thumb should render under pointer; thumb_center={thumb_center}, pointer_x={pointer_x}"
  );

  runtime.mouse_up(pointer_x, y, MouseButton::Left);
}
