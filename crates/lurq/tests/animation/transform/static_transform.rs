use lurq::{
  app::Tree,
  layout::{Constraints, Size},
  node::transform::Transform2D,
};

use crate::support::run_pass;

fn rt() -> Tree {
  Tree::new()
}

#[test]
fn identity_transform_by_default() {
  let mut rt = rt();
  rt.set_root(lurq::components::Rect::new(60.0, 40.0).background("#ff0000"));
  rt.set_layout_constraints_override(Some(Constraints::loose(Size::new(400.0, 400.0))));
  run_pass(&mut rt);

  let result = rt.last_layout().unwrap();
  let quads = rt.resolve_quads(result);
  assert_eq!(quads[0].transform, Transform2D::IDENTITY);
}

#[test]
fn rotate_propagates_to_quad() {
  let mut rt = rt();
  rt.set_root(
    lurq::components::Rect::new(60.0, 40.0)
      .background("#ff0000")
      .transform(Transform2D::rotate_deg(45.0)),
  );
  rt.set_layout_constraints_override(Some(Constraints::loose(Size::new(400.0, 400.0))));
  run_pass(&mut rt);

  let result = rt.last_layout().unwrap();
  let quads = rt.resolve_quads(result);
  assert_ne!(quads[0].transform, Transform2D::IDENTITY);
  let expected = Transform2D::rotate_deg(45.0);
  assert!((quads[0].transform.a - expected.a).abs() < 0.001);
  assert!((quads[0].transform.b - expected.b).abs() < 0.001);
}

#[test]
fn scale_propagates_to_quad() {
  let mut rt = rt();
  rt.set_root(
    lurq::components::Rect::new(60.0, 40.0)
      .background("#ff0000")
      .transform(Transform2D::scale(2.0, 0.5)),
  );
  rt.set_layout_constraints_override(Some(Constraints::loose(Size::new(400.0, 400.0))));
  run_pass(&mut rt);

  let result = rt.last_layout().unwrap();
  let quads = rt.resolve_quads(result);
  assert!((quads[0].transform.a - 2.0).abs() < 0.001);
  assert!((quads[0].transform.d - 0.5).abs() < 0.001);
}

#[test]
fn transform_does_not_affect_layout_size() {
  let mut rt = rt();
  rt.set_root(
    lurq::components::Rect::new(60.0, 40.0)
      .background("#ff0000")
      .transform(Transform2D::scale(3.0, 3.0)),
  );
  rt.set_layout_constraints_override(Some(Constraints::loose(Size::new(400.0, 400.0))));
  run_pass(&mut rt);

  let result = rt.last_layout().unwrap();
  assert_eq!(result.size.width, 60.0);
  assert_eq!(result.size.height, 40.0);
}
