//! Links in a Markdown document: what the app is told about each link and
//! how it asks for a link to be shown ([`MarkdownLinkPresentation`]). The
//! paragraph that draws them is `paragraph`.

use std::sync::Arc;

use crate::{
  app::theme::{MarkdownFocusRing, MarkdownInlineStyle, MarkdownLinkState, ThemeMarkdown},
  node::{border::BorderRadius, inline_link::InlineLinkLook},
};

/// A link of a Markdown document, as the document's presentation hook
/// ([`MarkdownProps::link_presentation`](super::MarkdownProps::link_presentation))
/// and [`MarkdownProps::on_link_click`](super::MarkdownProps::on_link_click)
/// see it. Both are given the same link: the words a link shows come from the
/// destination it opens.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MarkdownLink {
  pub(super) destination: Arc<str>,
  pub(super) title: Arc<str>,
  pub(super) text: Arc<str>,
  pub(super) index: usize,
}

impl MarkdownLink {
  /// The destination as the source gives it (a reference link's definition
  /// resolved), not checked or normalized.
  pub fn destination(&self) -> &str {
    &self.destination
  }

  pub fn title(&self) -> &str {
    &self.title
  }

  /// The link's words as drawn, without their formatting (inline HTML as
  /// its text, an image as its alt text) and without directional controls.
  pub fn text(&self) -> &str {
    &self.text
  }

  /// The link's position among the document's links, from 0, in the order
  /// the document draws them: source order, unless a list view
  /// ([`MarkdownProps::list_view`](super::MarkdownProps::list_view)) draws
  /// its items' content in another order.
  pub fn index(&self) -> usize {
    self.index
  }
}

/// Text the presentation puts after a link's words: a host, an icon glyph, an
/// address. Its style applies over the text around the link, not over the
/// link's style.
///
/// Trailing runs are presentation, like CSS `::after` content: they are not
/// part of the paragraph's text when it is copied or read by inspection
/// tools, and a selection does not highlight them. Each run is drawn in its
/// own bidi isolate, without directional controls, so neither the words
/// before it nor its own text can reorder it.
#[derive(Clone, Default, PartialEq)]
pub struct MarkdownRun {
  pub(super) text: Arc<str>,
  pub(super) style: MarkdownInlineStyle,
}

impl std::fmt::Debug for MarkdownRun {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("MarkdownRun")
      .field("text", &self.text)
      .finish_non_exhaustive()
  }
}

impl MarkdownRun {
  pub fn new(text: impl Into<Arc<str>>, style: MarkdownInlineStyle) -> Self {
    Self {
      text: text.into(),
      style,
    }
  }

  /// A run in the style of the text around the link.
  pub fn plain(text: impl Into<Arc<str>>) -> Self {
    Self::new(text, MarkdownInlineStyle::default())
  }
}

/// How a document shows one of its links, returned by the presentation hook
/// ([`MarkdownProps::link_presentation`](super::MarkdownProps::link_presentation)).
///
/// [`link`](Self::link) shows a link: its words in the link style, a Tab
/// stop that a click and Enter follow. Every look not set here is the theme's
/// (`ThemeMarkdown::link`, `link_hovered`, `link_focused`, `link_focus_ring`,
/// `link_radius`). [`text`](Self::text) shows the words as the text around
/// them: nothing to press, nothing to focus.
///
/// Either way, [`trailing`](Self::trailing) runs follow the words in the
/// same paragraph, so they wrap with it: a host after a web link, an address
/// after a link the app does not follow. After a link, the pointer takes them
/// as part of the link, and its hovered and focused looks can recolor them.
///
/// The words and each trailing run are drawn in bidi isolates, stripped of
/// directional controls (see [`crate::markdown::isolate_bidi`]): a
/// right-to-left override in the words or before them cannot reverse a host
/// or an address drawn after them.
#[derive(Clone, Default, PartialEq)]
pub struct MarkdownLinkPresentation {
  pub(super) as_text: bool,
  pub(super) style: Option<MarkdownInlineStyle>,
  pub(super) hovered: Option<MarkdownLinkState>,
  pub(super) focused: Option<MarkdownLinkState>,
  pub(super) focus_ring: Option<Option<MarkdownFocusRing>>,
  pub(super) radius: Option<BorderRadius>,
  pub(super) trailing: Vec<MarkdownRun>,
  pub(super) tooltip: Option<Arc<str>>,
}

impl std::fmt::Debug for MarkdownLinkPresentation {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("MarkdownLinkPresentation")
      .field("as_text", &self.as_text)
      .field("has_style", &self.style.is_some())
      .field("hovered", &self.hovered)
      .field("focused", &self.focused)
      .field("focus_ring", &self.focus_ring)
      .field("radius", &self.radius)
      .field("trailing", &self.trailing)
      .field("tooltip", &self.tooltip)
      .finish()
  }
}

impl MarkdownLinkPresentation {
  /// A link in the theme's link looks.
  pub fn link() -> Self {
    Self::default()
  }

  /// Not a link: the words in the style of the text around them.
  pub fn text() -> Self {
    Self {
      as_text: true,
      ..Self::default()
    }
  }

  /// The words' style over the text around them, instead of the theme's
  /// `link`. An underline goes here (`MarkdownTextStyle::underline`).
  pub fn style(mut self, style: MarkdownInlineStyle) -> Self {
    self.style = Some(style);
    self
  }

  /// The look while the pointer is on the link.
  pub fn hovered(mut self, state: MarkdownLinkState) -> Self {
    self.hovered = Some(state);
    self
  }

  /// The look while the link has keyboard focus.
  pub fn focused(mut self, state: MarkdownLinkState) -> Self {
    self.focused = Some(state);
    self
  }

  /// The ring around the link while it has keyboard focus; `None` draws
  /// none.
  pub fn focus_ring(mut self, ring: Option<MarkdownFocusRing>) -> Self {
    self.focus_ring = Some(ring);
    self
  }

  /// Corner radii of the hovered and focused fills.
  pub fn radius(mut self, radius: BorderRadius) -> Self {
    self.radius = Some(radius);
    self
  }

  /// Runs after the words, in the same paragraph.
  pub fn trailing(mut self, runs: impl IntoIterator<Item = MarkdownRun>) -> Self {
    self.trailing = runs.into_iter().collect();
    self
  }

  /// A tooltip shown while the pointer rests on the link or the link has
  /// keyboard focus. Ignored for [`text`](Self::text).
  pub fn tooltip(mut self, text: impl Into<Arc<str>>) -> Self {
    self.tooltip = Some(text.into());
    self
  }

  pub(super) fn look(&self, theme: &ThemeMarkdown) -> Arc<InlineLinkLook> {
    Arc::new(InlineLinkLook {
      hovered: self.hovered.unwrap_or(theme.link_hovered),
      focused: self.focused.unwrap_or(theme.link_focused),
      focus_ring: self.focus_ring.unwrap_or(theme.link_focus_ring),
      radius: self.radius.unwrap_or(theme.link_radius),
    })
  }
}
