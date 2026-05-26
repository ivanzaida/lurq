use lurq::{
  app::Runtime,
  layout::{Alignment, Constraints, Size, layout_kind::FrameConstraints, quad::QuadContent},
  node::{color::Color, dimension::Dimension, node::Node, padding::Padding},
};

fn rt() -> Runtime {
  Runtime::new()
}

#[test]
fn no_quads_for_invisible_nodes() {
  let mut rt = rt();
  let node = Node::new().frame(FrameConstraints {
    width: Some(100.0),
    height: Some(50.0),
    ..Default::default()
  });
  rt.set_root(node);
  let result = rt.compute_layout(Constraints::loose(Size::new(400.0, 400.0))).unwrap();
  let quads = rt.resolve_quads(&result);
  assert!(quads.is_empty());
}

#[test]
fn background_produces_rect_quad() {
  let mut rt = rt();
  let node = Node::new()
    .frame(FrameConstraints {
      width: Some(100.0),
      height: Some(50.0),
      ..Default::default()
    })
    .background(Color::new(255, 0, 0, 255));
  rt.set_root(node);
  let result = rt.compute_layout(Constraints::loose(Size::new(400.0, 400.0))).unwrap();
  let quads = rt.resolve_quads(&result);
  assert_eq!(quads.len(), 1);
  assert_eq!(quads[0].x, 0.0);
  assert_eq!(quads[0].y, 0.0);
  assert_eq!(quads[0].width, 100.0);
  assert_eq!(quads[0].height, 50.0);
  assert!(matches!(quads[0].content, QuadContent::Rect { .. }));
}

#[test]
fn text_produces_text_quad() {
  let mut rt = rt();
  let node = Node::text("hello");
  rt.set_root(node);
  let result = rt.compute_layout(Constraints::loose(Size::new(400.0, 400.0))).unwrap();
  let quads = rt.resolve_quads(&result);
  assert_eq!(quads.len(), 1);
  if let QuadContent::Text { ref text, .. } = quads[0].content {
    assert_eq!(text, "hello");
  } else {
    panic!("expected text quad");
  }
}

#[test]
fn quads_absolute_positions_in_row() {
  let mut rt = rt();
  let node = Node::row(
    10.0,
    Alignment::Start,
    vec![
      Node::new()
        .frame(FrameConstraints {
          width: Some(50.0),
          height: Some(30.0),
          ..Default::default()
        })
        .background(Color::new(255, 0, 0, 255)),
      Node::new()
        .frame(FrameConstraints {
          width: Some(50.0),
          height: Some(30.0),
          ..Default::default()
        })
        .background(Color::new(0, 255, 0, 255)),
    ],
  );
  rt.set_root(node);
  let result = rt.compute_layout(Constraints::loose(Size::new(400.0, 400.0))).unwrap();
  let quads = rt.resolve_quads(&result);
  assert_eq!(quads.len(), 2);
  assert_eq!(quads[0].x, 0.0);
  assert_eq!(quads[1].x, 60.0); // 50 + 10 spacing
}

#[test]
fn quads_absolute_positions_in_column() {
  let mut rt = rt();
  let node = Node::column(
    5.0,
    Alignment::Start,
    vec![
      Node::new()
        .frame(FrameConstraints {
          width: Some(100.0),
          height: Some(40.0),
          ..Default::default()
        })
        .background(Color::new(255, 0, 0, 255)),
      Node::new()
        .frame(FrameConstraints {
          width: Some(100.0),
          height: Some(40.0),
          ..Default::default()
        })
        .background(Color::new(0, 255, 0, 255)),
    ],
  );
  rt.set_root(node);
  let result = rt.compute_layout(Constraints::loose(Size::new(400.0, 400.0))).unwrap();
  let quads = rt.resolve_quads(&result);
  assert_eq!(quads.len(), 2);
  assert_eq!(quads[0].y, 0.0);
  assert_eq!(quads[1].y, 45.0); // 40 + 5 spacing
}

#[test]
fn quads_nested_absolute_positions() {
  let mut rt = rt();
  let node = Node::column(
    0.0,
    Alignment::Start,
    vec![
      Node::new().frame(FrameConstraints {
        width: Some(100.0),
        height: Some(50.0),
        ..Default::default()
      }),
      Node::row(
        0.0,
        Alignment::Start,
        vec![
          Node::new().frame(FrameConstraints {
            width: Some(40.0),
            height: Some(30.0),
            ..Default::default()
          }),
          Node::new()
            .frame(FrameConstraints {
              width: Some(40.0),
              height: Some(30.0),
              ..Default::default()
            })
            .background(Color::new(0, 0, 255, 255)),
        ],
      ),
    ],
  );
  rt.set_root(node);
  let result = rt.compute_layout(Constraints::loose(Size::new(400.0, 400.0))).unwrap();
  let quads = rt.resolve_quads(&result);
  assert_eq!(quads.len(), 1);
  assert_eq!(quads[0].x, 40.0);
  assert_eq!(quads[0].y, 50.0);
}

#[test]
fn quads_with_padding_offset() {
  let mut rt = rt();
  let node = Node::new()
    .frame(FrameConstraints {
      width: Some(60.0),
      height: Some(40.0),
      ..Default::default()
    })
    .background(Color::new(255, 0, 0, 255))
    .padding(Padding::all(Dimension::Px(20.0)));
  rt.set_root(node);
  let result = rt.compute_layout(Constraints::loose(Size::new(400.0, 400.0))).unwrap();
  let quads = rt.resolve_quads(&result);
  assert_eq!(quads.len(), 1);
  assert_eq!(quads[0].x, 20.0);
  assert_eq!(quads[0].y, 20.0);
  assert_eq!(quads[0].width, 60.0);
  assert_eq!(quads[0].height, 40.0);
}
