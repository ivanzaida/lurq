//! Letter spacing (tracking) for cosmic-text attributes.

use cosmic_text::Attrs;

/// Adds `letter_spacing` logical pixels after every glyph. cosmic-text takes
/// tracking in ems and scales it by the size the glyph is drawn at: the
/// span's own where it has metrics of its own, the buffer's otherwise, so
/// `em_px` must be that size. Zero and non-finite spacing leave the
/// attributes unchanged.
pub(crate) fn with_letter_spacing(attrs: Attrs<'_>, letter_spacing: f32, em_px: f32) -> Attrs<'_> {
  if letter_spacing == 0.0 || !letter_spacing.is_finite() || !(em_px > 0.0 && em_px.is_finite()) {
    return attrs;
  }
  attrs.letter_spacing(letter_spacing / em_px)
}

/// Cache-key bits for a style's letter spacing: the value that reaches shaping.
pub(super) fn letter_spacing_bits(letter_spacing: f32) -> u32 {
  if letter_spacing.is_finite() && letter_spacing != 0.0 {
    letter_spacing.to_bits()
  } else {
    0
  }
}

#[cfg(test)]
mod tests {
  use crate::{
    app::glyph_engine::GlyphEngine,
    layout::{quad::RichTextSpan, text_style::TextStyle},
  };

  // "a" advances 0.5em in the weight-probe face.
  const REGULAR: &[u8] = include_bytes!("../../../tests/assets/weight_probe/LurqWeightProbe-Regular.ttf");

  fn span(text: &str, font_size: f32, letter_spacing: f32) -> RichTextSpan {
    RichTextSpan {
      text: text.to_owned(),
      style: TextStyle {
        font_family: "Lurq Weight Probe".into(),
        font_size,
        letter_spacing,
        ..TextStyle::default()
      },
    }
  }

  #[test]
  fn rich_spans_space_their_own_glyphs_in_pixels() {
    let mut engine = GlyphEngine::new();
    engine.load_font(REGULAR.to_vec());
    let width = |engine: &mut GlyphEngine, spans: &[RichTextSpan]| engine.measure_rich_text(spans, 1.0, f32::MAX).width;

    let unspaced = [span("aa", 10.0, 0.0), span("aa", 10.0, 0.0)];
    assert!((width(&mut engine, &unspaced) - 20.0).abs() < 0.01);
    let spaced = [span("aa", 10.0, 1.0), span("aa", 10.0, 3.0)];
    assert!((width(&mut engine, &spaced) - 28.0).abs() < 0.01, "2x1px + 2x3px");
    // A span of another size is drawn and advances at its own size, and its spacing is in pixels all the same:
    // 10 for the first, 20 and 2x4px for the second.
    let mixed = [span("aa", 10.0, 0.0), span("aa", 20.0, 4.0)];
    assert!((width(&mut engine, &mixed) - 38.0).abs() < 0.01, "10 + 20 + 2x4px");
    // Same text and sizes, only the spacing changed: no stale cached layout.
    let respaced = [span("aa", 10.0, 1.0), span("aa", 10.0, -1.0)];
    assert!((width(&mut engine, &respaced) - 20.0).abs() < 0.01);
  }

  #[test]
  fn a_span_shifted_above_the_baseline_draws_its_glyphs_higher_and_keeps_its_line() {
    let mut engine = GlyphEngine::new();
    engine.load_font(REGULAR.to_vec());
    let raised = |shift: f32| {
      let mut spans = [span("aa", 10.0, 0.0), span("aa", 10.0, 0.0)];
      spans[1].style.baseline_shift = shift;
      spans
    };
    let flat = engine.shape_rich_text_layout(&raised(0.0), f32::MAX, false, false);
    let shifted = engine.shape_rich_text_layout(&raised(4.0), f32::MAX, false, false);
    let ys =
      |layout: &super::super::CachedRichShapedLayout| layout.glyphs.iter().map(|glyph| glyph.y).collect::<Vec<_>>();
    let (flat, shifted_ys) = (ys(&flat), ys(&shifted));
    assert_eq!(flat.len(), 4);
    assert_eq!(shifted_ys[..2], flat[..2], "the first span stays on the baseline");
    assert!(
      shifted_ys[2..]
        .iter()
        .zip(&flat[2..])
        .all(|(up, down)| (down - up - 4.0).abs() <= 1.0),
      "the second is drawn 4 higher: {shifted_ys:?} for {flat:?}"
    );
    assert_eq!(
      shifted.size,
      engine.shape_rich_text_layout(&raised(0.0), f32::MAX, false, false).size,
      "its line is the same"
    );
  }
}
