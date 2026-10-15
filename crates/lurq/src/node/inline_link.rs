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
