use lurq::{
  app::Runtime,
  layout::{Alignment, Constraints, Size, layout_kind::FrameConstraints},
  node::node::Node,
};

fn rt() -> Runtime {
  Runtime::new()
}

#[test]
fn empty_row() {
  let mut rt = rt();
  let node = Node::row(0.0, Alignment::Start, vec![]);
  rt.set_root(node);
  let result = rt.compute_layout(Constraints::loose(Size::new(400.0, 300.0))).unwrap();
  assert_eq!(result.size.width, 0.0);
  assert_eq!(result.size.height, 0.0);
}

#[test]
fn row_with_fixed_children() {
  let mut rt = rt();
  let node = Node::row(
    0.0,
    Alignment::Start,
    vec![
      Node::new().frame(FrameConstraints {
        width: Some(100.0),
        height: Some(50.0),
        ..Default::default()
      }),
      Node::new().frame(FrameConstraints {
        width: Some(80.0),
        height: Some(40.0),
        ..Default::default()
      }),
    ],
  );
  rt.set_root(node);
  let result = rt.compute_layout(Constraints::loose(Size::new(400.0, 300.0))).unwrap();
  assert_eq!(result.size.width, 180.0);
  assert_eq!(result.size.height, 50.0);
  assert_eq!(result.children[0].offset.x, 0.0);
  assert_eq!(result.children[1].offset.x, 100.0);
}

#[test]
fn row_with_spacing() {
  let mut rt = rt();
  let node = Node::row(
    10.0,
    Alignment::Start,
    vec![
      Node::new().frame(FrameConstraints {
        width: Some(50.0),
        height: Some(30.0),
        ..Default::default()
      }),
      Node::new().frame(FrameConstraints {
        width: Some(50.0),
        height: Some(30.0),
        ..Default::default()
      }),
      Node::new().frame(FrameConstraints {
        width: Some(50.0),
        height: Some(30.0),
        ..Default::default()
      }),
    ],
  );
  rt.set_root(node);
  let result = rt.compute_layout(Constraints::loose(Size::new(400.0, 300.0))).unwrap();
  assert_eq!(result.size.width, 170.0); // 50*3 + 10*2
  assert_eq!(result.children[0].offset.x, 0.0);
  assert_eq!(result.children[1].offset.x, 60.0);
  assert_eq!(result.children[2].offset.x, 120.0);
}

#[test]
fn row_align_center() {
  let mut rt = rt();
  let node = Node::row(
    0.0,
    Alignment::Center,
    vec![
      Node::new().frame(FrameConstraints {
        width: Some(50.0),
        height: Some(20.0),
        ..Default::default()
      }),
      Node::new().frame(FrameConstraints {
        width: Some(50.0),
        height: Some(60.0),
        ..Default::default()
      }),
    ],
  );
  rt.set_root(node);
  let result = rt.compute_layout(Constraints::loose(Size::new(400.0, 300.0))).unwrap();
  assert_eq!(result.size.height, 60.0);
  assert_eq!(result.children[0].offset.y, 20.0); // (60-20)/2
  assert_eq!(result.children[1].offset.y, 0.0);
}

#[test]
fn row_align_end() {
  let mut rt = rt();
  let node = Node::row(
    0.0,
    Alignment::End,
    vec![
      Node::new().frame(FrameConstraints {
        width: Some(50.0),
        height: Some(20.0),
        ..Default::default()
      }),
      Node::new().frame(FrameConstraints {
        width: Some(50.0),
        height: Some(60.0),
        ..Default::default()
      }),
    ],
  );
  rt.set_root(node);
  let result = rt.compute_layout(Constraints::loose(Size::new(400.0, 300.0))).unwrap();
  assert_eq!(result.children[0].offset.y, 40.0); // 60-20
  assert_eq!(result.children[1].offset.y, 0.0);
}

#[test]
fn row_single_child() {
  let mut rt = rt();
  let node = Node::row(
    10.0,
    Alignment::Start,
    vec![Node::new().frame(FrameConstraints {
      width: Some(100.0),
      height: Some(50.0),
      ..Default::default()
    })],
  );
  rt.set_root(node);
  let result = rt.compute_layout(Constraints::loose(Size::new(400.0, 300.0))).unwrap();
  assert_eq!(result.size.width, 100.0);
  assert_eq!(result.size.height, 50.0);
  assert_eq!(result.children[0].offset.x, 0.0);
}
