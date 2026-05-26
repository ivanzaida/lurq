use lurq::{
  app::Runtime,
  layout::{Alignment, Constraints, Size, layout_kind::FrameConstraints},
  node::{dsl::*, node::Node},
};

fn rt() -> Runtime {
  Runtime::new()
}

#[test]
fn scroll_vertical_child_grows_unbounded() {
  let mut rt = rt();
  let node = scroll_vertical(column().spacing(0.0).with_children((0..10).map(|_| {
    Node::new().frame(FrameConstraints {
      width: Some(100.0),
      height: Some(50.0),
      ..Default::default()
    })
  })))
  .size(100.0, 200.0);

  rt.set_root(node);
  let result = rt.compute_layout(Constraints::loose(Size::new(400.0, 400.0))).unwrap();
  // Scroll container itself is 200 tall
  assert_eq!(result.size.height, 200.0);
  assert_eq!(result.size.width, 100.0);
  // The child (column) inside should be 500 tall (10 * 50)
  let scroll_child = &result.children[0].result;
  let column_child = &scroll_child.children[0].result;
  assert_eq!(column_child.size.height, 500.0);
}

#[test]
fn scroll_vertical_offset_applied() {
  let mut rt = rt();
  let node = scroll_vertical(Node::new().frame(FrameConstraints {
    width: Some(100.0),
    height: Some(500.0),
    ..Default::default()
  }))
  .size(100.0, 200.0);

  rt.set_root(node);
  let result = rt.compute_layout(Constraints::loose(Size::new(400.0, 400.0))).unwrap();
  // Default scroll offset is 0
  let scroll_child = &result.children[0].result;
  assert_eq!(scroll_child.children[0].offset.y, 0.0);
}

#[test]
fn scroll_horizontal_child_grows_unbounded() {
  let mut rt = rt();
  let node = scroll_horizontal(row().spacing(0.0).with_children((0..10).map(|_| {
    Node::new().frame(FrameConstraints {
      width: Some(100.0),
      height: Some(50.0),
      ..Default::default()
    })
  })))
  .size(200.0, 50.0);

  rt.set_root(node);
  let result = rt.compute_layout(Constraints::loose(Size::new(400.0, 400.0))).unwrap();
  assert_eq!(result.size.width, 200.0);
  assert_eq!(result.size.height, 50.0);
  let scroll_child = &result.children[0].result;
  let row_child = &scroll_child.children[0].result;
  assert_eq!(row_child.size.width, 1000.0);
}

#[test]
fn scroll_both_unbounded() {
  let mut rt = rt();
  let node = scroll_both(Node::new().frame(FrameConstraints {
    width: Some(800.0),
    height: Some(600.0),
    ..Default::default()
  }))
  .size(200.0, 150.0);

  rt.set_root(node);
  let result = rt.compute_layout(Constraints::loose(Size::new(400.0, 400.0))).unwrap();
  assert_eq!(result.size.width, 200.0);
  assert_eq!(result.size.height, 150.0);
  let scroll_child = &result.children[0].result;
  assert_eq!(scroll_child.children[0].result.size.width, 800.0);
  assert_eq!(scroll_child.children[0].result.size.height, 600.0);
}

#[test]
fn scroll_container_without_frame_uses_parent_constraints() {
  let mut rt = rt();
  let node = scroll_vertical(column().spacing(0.0).with_children((0..5).map(|_| {
    Node::new().frame(FrameConstraints {
      width: Some(100.0),
      height: Some(40.0),
      ..Default::default()
    })
  })));

  rt.set_root(node);
  let result = rt.compute_layout(Constraints::tight(Size::new(300.0, 100.0))).unwrap();
  // Container takes parent's tight constraint
  assert_eq!(result.size.width, 300.0);
  assert_eq!(result.size.height, 100.0);
  // Child column is taller
  assert_eq!(result.children[0].result.size.height, 200.0);
}

#[test]
fn scroll_empty_child() {
  let mut rt = rt();
  let node = scroll_vertical(Node::new()).size(100.0, 100.0);

  rt.set_root(node);
  let result = rt.compute_layout(Constraints::loose(Size::new(400.0, 400.0))).unwrap();
  assert_eq!(result.size.width, 100.0);
  assert_eq!(result.size.height, 100.0);
}
