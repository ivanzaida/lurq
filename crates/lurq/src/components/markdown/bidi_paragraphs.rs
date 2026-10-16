//! Which way the bidi paragraph each link of a paragraph falls in runs.
//!
//! A link is laid out for its bidi paragraph's direction
//! ([`super::link_block`]), which bidi takes from the paragraph's first
//! letter (rule P2), skipping isolates: a link's own words never count. A
//! hard break or another line or paragraph separator starts a new bidi
//! paragraph, and so does an image in a row of texts and images, which ends
//! the text before it. The paragraph's text is stripped of directional
//! controls and marks before it is drawn, so they do not count either.

use std::iter;

use crate::{
  app::theme::ThemeMarkdown,
  layout::text_style::TextStyle,
  markdown::{Direction, MarkdownInline, is_bidi_control, is_paragraph_break, markdown_inline_rich_text},
};

/// The direction of each link's bidi paragraph in `inlines`, in the order
/// the paragraph draws its links. With `flow`, an image outside a link ends
/// the text before it.
pub(super) fn link_directions(inlines: &[MarkdownInline], flow: bool, theme: &ThemeMarkdown) -> Vec<Direction> {
  let mut scan = Scan::default();
  scan.inlines(inlines, flow, theme);
  scan.end_paragraph();
  scan.directions
}

#[derive(Default)]
struct Scan {
  directions: Vec<Direction>,
  /// Links of the current bidi paragraph before its first letter.
  waiting: usize,
  /// The current bidi paragraph's direction, once a letter set it.
  direction: Option<Direction>,
}

impl Scan {
  fn inlines(&mut self, inlines: &[MarkdownInline], flow: bool, theme: &ThemeMarkdown) {
    for inline in inlines {
      match inline {
        MarkdownInline::Link { .. } => match self.direction {
          Some(direction) => self.directions.push(direction),
          None => self.waiting += 1,
        },
        MarkdownInline::Image { .. } if flow => self.end_paragraph(),
        MarkdownInline::Emphasis(children)
        | MarkdownInline::Strong(children)
        | MarkdownInline::Strikethrough(children) => self.inlines(children, flow, theme),
        _ => {
          for span in markdown_inline_rich_text(std::slice::from_ref(inline), &TextStyle::default(), theme) {
            self.text(&span.text);
          }
        }
      }
    }
  }

  fn text(&mut self, text: &str) {
    for ch in text.chars() {
      if is_paragraph_break(ch) {
        self.end_paragraph();
      } else if self.direction.is_none()
        && !is_bidi_control(ch)
        && let Some(direction) = Direction::of_letter(ch)
      {
        self.direction = Some(direction);
        self.settle(direction);
      }
    }
  }

  /// Ends the current bidi paragraph: left to right if it had no letter.
  fn end_paragraph(&mut self) {
    let direction = self.direction.take().unwrap_or(Direction::LeftToRight);
    self.settle(direction);
  }

  fn settle(&mut self, direction: Direction) {
    self
      .directions
      .extend(iter::repeat_n(direction, std::mem::take(&mut self.waiting)));
  }
}
