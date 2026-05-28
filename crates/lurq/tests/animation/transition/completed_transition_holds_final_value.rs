use lurq::{
  animation::Transition,
  app::Runtime,
  layout::{Constraints, Size, quad::QuadContent},
  node::color::Color,
};

use crate::support::run_pass;

#[test]
fn completed_transition_holds_target_value() {
  let mut rt = Runtime::new();

  let node = lurq::components::Rect::new(100.0, 50.0)
    .fill("#ff0000")
    .transition(Transition::background_color().duration_ms(1).linear())
    .hovered(|s| s.fill("#0000ff"));
  rt.set_root(node);

  rt.set_layout_constraints_override(Some(Constraints::loose(Size::new(400.0, 400.0))));
  run_pass(&mut rt);

  rt.mouse_move(50.0, 25.0);

  std::thread::sleep(std::time::Duration::from_millis(20));
  run_pass(&mut rt);

  std::thread::sleep(std::time::Duration::from_millis(20));
  run_pass(&mut rt);

  let result = rt.last_layout().unwrap();
  let quads = rt.resolve_quads(result);
  let color = match &quads[0].content {
    QuadContent::Rect { color } => *color,
    _ => panic!("expected rect"),
  };
  assert_eq!(
    color,
    Color::from_hex("#0000ff"),
    "color should hold at target after transition completes"
  );
}
