use super::*;
use crate::canvas::text::tests::engine;
use crate::layout::text_style::{FontFeature, FontFeatures, FontStyle, FontWeight};

const RED: Color = Color::new(255, 0, 0, 255);

#[test]
fn rendered_identity_survives_more_than_256_real_shaped_results() {
  let mut engine = engine();
  let font = CanvasFont::new("Lurq Weight Probe", 10.);
  let first = engine.shape("aaaa", &font, 1., RED).unwrap();
  let pixels = first.data.as_ref().clone();
  for index in 0..300 {
    engine.shape(&format!("a{index}"), &font, 1., RED).unwrap();
  }
  assert!(!engine.shaped.iter().any(|entry| entry.0 == "aaaa"));
  let revisited = engine.shape("aaaa", &font, 1., RED).unwrap();
  assert!(!std::sync::Arc::ptr_eq(&first, &revisited));
  assert_eq!(revisited.data.as_ref(), &pixels);
  assert_eq!(revisited.asset_id, first.asset_id);
  assert_eq!(first.data.as_ref(), &pixels); // Prior drawing owners remain immutable.
  assert!(engine.shaped_bytes <= super::super::MAX_SHAPED_BYTES);
  assert!(engine.identities.charged_bytes() <= MAX_BYTES);
  assert!(engine.identities.len() > 256);
}

#[test]
fn identity_compares_the_complete_render_key_and_engine_scope() {
  let font = CanvasFont::new("Lurq Weight Probe", 10.);
  let mut cache = IdentityCache::new();
  let first = cache.resolve("a", &font, 1., RED);
  assert_eq!(cache.resolve("a", &font, 1., RED), first);
  let mut ids = vec![first, cache.resolve("aa", &font, 1., RED)];
  ids.push(cache.resolve("a", &font, 2., RED));
  ids.push(cache.resolve("a", &font, 1., Color::new(255, 0, 0, 128)));
  ids.push(cache.resolve("a", &font, 1., Color::new(0, 0, 255, 255)));
  for change in 0..6 {
    let mut changed = font.clone();
    match change {
      0 => changed.family = "Different family".into(),
      1 => changed.size = 11.,
      2 => changed.weight = FontWeight::Bold,
      3 => changed.style = FontStyle::Italic,
      4 => changed.letter_spacing = 1.,
      _ => changed.font_features = FontFeatures::from([FontFeature::disable(*b"liga")]),
    }
    ids.push(cache.resolve("a", &changed, 1., RED));
  }
  let mut equivalent = font.clone();
  equivalent.weight = FontWeight::Numeric(400);
  equivalent.letter_spacing = -0.;
  assert_eq!(cache.resolve("a", &equivalent, 1., RED), first);
  let mut other_engine = engine();
  ids.push(other_engine.shape("a", &font, 1., RED).unwrap().asset_id);
  let count = ids.len();
  ids.sort_unstable();
  ids.dedup();
  assert_eq!(ids.len(), count); // Global allocation, never per-engine ID counters.
}

#[test]
fn bounded_metadata_eviction_reinterns_fresh_and_does_not_recycle() {
  let font = CanvasFont::new("Lurq Weight Probe", 10.);
  let mut cache = IdentityCache::new();
  let first = cache.resolve("a", &font, 1., RED);
  let padding = "x".repeat(2048);
  for index in 0..700 {
    cache.resolve(&format!("{index}-{padding}"), &font, 1., RED);
    assert!(cache.len() <= MAX_KEYS);
    assert!(cache.charged_bytes() <= MAX_BYTES);
  }
  let revisited = cache.resolve("a", &font, 1., RED);
  assert_ne!(revisited, first);
  assert_eq!(cache.resolve("a", &font, 1., RED), revisited);
  let mut invalid = font.clone();
  invalid.letter_spacing = f32::NAN;
  assert_ne!(
    cache.resolve("a", &invalid, 1., RED),
    cache.resolve("a", &invalid, 1., RED)
  );
  let huge = "x".repeat(MAX_BYTES);
  let before = (cache.len(), cache.charged_bytes());
  assert_ne!(
    cache.resolve(&huge, &font, 1., RED),
    cache.resolve(&huge, &font, 1., RED)
  );
  assert_eq!((cache.len(), cache.charged_bytes()), before);
}
