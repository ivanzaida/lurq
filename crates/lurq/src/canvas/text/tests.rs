use super::*;

fn engine() -> CanvasTextEngine {
  let mut fonts = FontSystem::new_with_locale_and_db("en-US".into(), Default::default());
  for data in [
    include_bytes!("../../../tests/assets/weight_probe/LurqWeightProbe-Regular.ttf").as_slice(),
    include_bytes!("../../../tests/assets/ligature_probe/LurqLigatureProbe-Regular.ttf").as_slice(),
  ] {
    fonts.db_mut().load_font_data(data.to_vec());
  }
  CanvasTextEngine::new(fonts, Default::default())
}

const RED: Color = Color::new(255, 0, 0, 255);
const BLUE: Color = Color::new(0, 0, 255, 128);

#[test]
fn metrics_preserve_real_glyph_bounds_without_rgba_and_fill_after_measure_is_visible() {
  let mut engine = engine();
  let font = CanvasFont::new("Lurq Weight Probe", 10.);
  let measured = engine.measure("aaaa", &font, RED).unwrap();
  assert_eq!(measured.width, 20.);
  assert!(measured.pixels.is_none());
  assert!(measured.data.is_empty());
  assert_eq!(measured.asset_id, 0);
  let rendered = engine.shape("aaaa", &font, 1., RED).unwrap();
  assert!(rendered.pixels.is_some());
  assert!(rendered.data.chunks_exact(4).any(|p| p[3] > 0));
  for align in [TextAlign::Left, TextAlign::Center, TextAlign::Right] {
    for baseline in [
      TextBaseline::Alphabetic,
      TextBaseline::Top,
      TextBaseline::Middle,
      TextBaseline::Bottom,
    ] {
      assert_eq!(measured.metrics(align, baseline), rendered.metrics(align, baseline));
    }
  }
  assert!(!Arc::ptr_eq(&measured, &rendered));
  assert!(Arc::ptr_eq(&measured, &engine.measure("aaaa", &font, RED).unwrap()));
  assert!(Arc::ptr_eq(&rendered, &engine.shape("aaaa", &font, 1., RED).unwrap()));
}

#[test]
fn rendered_cache_preserves_color_scale_and_previous_drawing_owners() {
  let mut engine = engine();
  let font = CanvasFont::new("Lurq Weight Probe", 20.);
  let red = engine.shape("aaaa", &font, 1., RED).unwrap();
  let original = red.data.as_ref().clone();
  engine.measure("aaaa", &font, BLUE).unwrap();
  let blue = engine.shape("aaaa", &font, 1., BLUE).unwrap();
  assert!(red.data.chunks_exact(4).all(|p| p[1] == 0 && p[2] == 0 && p[0] == p[3]));
  assert!(
    blue
      .data
      .chunks_exact(4)
      .all(|p| p[0] == 0 && p[1] == 0 && p[2] == p[3] && p[3] <= 128)
  );
  assert!(blue.data.chunks_exact(4).any(|p| p[3] > 0));
  let scaled = engine.shape("aaaa", &font, 2., RED).unwrap();
  assert_eq!((red.width, scaled.width), (40., 40.));
  assert!(scaled.data.len() > red.data.len());
  assert_ne!(red.asset_id, blue.asset_id);
  assert_ne!(red.asset_id, scaled.asset_id);
  for size in 1..=257 {
    engine
      .measure("", &CanvasFont::new("Lurq Weight Probe", size as f32), RED)
      .unwrap();
  }
  assert_eq!(red.data.as_ref(), &original);
  let revisited = engine.shape("aaaa", &font, 1., RED).unwrap();
  assert_eq!(revisited.data.as_ref(), &original);
  assert_ne!(revisited.asset_id, red.asset_id);
  assert!(engine.shaped.len() <= 256);
  assert!(engine.shaped_bytes <= 8 * 1024 * 1024);
}

#[test]
fn metrics_keep_spacing_features_and_refusals() {
  let mut engine = engine();
  let mut font = CanvasFont::new("Lurq Weight Probe", 10.);
  for (spacing, expected) in [(0., 20.), (1.5, 26.), (-1., 16.)] {
    font.letter_spacing = spacing;
    assert_eq!(engine.measure("aaaa", &font, RED).unwrap().width, expected);
  }
  let mut font = CanvasFont::new("Lurq Ligature Probe", 10.);
  assert_eq!(engine.measure("a --a", &font, RED).unwrap().width, 20.);
  font.font_features = FontFeatures::from([crate::layout::text_style::FontFeature::disable(*b"liga")]);
  assert_eq!(engine.measure("a --a", &font, RED).unwrap().width, 25.);
  let oversized = "a".repeat(65_537);
  assert!(matches!(
    engine.measure(&oversized, &font, RED),
    Err(CanvasError::TextTooLarge)
  ));
  assert!(matches!(
    engine.shape(&oversized, &font, 1., RED),
    Err(CanvasError::TextTooLarge)
  ));
  let oversized_font = CanvasFont::new("Lurq Weight Probe", 4097.);
  assert!(matches!(
    engine.measure("a", &oversized_font, RED),
    Err(CanvasError::TextTooLarge)
  ));
  assert!(matches!(
    engine.shape("a", &oversized_font, 1., RED),
    Err(CanvasError::TextTooLarge)
  ));
}

/// One frame of a dense page, in the order it is drawn every frame: a measure
/// and a fill of each label. 700 distinct keys, like the ~684 of a real page.
fn page(engine: &mut CanvasTextEngine, font: &CanvasFont) -> Vec<Arc<ShapedText>> {
  (0..350)
    .flat_map(|index: usize| {
      let text = "a".repeat(1 + index % 4);
      let color = Color::new(index as u8, (index / 256) as u8, 96, 255);
      [
        engine.measure(&text, font, color).unwrap(),
        engine.shape(&text, font, 1., color).unwrap(),
      ]
    })
    .collect()
}

#[test]
fn a_page_with_more_keys_than_the_old_entry_limit_is_shaped_once() {
  let mut engine = engine();
  let font = CanvasFont::new("Lurq Weight Probe", 10.);
  let first = page(&mut engine, &font);
  assert!(first.iter().skip(1).step_by(2).all(|shaped| shaped.pixels.is_some()));
  for frame in 2..=4 {
    let next = page(&mut engine, &font);
    let reshaped = first.iter().zip(&next).filter(|(a, b)| !Arc::ptr_eq(a, b)).count();
    assert_eq!(reshaped, 0, "frame {frame} shaped {reshaped} of {} texts again", first.len());
  }
}
