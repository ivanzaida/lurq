use lurq::{
  app::Runtime,
  layout::{Alignment, Constraints, Size, quad::QuadContent},
  node::{color::Color, transform::Transform2D},
};

use crate::support::run_pass;

fn rt() -> Runtime {
  Runtime::new()
}

#[test]
fn child_rect_inherits_parent_rotation() {
  let mut rt = rt();
  let node = lurq::components::Stack::new()
    .child(lurq::components::Rect::new(30.0, 30.0).fill("#00ff00"))
    .size(60.0, 60.0)
    .fill("#ff0000")
    .transform(Transform2D::rotate_deg(90.0));
  rt.set_root(node);
  rt.set_layout_constraints_override(Some(Constraints::loose(Size::new(400.0, 400.0))));
  run_pass(&mut rt);

  let result = rt.last_layout().unwrap();
  let quads = rt.resolve_quads(result);

  let parent_quad = quads
    .iter()
    .find(|q| match &q.content {
      QuadContent::Rect { color } => *color == Color::from_hex("#ff0000"),
      _ => false,
    })
    .expect("parent rect");

  let child_quad = quads
    .iter()
    .find(|q| match &q.content {
      QuadContent::Rect { color } => *color == Color::from_hex("#00ff00"),
      _ => false,
    })
    .expect("child rect");

  let expected = Transform2D::rotate_deg(90.0);
  assert!(
    (parent_quad.transform.b - expected.b).abs() < 0.01,
    "parent should be rotated 90°"
  );
  assert!(
    (child_quad.transform.b - expected.b).abs() < 0.01,
    "child should inherit parent's 90° rotation, got transform: {:?}",
    child_quad.transform
  );
}

#[test]
fn child_with_own_transform_composes_with_parent() {
  let mut rt = rt();
  let node = lurq::components::Stack::new()
    .child(
      lurq::components::Rect::new(30.0, 30.0)
        .fill("#00ff00")
        .transform(Transform2D::scale(2.0, 2.0)),
    )
    .size(60.0, 60.0)
    .fill("#ff0000")
    .transform(Transform2D::rotate_deg(90.0));
  rt.set_root(node);
  rt.set_layout_constraints_override(Some(Constraints::loose(Size::new(400.0, 400.0))));
  run_pass(&mut rt);

  let result = rt.last_layout().unwrap();
  let quads = rt.resolve_quads(result);

  let child_quad = quads
    .iter()
    .find(|q| match &q.content {
      QuadContent::Rect { color } => *color == Color::from_hex("#00ff00"),
      _ => false,
    })
    .expect("child rect");

  let composed = Transform2D::rotate_deg(90.0).then(&Transform2D::scale(2.0, 2.0));
  assert!(
    (child_quad.transform.a - composed.a).abs() < 0.01
      && (child_quad.transform.b - composed.b).abs() < 0.01
      && (child_quad.transform.c - composed.c).abs() < 0.01
      && (child_quad.transform.d - composed.d).abs() < 0.01,
    "child transform should be parent * child composition.\nexpected: {:?}\ngot: {:?}",
    composed,
    child_quad.transform
  );
}

#[test]
fn untransformed_child_of_untransformed_parent_stays_identity() {
  let mut rt = rt();
  let node = lurq::components::Stack::new()
    .child(lurq::components::Rect::new(30.0, 30.0).fill("#00ff00"))
    .size(60.0, 60.0)
    .fill("#ff0000");
  rt.set_root(node);
  rt.set_layout_constraints_override(Some(Constraints::loose(Size::new(400.0, 400.0))));
  run_pass(&mut rt);

  let result = rt.last_layout().unwrap();
  let quads = rt.resolve_quads(result);
  for q in &quads {
    assert_eq!(q.transform, Transform2D::IDENTITY);
  }
}
