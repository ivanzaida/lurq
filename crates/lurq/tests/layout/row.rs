use lurq::{
  app::Runtime,
  layout::{Alignment, Constraints, Size, layout_kind::FrameConstraints},
};

fn rt() -> Runtime {
  Runtime::new()
}

#[test]
fn empty_row() {
  let mut rt = rt();
  let node = lurq::components::Row::new().spacing(0.0).align_items(Alignment::Start);
  rt.set_root(node);
  let result = rt.compute_layout(Constraints::loose(Size::new(400.0, 300.0))).unwrap();
  assert_eq!(result.size.width, 0.0);
  assert_eq!(result.size.height, 0.0);
}

#[test]
fn row_with_fixed_children() {
  let mut rt = rt();
  let node = lurq::components::Row::with(
    0.0,
    Alignment::Start,
    vec![
      lurq::components::Spacer::new().frame(FrameConstraints {
        width: Some(lurq::node::dimension::Dimension::Px(100.0)),
        height: Some(lurq::node::dimension::Dimension::Px(50.0)),
        ..Default::default()
      }),
      lurq::components::Spacer::new().frame(FrameConstraints {
        width: Some(lurq::node::dimension::Dimension::Px(80.0)),
        height: Some(lurq::node::dimension::Dimension::Px(40.0)),
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
  let node = lurq::components::Row::with(
    10.0,
    Alignment::Start,
    vec![
      lurq::components::Spacer::new().frame(FrameConstraints {
        width: Some(lurq::node::dimension::Dimension::Px(50.0)),
        height: Some(lurq::node::dimension::Dimension::Px(30.0)),
        ..Default::default()
      }),
      lurq::components::Spacer::new().frame(FrameConstraints {
        width: Some(lurq::node::dimension::Dimension::Px(50.0)),
        height: Some(lurq::node::dimension::Dimension::Px(30.0)),
        ..Default::default()
      }),
      lurq::components::Spacer::new().frame(FrameConstraints {
        width: Some(lurq::node::dimension::Dimension::Px(50.0)),
        height: Some(lurq::node::dimension::Dimension::Px(30.0)),
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
  let node = lurq::components::Row::with(
    0.0,
    Alignment::Center,
    vec![
      lurq::components::Spacer::new().frame(FrameConstraints {
        width: Some(lurq::node::dimension::Dimension::Px(50.0)),
        height: Some(lurq::node::dimension::Dimension::Px(20.0)),
        ..Default::default()
      }),
      lurq::components::Spacer::new().frame(FrameConstraints {
        width: Some(lurq::node::dimension::Dimension::Px(50.0)),
        height: Some(lurq::node::dimension::Dimension::Px(60.0)),
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
  let node = lurq::components::Row::with(
    0.0,
    Alignment::End,
    vec![
      lurq::components::Spacer::new().frame(FrameConstraints {
        width: Some(lurq::node::dimension::Dimension::Px(50.0)),
        height: Some(lurq::node::dimension::Dimension::Px(20.0)),
        ..Default::default()
      }),
      lurq::components::Spacer::new().frame(FrameConstraints {
        width: Some(lurq::node::dimension::Dimension::Px(50.0)),
        height: Some(lurq::node::dimension::Dimension::Px(60.0)),
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
  let node = lurq::components::Row::with(
    10.0,
    Alignment::Start,
    vec![lurq::components::Spacer::new().frame(FrameConstraints {
      width: Some(lurq::node::dimension::Dimension::Px(100.0)),
      height: Some(lurq::node::dimension::Dimension::Px(50.0)),
      ..Default::default()
    })],
  );
  rt.set_root(node);
  let result = rt.compute_layout(Constraints::loose(Size::new(400.0, 300.0))).unwrap();
  assert_eq!(result.size.width, 100.0);
  assert_eq!(result.size.height, 50.0);
  assert_eq!(result.children[0].offset.x, 0.0);
}
