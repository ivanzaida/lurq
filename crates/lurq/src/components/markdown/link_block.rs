//! The spans of one link in its paragraph: its words, its trailing runs and
//! the invisible marks that keep the runs right after the words, to their
//! right, on every laid-out line at any width.
//!
//! Bidi reorders each laid-out line on its own. A link that wraps across
//! lines leaves its first part at the end of one line and the rest at the
//! start of the next, so where its words land next to the text around them
//! depends on the paragraph's direction:
//!
//! - In a left-to-right paragraph the link is `LRM LRI words runs PDI LRM`. The marks make the link left to right among
//!   left-to-right neighbours, so the end of a line is its right end: a part of the words that ends a line has nothing
//!   after it, and the part that starts the next line is followed by the runs.
//! - In a right-to-left paragraph the end of a line is its left end, so a part of the words that ends a line would have
//!   the text before the link on its right. There the link is `RLM RLI LRI runs PDI FSI words PDI PDI RLM`: a
//!   right-to-left isolate that holds the runs first and the words second draws the words left of the runs. When the
//!   link wraps, the line it starts on has the runs with the words' first part on their left, and the line it ends on
//!   starts, at its right end, with the rest of the words.
//!
//! Either way the runs follow the words, left to right, and no line puts
//! another link's host or other text right after a link's words. Word
//! joiners at the line break opportunities inside the link keep it on one
//! line where the text's layout allows (cosmic-text may still break where
//! the bidi level changes, at a right-to-left word in a left-to-right
//! paragraph or anywhere in a right-to-left one, and splits a link wider
//! than its line); the marks keep such a break safe. The isolates nest at
//! most three deep. Marks and joiners are presentation, like the runs.

use std::ops::Range;

use crate::{
  layout::{quad::RichTextSpan, text_style::TextStyle},
  markdown::{
    Direction, FIRST_STRONG_ISOLATE, LEFT_TO_RIGHT_ISOLATE, LEFT_TO_RIGHT_MARK, POP_DIRECTIONAL_ISOLATE,
    RIGHT_TO_LEFT_ISOLATE, RIGHT_TO_LEFT_MARK, WORD_JOINER, has_right_to_left, line_break_offsets,
  },
};

/// What a span of a link is.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Role {
  Mark,
  Words,
  Run,
}

/// A link's spans in paragraph order, and which of them are its words, its
/// runs and its whole.
pub(super) struct LinkBlock {
  pub(super) spans: Vec<RichTextSpan>,
  /// Spans that are presentation: marks, joiners and runs.
  pub(super) generated: Vec<Range<usize>>,
  pub(super) label: Range<usize>,
  pub(super) trailing: Range<usize>,
  pub(super) target: Range<usize>,
}

impl LinkBlock {
  /// The block of a link in a paragraph that runs `direction`, with `words`
  /// and `runs` stripped of directional controls and paragraph breaks.
  /// `style` is the text's around the link.
  pub(super) fn new(
    direction: Direction,
    words: Vec<RichTextSpan>,
    runs: Vec<RichTextSpan>,
    style: &TextStyle,
  ) -> Self {
    let mut pieces = Pieces::new(style);
    match direction {
      Direction::LeftToRight => {
        pieces.mark(LEFT_TO_RIGHT_MARK);
        pieces.mark(LEFT_TO_RIGHT_ISOLATE);
        pieces.inside(|pieces| {
          pieces.isolated_if_needed(Role::Words, words);
          for run in runs {
            pieces.isolated_if_needed(Role::Run, vec![run]);
          }
        });
        pieces.mark(POP_DIRECTIONAL_ISOLATE);
        pieces.mark(LEFT_TO_RIGHT_MARK);
      }
      Direction::RightToLeft => {
        pieces.mark(RIGHT_TO_LEFT_MARK);
        pieces.mark(RIGHT_TO_LEFT_ISOLATE);
        pieces.inside(|pieces| {
          pieces.mark(LEFT_TO_RIGHT_ISOLATE);
          for run in runs {
            pieces.isolated_if_needed(Role::Run, vec![run]);
          }
          pieces.mark(POP_DIRECTIONAL_ISOLATE);
          pieces.mark(FIRST_STRONG_ISOLATE);
          pieces.push(Role::Words, words);
          pieces.mark(POP_DIRECTIONAL_ISOLATE);
        });
        pieces.mark(POP_DIRECTIONAL_ISOLATE);
        pieces.mark(RIGHT_TO_LEFT_MARK);
      }
    }
    pieces.join_words();
    pieces.into_block()
  }
}

/// The spans of a block being built, each with its role.
struct Pieces<'s> {
  spans: Vec<(Role, RichTextSpan)>,
  /// The spans between the link's outer isolate marks.
  inside: Range<usize>,
  style: &'s TextStyle,
}

impl<'s> Pieces<'s> {
  fn new(style: &'s TextStyle) -> Self {
    Self {
      spans: Vec::new(),
      inside: 0..0,
      style,
    }
  }

  fn mark(&mut self, mark: char) {
    self.spans.push((
      Role::Mark,
      RichTextSpan {
        text: mark.to_string(),
        style: self.style.clone(),
      },
    ));
  }

  fn push(&mut self, role: Role, spans: Vec<RichTextSpan>) {
    self.spans.extend(
      spans
        .into_iter()
        .filter(|span| !span.text.is_empty())
        .map(|span| (role, span)),
    );
  }

  fn inside(&mut self, push: impl FnOnce(&mut Self)) {
    let start = self.spans.len();
    push(self);
    self.inside = start..self.spans.len();
  }

  /// `spans` in their own first-strong isolate when they hold text that
  /// would reorder in the left-to-right isolate, as they are otherwise.
  fn isolated_if_needed(&mut self, role: Role, spans: Vec<RichTextSpan>) {
    if spans.iter().any(|span| has_right_to_left(&span.text)) {
      self.mark(FIRST_STRONG_ISOLATE);
      self.push(role, spans);
      self.mark(POP_DIRECTIONAL_ISOLATE);
    } else {
      self.push(role, spans);
    }
  }

  /// Puts a word joiner at each line break opportunity inside the link, so a
  /// line breaks before it or after it.
  fn join_words(&mut self) {
    let after = self.spans.split_off(self.inside.end);
    let inside = self.spans.split_off(self.inside.start);
    let text: String = inside.iter().map(|(_, span)| span.text.as_str()).collect();
    let mut breaks = line_break_offsets(&text).peekable();
    let mut offset = 0;
    let start = self.spans.len();
    for (role, span) in inside {
      let end = offset + span.text.len();
      let mut from = 0;
      while let Some(at) = breaks.next_if(|at| *at <= end) {
        let split = at - offset;
        if split > from {
          self.spans.push((role, piece_of(&span, from..split)));
        }
        self.spans.push((
          Role::Mark,
          RichTextSpan {
            text: WORD_JOINER.to_string(),
            style: span.style.clone(),
          },
        ));
        from = split;
      }
      if from < span.text.len() {
        self.spans.push((role, piece_of(&span, from..span.text.len())));
      }
      offset = end;
    }
    self.inside = start..self.spans.len();
    self.spans.extend(after);
  }

  fn into_block(self) -> LinkBlock {
    let span_of = |role: Role| {
      let has_role = |(span_role, _): &(Role, RichTextSpan)| *span_role == role;
      match (
        self.spans.iter().position(has_role),
        self.spans.iter().rposition(has_role),
      ) {
        (Some(first), Some(last)) => first..last + 1,
        _ => 0..0,
      }
    };
    let label = span_of(Role::Words);
    let trailing = span_of(Role::Run);
    let generated = self
      .spans
      .iter()
      .enumerate()
      .filter(|(_, (role, _))| *role != Role::Words)
      .map(|(index, _)| index..index + 1)
      .collect();
    LinkBlock {
      spans: self.spans.into_iter().map(|(_, span)| span).collect(),
      generated,
      label,
      trailing,
      target: self.inside,
    }
  }
}

fn piece_of(span: &RichTextSpan, bytes: Range<usize>) -> RichTextSpan {
  RichTextSpan {
    text: span.text[bytes].to_owned(),
    style: span.style.clone(),
  }
}
