use lurq::{
  app::Tree,
  layout::{Alignment, Constraints, Size, layout_kind::FrameConstraints},
};

use super::PassLayoutExt;

fn rt() -> Tree {
  Tree::new()
}

#[test]
fn empty_row() {
  let mut rt = rt();
  let node = lurq::components::Row::new().spacing(0.0).align_items(Alignment::Start);
  rt.set_root(node);
  let result = rt.pass_layout(Constraints::loose(Size::new(400.0, 300.0))).unwrap();
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
  let result = rt.pass_layout(Constraints::loose(Size::new(400.0, 300.0))).unwrap();
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
  let result = rt.pass_layout(Constraints::loose(Size::new(400.0, 300.0))).unwrap();
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
  let result = rt.pass_layout(Constraints::loose(Size::new(400.0, 300.0))).unwrap();
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
  let result = rt.pass_layout(Constraints::loose(Size::new(400.0, 300.0))).unwrap();
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
  let result = rt.pass_layout(Constraints::loose(Size::new(400.0, 300.0))).unwrap();
  assert_eq!(result.size.width, 100.0);
  assert_eq!(result.size.height, 50.0);
  assert_eq!(result.children[0].offset.x, 0.0);
}

#[test]
fn row_default_wrapping_text_uses_intrinsic_width_without_child_constraint() {
  let mut rt = rt();
  let node = lurq::components::Row::new()
    .width(lurq::node::dimension::Dimension::Pct(100.0))
    .align_items(Alignment::Center)
    .spacing(12.0)
    .child(
      lurq::components::Row::new()
        .align_items(Alignment::Center)
        .spacing(7.0)
        .padding_vertical(5.0)
        .padding_horizontal(9.0)
        .child(lurq::components::Text::new("STARTUP UPDATE")),
    )
    .child(lurq::components::Text::new("0.10.9 -> 0.10.10"));

  rt.set_root(node);
  let result = rt.pass_layout(Constraints::tight(Size::new(180.0, 80.0))).unwrap();
  let badge = &result.children[0].result;
  let version = &result.children[1].result;

  assert_eq!(result.size.width, 180.0);
  assert!(
    version.size.width > result.size.width - badge.size.width - 12.0,
    "text should keep its intrinsic width when row does not constrain the child"
  );
  assert!(
    version.size.height <= badge.size.height,
    "unconstrained text in a row should not soft-wrap to a taller line: version={}, badge={}",
    version.size.height,
    badge.size.height
  );
}
