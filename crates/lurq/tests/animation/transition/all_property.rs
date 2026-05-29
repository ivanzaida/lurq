use lurq::{
  animation::Transition,
  app::Tree,
  layout::{Constraints, Size, quad::QuadContent},
  node::color::Color,
};

use crate::support::{render_pass, run_pass};

#[test]
fn transition_all_applies_to_background_color() {
  let mut rt = Tree::new();

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

#[test]
fn transition_all_width_stays_at_hover_target_after_completion() {
  let mut rt = Tree::new();

  let node = lurq::components::Rect::new(120.0, 32.0)
    .fill("#3b82f6")
    .transition(Transition::all().duration_ms(5).linear())
    .hovered(|s| s.size(240.0, 32.0));
  rt.set_root(node);

  rt.set_layout_constraints_override(Some(Constraints::loose(Size::new(400.0, 400.0))));
  run_pass(&mut rt);

  rt.mouse_move(60.0, 16.0);
  let start = render_pass(&mut rt).rects[0].width;

  std::thread::sleep(std::time::Duration::from_millis(10));
  let completed = render_pass(&mut rt).rects[0].width;

  std::thread::sleep(std::time::Duration::from_millis(10));
  let held = render_pass(&mut rt).rects[0].width;

  assert_eq!(start, 120.0);
  assert_eq!(completed, 240.0);
  assert_eq!(held, 240.0);
}

#[test]
fn transition_all_width_reverses_to_base_after_hover_leave() {
  let mut rt = Tree::new();

  let node = lurq::components::Rect::new(120.0, 32.0)
    .fill("#3b82f6")
    .transition(Transition::all().duration_ms(5).linear())
    .hovered(|s| s.size(240.0, 32.0));
  rt.set_root(node);

  rt.set_layout_constraints_override(Some(Constraints::loose(Size::new(400.0, 400.0))));
  run_pass(&mut rt);

  rt.mouse_move(60.0, 16.0);
  render_pass(&mut rt);

  std::thread::sleep(std::time::Duration::from_millis(10));
  let hovered = render_pass(&mut rt).rects[0].width;

  rt.mouse_move(300.0, 16.0);
  let reverse_start = render_pass(&mut rt).rects[0].width;

  std::thread::sleep(std::time::Duration::from_millis(10));
  let completed = render_pass(&mut rt).rects[0].width;

  std::thread::sleep(std::time::Duration::from_millis(10));
  let held = render_pass(&mut rt).rects[0].width;

  assert_eq!(hovered, 240.0);
  assert_eq!(reverse_start, 240.0);
  assert_eq!(completed, 120.0);
  assert_eq!(held, 120.0);
}

#[test]
fn overlapping_transition_specs_do_not_restart_width_after_reverse() {
  let mut rt = Tree::new();

  let node = lurq::components::Rect::new(120.0, 32.0)
    .fill("#3b82f6")
    .transition(Transition::background_color().duration_ms(5).linear())
    .transition(Transition::all().duration_ms(5).linear())
    .hovered(|s| s.fill("#22c55e").size(240.0, 32.0));
  rt.set_root(node);

  rt.set_layout_constraints_override(Some(Constraints::loose(Size::new(400.0, 400.0))));
  run_pass(&mut rt);

  rt.mouse_move(60.0, 16.0);
  render_pass(&mut rt);
  std::thread::sleep(std::time::Duration::from_millis(10));
  assert_eq!(render_pass(&mut rt).rects[0].width, 240.0);

  rt.mouse_move(300.0, 16.0);
  render_pass(&mut rt);
  std::thread::sleep(std::time::Duration::from_millis(10));
  assert_eq!(render_pass(&mut rt).rects[0].width, 120.0);

  std::thread::sleep(std::time::Duration::from_millis(10));
  assert_eq!(render_pass(&mut rt).rects[0].width, 120.0);
}

#[test]
fn transition_state_is_cleared_when_root_is_replaced() {
  let mut rt = Tree::new();
  rt.set_layout_constraints_override(Some(Constraints::loose(Size::new(400.0, 400.0))));

  let first = lurq::components::Rect::new(120.0, 32.0)
    .fill("#3b82f6")
    .transition(Transition::all().duration_ms(5).linear())
    .hovered(|s| s.size(240.0, 32.0));
  rt.set_root(first);
  run_pass(&mut rt);
  rt.mouse_move(60.0, 16.0);
  render_pass(&mut rt);
  std::thread::sleep(std::time::Duration::from_millis(10));
  assert_eq!(render_pass(&mut rt).rects[0].width, 240.0);

  let second = lurq::components::Rect::new(120.0, 32.0)
    .fill("#3b82f6")
    .transition(Transition::all().duration_ms(5).linear())
    .hovered(|s| s.size(240.0, 32.0));
  rt.set_root(second);

  assert_eq!(render_pass(&mut rt).rects[0].width, 120.0);
}
