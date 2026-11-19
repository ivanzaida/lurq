use lurq::{
  app::Tree,
  layout::{Constraints, Size, layout_kind::FrameConstraints},
};

use super::PassLayoutExt;

fn rt() -> Tree {
  Tree::new()
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
  let result = rt.pass_layout(Constraints::loose(Size::new(400.0, 400.0))).unwrap();
  assert_eq!(result.size.width, 100.0);
  assert_eq!(result.size.height, 50.0);
}

#[test]
fn offset_shifts_child() {
  let mut rt = rt();
  let node = lurq::components::Stack::new().child(
    lurq::components::Spacer::new()
      .frame(FrameConstraints {
        width: Some(lurq::node::dimension::Dimension::Px(100.0)),
        height: Some(lurq::node::dimension::Dimension::Px(50.0)),
        ..Default::default()
      })
      .offset(20.0, 30.0),
  );
  rt.set_root(node);
  let result = rt.pass_layout(Constraints::loose(Size::new(400.0, 400.0))).unwrap();
  assert_eq!(result.children[0].offset.x, 20.0);
  assert_eq!(result.children[0].offset.y, 30.0);
}

#[test]
fn offset_negative() {
  let mut rt = rt();
  let node = lurq::components::Stack::new().child(
    lurq::components::Spacer::new()
      .frame(FrameConstraints {
        width: Some(lurq::node::dimension::Dimension::Px(100.0)),
        height: Some(lurq::node::dimension::Dimension::Px(50.0)),
        ..Default::default()
      })
      .offset(-10.0, -5.0),
  );
  rt.set_root(node);
  let result = rt.pass_layout(Constraints::loose(Size::new(400.0, 400.0))).unwrap();
  assert_eq!(result.children[0].offset.x, -10.0);
  assert_eq!(result.children[0].offset.y, -5.0);
}

#[test]
fn offset_zero_is_noop() {
  let mut rt = rt();
  let node = lurq::components::Stack::new().child(
    lurq::components::Spacer::new()
      .frame(FrameConstraints {
        width: Some(lurq::node::dimension::Dimension::Px(100.0)),
        height: Some(lurq::node::dimension::Dimension::Px(50.0)),
        ..Default::default()
      })
      .offset(0.0, 0.0),
  );
  rt.set_root(node);
  let result = rt.pass_layout(Constraints::loose(Size::new(400.0, 400.0))).unwrap();
  assert_eq!(result.children[0].offset.x, 0.0);
  assert_eq!(result.children[0].offset.y, 0.0);
  assert_eq!(result.children[0].result.size.width, 100.0);
}

#[test]
fn an_offset_by_size_moves_a_child_by_shares_of_its_own_size() {
  let sized = |width: f32| {
    lurq::components::Spacer::new().frame(FrameConstraints {
      width: Some(lurq::node::dimension::Dimension::Px(width)),
      height: Some(lurq::node::dimension::Dimension::Px(20.0)),
      ..Default::default()
    })
  };
  let mut rt = rt();
  // In a row, where each child is laid out after the one before it: the first is centred on where its
  // corner was, the second moved by its offset as well, and the room each takes stays where it was.
  let node = lurq::components::Row::new()
    .child(sized(100.0).offset_by_size(-0.5, -0.5))
    .child(sized(40.0).offset(10.0, 4.0).offset_by_size(-0.5, 1.0))
    .child(sized(10.0));
  rt.set_root(node);
  let result = rt.pass_layout(Constraints::loose(Size::new(400.0, 400.0))).unwrap();
  assert_eq!(result.size.width, 150.0, "the room of the three, unmoved");
  let at = |index: usize| (result.children[index].offset.x, result.children[index].offset.y);
  assert_eq!(at(0), (-50.0, -10.0));
  assert_eq!(at(1), (100.0 + 10.0 - 20.0, 4.0 + 20.0));
  assert_eq!(at(2), (140.0, 0.0));
}
