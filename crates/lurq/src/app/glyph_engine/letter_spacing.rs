//! Letter spacing (tracking) for cosmic-text attributes.

use cosmic_text::Attrs;

/// Adds `letter_spacing` logical pixels after every glyph. cosmic-text takes
/// tracking in ems and scales every glyph advance by the buffer's font size,
/// not the span's, so `em_px` must be the buffer's font size. Zero and
/// non-finite spacing leave the attributes unchanged.
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
    let width = |engine: &mut GlyphEngine, spans: &[RichTextSpan]| engine.measure_rich_text(spans, f32::MAX).width;

    let unspaced = [span("aa", 10.0, 0.0), span("aa", 10.0, 0.0)];
    assert!((width(&mut engine, &unspaced) - 20.0).abs() < 0.01);
    let spaced = [span("aa", 10.0, 1.0), span("aa", 10.0, 3.0)];
    assert!((width(&mut engine, &spaced) - 28.0).abs() < 0.01, "2x1px + 2x3px");
    // Every glyph uses the first span's size, so a later span's spacing must
    // still come out in pixels rather than in that span's own ems.
    let mixed = [span("aa", 10.0, 0.0), span("aa", 20.0, 4.0)];
    assert!((width(&mut engine, &mixed) - 28.0).abs() < 0.01, "2x4px");
    // Same text and sizes, only the spacing changed: no stale cached layout.
    let respaced = [span("aa", 10.0, 1.0), span("aa", 10.0, -1.0)];
    assert!((width(&mut engine, &respaced) - 20.0).abs() < 0.01);
  }
}
