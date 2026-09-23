//! Conservative width intervals for Cosmic Text 0.12's word wrapping.
//!
//! A left-aligned, single LTR span has width-independent glyph coordinates when
//! its wrap decisions stay the same. Record the bounds of those decisions once
//! after layout, then check a new width without walking words or glyphs.

use cosmic_text::{Align, BufferLine};

#[derive(Clone, Copy)]
pub(super) struct ReflowRange {
  min: f32,
  max: f32,
}

impl ReflowRange {
  pub(super) fn contains(self, width: Option<f32>) -> bool {
    let width = width.unwrap_or(f32::INFINITY);
    width >= self.min && (width < self.max || self.max == f32::INFINITY)
  }

  pub(super) fn new(line: &BufferLine, font_size: f32, width: Option<f32>, wrap: bool) -> Option<Self> {
    line.layout_opt()?;
    let shape = line.shape_opt()?;
    if line.align() != Some(Align::Left)
      || shape.rtl
      || shape.spans.len() > 1
      || shape.spans.first().is_some_and(|span| span.level.is_rtl())
    {
      return None;
    }
    let mut range = Self {
      min: 0.0,
      max: f32::INFINITY,
    };
    if !wrap {
      return Some(range);
    }
    let width = width?;
    if !width.is_finite() || width < 0.0 {
      return None;
    }
    let Some(span) = shape.spans.first() else {
      return Some(range);
    };
    let mut used = 0.0;
    for word in &span.words {
      let advance = word.width(font_size);
      // Preserve Cosmic's addition order and short circuiting, including the
      // one trailing blank allowed beyond a row's width. For a single span,
      // current_visual_line.w is always zero until the row is committed.
      if range.fits(used + advance, width)? || (word.blank && range.fits(used, width)?) {
        used += advance;
      } else {
        // Long words use Cosmic's glyph-wrap path. Leave those paragraphs to
        // Cosmic instead of maintaining a second glyph wrapping algorithm here.
        if !range.fits(advance, width)? {
          return None;
        }
        used = if word.blank { 0.0 } else { advance };
      }
    }
    Some(range)
  }

  fn fits(&mut self, advance: f32, width: f32) -> Option<bool> {
    if !advance.is_finite() || advance < 0.0 {
      return None;
    }
    let fits = advance <= width;
    if fits {
      self.min = self.min.max(advance);
    } else {
      self.max = self.max.min(advance);
    }
    Some(fits)
  }
}

#[cfg(test)]
mod tests {
  use cosmic_text::{Attrs, AttrsList, BufferLine, Ellipsize, Family, Hinting, LineEnding, Shaping, Wrap};

  use super::*;

  #[test]
  fn intervals_match_cosmic_at_wrap_boundaries() {
    let mut engine = super::super::GlyphEngine::new();
    let mut certified = 0;
    let mut wrapped = 0;
    let mut comparisons = 0;
    for text in [
      "",
      "  ",
      "a b",
      " a  b   c    ",
      "office cafe\u{301} — Ελληνικά 😀 漢字 words and more words",
      "one\ttwo\tthree  four\t",
      "a\u{a0}b\u{200b}c\u{2009}d\u{ad}e\u{2028}f",
      "short extraordinarilylongwordthatusesglyphwrapping end",
      "alpha beta gamma delta epsilon zeta eta theta iota kappa lambda",
    ] {
      for font_size in [13.0, 19.5] {
        for width in [0.0, 0.25, 1.0, 16.0, 37.25, 64.5, 113.0, 173.5, 240.0, 480.0] {
          let mut line = BufferLine::new(
            text,
            LineEnding::Lf,
            AttrsList::new(&Attrs::new().family(Family::SansSerif)),
            Shaping::Advanced,
          );
          line.set_align(Some(Align::Left));
          line.layout(
            &mut engine.font_system,
            font_size,
            Some(width),
            Wrap::WordOrGlyph,
            Ellipsize::None,
            None,
            8,
            Hinting::Disabled,
          );
          let Some(range) = ReflowRange::new(&line, font_size, Some(width), true) else {
            continue;
          };
          assert!(range.contains(Some(width)));
          certified += 1;
          wrapped += usize::from(line.layout_opt().unwrap().len() > 1);
          let expected = format!("{:?}", line.layout_opt());
          let mut candidates = (0..80).map(|i| i as f32 * 7.125).collect::<Vec<_>>();
          for bound in [range.min, range.max, width] {
            if bound.is_finite() {
              candidates.extend([bound.next_down(), bound, bound.next_up()]);
            }
          }
          for candidate in candidates {
            if !range.contains(Some(candidate)) {
              continue;
            }
            let mut fresh = line.clone();
            fresh.reset_layout();
            fresh.layout(
              &mut engine.font_system,
              font_size,
              Some(candidate),
              Wrap::WordOrGlyph,
              Ellipsize::None,
              None,
              8,
              Hinting::Disabled,
            );
            assert_eq!(
              format!("{:?}", fresh.layout_opt()),
              expected,
              "{text:?}: {width} -> {candidate}"
            );
            comparisons += 1;
          }
        }
      }
    }
    assert!(certified > 50 && wrapped > 10 && comparisons > 1000);
  }

  #[test]
  fn intervals_exclude_alignment_bidi_and_glyph_wrap() {
    let mut engine = super::super::GlyphEngine::new();
    for (text, align, width) in [
      ("alpha beta gamma", Align::Right, 100.0),
      ("alpha beta gamma", Align::Center, 100.0),
      ("alpha beta gamma", Align::Justified, 100.0),
      ("مرحبا שלום", Align::Left, 100.0),
      ("hello مرحبا world", Align::Left, 100.0),
      ("longunbrokenword", Align::Left, 1.0),
    ] {
      let mut line = BufferLine::new(
        text,
        LineEnding::None,
        AttrsList::new(&Attrs::new().family(Family::SansSerif)),
        Shaping::Advanced,
      );
      line.set_align(Some(align));
      line.layout(
        &mut engine.font_system,
        16.0,
        Some(width),
        Wrap::WordOrGlyph,
        Ellipsize::None,
        None,
        8,
        Hinting::Disabled,
      );
      assert!(
        ReflowRange::new(&line, 16.0, Some(width), true).is_none(),
        "{text:?} {align:?}"
      );
    }
  }

  #[test]
  fn nowrap_intervals_preserve_geometry_with_and_without_a_width_limit() {
    let mut engine = super::super::GlyphEngine::new();
    for text in ["", "alpha beta gamma delta", " cafe\u{301}\t漢字 😀 "] {
      let mut line = BufferLine::new(
        text,
        LineEnding::None,
        AttrsList::new(&Attrs::new().family(Family::SansSerif)),
        Shaping::Advanced,
      );
      line.set_align(Some(Align::Left));
      line.layout(
        &mut engine.font_system,
        19.5,
        Some(80.0),
        Wrap::None,
        Ellipsize::None,
        None,
        8,
        Hinting::Disabled,
      );
      let range = ReflowRange::new(&line, 19.5, Some(80.0), false).unwrap();
      let expected = format!("{:?}", line.layout_opt());
      for width in [Some(0.0), Some(1.0), Some(79.875), Some(480.0), None] {
        assert!(range.contains(width));
        let mut fresh = line.clone();
        fresh.reset_layout();
        fresh.layout(
          &mut engine.font_system,
          19.5,
          width,
          Wrap::None,
          Ellipsize::None,
          None,
          8,
          Hinting::Disabled,
        );
        assert_eq!(format!("{:?}", fresh.layout_opt()), expected);
      }
    }
  }
}
