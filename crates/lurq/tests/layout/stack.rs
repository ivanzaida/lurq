use lurq::{
  app::Runtime,
  layout::{Alignment, Constraints, Size, StackAlignment, layout_kind::FrameConstraints},
  node::Element,
};

fn rt() -> Runtime {
  Runtime::new()
}

#[test]
fn stack_sizes_to_largest_child() {
  let mut rt = rt();
  let node = Element::stack_with(
    StackAlignment::Center,
    vec![
      Element::new().frame(FrameConstraints {
        width: Some(200.0),
        height: Some(100.0),
        ..Default::default()
      }),
      Element::new().frame(FrameConstraints {
        width: Some(50.0),
        height: Some(50.0),
        ..Default::default()
      }),
    ],
  );
  rt.set_root(node);
  let result = rt.compute_layout(Constraints::loose(Size::new(400.0, 400.0))).unwrap();
  assert_eq!(result.size.width, 200.0);
  assert_eq!(result.size.height, 100.0);
}

#[test]
fn stack_center_alignment() {
  let mut rt = rt();
  let node = Element::stack_with(
    StackAlignment::Center,
    vec![
      Element::new().frame(FrameConstraints {
        width: Some(200.0),
        height: Some(200.0),
        ..Default::default()
      }),
      Element::new().frame(FrameConstraints {
        width: Some(50.0),
        height: Some(50.0),
        ..Default::default()
      }),
    ],
  );
  rt.set_root(node);
  let result = rt.compute_layout(Constraints::loose(Size::new(400.0, 400.0))).unwrap();
  assert_eq!(result.children[1].offset.x, 75.0); // (200-50)/2
  assert_eq!(result.children[1].offset.y, 75.0);
}

#[test]
fn stack_top_start() {
  let mut rt = rt();
  let node = Element::stack_with(
    StackAlignment::TopStart,
    vec![
      Element::new().frame(FrameConstraints {
        width: Some(200.0),
        height: Some(200.0),
        ..Default::default()
      }),
      Element::new().frame(FrameConstraints {
        width: Some(50.0),
        height: Some(50.0),
        ..Default::default()
      }),
    ],
  );
  rt.set_root(node);
  let result = rt.compute_layout(Constraints::loose(Size::new(400.0, 400.0))).unwrap();
  assert_eq!(result.children[1].offset.x, 0.0);
  assert_eq!(result.children[1].offset.y, 0.0);
}

#[test]
fn absolute_child_does_not_affect_stack_size() {
  let mut rt = rt();
  let node = Element::stack()
    .child(Element::rect(10.0, 20.0))
    .child(Element::rect(100.0, 120.0).absolute(30.0, 40.0, 100.0, 120.0));
  rt.set_root(node);
  let result = rt.compute_layout(Constraints::loose(Size::new(500.0, 500.0))).unwrap();

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
  let node = Element::stack_with(
    StackAlignment::BottomEnd,
    vec![
      Element::new().frame(FrameConstraints {
        width: Some(200.0),
        height: Some(200.0),
        ..Default::default()
      }),
      Element::new().frame(FrameConstraints {
        width: Some(50.0),
        height: Some(50.0),
        ..Default::default()
      }),
    ],
  );
  rt.set_root(node);
  let result = rt.compute_layout(Constraints::loose(Size::new(400.0, 400.0))).unwrap();
  assert_eq!(result.children[1].offset.x, 150.0);
  assert_eq!(result.children[1].offset.y, 150.0);
}

#[test]
fn stack_top_center() {
  let mut rt = rt();
  let node = Element::stack_with(
    StackAlignment::TopCenter,
    vec![
      Element::new().frame(FrameConstraints {
        width: Some(200.0),
        height: Some(200.0),
        ..Default::default()
      }),
      Element::new().frame(FrameConstraints {
        width: Some(50.0),
        height: Some(50.0),
        ..Default::default()
      }),
    ],
  );
  rt.set_root(node);
  let result = rt.compute_layout(Constraints::loose(Size::new(400.0, 400.0))).unwrap();
  assert_eq!(result.children[1].offset.x, 75.0);
  assert_eq!(result.children[1].offset.y, 0.0);
}

#[test]
fn stack_bottom_start() {
  let mut rt = rt();
  let node = Element::stack_with(
    StackAlignment::BottomStart,
    vec![
      Element::new().frame(FrameConstraints {
        width: Some(200.0),
        height: Some(200.0),
        ..Default::default()
      }),
      Element::new().frame(FrameConstraints {
        width: Some(50.0),
        height: Some(50.0),
        ..Default::default()
      }),
    ],
  );
  rt.set_root(node);
  let result = rt.compute_layout(Constraints::loose(Size::new(400.0, 400.0))).unwrap();
  assert_eq!(result.children[1].offset.x, 0.0);
  assert_eq!(result.children[1].offset.y, 150.0);
}

#[test]
fn stack_per_child_align_override() {
  let mut rt = rt();
  let node = Element::stack_with(
    StackAlignment::TopStart,
    vec![
      Element::new().frame(FrameConstraints {
        width: Some(200.0),
        height: Some(200.0),
        ..Default::default()
      }),
      Element::new()
        .frame(FrameConstraints {
          width: Some(50.0),
          height: Some(50.0),
          ..Default::default()
        })
        .align(Alignment::End),
    ],
  );
  rt.set_root(node);
  let result = rt.compute_layout(Constraints::loose(Size::new(400.0, 400.0))).unwrap();
  assert_eq!(result.children[0].offset.x, 0.0);
  assert_eq!(result.children[0].offset.y, 0.0);
  // Alignment::End maps to BottomEnd in stack context
  assert_eq!(result.children[1].offset.x, 150.0);
  assert_eq!(result.children[1].offset.y, 150.0);
}

#[test]
fn stack_empty() {
  let mut rt = rt();
  let node = Element::stack_with(StackAlignment::Center, vec![]);
  rt.set_root(node);
  let result = rt.compute_layout(Constraints::loose(Size::new(400.0, 400.0))).unwrap();
  assert_eq!(result.size.width, 0.0);
  assert_eq!(result.size.height, 0.0);
}
