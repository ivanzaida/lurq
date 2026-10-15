//! Links in a Markdown document: what the app is told about each link, how it
//! asks for a link to be shown ([`MarkdownLinkPresentation`]), and the
//! paragraph that draws them.
//!
//! A paragraph is one rich-text node, links included, so it wraps and selects
//! as one text. Each link is a child node of the paragraph placed on its words
//! ([`crate::node::inline_link`]): a Tab stop that Enter and a click follow,
//! with a pointer cursor, hover and focus looks and an optional tooltip.

use std::{
  cell::{Cell, RefCell},
  sync::Arc,
};

use super::tooltip::{LinkTooltip, TooltipHover};
use crate::{
  app::theme::{MarkdownFocusRing, MarkdownInlineStyle, MarkdownLinkState, ThemeMarkdown},
  layout::{quad::RichTextSpan, text_style::TextStyle},
  markdown::{MarkdownInline, markdown_inline_rich_text},
  node::{
    CursorIcon, Node,
    border::BorderRadius,
    inline_link::{InlineLink, InlineLinkLook},
  },
};

pub(super) type MarkdownLinkCallback = Arc<dyn Fn(&MarkdownLink) + Send + Sync>;
pub(super) type MarkdownLinkPresenter = Arc<dyn Fn(&MarkdownLink) -> MarkdownLinkPresentation + Send + Sync>;

/// A link of a Markdown document, as the document's presentation hook
/// ([`MarkdownProps::link_presentation`](super::MarkdownProps::link_presentation))
/// and [`MarkdownProps::on_link_click`](super::MarkdownProps::on_link_click)
/// see it. Both are given the same link: the words a link shows come from the
/// destination it opens.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MarkdownLink {
  destination: Arc<str>,
  title: Arc<str>,
  text: Arc<str>,
  index: usize,
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

  /// The plain text of the link's words, without their formatting.
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
#[derive(Clone, Default, PartialEq)]
pub struct MarkdownRun {
  text: Arc<str>,
  style: MarkdownInlineStyle,
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
#[derive(Clone, Default, PartialEq)]
pub struct MarkdownLinkPresentation {
  as_text: bool,
  style: Option<MarkdownInlineStyle>,
  hovered: Option<MarkdownLinkState>,
  focused: Option<MarkdownLinkState>,
  focus_ring: Option<Option<MarkdownFocusRing>>,
  radius: Option<BorderRadius>,
  trailing: Vec<MarkdownRun>,
  tooltip: Option<Arc<str>>,
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

  fn look(&self, theme: &ThemeMarkdown) -> Arc<InlineLinkLook> {
    Arc::new(InlineLinkLook {
      hovered: self.hovered.unwrap_or(theme.link_hovered),
      focused: self.focused.unwrap_or(theme.link_focused),
      focus_ring: self.focus_ring.unwrap_or(theme.link_focus_ring),
      radius: self.radius.unwrap_or(theme.link_radius),
    })
  }
}

/// What the links of one render of a document share: their numbering, the
/// app's hooks and the tooltips they register.
pub(super) struct LinkSession {
  next_index: Cell<usize>,
  presenter: Option<MarkdownLinkPresenter>,
  on_link_click: Option<MarkdownLinkCallback>,
  #[cfg(feature = "router")]
  navigator: Option<crate::router::Navigator>,
  hover: TooltipHover,
  tooltips: RefCell<Vec<LinkTooltip>>,
}

impl LinkSession {
  pub(super) fn new(
    presenter: Option<MarkdownLinkPresenter>,
    on_link_click: Option<MarkdownLinkCallback>,
    #[cfg(feature = "router")] navigator: Option<crate::router::Navigator>,
    hover: TooltipHover,
  ) -> Self {
    Self {
      next_index: Cell::new(0),
      presenter,
      on_link_click,
      #[cfg(feature = "router")]
      navigator,
      hover,
      tooltips: RefCell::new(Vec::new()),
    }
  }

  /// The tooltips the drawn links registered, in order.
  pub(super) fn take_tooltips(&self) -> Vec<LinkTooltip> {
    self.tooltips.take()
  }

  fn next_link(&self, destination: &str, title: &str, children: &[MarkdownInline]) -> MarkdownLink {
    let index = self.next_index.get();
    self.next_index.set(index + 1);
    MarkdownLink {
      destination: Arc::from(destination),
      title: Arc::from(title),
      text: Arc::from(plain_inline_text(children)),
      index,
    }
  }

  fn present(&self, link: &MarkdownLink) -> MarkdownLinkPresentation {
    self
      .presenter
      .as_ref()
      .map_or_else(MarkdownLinkPresentation::link, |present| present(link))
  }

  /// What a press or Enter on `link` does: the app's callback, or else a
  /// router navigation to an in-app path.
  fn click_handler(&self, link: MarkdownLink) -> impl Fn(crate::app::events::MouseEvent) + Send + Sync + 'static {
    let on_link_click = self.on_link_click.clone();
    #[cfg(feature = "router")]
    let navigator = self.navigator.clone();
    move |_| {
      if let Some(on_link_click) = &on_link_click {
        on_link_click(&link);
        return;
      }
      #[cfg(feature = "router")]
      if let Some(navigator) = &navigator
        && is_router_link(link.destination())
      {
        navigator.push(link.destination().to_owned());
      }
    }
  }

  /// `node` with the tooltip `text`: it registers an anchor that the
  /// document's tooltip shows at while the pointer rests on it.
  fn with_tooltip(&self, node: Node, text: Option<Arc<str>>) -> Node {
    let Some(text) = text else {
      return node;
    };
    let mut tooltips = self.tooltips.borrow_mut();
    let slot = tooltips.len();
    let anchor = self.hover.anchor(slot);
    tooltips.push(LinkTooltip::new(anchor.clone(), text));
    let (enter, leave) = (self.hover.clone(), self.hover.clone());
    node
      .ref_element(anchor)
      .on_mouse_enter(move || enter.enter(slot))
      .on_mouse_leave(move || leave.leave(slot))
  }
}

#[cfg(feature = "router")]
fn is_router_link(destination: &str) -> bool {
  destination.starts_with('/') || destination.starts_with('#') || destination.starts_with('?')
}

/// What a row of texts and images holds, in order.
pub(super) enum FlowItem<'i> {
  Text(Box<Node>),
  /// An image, and the style of the text it sits in.
  Image(&'i MarkdownInline, TextStyle),
}

/// Builds one paragraph: its spans and the link nodes placed on them.
pub(super) struct Paragraph<'a> {
  spans: Vec<RichTextSpan>,
  /// Spans before this index are closed: nothing merges into them, so a
  /// link's spans stay its own.
  sealed: usize,
  links: Vec<Node>,
  theme: &'a ThemeMarkdown,
  session: &'a LinkSession,
}

impl<'a> Paragraph<'a> {
  pub(super) fn new(theme: &'a ThemeMarkdown, session: &'a LinkSession) -> Self {
    Self {
      spans: Vec::new(),
      sealed: 0,
      links: Vec::new(),
      theme,
      session,
    }
  }

  pub(super) fn is_empty(&self) -> bool {
    self.spans.is_empty()
  }

  /// The paragraph so far as one rich-text node; the builder starts over.
  pub(super) fn take_node(&mut self) -> Node {
    self.sealed = 0;
    Node::rich_text_with_links(std::mem::take(&mut self.spans), std::mem::take(&mut self.links))
  }

  /// Adds `inlines` in `style`. With `flow`, an image outside a link ends
  /// the paragraph so far and goes into `flow` after it, for a row of texts
  /// and images; without it, an image is its alt text.
  pub(super) fn push_inlines<'i>(
    &mut self,
    inlines: &'i [MarkdownInline],
    style: &TextStyle,
    flow: &mut Option<&mut Vec<FlowItem<'i>>>,
  ) {
    for inline in inlines {
      match inline {
        MarkdownInline::Link {
          destination,
          title,
          children,
        } => self.push_link(destination, title, children, style),
        MarkdownInline::Image { .. } if flow.is_some() => {
          if let Some(flow) = flow.as_mut() {
            if !self.is_empty() {
              flow.push(FlowItem::Text(Box::new(self.take_node())));
            }
            flow.push(FlowItem::Image(inline, style.clone()));
          }
        }
        MarkdownInline::Emphasis(children) => {
          self.push_inlines(children, &self.theme.emphasis.apply(style), flow);
        }
        MarkdownInline::Strong(children) => {
          self.push_inlines(children, &self.theme.strong.apply(style), flow);
        }
        MarkdownInline::Strikethrough(children) => {
          self.push_inlines(children, &self.theme.strikethrough.apply(style), flow);
        }
        _ => {
          for span in markdown_inline_rich_text(std::slice::from_ref(inline), style, self.theme) {
            self.push_span(span);
          }
        }
      }
    }
  }

  fn push_link(&mut self, destination: &str, title: &str, children: &[MarkdownInline], style: &TextStyle) {
    let link = self.session.next_link(destination, title, children);
    let presentation = self.session.present(&link);
    if presentation.as_text {
      // The words of a link have no links in them (CommonMark), and an image
      // in them is its alt text, as in any link.
      self.push_inlines(children, style, &mut None);
      self.push_runs(&presentation.trailing, style);
      return;
    }
    let label_start = self.seal();
    let link_style = presentation.style.as_ref().unwrap_or(&self.theme.link).apply(style);
    self.push_inlines(children, &link_style, &mut None);
    let label_end = self.seal();
    self.push_runs(&presentation.trailing, style);
    let target_end = self.seal();
    if label_start == label_end {
      return;
    }
    let inline_link = InlineLink {
      label: label_start..label_end,
      target: label_start..target_end,
      look: presentation.look(self.theme),
    };
    let node = Node::inline_link(inline_link, link.text())
      .tab_index(0)
      .cursor(CursorIcon::Pointer)
      .on_click(self.session.click_handler(link));
    self.links.push(self.session.with_tooltip(node, presentation.tooltip));
  }

  fn push_runs(&mut self, runs: &[MarkdownRun], style: &TextStyle) {
    for run in runs {
      self.push_span(RichTextSpan {
        text: run.text.to_string(),
        style: run.style.apply(style),
      });
    }
  }

  fn push_span(&mut self, span: RichTextSpan) {
    if span.text.is_empty() {
      return;
    }
    if self.spans.len() > self.sealed
      && let Some(last) = self.spans.last_mut()
      && last.style == span.style
    {
      last.text.push_str(&span.text);
      return;
    }
    self.spans.push(span);
  }

  /// Closes the spans so far; returns their count.
  fn seal(&mut self) -> usize {
    self.sealed = self.spans.len();
    self.sealed
  }
}

pub(super) fn plain_inline_text(inlines: &[MarkdownInline]) -> String {
  let mut text = String::new();
  for inline in inlines {
    match inline {
      MarkdownInline::Text(value)
      | MarkdownInline::Code(value)
      | MarkdownInline::Math(value)
      | MarkdownInline::Html(value)
      | MarkdownInline::FootnoteReference(value) => text.push_str(value),
      MarkdownInline::Emphasis(children)
      | MarkdownInline::Strong(children)
      | MarkdownInline::Strikethrough(children)
      | MarkdownInline::Link { children, .. } => text.push_str(&plain_inline_text(children)),
      MarkdownInline::Image { alt, .. } => text.push_str(&plain_inline_text(alt)),
      MarkdownInline::SoftBreak | MarkdownInline::HardBreak => text.push(' '),
      MarkdownInline::TaskListMarker(checked) => text.push_str(if *checked { "[x] " } else { "[ ] " }),
    }
  }
  text
}
