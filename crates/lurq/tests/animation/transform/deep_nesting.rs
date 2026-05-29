use lurq::{
  app::Tree,
  layout::{Alignment, Constraints, Size, quad::QuadContent},
  node::{color::Color, transform::Transform2D},
};

use crate::support::run_pass;

fn rt() -> Tree {
  Tree::new()
}

/// El1 rotated 90°, El2 rotated 90° (accumulated 180°), El3 rotated 45° (accumulated 225°)
#[test]
fn three_level_rotation_accumulates() {
  let mut rt = rt();

  let el3 = lurq::components::Rect::new(20.0, 20.0)
    .fill("#0000ff")
    .transform(Transform2D::rotate_deg(45.0));

  let el2 = lurq::components::Stack::new()
    .child(el3)
    .size(40.0, 40.0)
    .fill("#00ff00")
    .transform(Transform2D::rotate_deg(90.0));

  let el1 = lurq::components::Stack::new()
    .child(el2)
    .size(60.0, 60.0)
    .fill("#ff0000")
    .transform(Transform2D::rotate_deg(90.0));

  rt.set_root(el1);
  rt.set_layout_constraints_override(Some(Constraints::loose(Size::new(400.0, 400.0))));
  run_pass(&mut rt);

  let result = rt.last_layout().unwrap();
  let quads = rt.resolve_quads(result);

  let red = quads
    .iter()
    .find(|q| matches!(&q.content, QuadContent::Rect { color } if *color == Color::from_hex("#ff0000")))
    .expect("el1");
  let green = quads
    .iter()
    .find(|q| matches!(&q.content, QuadContent::Rect { color } if *color == Color::from_hex("#00ff00")))
    .expect("el2");
  let blue = quads
    .iter()
    .find(|q| matches!(&q.content, QuadContent::Rect { color } if *color == Color::from_hex("#0000ff")))
    .expect("el3");

  let rot90 = Transform2D::rotate_deg(90.0);
  let rot180 = Transform2D::rotate_deg(90.0).then(&Transform2D::rotate_deg(90.0));
  let rot225 = Transform2D::rotate_deg(90.0)
    .then(&Transform2D::rotate_deg(90.0))
    .then(&Transform2D::rotate_deg(45.0));

  assert!(
    (red.transform.a - rot90.a).abs() < 0.01 && (red.transform.b - rot90.b).abs() < 0.01,
    "el1 should be 90°. got: {:?}",
    red.transform
  );
  assert!(
    (green.transform.a - rot180.a).abs() < 0.01 && (green.transform.b - rot180.b).abs() < 0.01,
    "el2 should be 180° (90+90). got: {:?}",
    green.transform
  );
  assert!(
    (blue.transform.a - rot225.a).abs() < 0.01 && (blue.transform.b - rot225.b).abs() < 0.01,
    "el3 should be 225° (90+90+45). got: {:?}",
    blue.transform
  );
}

#[test]
fn mixed_transform_types_accumulate() {
  let mut rt = rt();

  let inner = lurq::components::Rect::new(20.0, 20.0)
    .fill("#0000ff")
    .transform(Transform2D::rotate_deg(45.0));

  let outer = lurq::components::Stack::new()
    .child(inner)
    .size(60.0, 60.0)
    .fill("#ff0000")
    .transform(Transform2D::scale(2.0, 2.0));

  rt.set_root(outer);
  rt.set_layout_constraints_override(Some(Constraints::loose(Size::new(400.0, 400.0))));
  run_pass(&mut rt);

  let result = rt.last_layout().unwrap();
  let quads = rt.resolve_quads(result);

  let blue = quads
    .iter()
    .find(|q| matches!(&q.content, QuadContent::Rect { color } if *color == Color::from_hex("#0000ff")))
    .expect("inner");

  let composed = Transform2D::scale(2.0, 2.0).then(&Transform2D::rotate_deg(45.0));
  assert!(
    (blue.transform.a - composed.a).abs() < 0.01
      && (blue.transform.b - composed.b).abs() < 0.01
      && (blue.transform.c - composed.c).abs() < 0.01
      && (blue.transform.d - composed.d).abs() < 0.01,
    "inner should compose scale(2) * rotate(45°).\nexpected: {:?}\ngot: {:?}",
    composed,
    blue.transform
  );
}

#[test]
fn identity_parent_does_not_alter_child() {
  let mut rt = rt();

  let inner = lurq::components::Rect::new(20.0, 20.0)
    .fill("#0000ff")
    .transform(Transform2D::rotate_deg(30.0));

  let outer = lurq::components::Stack::new()
    .child(inner)
    .size(60.0, 60.0)
    .fill("#ff0000");

  rt.set_root(outer);
  rt.set_layout_constraints_override(Some(Constraints::loose(Size::new(400.0, 400.0))));
  run_pass(&mut rt);

  let result = rt.last_layout().unwrap();
  let quads = rt.resolve_quads(result);

  let red = quads
    .iter()
    .find(|q| matches!(&q.content, QuadContent::Rect { color } if *color == Color::from_hex("#ff0000")))
    .expect("outer");
  let blue = quads
    .iter()
    .find(|q| matches!(&q.content, QuadContent::Rect { color } if *color == Color::from_hex("#0000ff")))
    .expect("inner");

  assert_eq!(red.transform, Transform2D::IDENTITY);
  let rot30 = Transform2D::rotate_deg(30.0);
  assert!(
    (blue.transform.a - rot30.a).abs() < 0.01,
    "inner should just be its own 30° rotation"
  );
}
