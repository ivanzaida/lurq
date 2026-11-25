//! A styled text input's spans as the per-range text styles of a decorated
//! input layout, each mark's look taken from the theme's text marks, the
//! kinds of its paragraphs as the format each paragraph is laid out in, and
//! its objects as boxes in its rows.
use std::{ops::Range, sync::Arc};

use super::LayoutEngine;
use crate::{
  app::glyph_engine::input_tokens::{ParagraphBand, ParagraphFormat, ParagraphMark, TextLines, TokenChip, TokenStyle},
  components::{
    InlineObject, ParagraphKind, ParagraphMarker, ParagraphStyles, StyledSpan,
    styled_text::{paragraph_of, paragraph_ranges},
  },
  layout::{Size, text_style::TextStyle},
  node::node_kind::TextInputState,
};

impl LayoutEngine {
  /// What a styled input draws: its text (with a composition in progress),
  /// a style per span, and a format per paragraph where it has kinds of
  /// paragraphs. `None` for a plain input, a masked one, and an empty one
  /// showing its placeholder.
  ///
  /// The character of each object the input shows has a style of its own,
  /// a box of the size `sizes` gives for it, in the order of the text: one
  /// with no size yet is a square of its line.
  pub(super) fn styled_input_styles(
    &self,
    state: &TextInputState,
    base: &TextStyle,
    sizes: &[Option<Size>],
  ) -> Option<(Arc<str>, Vec<TokenStyle>, Vec<ParagraphFormat>)> {
    let objects = state.shown_objects().unwrap_or_default();
    let own_marks = state.text_marks();
    // Kinds of paragraphs are drawn where the lines wrap and are not justified: there an empty input is laid out
    // as its first paragraph too, for its placeholder to stand in it.
    let kinds_drawn = state.wraps_lines() && base.text_align != crate::layout::text_style::TextAlign::Justified;
    let (text, spans) = state.styled_display(kinds_drawn)?;
    let marks = own_marks.unwrap_or_else(|| self.text_marks.borrow().clone());
    let palette = self.palette.borrow();
    let token = |range: Range<usize>, span: &StyledSpan, base: &TextStyle| {
      let resolved = marks.resolve(&span.style, base, &palette);
      TokenStyle {
        range,
        text: resolved.text,
        padding: resolved.padding,
        fill: resolved.fill,
        lines: TextLines {
          underline: resolved.underline,
          strikethrough: resolved.strikethrough,
        },
        chip: Default::default(),
      }
    };
    let Some((kinds, looks)) = state.paragraph_display().filter(|_| kinds_drawn) else {
      let styles = spans.iter().map(|span| token(span.range.clone(), span, base)).collect();
      return Some((text, with_objects(styles, &objects, sizes, |_| base), Vec::new()));
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
    // A box stands in the type of the paragraph it is in.
    let styles = with_objects(styles, &objects, sizes, |at| &formats[paragraph_of(&ranges, at)].base);
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
      let continues = std::mem::replace(&mut before, Some(kind)).is_some_and(|before| kind.continues(before));
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
        ParagraphMarker::Check {
          size,
          gap,
          radius,
          edge_width,
          edge,
          fill,
          tick,
          icon,
        } => {
          let size = finite(*size);
          let checked = matches!(kind, ParagraphKind::Checklist { checked: true }).then(|| {
            let icon = icon.as_ref().map(|icon| {
              let style = TextStyle {
                font_family: icon.family.clone(),
                font_size: size * 0.75,
                line_height: 1.0,
                color: color(tick),
                ..TextStyle::default()
              };
              (style, icon.glyph)
            });
            (color(fill), icon)
          });
          ParagraphMark::Check {
            size,
            gap: finite(*gap),
            radius: finite(*radius),
            edge: (finite(*edge_width), color(edge)),
            checked,
          }
        }
        ParagraphMarker::Rule { thickness, color: line } => ParagraphMark::Rule {
          thickness: finite(*thickness),
          color: color(line),
        },
      };
      let fill = look
        .background
        .as_ref()
        .and_then(|fill| fill.resolve(&palette))
        .map(|fill| (fill, finite(look.background_radius)));
      let gap_before = match (continues, look.joined, look.item_gap) {
        (true, true, _) => 0.0,
        (true, false, Some(gap)) => finite(gap),
        _ => finite(looks.gap),
      };
      // The band that names the run this paragraph starts: a block of code's language.
      let header = look
        .header
        .as_ref()
        .filter(|_| !continues)
        .zip(kind.language())
        .map(|(header, language)| {
          let mut style = text.clone();
          if let Some(family) = &header.font_family {
            style.font_family = family.clone();
          }
          if let Some(size) = header.font_size.filter(|size| size.is_finite() && *size > 0.0) {
            style.font_size = size;
          }
          if let Some(weight) = header.weight {
            style.weight = weight;
          }
          if let Some(ink) = &header.color {
            style.color = color(ink);
          }
          style.line_height = 1.0;
          ParagraphBand {
            text: header
              .label
              .as_ref()
              .map_or_else(|| language.into(), |label| label.of(language).into()),
            style,
            height: finite(header.height),
            padding_x: finite(header.padding_x),
            fill: header.background.as_ref().and_then(|fill| fill.resolve(&palette)),
            line: header
              .line
              .as_ref()
              .map(|(thickness, line)| (finite(*thickness), color(line))),
          }
        });
      let edge = look
        .edge
        .as_ref()
        .map(|(width, edge)| (finite(*width), color(edge)))
        .filter(|(width, _)| *width > 0.0);
      formats.push(ParagraphFormat {
        header,
        marker_text: look.marker_text(&text),
        base: text,
        indent: finite(look.indent),
        gap_before,
        padding: (finite(look.padding_top), finite(look.padding_bottom)),
        marker,
        fill,
        edge,
        continues,
      });
    }
    formats
  }
}

/// `styles`, in the order of the text, with the character of each of `objects` given a style of its own: a box
/// of the size `sizes` has for it, in the type `base` gives for its byte, which the caret goes past as one thing.
/// A style that covered the character is cut around it.
fn with_objects<'a>(
  styles: Vec<TokenStyle>,
  objects: &[InlineObject],
  sizes: &[Option<Size>],
  base: impl Fn(usize) -> &'a TextStyle,
) -> Vec<TokenStyle> {
  if objects.is_empty() {
    return styles;
  }
  let boxed = |index: usize, object: &InlineObject| {
    let text = base(object.at).clone();
    let line = (text.font_size * text.line_height).max(1.0);
    let size = sizes.get(index).copied().flatten().unwrap_or(Size::new(line, line));
    TokenStyle {
      range: object.range(),
      text,
      padding: 0.0,
      fill: None,
      lines: TextLines::default(),
      chip: TokenChip {
        atomic: true,
        object: Some((size.width, size.height)),
        ..TokenChip::default()
      },
    }
  };
  let mut out = Vec::with_capacity(styles.len() + objects.len() * 2);
  let mut objects = objects.iter().enumerate().peekable();
  for mut style in styles {
    // Every object that starts before this style ends is before it or in it: a span never splits a character.
    while let Some((index, object)) = objects.next_if(|(_, object)| object.at < style.range.end) {
      let range = object.range();
      if range.start > style.range.start {
        out.push(TokenStyle {
          range: style.range.start..range.start,
          ..style.clone()
        });
      }
      out.push(boxed(index, object));
      style.range.start = style.range.start.max(range.end);
    }
    if style.range.start < style.range.end {
      out.push(style);
    }
  }
  out.extend(objects.map(|(index, object)| boxed(index, object)));
  out
}
