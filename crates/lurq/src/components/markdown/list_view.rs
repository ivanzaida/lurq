//! A list block the app draws itself
//! ([`MarkdownProps::list_view`](super::MarkdownProps::list_view)).

use std::sync::Arc;

use super::{MarkdownRenderContext, render_block, render_inline_block};
use crate::{
  app::theme::ThemeMarkdown,
  layout::text_style::TextStyle,
  markdown::{MarkdownBlock, MarkdownInline, MarkdownListItem},
  node::Element,
};

pub(super) type MarkdownListView = Arc<dyn Fn(&MarkdownList<'_>) -> Option<Element> + Send + Sync>;

/// A list block of a Markdown document, given to the document's list view.
/// The view draws its items with [`inlines`](Self::inlines) and
/// [`blocks`](Self::blocks), which draw content as the document does: its
/// links are presented, focusable and clickable through the document's hooks
/// and numbered in the order they are drawn.
pub struct MarkdownList<'a> {
  pub(super) ordered: bool,
  pub(super) start: Option<u64>,
  pub(super) items: &'a [MarkdownListItem],
  pub(super) style: &'a TextStyle,
  pub(super) theme: &'a ThemeMarkdown,
  pub(super) render_ctx: &'a MarkdownRenderContext<'a>,
}

impl std::fmt::Debug for MarkdownList<'_> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("MarkdownList")
      .field("ordered", &self.ordered)
      .field("start", &self.start)
      .field("items", &self.items)
      .finish_non_exhaustive()
  }
}

impl MarkdownList<'_> {
  pub fn ordered(&self) -> bool {
    self.ordered
  }

  /// The number of an ordered list's first item, as the source gives it.
  pub fn start(&self) -> Option<u64> {
    self.start
  }

  pub fn items(&self) -> &[MarkdownListItem] {
    self.items
  }

  /// The text style of the document's body text.
  pub fn style(&self) -> &TextStyle {
    self.style
  }

  /// The Markdown theme the document is drawn with.
  pub fn theme(&self) -> &ThemeMarkdown {
    self.theme
  }

  /// `inlines` in `style`, drawn as the document draws a paragraph: one
  /// text that wraps and selects as one, links in it.
  pub fn inlines(&self, inlines: &[MarkdownInline], style: &TextStyle) -> Element {
    render_inline_block(inlines, style.clone(), self.theme, self.render_ctx)
  }

  /// `blocks`, drawn as the document draws them, in the document's style.
  pub fn blocks(&self, blocks: &[MarkdownBlock]) -> Vec<Element> {
    blocks
      .iter()
      .map(|block| render_block(block, self.style, self.theme, self.render_ctx))
      .collect()
  }
}
