use lurq::{
  animation::{Easing, Transition},
  app::Runtime,
  layout::{Constraints, Size, quad::QuadContent},
};

use crate::support::run_pass;

#[test]
fn linear_easing_produces_proportional_interpolation() {
  let mut rt = Runtime::new();

  let node = lurq::components::Rect::new(100.0, 50.0)
    .fill("#000000")
    .transition(Transition::background_color().duration_ms(1000).linear())
    .hovered(|s| s.fill("#ffffff"));
  rt.set_root(node);

  rt.set_layout_constraints_override(Some(Constraints::loose(Size::new(400.0, 400.0))));
  run_pass(&mut rt);

  rt.mouse_move(50.0, 25.0);
  run_pass(&mut rt);
  run_pass(&mut rt);

  let result = rt.last_layout().unwrap();
  let quads = rt.resolve_quads(result);
  let color = match &quads[0].content {
    QuadContent::Rect { color } => *color,
    _ => panic!("expected rect"),
  };

  assert!(color.r() < 200, "should not have reached white yet");
}

#[test]
fn default_easing_is_ease() {
  let t = Transition::background_color();
  assert_eq!(t.easing, Easing::EASE);
}

#[test]
fn linear_convenience_sets_linear_easing() {
  let t = Transition::background_color().linear();
  assert_eq!(t.easing, Easing::Linear);
}
