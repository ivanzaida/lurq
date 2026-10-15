//! `content_without_generated` against a direct reference: every byte range
//! of a paragraph whose spans alternate between text and presentation.

use std::ops::Range;

use super::content_without_generated;
use crate::layout::{quad::RichTextSpan, text_style::TextStyle};

fn span(text: &str) -> RichTextSpan {
  RichTextSpan {
    text: text.to_owned(),
    style: TextStyle::default(),
  }
}

/// The bytes of `range` that lie in spans not in `generated`, span by span.
fn reference(spans: &[RichTextSpan], generated: &[Range<usize>], text: &str, range: Range<usize>) -> String {
  let mut offset = 0;
  let mut content = String::new();
  for (index, span) in spans.iter().enumerate() {
    let bytes = offset..offset + span.text.len();
    offset = bytes.end;
    if generated.iter().any(|generated| generated.contains(&index)) {
      continue;
    }
    for byte in bytes.start.max(range.start)..bytes.end.min(range.end) {
      content.push(char::from(text.as_bytes()[byte]));
    }
  }
  content
}

#[test]
fn content_without_generated_matches_the_reference_for_every_range() {
  // Text, then a link's marks and words, then a run between marks, and so on,
  // with empty and adjacent presentation ranges.
  let texts = [
    "Open ", "<", "PR", ">", "<", " host", ">", " and ", "<", "x", ">", "", "<", ">", " end.",
  ];
  let spans = texts.iter().map(|text| span(text)).collect::<Vec<_>>();
  let generated = vec![1..2, 3..4, 4..7, 8..9, 10..11, 12..13, 13..14];
  let text = texts.concat();
  for start in 0..=text.len() {
    for end in start..=text.len() {
      assert_eq!(
        content_without_generated(&spans, &generated, &text, start..end),
        reference(&spans, &generated, &text, start..end),
        "{start}..{end}"
      );
    }
  }
}
