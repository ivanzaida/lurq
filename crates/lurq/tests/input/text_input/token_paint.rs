use super::*;
use crate::{
  app::glyph_engine::input_tokens::{InputConfig, TokenStyle},
  layout::text_style::{TextShadow, TextStyle},
  node::transform::Transform2D,
};

fn fixture() -> (GlyphEngine, std::sync::Arc<DecoratedInputLayout>, Quad) {
  let mut engine = GlyphEngine::new();
  let base = TextStyle {
    font_size: 13.0,
    line_height: 1.4,
    ..TextStyle::default()
  };
  let token = TextStyle {
    color: Color::new(20, 80, 130, 255),
    shadow: Some(TextShadow::new(2.0, 3.0, 4.0, Color::new(2, 3, 4, 128))),
    ..base.clone()
  };
  let layout = engine.decorated_input(InputConfig {
    source: "a {{task.title}} b".into(),
    base,
    styles: vec![TokenStyle {
      range: 2..16,
      text: token,
      padding: 3.0,
      fill: None,
    }],
    width: 150.0,
    min_width: 150.0,
    wrap: true,
  });
  let quad = Quad {
    x: 12.0,
    y: 18.0,
    width: 150.0,
    height: layout.size.height,
    opacity: 0.5,
    transform: Transform2D::IDENTITY,
    transform_origin: None,
    content: QuadContent::DecoratedInput { layout: layout.clone() },
    border_radius: None,
    border: None,
    clip: ClipRect::default(),
  };
  (engine, layout, quad)
}

#[test]
fn input_tokens_rtl_paint_exact_scale_clip_order_shadow_and_opacity_once() {
  let (mut engine, layout, quad) = fixture();
  let clip = ClipRect {
    x: 20.0,
    y: 30.0,
    width: 200.0,
    height: 40.0,
    active: true,
    ..ClipRect::default()
  };
  let mut one = Vec::new();
  let mut two = Vec::new();
  resolve_input_quad(&mut engine, &layout, &quad, 1.0, 7, clip, &mut one);
  resolve_input_quad(&mut engine, &layout, &quad, 2.0, 7, clip, &mut two);
  assert!(!one.is_empty());
  assert_eq!(one.len(), two.len());
  for (one, two) in one.iter().zip(&two) {
    assert_eq!(one.order, 7);
    assert_eq!(one.clip.x, 20.0);
    assert!(one.clip.active);
    assert!((two.x / 2.0 - one.x).abs() <= 1.5 && (two.y / 2.0 - one.y).abs() <= 1.5);
    assert!(one.color[3] <= 0.5 && two.color[3] == one.color[3]);
    if one.shadow_sigma > 0.0 {
      assert!((one.color[3] - 128.0 / 255.0 * 0.5).abs() < 0.001);
    } else {
      assert_eq!(one.color[3], 0.5);
    }
  }
  assert_eq!(one.iter().filter(|glyph| glyph.shadow_sigma > 0.0).count(), 14);
  assert_eq!(layout.config.source.as_ref(), "a {{task.title}} b");
}

#[test]
fn input_tokens_rtl_transformed_exact_glyphs_retain_logical_rows_and_clip() {
  let (mut engine, layout, mut quad) = fixture();
  quad.transform = Transform2D::scale(1.5, 1.2).then(&Transform2D::rotate_deg(12.0));
  quad.transform_origin = Some([5.0, 7.0]);
  let mut out = Vec::new();
  let before = layout.positions();
  let clip = ClipRect {
    x: 0.0,
    y: 0.0,
    width: 500.0,
    height: 300.0,
    active: true,
    ..ClipRect::default()
  };
  let range = resolve_input_quad(&mut engine, &layout, &quad, 2.0, 11, clip, &mut out);
  assert_eq!(range, 0..out.len());
  assert!(
    out
      .iter()
      .all(|glyph| glyph.transform == quad.transform.matrix_2x2() && glyph.order == 11 && glyph.clip.active)
  );
  assert!(
    out
      .iter()
      .all(|glyph| (glyph.x + glyph.transform_origin[0] - 34.0).abs() < 0.01
        && (glyph.y + glyph.transform_origin[1] - 50.0).abs() < 0.01)
  );
  let after = layout.positions();
  assert_eq!(
    before
      .iter()
      .map(|caret| (caret.index, caret.x, caret.y))
      .collect::<Vec<_>>(),
    after
      .iter()
      .map(|caret| (caret.index, caret.x, caret.y))
      .collect::<Vec<_>>()
  );
}

#[cfg(feature = "screenshot")]
#[test]
fn input_tokens_rtl_actual_glyph_redaction_range_includes_shadow_and_transform() {
  let (mut engine, layout, mut quad) = fixture();
  quad.transform = Transform2D::scale(1.3, 1.3);
  let mut out = Vec::new();
  let range = resolve_input_quad(&mut engine, &layout, &quad, 2.0, 3, ClipRect::default(), &mut out);
  let area = crate::app::capture_redaction::RedactedArea::of_glyph_run(&out[range]).unwrap();
  assert!(area.x0 < area.x1 && area.y0 < area.y1);
  let without_shadow: Vec<_> = out.iter().filter(|glyph| glyph.shadow_sigma == 0.0).cloned().collect();
  let ink = crate::app::capture_redaction::RedactedArea::of_glyph_run(&without_shadow).unwrap();
  assert!(area.x0 <= ink.x0 && area.y0 <= ink.y0 && area.x1 >= ink.x1 && area.y1 >= ink.y1);
}
