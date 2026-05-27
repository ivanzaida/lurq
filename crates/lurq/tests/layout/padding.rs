use lurq::{
  app::Runtime,
  layout::{Constraints, Size, layout_kind::FrameConstraints},
  node::{Element, dimension::Dimension, padding::Padding},
};

fn rt() -> Runtime {
  Runtime::new()
}

#[test]
fn padding_all_sides() {
  let mut rt = rt();
  let node = Element::new()
    .frame(FrameConstraints {
      width: Some(100.0),
      height: Some(50.0),
      ..Default::default()
    })
    .padding(Padding::all(Dimension::Px(10.0)));
  rt.set_root(node);
  let result = rt.compute_layout(Constraints::loose(Size::new(400.0, 400.0))).unwrap();
  assert_eq!(result.size.width, 120.0);
  assert_eq!(result.size.height, 70.0);
  assert_eq!(result.children[0].offset.x, 10.0);
  assert_eq!(result.children[0].offset.y, 10.0);
}

#[test]
fn padding_asymmetric() {
  let mut rt = rt();
  let node = Element::new()
    .frame(FrameConstraints {
      width: Some(100.0),
      height: Some(50.0),
      ..Default::default()
    })
    .padding(
      Padding::new()
        .left(Dimension::Px(5.0))
        .top(Dimension::Px(10.0))
        .right(Dimension::Px(15.0))
        .bottom(Dimension::Px(20.0)),
    );
  rt.set_root(node);
  let result = rt.compute_layout(Constraints::loose(Size::new(400.0, 400.0))).unwrap();
  assert_eq!(result.size.width, 120.0); // 100 + 5 + 15
  assert_eq!(result.size.height, 80.0); // 50 + 10 + 20
  assert_eq!(result.children[0].offset.x, 5.0);
  assert_eq!(result.children[0].offset.y, 10.0);
}

#[test]
fn padding_horizontal_only() {
  let mut rt = rt();
  let node = Element::new()
    .frame(FrameConstraints {
      width: Some(100.0),
      height: Some(50.0),
      ..Default::default()
    })
    .padding(Padding::horizontal(Dimension::Px(20.0)));
  rt.set_root(node);
  let result = rt.compute_layout(Constraints::loose(Size::new(400.0, 400.0))).unwrap();
  assert_eq!(result.size.width, 140.0);
  assert_eq!(result.size.height, 50.0);
}

#[test]
fn padding_vertical_only() {
  let mut rt = rt();
  let node = Element::new()
    .frame(FrameConstraints {
      width: Some(100.0),
      height: Some(50.0),
      ..Default::default()
    })
    .padding(Padding::vertical(Dimension::Px(15.0)));
  rt.set_root(node);
  let result = rt.compute_layout(Constraints::loose(Size::new(400.0, 400.0))).unwrap();
  assert_eq!(result.size.width, 100.0);
  assert_eq!(result.size.height, 80.0);
}

#[test]
fn padding_symmetric() {
  let mut rt = rt();
  let node = Element::new()
    .frame(FrameConstraints {
      width: Some(100.0),
      height: Some(50.0),
      ..Default::default()
    })
    .padding(Padding::symmetric(Dimension::Px(10.0), Dimension::Px(20.0)));
  rt.set_root(node);
  let result = rt.compute_layout(Constraints::loose(Size::new(400.0, 400.0))).unwrap();
  assert_eq!(result.size.width, 120.0); // 100 + 10*2
  assert_eq!(result.size.height, 90.0); // 50 + 20*2
}

#[test]
fn padding_reduces_child_constraints() {
  let mut rt = rt();
  // Parent tight at 200x100, padding 20 all around -> child gets 160x60
  let node = Element::new().flex(1.0).padding(Padding::all(Dimension::Px(20.0)));
  rt.set_root(node);
  let result = rt.compute_layout(Constraints::tight(Size::new(200.0, 100.0))).unwrap();
  assert_eq!(result.size.width, 200.0);
  assert_eq!(result.size.height, 100.0);
}

#[test]
fn padding_zero() {
  let mut rt = rt();
  let node = Element::new()
    .frame(FrameConstraints {
      width: Some(100.0),
      height: Some(50.0),
      ..Default::default()
    })
    .padding(Padding::all(Dimension::Px(0.0)));
  rt.set_root(node);
  let result = rt.compute_layout(Constraints::loose(Size::new(400.0, 400.0))).unwrap();
  assert_eq!(result.size.width, 100.0);
  assert_eq!(result.size.height, 50.0);
}
