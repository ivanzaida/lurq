use lurq::{
  app::Runtime,
  layout::{Alignment, Constraints, Size, layout_kind::FrameConstraints},
  node::node::Node,
};

fn rt() -> Runtime {
  Runtime::new()
}

#[test]
fn frame_fixed_size() {
  let mut rt = rt();
  let node = Node::new().frame(FrameConstraints {
    width: Some(150.0),
    height: Some(80.0),
    ..Default::default()
  });
  rt.set_root(node);
  let result = rt.compute_layout(Constraints::loose(Size::new(400.0, 400.0))).unwrap();
  assert_eq!(result.size.width, 150.0);
  assert_eq!(result.size.height, 80.0);
}

#[test]
fn frame_width_only() {
  let mut rt = rt();
  let node = Node::new().frame(FrameConstraints {
    width: Some(150.0),
    ..Default::default()
  });
  rt.set_root(node);
  let result = rt.compute_layout(Constraints::loose(Size::new(400.0, 400.0))).unwrap();
  assert_eq!(result.size.width, 150.0);
}

#[test]
fn frame_min_width() {
  let mut rt = rt();
  let node = Node::new().frame(FrameConstraints {
    min_width: Some(200.0),
    ..Default::default()
  });
  rt.set_root(node);
  let result = rt.compute_layout(Constraints::loose(Size::new(400.0, 400.0))).unwrap();
  assert!(result.size.width >= 200.0);
}

#[test]
fn frame_max_width_limits() {
  let mut rt = rt();
  let node = Node::new()
    .frame(FrameConstraints {
      width: Some(500.0),
      ..Default::default()
    })
    .frame(FrameConstraints {
      max_width: Some(200.0),
      ..Default::default()
    });
  rt.set_root(node);
  let result = rt.compute_layout(Constraints::loose(Size::new(400.0, 400.0))).unwrap();
  assert!(result.size.width <= 200.0);
}

#[test]
fn frame_min_height() {
  let mut rt = rt();
  let node = Node::new().frame(FrameConstraints {
    min_height: Some(100.0),
    ..Default::default()
  });
  rt.set_root(node);
  let result = rt.compute_layout(Constraints::loose(Size::new(400.0, 400.0))).unwrap();
  assert!(result.size.height >= 100.0);
}

#[test]
fn frame_max_height_limits() {
  let mut rt = rt();
  let node = Node::new()
    .frame(FrameConstraints {
      height: Some(500.0),
      ..Default::default()
    })
    .frame(FrameConstraints {
      max_height: Some(150.0),
      ..Default::default()
    });
  rt.set_root(node);
  let result = rt.compute_layout(Constraints::loose(Size::new(400.0, 400.0))).unwrap();
  assert!(result.size.height <= 150.0);
}

#[test]
fn frame_no_constraints_leaf_is_zero() {
  let mut rt = rt();
  let node = Node::new().frame(FrameConstraints::default());
  rt.set_root(node);
  let result = rt.compute_layout(Constraints::loose(Size::new(400.0, 400.0))).unwrap();
  assert_eq!(result.size.width, 0.0);
  assert_eq!(result.size.height, 0.0);
}
