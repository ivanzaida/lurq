use lurq::{
  app::Runtime,
  layout::{Alignment, Constraints, Size, StackAlignment, layout_kind::FrameConstraints},
  node::{Element, dimension::Dimension, padding::Padding},
};

fn rt() -> Runtime {
  Runtime::new()
}

#[test]
fn column_of_rows() {
  let mut rt = rt();
  let node = lurq::components::Column::with(
    10.0,
    Alignment::Start,
    vec![
      Element::from(lurq::components::Row::with(
        5.0,
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
        ],
      )),
      Element::from(lurq::components::Row::with(
        5.0,
        Alignment::Start,
        vec![
          lurq::components::Spacer::new().frame(FrameConstraints {
            width: Some(lurq::node::dimension::Dimension::Px(60.0)),
            height: Some(lurq::node::dimension::Dimension::Px(40.0)),
            ..Default::default()
          }),
          lurq::components::Spacer::new().frame(FrameConstraints {
            width: Some(lurq::node::dimension::Dimension::Px(60.0)),
            height: Some(lurq::node::dimension::Dimension::Px(40.0)),
            ..Default::default()
          }),
        ],
      )),
    ],
  );
  rt.set_root(node);
  let result = rt.compute_layout(Constraints::loose(Size::new(400.0, 400.0))).unwrap();
  assert_eq!(result.size.width, 125.0); // max(50+5+50, 60+5+60) = 125
  assert_eq!(result.size.height, 80.0); // 30 + 10 + 40
  assert_eq!(result.children[0].offset.y, 0.0);
  assert_eq!(result.children[1].offset.y, 40.0); // 30 + 10
}

#[test]
fn row_of_columns() {
  let mut rt = rt();
  let node = lurq::components::Row::with(
    10.0,
    Alignment::Start,
    vec![
      Element::from(lurq::components::Column::with(
        5.0,
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
        ],
      )),
      Element::from(lurq::components::Column::with(
        5.0,
        Alignment::Start,
        vec![
          lurq::components::Spacer::new().frame(FrameConstraints {
            width: Some(lurq::node::dimension::Dimension::Px(60.0)),
            height: Some(lurq::node::dimension::Dimension::Px(20.0)),
            ..Default::default()
          }),
          lurq::components::Spacer::new().frame(FrameConstraints {
            width: Some(lurq::node::dimension::Dimension::Px(60.0)),
            height: Some(lurq::node::dimension::Dimension::Px(20.0)),
            ..Default::default()
          }),
        ],
      )),
    ],
  );
  rt.set_root(node);
  let result = rt.compute_layout(Constraints::loose(Size::new(400.0, 400.0))).unwrap();
  assert_eq!(result.size.width, 120.0); // 50 + 10 + 60
  assert_eq!(result.size.height, 65.0); // max(30+5+30, 20+5+20) = 65
}

#[test]
fn padding_inside_row() {
  let mut rt = rt();
  let node = lurq::components::Row::with(
    0.0,
    Alignment::Start,
    vec![
      lurq::components::Spacer::new()
        .frame(FrameConstraints {
          width: Some(lurq::node::dimension::Dimension::Px(80.0)),
          height: Some(lurq::node::dimension::Dimension::Px(40.0)),
          ..Default::default()
        })
        .padding(Padding::all(Dimension::Px(10.0))),
      lurq::components::Spacer::new().frame(FrameConstraints {
        width: Some(lurq::node::dimension::Dimension::Px(80.0)),
        height: Some(lurq::node::dimension::Dimension::Px(40.0)),
        ..Default::default()
      }),
    ],
  );
  rt.set_root(node);
  let result = rt.compute_layout(Constraints::loose(Size::new(400.0, 400.0))).unwrap();
  assert_eq!(result.size.width, 180.0); // (80+20) + 80
  assert_eq!(result.children[0].result.size.width, 100.0);
  assert_eq!(result.children[1].offset.x, 100.0);
}

#[test]
fn frame_inside_padding() {
  let mut rt = rt();
  let node = lurq::components::Spacer::new()
    .frame(FrameConstraints {
      width: Some(lurq::node::dimension::Dimension::Px(100.0)),
      height: Some(lurq::node::dimension::Dimension::Px(50.0)),
      ..Default::default()
    })
    .padding(Padding::all(Dimension::Px(20.0)));
  rt.set_root(node);
  let result = rt.compute_layout(Constraints::loose(Size::new(400.0, 400.0))).unwrap();
  assert_eq!(result.size.width, 140.0);
  assert_eq!(result.size.height, 90.0);
  assert_eq!(result.children[0].offset.x, 20.0);
  assert_eq!(result.children[0].offset.y, 20.0);
}

#[test]
fn stack_inside_column() {
  let mut rt = rt();
  let node = lurq::components::Column::with(
    0.0,
    Alignment::Start,
    vec![
      Element::from(lurq::components::Stack::with(
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
      )),
      Element::from(lurq::components::Spacer::new().frame(FrameConstraints {
        width: Some(lurq::node::dimension::Dimension::Px(100.0)),
        height: Some(lurq::node::dimension::Dimension::Px(30.0)),
        ..Default::default()
      })),
    ],
  );
  rt.set_root(node);
  let result = rt.compute_layout(Constraints::loose(Size::new(400.0, 400.0))).unwrap();
  assert_eq!(result.size.height, 130.0); // 100 + 30
  assert_eq!(result.children[1].offset.y, 100.0);
}

#[test]
fn flex_children_in_nested_row() {
  let mut rt = rt();
  let node = lurq::components::Row::with(
    0.0,
    Alignment::Start,
    vec![
      Element::from(lurq::components::Spacer::new().frame(FrameConstraints {
        width: Some(lurq::node::dimension::Dimension::Px(100.0)),
        height: Some(lurq::node::dimension::Dimension::Px(50.0)),
        ..Default::default()
      })),
      Element::from(
        lurq::components::Row::with(
          0.0,
          Alignment::Start,
          vec![
            lurq::components::Spacer::new().flex(1.0),
            lurq::components::Spacer::new().flex(1.0),
          ],
        )
        .flex(1.0),
      ),
    ],
  );
  rt.set_root(node);
  let result = rt.compute_layout(Constraints::tight(Size::new(400.0, 100.0))).unwrap();
  assert_eq!(result.children[0].result.size.width, 100.0);
  assert_eq!(result.children[1].result.size.width, 300.0);
}

#[test]
fn deeply_nested_padding() {
  let mut rt = rt();
  let node = lurq::components::Spacer::new()
    .frame(FrameConstraints {
      width: Some(lurq::node::dimension::Dimension::Px(50.0)),
      height: Some(lurq::node::dimension::Dimension::Px(50.0)),
      ..Default::default()
    })
    .padding(Padding::all(Dimension::Px(10.0)))
    .padding(Padding::all(Dimension::Px(10.0)))
    .padding(Padding::all(Dimension::Px(10.0)));
  rt.set_root(node);
  let result = rt.compute_layout(Constraints::loose(Size::new(400.0, 400.0))).unwrap();
  assert_eq!(result.size.width, 110.0); // 50 + 10*2*3
  assert_eq!(result.size.height, 110.0);
}

#[test]
fn offset_inside_row_does_not_affect_siblings() {
  let mut rt = rt();
  let node = lurq::components::Row::with(
    0.0,
    Alignment::Start,
    vec![
      lurq::components::Spacer::new()
        .frame(FrameConstraints {
          width: Some(lurq::node::dimension::Dimension::Px(50.0)),
          height: Some(lurq::node::dimension::Dimension::Px(50.0)),
          ..Default::default()
        })
        .offset(100.0, 100.0),
      lurq::components::Spacer::new().frame(FrameConstraints {
        width: Some(lurq::node::dimension::Dimension::Px(50.0)),
        height: Some(lurq::node::dimension::Dimension::Px(50.0)),
        ..Default::default()
      }),
    ],
  );
  rt.set_root(node);
  let result = rt.compute_layout(Constraints::loose(Size::new(400.0, 400.0))).unwrap();
  // offset doesn't change the size of the first child in the row
  assert_eq!(result.children[1].offset.x, 50.0);
}
