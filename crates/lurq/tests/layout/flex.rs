use lurq::{
  app::Runtime,
  layout::{Alignment, Constraints, Size, layout_kind::FrameConstraints},
  node::node::Node,
};

fn rt() -> Runtime {
  Runtime::new()
}

#[test]
fn row_flex_equal_split() {
  let mut rt = rt();
  let node = Node::row(
    0.0,
    Alignment::Start,
    vec![
      Node::new()
        .frame(FrameConstraints {
          height: Some(50.0),
          ..Default::default()
        })
        .flex(1.0),
      Node::new()
        .frame(FrameConstraints {
          height: Some(50.0),
          ..Default::default()
        })
        .flex(1.0),
    ],
  );
  rt.set_root(node);
  let result = rt.compute_layout(Constraints::tight(Size::new(200.0, 100.0))).unwrap();
  assert_eq!(result.children[0].result.size.width, 100.0);
  assert_eq!(result.children[1].result.size.width, 100.0);
  assert_eq!(result.children[0].offset.x, 0.0);
  assert_eq!(result.children[1].offset.x, 100.0);
}

#[test]
fn row_flex_weighted_split() {
  let mut rt = rt();
  let node = Node::row(
    0.0,
    Alignment::Start,
    vec![
      Node::new()
        .frame(FrameConstraints {
          height: Some(50.0),
          ..Default::default()
        })
        .flex(1.0),
      Node::new()
        .frame(FrameConstraints {
          height: Some(50.0),
          ..Default::default()
        })
        .flex(3.0),
    ],
  );
  rt.set_root(node);
  let result = rt.compute_layout(Constraints::tight(Size::new(400.0, 100.0))).unwrap();
  assert_eq!(result.children[0].result.size.width, 100.0);
  assert_eq!(result.children[1].result.size.width, 300.0);
}

#[test]
fn row_flex_with_fixed_sibling() {
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
      Node::new()
        .frame(FrameConstraints {
          height: Some(50.0),
          ..Default::default()
        })
        .flex(1.0),
    ],
  );
  rt.set_root(node);
  let result = rt.compute_layout(Constraints::tight(Size::new(300.0, 100.0))).unwrap();
  assert_eq!(result.children[0].result.size.width, 100.0);
  assert_eq!(result.children[1].result.size.width, 200.0);
  assert_eq!(result.children[1].offset.x, 100.0);
}

#[test]
fn row_flex_with_spacing() {
  let mut rt = rt();
  let node = Node::row(
    20.0,
    Alignment::Start,
    vec![
      Node::new()
        .frame(FrameConstraints {
          height: Some(50.0),
          ..Default::default()
        })
        .flex(1.0),
      Node::new()
        .frame(FrameConstraints {
          height: Some(50.0),
          ..Default::default()
        })
        .flex(1.0),
    ],
  );
  rt.set_root(node);
  let result = rt.compute_layout(Constraints::tight(Size::new(220.0, 100.0))).unwrap();
  assert_eq!(result.children[0].result.size.width, 100.0);
  assert_eq!(result.children[1].result.size.width, 100.0);
  assert_eq!(result.children[1].offset.x, 120.0); // 100 + 20 spacing
}

#[test]
fn column_flex_equal_split() {
  let mut rt = rt();
  let node = Node::column(
    0.0,
    Alignment::Start,
    vec![
      Node::new()
        .frame(FrameConstraints {
          width: Some(100.0),
          ..Default::default()
        })
        .flex(1.0),
      Node::new()
        .frame(FrameConstraints {
          width: Some(100.0),
          ..Default::default()
        })
        .flex(1.0),
    ],
  );
  rt.set_root(node);
  let result = rt.compute_layout(Constraints::tight(Size::new(200.0, 300.0))).unwrap();
  assert_eq!(result.children[0].result.size.height, 150.0);
  assert_eq!(result.children[1].result.size.height, 150.0);
  assert_eq!(result.children[1].offset.y, 150.0);
}

#[test]
fn column_flex_with_fixed_sibling() {
  let mut rt = rt();
  let node = Node::column(
    0.0,
    Alignment::Start,
    vec![
      Node::new().frame(FrameConstraints {
        width: Some(100.0),
        height: Some(60.0),
        ..Default::default()
      }),
      Node::new()
        .frame(FrameConstraints {
          width: Some(100.0),
          ..Default::default()
        })
        .flex(1.0),
    ],
  );
  rt.set_root(node);
  let result = rt.compute_layout(Constraints::tight(Size::new(200.0, 300.0))).unwrap();
  assert_eq!(result.children[0].result.size.height, 60.0);
  assert_eq!(result.children[1].result.size.height, 240.0);
}

#[test]
fn three_way_flex_split() {
  let mut rt = rt();
  let node = Node::row(
    0.0,
    Alignment::Start,
    vec![Node::new().flex(1.0), Node::new().flex(2.0), Node::new().flex(3.0)],
  );
  rt.set_root(node);
  let result = rt.compute_layout(Constraints::tight(Size::new(600.0, 100.0))).unwrap();
  assert_eq!(result.children[0].result.size.width, 100.0);
  assert_eq!(result.children[1].result.size.width, 200.0);
  assert_eq!(result.children[2].result.size.width, 300.0);
}
