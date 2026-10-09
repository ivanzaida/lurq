//! How the blocks of a Markdown document take their width: filling the width
//! they are given (the default), or fitting their content up to it
//! ([`MarkdownProps::fit_content`](super::MarkdownProps::fit_content)).

use crate::{
  components::{Column, Row},
  layout::Alignment,
  node::{Element, dimension::Dimension},
};

pub(super) const FILL_WIDTH: Dimension = Dimension::Pct(100.0);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum BlockWidth {
  /// The document is as wide as its container, and every block as wide as
  /// the document.
  Fill,
  /// The document is as wide as its widest block, up to its container's
  /// width, and every block is stretched to the document's width, like the
  /// blocks of a CSS `width: fit-content; max-width: 100%` box.
  Fit,
}

impl BlockWidth {
  pub(super) fn new(fit_content: bool) -> Self {
    if fit_content { Self::Fit } else { Self::Fill }
  }

  /// A column of blocks: the document, a list, a code block. Fitting, it is
  /// as wide as its widest block, at most its container's width, and every
  /// block in it is stretched to that width.
  pub(super) fn blocks(self, column: Column) -> Column {
    match self {
      Self::Fill => column.width(FILL_WIDTH),
      Self::Fit => column.max_width(FILL_WIDTH).align_items(Alignment::Stretch),
    }
  }

  /// A row of one block (a list item, a quote, a footnote). Fitting, it is
  /// as wide as its content, and its column stretches it.
  pub(super) fn row(self, row: Row) -> Row {
    match self {
      Self::Fill => row.width(FILL_WIDTH),
      Self::Fit => row,
    }
  }

  /// The blocks beside a list marker, a quote bar or a footnote label.
  /// Filling, they take the rest of the row. Fitting, they are as wide as
  /// their content and shrink to the rest of the row when that is longer, so
  /// their text wraps there.
  pub(super) fn row_body(self, column: Column) -> Column {
    match self {
      Self::Fill => column.flex(1.0),
      Self::Fit => column
        .flex_shrink(1.0)
        .max_width(FILL_WIDTH)
        .align_items(Alignment::Stretch),
    }
  }

  /// A leaf block: a paragraph, a heading, a label, a line of code, a rule.
  /// Fitting, it is as wide as its content (a text as its longest line, at
  /// most its column's width, where it wraps), and its column stretches it.
  pub(super) fn leaf(self, element: impl Into<Element>) -> Element {
    let element = element.into();
    match self {
      Self::Fill => element.map_node(|node| node.width(FILL_WIDTH)),
      Self::Fit => element,
    }
  }
}
