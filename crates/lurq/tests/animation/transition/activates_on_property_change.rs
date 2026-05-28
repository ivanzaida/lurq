use lurq::{
  animation::Transition,
  app::Runtime,
  layout::{Constraints, Size, quad::QuadContent},
  node::color::Color,
};

use crate::support::run_pass;

#[test]
fn transition_activates_when_hovered_style_changes_color() {
  let mut rt = Runtime::new();

  let node = lurq::components::Rect::new(100.0, 50.0)
    .fill("#ff0000")
    .transition(Transition::background_color().duration_ms(1000).linear())
    .hovered(|s| s.fill("#0000ff"));
  rt.set_root(node);

  rt.set_layout_constraints_override(Some(Constraints::loose(Size::new(400.0, 400.0))));
  run_pass(&mut rt);

  let result = rt.last_layout().unwrap();
  let quads = rt.resolve_quads(result);
  assert_eq!(quads.len(), 1);
  let color_before = match &quads[0].content {
    QuadContent::Rect { color } => *color,
    _ => panic!("expected rect"),
  };
  assert_eq!(color_before, Color::from_hex("#ff0000"));

  rt.mouse_move(50.0, 25.0);

  // First frame after hover: transition starts at t=0, so color equals "from" value
  run_pass(&mut rt);

  let result = rt.last_layout().unwrap();
  let quads = rt.resolve_quads(result);
  let color_at_start = match &quads[0].content {
    QuadContent::Rect { color } => *color,
    _ => panic!("expected rect"),
  };
  assert_eq!(
    color_at_start,
    Color::from_hex("#ff0000"),
    "first frame of transition should show 'from' value"
  );

  // Let time elapse so transition progresses
  std::thread::sleep(std::time::Duration::from_millis(50));
  run_pass(&mut rt);

  let result = rt.last_layout().unwrap();
  let quads = rt.resolve_quads(result);
  let color_during = match &quads[0].content {
    QuadContent::Rect { color } => *color,
    _ => panic!("expected rect"),
  };
  assert_ne!(
    color_during,
    Color::from_hex("#ff0000"),
    "color should have started transitioning away from red"
  );
  assert_ne!(
    color_during,
    Color::from_hex("#0000ff"),
    "color should not have reached blue yet"
  );
}
