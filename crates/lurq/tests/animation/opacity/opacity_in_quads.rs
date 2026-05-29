use lurq::{
  app::Tree,
  layout::{Constraints, Size},
};

use crate::support::run_pass;

#[test]
fn explicit_opacity_propagates_to_quad() {
  let mut rt = Tree::new();
  let node = lurq::components::Rect::new(100.0, 50.0).fill("#ff0000").opacity(0.5);
  rt.set_root(node);

  rt.set_layout_constraints_override(Some(Constraints::loose(Size::new(400.0, 400.0))));
  run_pass(&mut rt);

  let result = rt.last_layout().unwrap();
  let quads = rt.resolve_quads(result);
  assert_eq!(quads[0].opacity, 0.5);
}

#[test]
fn zero_opacity_propagates_to_quad() {
  let mut rt = Tree::new();
  let node = lurq::components::Rect::new(100.0, 50.0).fill("#ff0000").opacity(0.0);
  rt.set_root(node);

  rt.set_layout_constraints_override(Some(Constraints::loose(Size::new(400.0, 400.0))));
  run_pass(&mut rt);

  let result = rt.last_layout().unwrap();
  let quads = rt.resolve_quads(result);
  assert_eq!(quads[0].opacity, 0.0);
}

#[test]
fn opacity_does_not_affect_layout_size() {
  let mut rt = Tree::new();
  let node = lurq::components::Rect::new(100.0, 50.0).fill("#ff0000").opacity(0.1);
  rt.set_root(node);

  rt.set_layout_constraints_override(Some(Constraints::loose(Size::new(400.0, 400.0))));
  run_pass(&mut rt);

  let result = rt.last_layout().unwrap();
  assert_eq!(result.size.width, 100.0);
  assert_eq!(result.size.height, 50.0);
}
