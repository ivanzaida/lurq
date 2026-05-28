use lurq::{
  animation::Transition,
  app::Runtime,
  layout::{Constraints, Size, quad::QuadContent},
  node::color::Color,
};

use crate::support::run_pass;

#[test]
fn transition_all_applies_to_background_color() {
  let mut rt = Runtime::new();

  let node = lurq::components::Rect::new(100.0, 50.0)
    .fill("#ff0000")
    .transition(Transition::all().duration_ms(1000).linear())
    .hovered(|s| s.fill("#0000ff"));
  rt.set_root(node);

  rt.set_layout_constraints_override(Some(Constraints::loose(Size::new(400.0, 400.0))));
  run_pass(&mut rt);

  rt.mouse_move(50.0, 25.0);
  run_pass(&mut rt);

  std::thread::sleep(std::time::Duration::from_millis(50));
  run_pass(&mut rt);

  let result = rt.last_layout().unwrap();
  let quads = rt.resolve_quads(result);
  let color = match &quads[0].content {
    QuadContent::Rect { color } => *color,
    _ => panic!("expected rect"),
  };
  assert_ne!(color, Color::from_hex("#ff0000"), "transition all should animate color");
  assert_ne!(color, Color::from_hex("#0000ff"), "should not be at target yet");
}
