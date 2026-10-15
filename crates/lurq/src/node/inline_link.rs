//! A link inside a rich-text paragraph.
//!
//! The paragraph stays one rich-text node, so its text wraps and selects as
//! one; each link in it is a child node of that paragraph with no box of its
//! own. Laying out the paragraph places the link where its words fell: the
//! link's bounds are the box of its first laid-out line, and the pointer hits
//! it anywhere on its words and what follows them (its target), line by line.
//! Being a node, a link takes the keyboard focus, is a Tab stop, takes clicks
//! and Enter, shows a pointer cursor and is listed by inspection tools like
//! any control. The paragraph paints the link's states (hover and keyboard
//! focus colors, fills and focus ring) from the link's laid-out lines.

use std::{ops::Range, sync::Arc};

use crate::{
  app::theme::{MarkdownFocusRing, MarkdownLinkState},
  node::border::BorderRadius,
};

/// Which spans of its paragraph a link covers, and how the paragraph paints
/// its states.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct InlineLink {
  /// The spans of the link's words: filled, recolored and ringed.
  pub(crate) label: Range<usize>,
  /// The spans the pointer takes: the words and the runs that follow them.
  pub(crate) target: Range<usize>,
  pub(crate) look: Arc<InlineLinkLook>,
}

impl InlineLink {
  /// The spans after the words that belong to the link.
  pub(crate) fn trailing(&self) -> Range<usize> {
    self.label.end..self.target.end
  }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct InlineLinkLook {
  pub(crate) hovered: MarkdownLinkState,
  pub(crate) focused: MarkdownLinkState,
  pub(crate) focus_ring: Option<MarkdownFocusRing>,
  pub(crate) radius: BorderRadius,
}

/// The bytes `range` of `text` (the concatenated `spans`) without the bytes
/// of the `generated` spans. `generated` is sorted, as a paragraph records
/// it, so spans and ranges are walked together once: linear in both, for a
/// paragraph with many links that renders on every streamed update.
pub(crate) fn content_without_generated(
  spans: &[crate::layout::quad::RichTextSpan],
  generated: &[Range<usize>],
  text: &str,
  range: Range<usize>,
) -> String {
  let mut content = String::with_capacity(range.len());
  let mut offset = 0;
  let mut ranges = generated.iter().peekable();
  for (index, span) in spans.iter().enumerate() {
    let span_bytes = offset..offset + span.text.len();
    offset = span_bytes.end;
    while ranges.next_if(|generated| generated.end <= index).is_some() {}
    if ranges.peek().is_some_and(|generated| generated.contains(&index)) {
      continue;
    }
    let start = span_bytes.start.max(range.start);
    let end = span_bytes.end.min(range.end);
    if let Some(part) = text.get(start..end).filter(|_| start < end) {
      content.push_str(part);
    }
  }
  content
}

#[cfg(test)]
#[path = "inline_link_tests.rs"]
mod tests;
