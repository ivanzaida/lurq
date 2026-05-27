use lurq::{
  app::Runtime,
  layout::{Alignment, Constraints, Size, layout_kind::FrameConstraints},
  node::Element,
};

fn rt() -> Runtime {
  Runtime::new()
}

#[test]
fn empty_column() {
  let mut rt = rt();
  let node = Element::column_with(0.0, Alignment::Start, vec![]);
  rt.set_root(node);
  let result = rt.compute_layout(Constraints::loose(Size::new(400.0, 300.0))).unwrap();
  assert_eq!(result.size.width, 0.0);
  assert_eq!(result.size.height, 0.0);
}

#[test]
fn column_with_fixed_children() {
  let mut rt = rt();
  let node = Element::column_with(
    0.0,
    Alignment::Start,
    vec![
      Element::new().frame(FrameConstraints {
        width: Some(lurq::node::dimension::Dimension::Px(100.0)),
        height: Some(lurq::node::dimension::Dimension::Px(50.0)),
        ..Default::default()
      }),
      Element::new().frame(FrameConstraints {
        width: Some(lurq::node::dimension::Dimension::Px(80.0)),
        height: Some(lurq::node::dimension::Dimension::Px(40.0)),
        ..Default::default()
      }),
    ],
  );
  rt.set_root(node);
  let result = rt.compute_layout(Constraints::loose(Size::new(400.0, 300.0))).unwrap();
  assert_eq!(result.size.width, 100.0);
  assert_eq!(result.size.height, 90.0);
  assert_eq!(result.children[0].offset.y, 0.0);
  assert_eq!(result.children[1].offset.y, 50.0);
}

#[test]
fn column_with_spacing() {
  let mut rt = rt();
  let node = Element::column_with(
    8.0,
    Alignment::Start,
    vec![
      Element::new().frame(FrameConstraints {
        width: Some(lurq::node::dimension::Dimension::Px(100.0)),
        height: Some(lurq::node::dimension::Dimension::Px(30.0)),
        ..Default::default()
      }),
      Element::new().frame(FrameConstraints {
        width: Some(lurq::node::dimension::Dimension::Px(100.0)),
        height: Some(lurq::node::dimension::Dimension::Px(30.0)),
        ..Default::default()
      }),
      Element::new().frame(FrameConstraints {
        width: Some(lurq::node::dimension::Dimension::Px(100.0)),
        height: Some(lurq::node::dimension::Dimension::Px(30.0)),
        ..Default::default()
      }),
    ],
  );
  rt.set_root(node);
  let result = rt.compute_layout(Constraints::loose(Size::new(400.0, 300.0))).unwrap();
  assert_eq!(result.size.height, 106.0); // 30*3 + 8*2
  assert_eq!(result.children[0].offset.y, 0.0);
  assert_eq!(result.children[1].offset.y, 38.0);
  assert_eq!(result.children[2].offset.y, 76.0);
}

#[test]
fn column_align_center() {
  let mut rt = rt();
  let node = Element::column_with(
    0.0,
    Alignment::Center,
    vec![
      Element::new().frame(FrameConstraints {
        width: Some(lurq::node::dimension::Dimension::Px(40.0)),
        height: Some(lurq::node::dimension::Dimension::Px(30.0)),
        ..Default::default()
      }),
      Element::new().frame(FrameConstraints {
        width: Some(lurq::node::dimension::Dimension::Px(100.0)),
        height: Some(lurq::node::dimension::Dimension::Px(30.0)),
        ..Default::default()
      }),
    ],
  );
  rt.set_root(node);
  let result = rt.compute_layout(Constraints::loose(Size::new(400.0, 300.0))).unwrap();
  assert_eq!(result.size.width, 100.0);
  assert_eq!(result.children[0].offset.x, 30.0); // (100-40)/2
  assert_eq!(result.children[1].offset.x, 0.0);
}

#[test]
fn column_align_end() {
  let mut rt = rt();
  let node = Element::column_with(
    0.0,
    Alignment::End,
    vec![
      Element::new().frame(FrameConstraints {
        width: Some(lurq::node::dimension::Dimension::Px(40.0)),
        height: Some(lurq::node::dimension::Dimension::Px(30.0)),
        ..Default::default()
      }),
      Element::new().frame(FrameConstraints {
        width: Some(lurq::node::dimension::Dimension::Px(100.0)),
        height: Some(lurq::node::dimension::Dimension::Px(30.0)),
        ..Default::default()
      }),
    ],
  );
  rt.set_root(node);
  let result = rt.compute_layout(Constraints::loose(Size::new(400.0, 300.0))).unwrap();
  assert_eq!(result.children[0].offset.x, 60.0); // 100-40
  assert_eq!(result.children[1].offset.x, 0.0);
}
