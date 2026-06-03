use lurq::{
  app::Tree,
  layout::{
    Alignment, Constraints, Size, StackAlignment, layout_kind::FrameConstraints, quad::QuadContent,
    text_style::TextStyle,
  },
  node::{Element, TextTransformMode, color::Color, dimension::Dimension, padding::Padding, transform::Transform2D},
};

use super::PassLayoutExt;
use crate::support::{render_pass, run_pass};

fn rt() -> Tree {
  Tree::new()
}

#[test]
fn no_quads_for_invisible_nodes() {
  let mut rt = rt();
  let node = lurq::components::Spacer::new().frame(FrameConstraints {
    width: Some(lurq::node::dimension::Dimension::Px(100.0)),
    height: Some(lurq::node::dimension::Dimension::Px(50.0)),
    ..Default::default()
  });
  rt.set_root(node);
  let result = rt.pass_layout(Constraints::loose(Size::new(400.0, 400.0))).unwrap();
  let quads = rt.resolve_quads(&result);
  assert!(quads.is_empty());
}

#[test]
fn background_produces_rect_quad() {
  let mut rt = rt();
  let node = lurq::components::Spacer::new()
    .frame(FrameConstraints {
      width: Some(lurq::node::dimension::Dimension::Px(100.0)),
      height: Some(lurq::node::dimension::Dimension::Px(50.0)),
      ..Default::default()
    })
    .background(Color::new(255, 0, 0, 255));
  rt.set_root(node);
  let result = rt.pass_layout(Constraints::loose(Size::new(400.0, 400.0))).unwrap();
  let quads = rt.resolve_quads(&result);
  assert_eq!(quads.len(), 1);
  assert_eq!(quads[0].x, 0.0);
  assert_eq!(quads[0].y, 0.0);
  assert_eq!(quads[0].width, 100.0);
  assert_eq!(quads[0].height, 50.0);
  assert!(matches!(quads[0].content, QuadContent::Rect { .. }));
}

#[cfg(feature = "image")]
#[test]
fn image_width_preserves_intrinsic_aspect_ratio() {
  let mut rt = rt();
  let img = lurq::images::ImageData::from_rgba(vec![255; 200 * 100 * 4], 200, 100);
  rt.set_root(lurq::components::Image::new(img).width(100.0));

  let result = rt.pass_layout(Constraints::loose(Size::new(400.0, 400.0))).unwrap();
  assert_eq!(result.size.width, 100.0);
  assert_eq!(result.size.height, 50.0);
}

#[cfg(feature = "image")]
#[test]
fn image_height_preserves_intrinsic_aspect_ratio() {
  let mut rt = rt();
  let img = lurq::images::ImageData::from_rgba(vec![255; 200 * 100 * 4], 200, 100);
  rt.set_root(lurq::components::Image::new(img).height(25.0));

  let result = rt.pass_layout(Constraints::loose(Size::new(400.0, 400.0))).unwrap();
  assert_eq!(result.size.width, 50.0);
  assert_eq!(result.size.height, 25.0);
}

#[cfg(feature = "image")]
#[test]
fn background_cover_crops_with_uvs() {
  let mut rt = rt();
  let img = lurq::images::ImageData::from_rgba(vec![255; 200 * 100 * 4], 200, 100);
  rt.set_root(
    lurq::components::Spacer::new()
      .size(100.0, 100.0)
      .background_image(img)
      .background_cover(),
  );

  let result = rt.pass_layout(Constraints::loose(Size::new(400.0, 400.0))).unwrap();
  let quads = rt.resolve_quads(&result);
  assert_eq!(quads.len(), 1);
  match quads[0].content {
    QuadContent::Image { uv_min, uv_max, .. } => {
      assert_eq!(uv_min, [0.25, 0.0]);
      assert_eq!(uv_max, [0.75, 1.0]);
    }
    _ => panic!("expected image quad"),
  }
}

#[cfg(feature = "image")]
#[test]
fn background_contain_fits_inside_box() {
  let mut rt = rt();
  let img = lurq::images::ImageData::from_rgba(vec![255; 200 * 100 * 4], 200, 100);
  rt.set_root(
    lurq::components::Spacer::new()
      .size(100.0, 100.0)
      .background_image(img)
      .background_contain(),
  );

  let result = rt.pass_layout(Constraints::loose(Size::new(400.0, 400.0))).unwrap();
  let quads = rt.resolve_quads(&result);
  assert_eq!(quads.len(), 1);
  assert_eq!(quads[0].x, 0.0);
  assert_eq!(quads[0].y, 25.0);
  assert_eq!(quads[0].width, 100.0);
  assert_eq!(quads[0].height, 50.0);
}

#[test]
fn text_produces_text_quad() {
  let mut rt = rt();
  let node = lurq::components::Text::new("hello");
  rt.set_root(node);
  let result = rt.pass_layout(Constraints::loose(Size::new(400.0, 400.0))).unwrap();
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
  let node = lurq::components::Row::with(
    10.0,
    Alignment::Start,
    vec![
      lurq::components::Spacer::new()
        .frame(FrameConstraints {
          width: Some(lurq::node::dimension::Dimension::Px(50.0)),
          height: Some(lurq::node::dimension::Dimension::Px(30.0)),
          ..Default::default()
        })
        .background(Color::new(255, 0, 0, 255)),
      lurq::components::Spacer::new()
        .frame(FrameConstraints {
          width: Some(lurq::node::dimension::Dimension::Px(50.0)),
          height: Some(lurq::node::dimension::Dimension::Px(30.0)),
          ..Default::default()
        })
        .background(Color::new(0, 255, 0, 255)),
    ],
  );
  rt.set_root(node);
  let result = rt.pass_layout(Constraints::loose(Size::new(400.0, 400.0))).unwrap();
  let quads = rt.resolve_quads(&result);
  assert_eq!(quads.len(), 2);
  assert_eq!(quads[0].x, 0.0);
  assert_eq!(quads[1].x, 60.0); // 50 + 10 spacing
}

#[test]
fn quads_absolute_positions_in_column() {
  let mut rt = rt();
  let node = lurq::components::Column::with(
    5.0,
    Alignment::Start,
    vec![
      lurq::components::Spacer::new()
        .frame(FrameConstraints {
          width: Some(lurq::node::dimension::Dimension::Px(100.0)),
          height: Some(lurq::node::dimension::Dimension::Px(40.0)),
          ..Default::default()
        })
        .background(Color::new(255, 0, 0, 255)),
      lurq::components::Spacer::new()
        .frame(FrameConstraints {
          width: Some(lurq::node::dimension::Dimension::Px(100.0)),
          height: Some(lurq::node::dimension::Dimension::Px(40.0)),
          ..Default::default()
        })
        .background(Color::new(0, 255, 0, 255)),
    ],
  );
  rt.set_root(node);
  let result = rt.pass_layout(Constraints::loose(Size::new(400.0, 400.0))).unwrap();
  let quads = rt.resolve_quads(&result);
  assert_eq!(quads.len(), 2);
  assert_eq!(quads[0].y, 0.0);
  assert_eq!(quads[1].y, 45.0); // 40 + 5 spacing
}

#[test]
fn default_overflow_clips_children() {
  let mut rt = rt();
  let node = lurq::components::Row::new()
    .child(lurq::components::Rect::new(200.0, 50.0).background("#ff0000"))
    .width(100.0)
    .height(50.0)
    .background("#000000");
  rt.set_root(node);
  let result = rt.pass_layout(Constraints::tight(Size::new(100.0, 50.0))).unwrap();
  let quads = rt.resolve_quads(&result);
  assert!(quads[1].clip.active);
  assert_eq!(quads[1].clip.width, 100.0);
  assert_eq!(quads[1].clip.height, 50.0);
}

#[test]
fn overflow_visible_allows_children_to_escape() {
  let mut rt = rt();
  let node = lurq::components::Row::new()
    .child(lurq::components::Rect::new(200.0, 50.0).background("#ff0000"))
    .width(100.0)
    .height(50.0)
    .background("#000000")
    .overflow_visible();
  rt.set_root(node);
  let result = rt.pass_layout(Constraints::tight(Size::new(100.0, 50.0))).unwrap();
  let quads = rt.resolve_quads(&result);
  assert!(!quads[1].clip.active);
}

#[test]
fn offset_visuals_move_with_the_offset_and_clip_by_default() {
  let mut rt = rt();
  let node = lurq::components::Stack::new().size(100.0, 100.0).child(
    lurq::components::Rect::new(80.0, 40.0)
      .background("#ff0000")
      .offset(20.0, 10.0)
      .absolute_position(0.0, 0.0),
  );
  rt.set_root(node);
  let result = rt.pass_layout(Constraints::tight(Size::new(100.0, 100.0))).unwrap();
  let quads = rt.resolve_quads(&result);
  let rect = quads
    .iter()
    .find(|quad| match &quad.content {
      QuadContent::Rect { color } => *color == Color::from_hex("#ff0000"),
      _ => false,
    })
    .expect("expected shifted child rect");

  assert_eq!(rect.x, 20.0);
  assert_eq!(rect.y, 10.0);
  assert!(rect.clip.active);
  assert_eq!(rect.clip.width, 80.0);
  assert_eq!(rect.clip.height, 40.0);
}

#[test]
fn container_children_can_be_added_after_frame_modifiers() {
  let mut rt = rt();
  let node = lurq::components::Stack::new()
    .size(100.0, 100.0)
    .background("#000000")
    .child(lurq::components::Rect::new(20.0, 20.0).background("#ff0000"));
  rt.set_root(node);
  let result = rt.pass_layout(Constraints::tight(Size::new(100.0, 100.0))).unwrap();
  let quads = rt.resolve_quads(&result);
  assert!(quads.iter().any(|quad| match &quad.content {
    QuadContent::Rect { color } => *color == Color::from_hex("#ff0000"),
    _ => false,
  }));
}

#[test]
fn container_props_can_be_set_after_frame_modifiers() {
  let mut rt = rt();
  let node = lurq::components::Stack::new()
    .size(100.0, 100.0)
    .stack_align(StackAlignment::BottomEnd)
    .child(lurq::components::Rect::new(20.0, 20.0).background("#ff0000"));
  rt.set_root(node);
  let result = rt.pass_layout(Constraints::tight(Size::new(100.0, 100.0))).unwrap();
  let quads = rt.resolve_quads(&result);
  let child = quads
    .iter()
    .find(|quad| match &quad.content {
      QuadContent::Rect { color } => *color == Color::from_hex("#ff0000"),
      _ => false,
    })
    .unwrap();
  assert_eq!(child.x, 80.0);
  assert_eq!(child.y, 80.0);
}

#[test]
fn nested_default_overflow_intersects_text_clip() {
  let mut rt = rt();
  let node = lurq::components::Row::new()
    .child(lurq::components::Rect::new(100.0, 40.0).background("#0000ff"))
    .child(
      lurq::components::Row::new()
        .child(lurq::components::Text::new("B"))
        .size(50.0, 40.0)
        .background("#ff0000"),
    )
    .width(100.0)
    .height(40.0)
    .background("#000000");
  rt.set_root(node);
  let result = rt.pass_layout(Constraints::tight(Size::new(100.0, 40.0))).unwrap();
  let quads = rt.resolve_quads(&result);
  assert!(
    !quads
      .iter()
      .any(|quad| matches!(quad.content, QuadContent::Text { .. })),
    "fully clipped nested text should be culled"
  );
}

#[test]
fn quads_nested_absolute_positions() {
  let mut rt = rt();
  let node = lurq::components::Column::with(
    0.0,
    Alignment::Start,
    vec![
      Element::from(lurq::components::Spacer::new().frame(FrameConstraints {
        width: Some(lurq::node::dimension::Dimension::Px(100.0)),
        height: Some(lurq::node::dimension::Dimension::Px(50.0)),
        ..Default::default()
      })),
      Element::from(lurq::components::Row::with(
        0.0,
        Alignment::Start,
        vec![
          lurq::components::Spacer::new().frame(FrameConstraints {
            width: Some(lurq::node::dimension::Dimension::Px(40.0)),
            height: Some(lurq::node::dimension::Dimension::Px(30.0)),
            ..Default::default()
          }),
          lurq::components::Spacer::new()
            .frame(FrameConstraints {
              width: Some(lurq::node::dimension::Dimension::Px(40.0)),
              height: Some(lurq::node::dimension::Dimension::Px(30.0)),
              ..Default::default()
            })
            .background(Color::new(0, 0, 255, 255)),
        ],
      )),
    ],
  );
  rt.set_root(node);
  let result = rt.pass_layout(Constraints::loose(Size::new(400.0, 400.0))).unwrap();
  let quads = rt.resolve_quads(&result);
  assert_eq!(quads.len(), 1);
  assert_eq!(quads[0].x, 40.0);
  assert_eq!(quads[0].y, 50.0);
}

#[test]
fn quads_with_padding_offset() {
  let mut rt = rt();
  let node = lurq::components::Spacer::new()
    .frame(FrameConstraints {
      width: Some(lurq::node::dimension::Dimension::Px(60.0)),
      height: Some(lurq::node::dimension::Dimension::Px(40.0)),
      ..Default::default()
    })
    .background(Color::new(255, 0, 0, 255))
    .padding(Padding::all(Dimension::Px(20.0)));
  rt.set_root(node);
  let result = rt.pass_layout(Constraints::loose(Size::new(400.0, 400.0))).unwrap();
  let quads = rt.resolve_quads(&result);
  assert_eq!(quads.len(), 1);
  assert_eq!(quads[0].x, 0.0);
  assert_eq!(quads[0].y, 0.0);
  assert_eq!(quads[0].width, 100.0);
  assert_eq!(quads[0].height, 80.0);
}

fn assert_close(actual: f32, expected: f32) {
  assert!(
    (actual - expected).abs() < 0.01,
    "expected {actual} to be within 0.01 of {expected}"
  );
}

#[test]
fn transformed_padding_child_transforms_padding_offset() {
  let mut rt = rt();
  let node = lurq::components::Stack::new()
    .child(lurq::components::Rect::new(20.0, 20.0).background("#0000ff"))
    .padding(Padding {
      top: Dimension::Px(10.0).into(),
      right: Dimension::Px(0.0).into(),
      bottom: Dimension::Px(0.0).into(),
      left: Dimension::Px(30.0).into(),
    })
    .transform(Transform2D::rotate_deg(30.0));

  rt.set_root(node);
  let result = rt.pass_layout(Constraints::loose(Size::new(400.0, 400.0))).unwrap();
  let quads = rt.resolve_quads(&result);
  let child = quads
    .iter()
    .find(|quad| matches!(&quad.content, QuadContent::Rect { color } if *color == Color::from_hex("#0000ff")))
    .expect("child quad");

  assert_close(child.x, 31.83);
  assert_close(child.y, 13.17);
  assert_eq!(child.transform_origin, Some([0.0, 0.0]));
}

#[test]
fn transformed_padded_text_sends_glyphs_with_parent_transform() {
  let mut rt = rt();
  let transform = Transform2D::rotate_deg(-2.0).then(&Transform2D::scale(1.02, 1.02));
  let node = lurq::components::Column::new()
    .child(
      lurq::components::Text::new(
        "This selectable text lives inside a transformed parent.\nIts selection highlight inherits the same parent transform.",
      )
      .width(430.0),
    )
    .padding(14.0)
    .width(480.0)
    .background("#172033")
    .border_inside(1.0, Color::from_hex("#475569"))
    .rounded(8.0)
    .transform(transform)
    .overflow_visible();

  rt.set_root(node);
  let result = rt.pass_layout(Constraints::loose(Size::new(800.0, 400.0))).unwrap();
  let quads = rt.resolve_quads(&result);
  let text_quad = quads
    .iter()
    .find(|quad| matches!(quad.content, QuadContent::Text { .. }))
    .expect("text quad");
  let origin = [240.0, result.size.height * 0.5];
  let dx = 14.0 - origin[0];
  let dy = 14.0 - origin[1];
  let expected = (
    origin[0] + transform.a * dx + transform.c * dy + transform.tx,
    origin[1] + transform.b * dx + transform.d * dy + transform.ty,
  );

  assert_close(text_quad.x, expected.0);
  assert_close(text_quad.y, expected.1);
  assert_eq!(text_quad.transform.matrix_2x2(), transform.matrix_2x2());
  assert_eq!(text_quad.transform_origin, Some([0.0, 0.0]));

  run_pass(&mut rt);
  let snapshot = render_pass(&mut rt);
  let first_glyph = snapshot.glyphs.first().expect("glyph");
  assert_eq!(first_glyph.transform, transform.matrix_2x2());
  assert!(
    first_glyph.x.fract().abs() > 0.001 || first_glyph.y.fract().abs() > 0.001,
    "transformed text should preserve fractional glyph origin instead of pre-snapping: glyph=({}, {})",
    first_glyph.x,
    first_glyph.y
  );
  assert!(first_glyph.transform_origin[0].abs() < 2.0);
  assert_close(first_glyph.clip.x, -1.0);
  assert_close(first_glyph.clip.y, -1.0);
  assert_close(first_glyph.clip.width, 802.0);
  assert_close(first_glyph.clip.height, 602.0);
}

#[test]
fn rasterized_transform_mode_bakes_transform_into_glyph_mask() {
  let mut rt = rt();
  let transform = Transform2D::rotate_deg(-2.0).then(&Transform2D::scale(1.02, 1.02));

  rt.set_root(
    lurq::components::Text::new("Stable transformed text")
      .text_transform_mode(TextTransformMode::Rasterized)
      .width(260.0)
      .transform(transform)
      .overflow_visible(),
  );

  let snapshot = render_pass(&mut rt);
  let first_glyph = snapshot.glyphs.first().expect("glyph");

  assert_eq!(first_glyph.transform, [1.0, 0.0, 0.0, 1.0]);
  assert_eq!(first_glyph.transform_origin, [0.0, 0.0]);
  assert!(first_glyph.width > 0.0);
  assert!(first_glyph.height > 0.0);
}

#[test]
fn rasterized_transform_mode_rotates_glyph_mask_bounds() {
  let style = TextStyle {
    font_size: 32.0,
    ..TextStyle::default()
  };

  let mut plain = rt();
  plain.set_root(lurq::components::Text::styled("I", style.clone()).overflow_visible());
  let plain_snapshot = render_pass(&mut plain);
  let plain_glyph = plain_snapshot.glyphs.first().expect("plain glyph");

  let mut rotated = rt();
  rotated.set_root(
    lurq::components::Text::styled("I", style)
      .text_transform_mode(TextTransformMode::Rasterized)
      .transform(Transform2D::rotate_deg(45.0))
      .overflow_visible(),
  );
  let rotated_snapshot = render_pass(&mut rotated);
  let rotated_glyph = rotated_snapshot.glyphs.first().expect("rotated glyph");

  assert!(
    rotated_glyph.width > plain_glyph.width + 2.0,
    "baked rotated glyph mask should have a wider bounding box than the upright glyph"
  );
}

#[test]
fn rasterized_transform_mode_preserves_float_transformed_glyph_position() {
  let mut rt = rt();

  rt.set_root(
    lurq::components::Text::styled(
      "I",
      TextStyle {
        font_size: 32.0,
        ..TextStyle::default()
      },
    )
    .text_transform_mode(TextTransformMode::Rasterized)
    .transform(Transform2D::rotate_deg(-8.0))
    .offset(13.25, 17.5)
    .overflow_visible(),
  );

  let snapshot = render_pass(&mut rt);
  let glyph = snapshot.glyphs.first().expect("glyph");

  assert!(
    glyph.x.fract().abs() > 0.001 || glyph.y.fract().abs() > 0.001,
    "baked transformed glyph placement should stay in float screen space"
  );
}
