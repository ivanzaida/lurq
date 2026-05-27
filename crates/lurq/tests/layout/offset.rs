use lurq::{
  app::Runtime,
  layout::{Constraints, Size, layout_kind::FrameConstraints},
};

fn rt() -> Runtime {
  Runtime::new()
}

#[test]
fn offset_does_not_affect_size() {
  let mut rt = rt();
  let node = lurq::components::Spacer::new()
    .frame(FrameConstraints {
      width: Some(lurq::node::dimension::Dimension::Px(100.0)),
      height: Some(lurq::node::dimension::Dimension::Px(50.0)),
      ..Default::default()
    })
    .offset(20.0, 30.0);
  rt.set_root(node);
  let result = rt.compute_layout(Constraints::loose(Size::new(400.0, 400.0))).unwrap();
  assert_eq!(result.size.width, 100.0);
  assert_eq!(result.size.height, 50.0);
}

#[test]
fn offset_shifts_child() {
  let mut rt = rt();
  let node = lurq::components::Spacer::new()
    .frame(FrameConstraints {
      width: Some(lurq::node::dimension::Dimension::Px(100.0)),
      height: Some(lurq::node::dimension::Dimension::Px(50.0)),
      ..Default::default()
    })
    .offset(20.0, 30.0);
  rt.set_root(node);
  let result = rt.compute_layout(Constraints::loose(Size::new(400.0, 400.0))).unwrap();
  assert_eq!(result.children[0].offset.x, 20.0);
  assert_eq!(result.children[0].offset.y, 30.0);
}

#[test]
fn offset_negative() {
  let mut rt = rt();
  let node = lurq::components::Spacer::new()
    .frame(FrameConstraints {
      width: Some(lurq::node::dimension::Dimension::Px(100.0)),
      height: Some(lurq::node::dimension::Dimension::Px(50.0)),
      ..Default::default()
    })
    .offset(-10.0, -5.0);
  rt.set_root(node);
  let result = rt.compute_layout(Constraints::loose(Size::new(400.0, 400.0))).unwrap();
  assert_eq!(result.children[0].offset.x, -10.0);
  assert_eq!(result.children[0].offset.y, -5.0);
}

#[test]
fn offset_zero_is_noop() {
  let mut rt = rt();
  let node = lurq::components::Spacer::new()
    .frame(FrameConstraints {
      width: Some(lurq::node::dimension::Dimension::Px(100.0)),
      height: Some(lurq::node::dimension::Dimension::Px(50.0)),
      ..Default::default()
    })
    .offset(0.0, 0.0);
  rt.set_root(node);
  let result = rt.compute_layout(Constraints::loose(Size::new(400.0, 400.0))).unwrap();
  assert_eq!(result.children[0].offset.x, 0.0);
  assert_eq!(result.children[0].offset.y, 0.0);
  assert_eq!(result.size.width, 100.0);
}
