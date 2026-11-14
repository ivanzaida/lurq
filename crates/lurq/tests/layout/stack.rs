use lurq::{
  app::Tree,
  layout::{Alignment, Constraints, Size, StackAlignment, layout_kind::FrameConstraints},
};

use super::PassLayoutExt;

fn rt() -> Tree {
  Tree::new()
}

#[test]
fn stack_sizes_to_largest_child() {
  let mut rt = rt();
  let node = lurq::components::Stack::with(
    StackAlignment::Center,
    vec![
      lurq::components::Spacer::new().frame(FrameConstraints {
        width: Some(lurq::node::dimension::Dimension::Px(200.0)),
        height: Some(lurq::node::dimension::Dimension::Px(100.0)),
        ..Default::default()
      }),
      lurq::components::Spacer::new().frame(FrameConstraints {
        width: Some(lurq::node::dimension::Dimension::Px(50.0)),
        height: Some(lurq::node::dimension::Dimension::Px(50.0)),
        ..Default::default()
      }),
    ],
  );
  rt.set_root(node);
  let result = rt.pass_layout(Constraints::loose(Size::new(400.0, 400.0))).unwrap();
  assert_eq!(result.size.width, 200.0);
  assert_eq!(result.size.height, 100.0);
}

#[test]
fn stack_center_alignment() {
  let mut rt = rt();
  let node = lurq::components::Stack::with(
    StackAlignment::Center,
    vec![
      lurq::components::Spacer::new().frame(FrameConstraints {
        width: Some(lurq::node::dimension::Dimension::Px(200.0)),
        height: Some(lurq::node::dimension::Dimension::Px(200.0)),
        ..Default::default()
      }),
      lurq::components::Spacer::new().frame(FrameConstraints {
        width: Some(lurq::node::dimension::Dimension::Px(50.0)),
        height: Some(lurq::node::dimension::Dimension::Px(50.0)),
        ..Default::default()
      }),
    ],
  );
  rt.set_root(node);
  let result = rt.pass_layout(Constraints::loose(Size::new(400.0, 400.0))).unwrap();
  assert_eq!(result.children[1].offset.x, 75.0); // (200-50)/2
  assert_eq!(result.children[1].offset.y, 75.0);
}

#[test]
fn fixed_size_stack_does_not_force_child_cross_axis_size() {
  let mut rt = rt();
  let node = lurq::components::Stack::new()
    .stack_align(StackAlignment::Center)
    .child(lurq::components::Spacer::new().width(100.0))
    .size(300.0, 200.0);

  rt.set_root(node);
  let result = rt.pass_layout(Constraints::loose(Size::new(400.0, 400.0))).unwrap();

  assert_eq!(result.size.width, 300.0);
  assert_eq!(result.size.height, 200.0);
  assert_eq!(result.children[0].result.size.width, 100.0);
  assert_eq!(result.children[0].result.size.height, 0.0);
  assert_eq!(result.children[0].offset.x, 100.0);
  assert_eq!(result.children[0].offset.y, 100.0);
}

#[test]
fn stack_top_start() {
  let mut rt = rt();
  let node = lurq::components::Stack::with(
    StackAlignment::TopStart,
    vec![
      lurq::components::Spacer::new().frame(FrameConstraints {
        width: Some(lurq::node::dimension::Dimension::Px(200.0)),
        height: Some(lurq::node::dimension::Dimension::Px(200.0)),
        ..Default::default()
      }),
      lurq::components::Spacer::new().frame(FrameConstraints {
        width: Some(lurq::node::dimension::Dimension::Px(50.0)),
        height: Some(lurq::node::dimension::Dimension::Px(50.0)),
        ..Default::default()
      }),
    ],
  );
  rt.set_root(node);
  let result = rt.pass_layout(Constraints::loose(Size::new(400.0, 400.0))).unwrap();
  assert_eq!(result.children[1].offset.x, 0.0);
  assert_eq!(result.children[1].offset.y, 0.0);
}

#[test]
fn absolute_child_does_not_affect_stack_size() {
  let mut rt = rt();
  let node = lurq::components::Stack::new()
    .child(lurq::components::Rect::new(10.0, 20.0))
    .child(lurq::components::Rect::new(100.0, 120.0).absolute(30.0, 40.0, 100.0, 120.0));
  rt.set_root(node);
  let result = rt.pass_layout(Constraints::loose(Size::new(500.0, 500.0))).unwrap();

  assert_eq!(result.size.width, 10.0);
  assert_eq!(result.size.height, 20.0);
  assert_eq!(result.children[1].offset.x, 30.0);
  assert_eq!(result.children[1].offset.y, 40.0);
  assert_eq!(result.children[1].result.size.width, 100.0);
  assert_eq!(result.children[1].result.size.height, 120.0);
}

#[test]
fn stack_bottom_end() {
  let mut rt = rt();
  let node = lurq::components::Stack::with(
    StackAlignment::BottomEnd,
    vec![
      lurq::components::Spacer::new().frame(FrameConstraints {
        width: Some(lurq::node::dimension::Dimension::Px(200.0)),
        height: Some(lurq::node::dimension::Dimension::Px(200.0)),
        ..Default::default()
      }),
      lurq::components::Spacer::new().frame(FrameConstraints {
        width: Some(lurq::node::dimension::Dimension::Px(50.0)),
        height: Some(lurq::node::dimension::Dimension::Px(50.0)),
        ..Default::default()
      }),
    ],
  );
  rt.set_root(node);
  let result = rt.pass_layout(Constraints::loose(Size::new(400.0, 400.0))).unwrap();
  assert_eq!(result.children[1].offset.x, 150.0);
  assert_eq!(result.children[1].offset.y, 150.0);
}

#[test]
fn stack_top_center() {
  let mut rt = rt();
  let node = lurq::components::Stack::with(
    StackAlignment::TopCenter,
    vec![
      lurq::components::Spacer::new().frame(FrameConstraints {
        width: Some(lurq::node::dimension::Dimension::Px(200.0)),
        height: Some(lurq::node::dimension::Dimension::Px(200.0)),
        ..Default::default()
      }),
      lurq::components::Spacer::new().frame(FrameConstraints {
        width: Some(lurq::node::dimension::Dimension::Px(50.0)),
        height: Some(lurq::node::dimension::Dimension::Px(50.0)),
        ..Default::default()
      }),
    ],
  );
  rt.set_root(node);
  let result = rt.pass_layout(Constraints::loose(Size::new(400.0, 400.0))).unwrap();
  assert_eq!(result.children[1].offset.x, 75.0);
  assert_eq!(result.children[1].offset.y, 0.0);
}

#[test]
fn stack_bottom_start() {
  let mut rt = rt();
  let node = lurq::components::Stack::with(
    StackAlignment::BottomStart,
    vec![
      lurq::components::Spacer::new().frame(FrameConstraints {
        width: Some(lurq::node::dimension::Dimension::Px(200.0)),
        height: Some(lurq::node::dimension::Dimension::Px(200.0)),
        ..Default::default()
      }),
      lurq::components::Spacer::new().frame(FrameConstraints {
        width: Some(lurq::node::dimension::Dimension::Px(50.0)),
        height: Some(lurq::node::dimension::Dimension::Px(50.0)),
        ..Default::default()
      }),
    ],
  );
  rt.set_root(node);
  let result = rt.pass_layout(Constraints::loose(Size::new(400.0, 400.0))).unwrap();
  assert_eq!(result.children[1].offset.x, 0.0);
  assert_eq!(result.children[1].offset.y, 150.0);
}

#[test]
fn stack_per_child_align_override() {
  let mut rt = rt();
  let node = lurq::components::Stack::with(
    StackAlignment::TopStart,
    vec![
      lurq::components::Spacer::new().frame(FrameConstraints {
        width: Some(lurq::node::dimension::Dimension::Px(200.0)),
        height: Some(lurq::node::dimension::Dimension::Px(200.0)),
        ..Default::default()
      }),
      lurq::components::Spacer::new()
        .frame(FrameConstraints {
          width: Some(lurq::node::dimension::Dimension::Px(50.0)),
          height: Some(lurq::node::dimension::Dimension::Px(50.0)),
          ..Default::default()
        })
        .align(Alignment::End),
    ],
  );
  rt.set_root(node);
  let result = rt.pass_layout(Constraints::loose(Size::new(400.0, 400.0))).unwrap();
  assert_eq!(result.children[0].offset.x, 0.0);
  assert_eq!(result.children[0].offset.y, 0.0);
  // Alignment::End maps to BottomEnd in stack context
  assert_eq!(result.children[1].offset.x, 150.0);
  assert_eq!(result.children[1].offset.y, 150.0);
}

#[test]
fn stack_empty() {
  let mut rt = rt();
  let node = lurq::components::Stack::new().stack_align(StackAlignment::Center);
  rt.set_root(node);
  let result = rt.pass_layout(Constraints::loose(Size::new(400.0, 400.0))).unwrap();
  assert_eq!(result.size.width, 0.0);
  assert_eq!(result.size.height, 0.0);
}

#[test]
fn an_absolute_child_in_percent_of_an_unbounded_stack_is_a_share_of_the_stack_as_laid_out() {
  use lurq::node::dimension::Dimension;
  let mut rt = rt();
  // What lays the stack out bounds neither way: a share of nothing would be nothing, so it is a share of the stack.
  let node = lurq::components::Stack::new()
    .child(lurq::components::Rect::new(300.0, 200.0))
    .child(lurq::components::Stack::new().absolute(0.0, 0.0, Dimension::Pct(100.0), Dimension::Pct(50.0)));
  rt.set_root(node);
  let result = rt
    .pass_layout(Constraints::loose(Size::new(f32::INFINITY, f32::INFINITY)))
    .unwrap();
  assert_eq!((result.size.width, result.size.height), (300.0, 200.0));
  let layer = &result.children[1].result.size;
  assert_eq!((layer.width, layer.height), (300.0, 100.0));
}

#[test]
fn an_absolute_child_in_percent_of_a_bounded_stack_is_a_share_of_its_bounds_as_before() {
  use lurq::node::dimension::Dimension;
  let mut rt = rt();
  let node = lurq::components::Stack::new()
    .child(lurq::components::Rect::new(300.0, 200.0))
    .child(lurq::components::Stack::new().absolute(0.0, 0.0, Dimension::Pct(100.0), Dimension::Pct(100.0)));
  rt.set_root(node);
  let result = rt.pass_layout(Constraints::loose(Size::new(400.0, 500.0))).unwrap();
  let layer = &result.children[1].result.size;
  assert_eq!((layer.width, layer.height), (400.0, 500.0));
}
