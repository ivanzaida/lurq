use lurq::{
  app::Tree,
  layout::{Constraints, Size, quad::QuadContent},
  node::transform::Transform2D,
};

use crate::support::run_pass;

fn rt() -> Tree {
  Tree::new()
}

#[test]
fn text_inside_rotated_container_inherits_rotation() {
  let mut rt = rt();

  let node = lurq::components::Row::new()
    .child(lurq::components::Text::new("hello"))
    .size(100.0, 40.0)
    .background("#ff0000")
    .transform(Transform2D::rotate_deg(45.0));
  rt.set_root(node);
  rt.set_layout_constraints_override(Some(Constraints::loose(Size::new(400.0, 400.0))));
  run_pass(&mut rt);

  let result = rt.last_layout().unwrap();
  let quads = rt.resolve_quads(result);

  let text_quad = quads
    .iter()
    .find(|q| matches!(&q.content, QuadContent::Text { .. }))
    .expect("should have a text quad");

  let expected = Transform2D::rotate_deg(45.0);
  assert!(
    (text_quad.transform.a - expected.a).abs() < 0.01 && (text_quad.transform.b - expected.b).abs() < 0.01,
    "text quad should inherit container's 45° rotation.\ngot: {:?}",
    text_quad.transform
  );
}

#[test]
fn text_inside_scaled_container_inherits_scale() {
  let mut rt = rt();

  let node = lurq::components::Row::new()
    .child(lurq::components::Text::new("hello"))
    .size(100.0, 40.0)
    .background("#ff0000")
    .transform(Transform2D::scale(2.0, 0.5));
  rt.set_root(node);
  rt.set_layout_constraints_override(Some(Constraints::loose(Size::new(400.0, 400.0))));
  run_pass(&mut rt);

  let result = rt.last_layout().unwrap();
  let quads = rt.resolve_quads(result);

  let text_quad = quads
    .iter()
    .find(|q| matches!(&q.content, QuadContent::Text { .. }))
    .expect("should have a text quad");

  assert!(
    (text_quad.transform.a - 2.0).abs() < 0.01 && (text_quad.transform.d - 0.5).abs() < 0.01,
    "text quad should inherit container's scale(2, 0.5).\ngot: {:?}",
    text_quad.transform
  );
}

#[test]
fn text_in_nested_transformed_containers() {
  let mut rt = rt();

  let inner = lurq::components::Row::new()
    .child(lurq::components::Text::new("deep"))
    .size(80.0, 30.0)
    .transform(Transform2D::rotate_deg(45.0));

  let outer = lurq::components::Stack::new()
    .child(inner)
    .size(100.0, 100.0)
    .transform(Transform2D::rotate_deg(90.0));

  rt.set_root(outer);
  rt.set_layout_constraints_override(Some(Constraints::loose(Size::new(400.0, 400.0))));
  run_pass(&mut rt);

  let result = rt.last_layout().unwrap();
  let quads = rt.resolve_quads(result);

  let text_quad = quads
    .iter()
    .find(|q| matches!(&q.content, QuadContent::Text { .. }))
    .expect("should have a text quad");

  let composed = Transform2D::rotate_deg(90.0).then(&Transform2D::rotate_deg(45.0));
  assert!(
    (text_quad.transform.a - composed.a).abs() < 0.01 && (text_quad.transform.b - composed.b).abs() < 0.01,
    "text should carry composed 135° rotation (90+45).\nexpected: {:?}\ngot: {:?}",
    composed,
    text_quad.transform
  );
}

#[test]
fn rect_and_text_in_same_container_share_transform() {
  let mut rt = rt();

  let node = lurq::components::Row::new()
    .child(lurq::components::Text::new("label"))
    .size(100.0, 40.0)
    .background("#ff0000")
    .transform(Transform2D::rotate_deg(20.0));
  rt.set_root(node);
  rt.set_layout_constraints_override(Some(Constraints::loose(Size::new(400.0, 400.0))));
  run_pass(&mut rt);

  let result = rt.last_layout().unwrap();
  let quads = rt.resolve_quads(result);

  let rect_xf = quads
    .iter()
    .find(|q| matches!(&q.content, QuadContent::Rect { .. }))
    .map(|q| q.transform);
  let text_xf = quads
    .iter()
    .find(|q| matches!(&q.content, QuadContent::Text { .. }))
    .map(|q| q.transform);

  assert!(rect_xf.is_some() && text_xf.is_some());
  let rect_xf = rect_xf.unwrap();
  let text_xf = text_xf.unwrap();
  assert!(
    (rect_xf.a - text_xf.a).abs() < 0.001
      && (rect_xf.b - text_xf.b).abs() < 0.001
      && (rect_xf.c - text_xf.c).abs() < 0.001
      && (rect_xf.d - text_xf.d).abs() < 0.001,
    "rect and text in same container must have identical transforms.\nrect: {:?}\ntext: {:?}",
    rect_xf,
    text_xf
  );
}
