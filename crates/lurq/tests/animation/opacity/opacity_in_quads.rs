use lurq::{
  app::Tree,
  layout::{Constraints, Size},
};

use crate::support::run_pass;

#[test]
fn explicit_opacity_propagates_to_quad() {
  let mut rt = Tree::new();
  let node = lurq::components::Rect::new(100.0, 50.0)
    .background("#ff0000")
    .opacity(0.5);
  rt.set_root(node);

  rt.set_layout_constraints_override(Some(Constraints::loose(Size::new(400.0, 400.0))));
  run_pass(&mut rt);

  let result = rt.last_layout().unwrap();
  let quads = rt.resolve_quads(result);
  assert_eq!(quads[0].opacity, 0.5);
}

#[test]
fn container_opacity_composes_into_descendants() {
  // Fading a group must fade what is inside it, multiplicatively — the same
  // inheritance contract transforms follow.
  let mut rt = Tree::new();
  let node = lurq::components::Column::new()
    .opacity(0.4)
    .child(lurq::components::Rect::new(100.0, 50.0).background("#ff0000"))
    .child(
      lurq::components::Rect::new(100.0, 50.0)
        .background("#00ff00")
        .opacity(0.5),
    );
  rt.set_root(node);

  rt.set_layout_constraints_override(Some(Constraints::loose(Size::new(400.0, 400.0))));
  run_pass(&mut rt);

  let result = rt.last_layout().unwrap();
  let quads = rt.resolve_quads(result);
  let mut opacities: Vec<f32> = quads.iter().map(|quad| quad.opacity).collect();
  opacities.sort_by(f32::total_cmp);
  assert_eq!(opacities, vec![0.2, 0.4], "0.4 inherited, 0.4 × 0.5 composed");
}

#[test]
fn a_plain_wrapper_passes_inherited_opacity_through() {
  // A logical wrapper takes the fast path in quad collection; the inherited
  // opacity must survive it.
  let mut rt = Tree::new();
  let node = lurq::components::Column::new().opacity(0.25).child(
    lurq::components::Column::new()
      .child(lurq::components::Rect::new(100.0, 50.0).background("#ff0000")),
  );
  rt.set_root(node);

  rt.set_layout_constraints_override(Some(Constraints::loose(Size::new(400.0, 400.0))));
  run_pass(&mut rt);

  let result = rt.last_layout().unwrap();
  let quads = rt.resolve_quads(result);
  assert!(!quads.is_empty());
  assert!(quads.iter().all(|quad| quad.opacity == 0.25));
}

#[test]
fn zero_opacity_propagates_to_quad() {
  let mut rt = Tree::new();
  let node = lurq::components::Rect::new(100.0, 50.0)
    .background("#ff0000")
    .opacity(0.0);
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
  let node = lurq::components::Rect::new(100.0, 50.0)
    .background("#ff0000")
    .opacity(0.1);
  rt.set_root(node);

  rt.set_layout_constraints_override(Some(Constraints::loose(Size::new(400.0, 400.0))));
  run_pass(&mut rt);

  let result = rt.last_layout().unwrap();
  assert_eq!(result.size.width, 100.0);
  assert_eq!(result.size.height, 50.0);
}
