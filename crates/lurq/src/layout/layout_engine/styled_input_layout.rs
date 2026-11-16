//! A styled text input's spans as the per-range text styles of a decorated
//! input layout, each mark's look taken from the theme's text marks, and the
//! kinds of its paragraphs as the format each paragraph is laid out in.
use std::{ops::Range, sync::Arc};

use super::LayoutEngine;
use crate::{
  app::glyph_engine::input_tokens::{ParagraphFormat, ParagraphMark, TextLines, TokenStyle},
  components::{
    ParagraphKind, ParagraphMarker, ParagraphStyles, StyledSpan,
    styled_text::{paragraph_of, paragraph_ranges},
  },
  layout::text_style::TextStyle,
  node::node_kind::TextInputState,
};

impl LayoutEngine {
  /// What a styled input draws: its text (with a composition in progress),
  /// a style per span, and a format per paragraph where it has kinds of
  /// paragraphs. `None` for a plain input, a masked one, and an empty one
  /// showing its placeholder.
  pub(super) fn styled_input_styles(
    &self,
    state: &TextInputState,
    base: &TextStyle,
  ) -> Option<(Arc<str>, Vec<TokenStyle>, Vec<ParagraphFormat>)> {
    let (text, spans) = state.styled_display()?;
    let marks = self.text_marks.borrow().clone();
    let palette = self.palette.borrow();
    let token = |range: Range<usize>, span: &StyledSpan, base: &TextStyle| {
      let resolved = marks.resolve(&span.style, base, &palette);
      TokenStyle {
        range,
        text: resolved.text,
        padding: 0.0,
        fill: resolved.fill,
        lines: TextLines {
          underline: resolved.underline,
          strikethrough: resolved.strikethrough,
        },
        chip: Default::default(),
      }
    };
    let Some((kinds, looks)) = state.paragraph_display() else {
      let styles = spans.iter().map(|span| token(span.range.clone(), span, base)).collect();
      return Some((text, styles, Vec::new()));
    };
    let ranges = paragraph_ranges(&text);
    let formats = self.paragraph_formats(&kinds, &looks, base, ranges.len());
    // A mark is drawn over the type of the paragraph it is in: a span that runs over several is cut at each.
    let mut styles = Vec::with_capacity(spans.len());
    for span in spans.iter() {
      let first = paragraph_of(&ranges, span.range.start);
      for (index, paragraph) in ranges.iter().enumerate().skip(first) {
        if paragraph.start >= span.range.end {
          break;
        }
        let start = span.range.start.max(paragraph.start);
        let end = span.range.end.min(paragraph.end);
        if start < end {
          styles.push(token(start..end, span, &formats[index].base));
        }
      }
    }
    Some((text, styles, formats))
  }

  /// The format of each of `count` paragraphs: its kind's type over `base`
  /// with its color resolved, its indent, the room before it and its marker,
  /// a numbered item's number counted within its run of items.
  fn paragraph_formats(
    &self,
    kinds: &[ParagraphKind],
    looks: &ParagraphStyles,
    base: &TextStyle,
    count: usize,
  ) -> Vec<ParagraphFormat> {
    let palette = self.palette.borrow();
    let finite = |value: f32| if value.is_finite() { value.max(0.0) } else { 0.0 };
    let mut number = 0;
    let mut formats: Vec<ParagraphFormat> = Vec::with_capacity(count);
    let mut before = None;
    for index in 0..count {
      let kind = kinds.get(index).copied().unwrap_or_default();
      let continues = std::mem::replace(&mut before, Some(kind)) == Some(kind);
      let look = looks.of(kind);
      number = if kind == ParagraphKind::Ordered { number + 1 } else { 0 };
      let mut text = look.text(base);
      if let Some(color) = look.color.as_ref().and_then(|color| color.resolve(&palette)) {
        text.color = color;
      }
      let color = |color: &crate::node::TextColor| color.resolve(&palette).unwrap_or(text.color);
      let marker = match &look.marker {
        ParagraphMarker::None => ParagraphMark::None,
        ParagraphMarker::Dot { size, gap, color: dot } => ParagraphMark::Dot {
          size: finite(*size),
          gap: finite(*gap),
          color: color(dot),
        },
        ParagraphMarker::Number { gap, color: digits } => ParagraphMark::Text {
          text: format!("{number}.").into(),
          gap: finite(*gap),
          color: color(digits),
        },
        ParagraphMarker::Bar { width, color: bar } => ParagraphMark::Bar {
          width: finite(*width),
          color: color(bar),
        },
      };
      let fill = look
        .background
        .as_ref()
        .and_then(|fill| fill.resolve(&palette))
        .map(|fill| (fill, finite(look.background_radius)));
      formats.push(ParagraphFormat {
        base: text,
        indent: finite(look.indent),
        gap_before: if continues && look.joined {
          0.0
        } else {
          finite(looks.gap)
        },
        marker,
        fill,
        continues,
      });
    }
    formats
  }
}
