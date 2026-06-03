use lurq::{
  animation::Transition,
  app::Tree,
  layout::{Constraints, Size, quad::QuadContent},
  node::color::Color,
};

use crate::support::run_pass;

#[test]
fn multiple_transitions_on_same_node() {
  let mut rt = Tree::new();

  let node = lurq::components::Rect::new(100.0, 50.0)
    .background("#ff0000")
    .rounded(0.0)
    .transition(Transition::background_color().duration_ms(1000).linear())
    .transition(Transition::border_radius_top_left().duration_ms(1000).linear())
    .hovered(|s| s.background("#0000ff").rounded(20.0));
  rt.set_root(node);

  rt.set_layout_constraints_override(Some(Constraints::loose(Size::new(400.0, 400.0))));
  run_pass(&mut rt);

  rt.mouse_move(50.0, 25.0);
  run_pass(&mut rt);

  std::thread::sleep(std::time::Duration::from_millis(50));
  run_pass(&mut rt);

  let result = rt.last_layout().unwrap();
  let quads = rt.resolve_quads(result);
  assert!(!quads.is_empty());

  let color = match &quads[0].content {
    QuadContent::Rect { color } => *color,
    _ => panic!("expected rect"),
  };
  assert_ne!(color, Color::from_hex("#ff0000"), "color should be transitioning");
  assert_ne!(color, Color::from_hex("#0000ff"), "color should not be at target yet");
}
