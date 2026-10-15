//! The paragraph that draws a document's links.
//!
//! A paragraph is one rich-text node, links included, so it wraps and selects
//! as one text. Each link is a child node of the paragraph placed on its words
//! ([`crate::node::inline_link`]): a Tab stop that Enter and a click follow,
//! with a pointer cursor, hover and focus looks and an optional tooltip.
//!
//! A link's words and each trailing run are drawn between invisible isolate
//! marks, stripped of directional controls ([`crate::markdown::isolate_bidi`]).
//! The marks and the trailing runs are presentation: the paragraph's copied
//! and inspected text leaves them out.

use std::{
  cell::{Cell, RefCell},
  collections::HashMap,
  ops::Range,
  sync::Arc,
};

use super::{
  links::{MarkdownLink, MarkdownLinkPresentation},
  tooltip::{LinkTooltip, TooltipHover, TooltipKey},
};
use crate::{
  app::theme::ThemeMarkdown,
  layout::{quad::RichTextSpan, text_style::TextStyle},
  markdown::{
    FIRST_STRONG_ISOLATE, MarkdownInline, POP_DIRECTIONAL_ISOLATE, markdown_inline_rich_text, strip_bidi_controls,
  },
  node::{CursorIcon, Node, inline_link::InlineLink},
};

pub(super) type MarkdownLinkCallback = Arc<dyn Fn(&MarkdownLink) + Send + Sync>;
pub(super) type MarkdownLinkPresenter = Arc<dyn Fn(&MarkdownLink) -> MarkdownLinkPresentation + Send + Sync>;

/// What the links of one render of a document share: their numbering, the
/// app's hooks and the tooltips they register.
pub(super) struct LinkSession {
  next_index: Cell<usize>,
  /// How many links to each destination came so far, for tooltip keys.
  occurrences: RefCell<HashMap<Arc<str>, usize>>,
  presenter: Option<MarkdownLinkPresenter>,
  on_link_click: Option<MarkdownLinkCallback>,
  #[cfg(feature = "router")]
  navigator: Option<crate::router::Navigator>,
  hover: TooltipHover,
  tooltips: RefCell<Vec<LinkTooltip>>,
}

/// Where a session was, to forget the links drawn after it.
pub(super) struct LinkCheckpoint {
  next_index: usize,
  occurrences: HashMap<Arc<str>, usize>,
  tooltips: usize,
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
      occurrences: RefCell::new(HashMap::new()),
      presenter,
      on_link_click,
      #[cfg(feature = "router")]
      navigator,
      hover,
      tooltips: RefCell::new(Vec::new()),
    }
  }

  /// The tooltips the drawn links registered, in order; anchors of links
  /// that are gone are forgotten.
  pub(super) fn take_tooltips(&self) -> Vec<LinkTooltip> {
    let tooltips = self.tooltips.take();
    self.hover.keep_anchors(&tooltips);
    tooltips
  }

  pub(super) fn checkpoint(&self) -> LinkCheckpoint {
    LinkCheckpoint {
      next_index: self.next_index.get(),
      occurrences: self.occurrences.borrow().clone(),
      tooltips: self.tooltips.borrow().len(),
    }
  }

  /// Forgets the links drawn since `checkpoint` (drawn and then discarded).
  pub(super) fn restore(&self, checkpoint: LinkCheckpoint) {
    self.next_index.set(checkpoint.next_index);
    *self.occurrences.borrow_mut() = checkpoint.occurrences;
    self.tooltips.borrow_mut().truncate(checkpoint.tooltips);
  }

  fn next_link(&self, destination: &str, title: &str, words: String) -> (MarkdownLink, TooltipKey) {
    let index = self.next_index.get();
    self.next_index.set(index + 1);
    let destination: Arc<str> = Arc::from(destination);
    let occurrence = {
      let mut occurrences = self.occurrences.borrow_mut();
      let count = occurrences.entry(destination.clone()).or_default();
      *count += 1;
      *count - 1
    };
    let link = MarkdownLink {
      destination: destination.clone(),
      title: Arc::from(title),
      text: Arc::from(words),
      index,
    };
    (link, TooltipKey::new(destination, occurrence))
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
  fn with_tooltip(&self, node: Node, key: TooltipKey, text: Option<Arc<str>>) -> Node {
    let Some(text) = text else {
      return node;
    };
    let anchor = self.hover.anchor(&key);
    self
      .tooltips
      .borrow_mut()
      .push(LinkTooltip::new(key.clone(), anchor.clone(), text));
    let (enter, leave) = (self.hover.clone(), self.hover.clone());
    let enter_key = key.clone();
    node
      .ref_element(anchor)
      .on_mouse_enter(move || enter.enter(&enter_key))
      .on_mouse_leave(move || leave.leave(&key))
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
  /// Spans that are presentation, not the paragraph's text: isolate marks
  /// and trailing runs.
  generated: Vec<Range<usize>>,
  /// Inside a link's words: directional controls are dropped.
  in_words: bool,
  links: Vec<Node>,
  theme: &'a ThemeMarkdown,
  session: &'a LinkSession,
}

impl<'a> Paragraph<'a> {
  pub(super) fn new(theme: &'a ThemeMarkdown, session: &'a LinkSession) -> Self {
    Self {
      spans: Vec::new(),
      sealed: 0,
      generated: Vec::new(),
      in_words: false,
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
    Node::rich_text_with_links(
      std::mem::take(&mut self.spans),
      std::mem::take(&mut self.links),
      std::mem::take(&mut self.generated),
    )
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
    let (link, key) = self
      .session
      .next_link(destination, title, link_words(children, self.theme));
    let presentation = self.session.present(&link);
    let words_style = if presentation.as_text {
      style.clone()
    } else {
      presentation.style.as_ref().unwrap_or(&self.theme.link).apply(style)
    };
    let label = self.push_isolated(&words_style, |paragraph| {
      // The words of a link have no links in them (CommonMark), and an image
      // in them is its alt text, as in any link.
      paragraph.in_words = true;
      paragraph.push_inlines(children, &words_style, &mut None);
      paragraph.in_words = false;
    });
    for run in &presentation.trailing {
      let run_style = run.style.apply(style);
      let run_span = self.push_isolated(&run_style, |paragraph| {
        paragraph.push_span(RichTextSpan {
          text: strip_bidi_controls(&run.text).into_owned(),
          style: run_style.clone(),
        });
      });
      self.generated.push(run_span);
    }
    let target_end = self.seal();
    if presentation.as_text || label.is_empty() {
      return;
    }
    let inline_link = InlineLink {
      label: label.clone(),
      target: label.start..target_end,
      look: presentation.look(self.theme),
    };
    let node = Node::inline_link(inline_link, link.text())
      .tab_index(0)
      .cursor(CursorIcon::Pointer)
      .on_click(self.session.click_handler(link));
    self
      .links
      .push(self.session.with_tooltip(node, key, presentation.tooltip));
  }

  /// Draws what `push` adds between isolate marks in `style`; returns the
  /// spans `push` added, sealed.
  fn push_isolated(&mut self, style: &TextStyle, push: impl FnOnce(&mut Self)) -> Range<usize> {
    self.push_mark(FIRST_STRONG_ISOLATE, style);
    let start = self.spans.len();
    push(self);
    let end = self.seal();
    self.push_mark(POP_DIRECTIONAL_ISOLATE, style);
    start..end
  }

  /// An isolate mark: a span of its own, never merged, never copied.
  fn push_mark(&mut self, mark: char, style: &TextStyle) {
    let index = self.seal();
    self.spans.push(RichTextSpan {
      text: mark.to_string(),
      style: style.clone(),
    });
    self.generated.push(index..index + 1);
    self.seal();
  }

  fn push_span(&mut self, mut span: RichTextSpan) {
    if self.in_words {
      span.text = strip_bidi_controls(&span.text).into_owned();
    }
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

/// A link's words as the paragraph draws them, as plain text: inline HTML as
/// its text, an image as its alt text, breaks as spaces, without directional
/// controls.
fn link_words(children: &[MarkdownInline], theme: &ThemeMarkdown) -> String {
  let words = markdown_inline_rich_text(children, &TextStyle::default(), theme)
    .into_iter()
    .map(|span| span.text)
    .collect::<String>()
    .replace('\n', " ");
  strip_bidi_controls(&words).into_owned()
}
