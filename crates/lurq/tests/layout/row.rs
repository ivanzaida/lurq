use lurq::{
  app::Tree,
  layout::{Alignment, Constraints, Size, layout_kind::FrameConstraints, quad::QuadContent},
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

#[test]
fn row_default_wrapping_text_emits_unwrapped_quad_when_layout_is_unbounded() {
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
    .child(lurq::components::Text::new("0.1.8 -> 99.99.99"));

  rt.set_root(node);
  let result = rt.pass_layout(Constraints::tight(Size::new(180.0, 80.0))).unwrap();
  let quads = rt.resolve_quads(&result);
  let version = quads
    .iter()
    .find_map(|quad| match &quad.content {
      QuadContent::Text { text, wrap, .. } if text == "0.1.8 -> 99.99.99" => Some(wrap),
      _ => None,
    })
    .expect("version text quad should be emitted");

  assert!(!version, "unbounded row text should render without soft-wrap");
}

/// A fit-content child of a wrapping row (e.g. a value pill: text + glyph)
/// must measure its text at intrinsic width — the same as it would in a
/// non-wrapping row.
#[test]
fn row_wrap_keeps_text_width_in_fit_content_children() {
  let pill = || {
    lurq::components::Row::new()
      .align_items(Alignment::Center)
      .spacing(6.0)
      .padding_vertical(5.0)
      .padding_horizontal(9.0)
      .child(lurq::components::Text::new("11208"))
      .child(lurq::components::Text::new("x"))
  };

  let mut plain = rt();
  plain.set_root(lurq::components::Row::new().spacing(7.0).child(pill()));
  let plain_result = plain.pass_layout(Constraints::tight(Size::new(400.0, 100.0))).unwrap();
  let plain_pill = plain_result.children[0].result.size;

  let mut wrapped = rt();
  wrapped.set_root(lurq::components::Row::new().wrap().spacing(7.0).child(pill()));
  let wrap_result = wrapped
    .pass_layout(Constraints::tight(Size::new(400.0, 100.0)))
    .unwrap();
  let wrap_pill = wrap_result.children[0].result.size;

  assert!(
    plain_pill.width > 20.0,
    "the pill must have measurable text: {}x{}",
    plain_pill.width,
    plain_pill.height
  );
  assert_eq!(
    wrap_pill.width, plain_pill.width,
    "a wrapped row must not change a fit-content child's measured width"
  );
  assert_eq!(wrap_pill.height, plain_pill.height);
}
