use std::{
  cell::Cell,
  collections::{HashMap, HashSet},
  sync::{Arc, Mutex},
};

use crate::{
  app::{
    component::Component,
    ctx::Ctx,
    theme::{MarkdownBlockStyle, PaletteColor, Theme, ThemeMarkdown, ThemePalette},
  },
  components::{Column, Rect, Row, ScrollHorizontal, Text},
  core::{Memo, Signal},
  layout::{
    Alignment,
    layout_kind::Justify,
    quad::RichTextSpan,
    scrollbar::{ScrollBarPlacement, ScrollBarStyle},
    text_style::{TextAlign, TextStyle},
  },
  markdown::{
    MarkdownBlock, MarkdownCodeBlockKind, MarkdownDocument, MarkdownHeadingLevel, MarkdownInline, MarkdownListItem,
    MarkdownTableAlignment, MarkdownTableRow, markdown_html_text, markdown_inline_rich_text, parse_markdown,
  },
  node::{Element, Node, color::Color, dimension::Dimension, text_highlight::TextHighlight},
  ui::{Case, InlineCode, InlineCodePart, Kbd, KbdPart, Length},
};

mod bidi_paragraphs;
mod blocks;
mod document_text;
mod link_block;
mod links;
mod list_view;
mod paragraph;
mod sizing;
mod tooltip;

use document_text::DocumentText;
pub use links::{MarkdownLink, MarkdownLinkPresentation, MarkdownRun};
pub use list_view::MarkdownList;
use list_view::MarkdownListView;
use paragraph::{
  FlowItem, LinkSession, MarkdownJump, MarkdownLinkActivator, MarkdownLinkCallback, MarkdownLinkPresenter, Paragraph,
};

/// What a task's change tells the app: the task's index among the document's tasks, and whether it is checked now.
pub(super) type TaskCallback = Arc<dyn Fn(usize, bool) + Send + Sync>;
use sizing::{BlockWidth, FILL_WIDTH};
use tooltip::{LinkTooltips, LinkTooltipsProps, TooltipHover};

#[derive(Clone)]
pub struct MarkdownProps {
  source: Arc<str>,
  style: TextStyle,
  theme: Option<ThemeMarkdown>,
  selectable: bool,
  width: Option<Dimension>,
  fit_content: bool,
  on_link_select: Option<MarkdownLinkCallback>,
  on_link_activate: Option<MarkdownLinkActivator>,
  link_presentation: Option<MarkdownLinkPresenter>,
  list_view: Option<MarkdownListView>,
  highlights: Arc<[TextHighlight]>,
  on_task_change: Option<TaskCallback>,
}

impl PartialEq for MarkdownProps {
  fn eq(&self, other: &Self) -> bool {
    self.source == other.source
      && self.style == other.style
      && self.theme == other.theme
      && self.selectable == other.selectable
      && self.width == other.width
      && self.fit_content == other.fit_content
      && same_callback(&self.on_link_select, &other.on_link_select)
      && same_callback(&self.on_link_activate, &other.on_link_activate)
      && same_callback(&self.link_presentation, &other.link_presentation)
      && same_callback(&self.list_view, &other.list_view)
      && same_callback(&self.on_task_change, &other.on_task_change)
      && self.highlights == other.highlights
  }
}

impl MarkdownProps {
  pub fn new(source: impl Into<Arc<str>>) -> Self {
    Self {
      source: source.into(),
      style: TextStyle::default(),
      theme: None,
      selectable: false,
      width: None,
      fit_content: false,
      on_link_select: None,
      on_link_activate: None,
      link_presentation: None,
      list_view: None,
      highlights: Arc::from([]),
      on_task_change: None,
    }
  }

  /// Called when a person checks or unchecks a task (`- [ ]`, `- [x]`),
  /// with the task's index among the document's tasks, from 0, and whether
  /// it is checked now. The source is the app's: it writes the change into
  /// it. Without it a task's box is read only: it says whether the task is
  /// done, and takes neither the pointer nor the focus.
  pub fn on_task_change(mut self, handler: impl Fn(usize, bool) + Send + Sync + 'static) -> Self {
    self.on_task_change = Some(Arc::new(handler));
    self
  }

  pub fn styled(source: impl Into<Arc<str>>, style: TextStyle) -> Self {
    Self::new(source).style(style)
  }

  pub fn style(mut self, style: TextStyle) -> Self {
    self.style = style;
    self
  }

  pub fn theme(mut self, theme: ThemeMarkdown) -> Self {
    self.theme = Some(theme);
    self
  }

  /// Fills behind ranges of the document's words, such as the matches of a
  /// search: one per laid-out line a range covers, each in its own color,
  /// under the glyphs and under a selection. Ranges are in bytes of the
  /// document's rendered text ([`Self::rendered_text`]), not of its source;
  /// a range may run over several paragraphs. Links, code and lists render
  /// as without them.
  pub fn highlights(mut self, highlights: impl Into<Arc<[TextHighlight]>>) -> Self {
    self.highlights = highlights.into();
    self
  }

  /// The document's words as it renders them, the text
  /// [`Self::highlights`] ranges index: each paragraph, heading, table cell,
  /// code block and HTML block in document order, one line feed between
  /// two. List markers, footnote labels and a code block's language are not
  /// in it, nor link marks added for display; a link is its words. Search
  /// this text to find what to highlight. Built with these props' link
  /// presentation and list view; it renders the document to find it.
  pub fn rendered_text(&self) -> String {
    let (_, text) = self.still(&crate::ui::current_theme());
    text
  }

  /// The document in `theme` once, with nothing kept: its links followed
  /// as the props say, with no tooltips, every `<details>` as it begins,
  /// and no way to a footnote but the scroll. For a window of its own, a
  /// test, a picture. Mounted ([`Markdown::mount`]) it keeps what a person
  /// does to it.
  pub fn build(&self, theme: &Theme) -> Element {
    let (element, _) = self.still(theme);
    element
  }

  /// The document built in `theme` with nothing kept, and its rendered text.
  fn still(&self, ui: &Theme) -> (Element, String) {
    let document = parse_markdown(&self.source);
    let theme = self
      .theme
      .clone()
      .unwrap_or_else(|| ui.markdown().clone())
      .resolve(&ui.palette());
    let links = LinkSession::new(
      self.link_presentation.clone(),
      None,
      None,
      #[cfg(feature = "router")]
      None,
      TooltipHover::new(crate::app::ctx::Timeout::detached),
    );
    let (code_fill, kbd_fill) = mark_fills(ui);
    let links = links.with_fills(code_fill, kbd_fill);
    links.set_footnotes(footnote_numbers(&document));
    let text = DocumentText::new(&self.highlights);
    let render_ctx = MarkdownRenderContext {
      width: BlockWidth::new(self.fit_content),
      links: &links,
      list_view: self.list_view.as_ref(),
      code_block_scrollbar: ScrollBarStyle {
        placement: ScrollBarPlacement::Overlay,
        ..theme
          .code_block_scrollbar
          .clone()
          .unwrap_or_else(|| ui.scrollbar().clone())
      },
      code_blocks: Cell::new(0),
      text: &text,
      ui: ui.clone(),
      depth: Cell::new(0),
      tasks: Cell::new(0),
      on_task_change: self.on_task_change.as_ref(),
      details: Cell::new(0),
      details_turned: Vec::new(),
      turn_details: None,
    };
    let style = document_style(&self.style, ui);
    let mut element: Element = render_document(&document, &style, &theme, &render_ctx).into();
    if let Some(width) = self.width {
      element = element.map_node(|node| node.width(width));
    }
    if self.selectable {
      element.node.selectable_recursive(true);
    }
    (element, text.into_text())
  }

  pub fn selectable(mut self, selectable: bool) -> Self {
    self.selectable = selectable;
    self
  }

  pub fn width(mut self, width: impl Into<Dimension>) -> Self {
    self.width = Some(width.into());
    self
  }

  /// Sizes the document to its content instead of filling its container,
  /// like CSS `width: fit-content; max-width: 100%`: the document is as wide
  /// as its widest block, and never wider than its container, where its
  /// text wraps. Off by default: the document and every block in it fill the
  /// container's width.
  ///
  /// A short message is as wide as its longest line, so a chat bubble around
  /// it hugs its text. Every block is then stretched to the document's
  /// width, as CSS blocks are: a code block's box and a rule span the widest
  /// block. A code block does not wrap, so it is as wide as its longest line
  /// up to the container's width, and a longer line scrolls sideways inside
  /// its box, as when filling. A table still fills the container's width, since its
  /// columns share that width and line up across its rows. An explicit
  /// [`width`](Self::width) sets the document's width as before.
  ///
  /// The container's width is the most a `Column` gives its children. A `Row`
  /// gives its children unbounded width, so a bubble in a row needs a
  /// `max_width` of `Dimension::Pct(100.0)` (or a `flex_shrink` factor) for
  /// its text to wrap at the row's width.
  pub fn fit_content(mut self, fit_content: bool) -> Self {
    self.fit_content = fit_content;
    self
  }

  /// Called when a link is followed: clicked with the left button, or Enter
  /// while it has keyboard focus. It runs inside the input's dispatch, so
  /// [`is_synthetic_input`](crate::app::events::is_synthetic_input) tells a
  /// person's press from synthetic input (an MCP client's). It is given the
  /// same [`MarkdownLink`] the presentation hook saw, with its
  /// [`index`](MarkdownLink::index). Without it and without
  /// [`on_link_activate`](Self::on_link_activate), a link to an in-app path
  /// (`/`, `#` or `?`) navigates the router, when there is one.
  pub fn on_link_select(mut self, f: impl Fn(&MarkdownLink) + Send + Sync + 'static) -> Self {
    self.on_link_select = Some(Arc::new(f));
    self
  }

  /// Called when a link is activated, with the input that did it: a left
  /// click, a middle click, or Enter while the link has keyboard focus. The
  /// [`MouseEvent`](crate::app::events::MouseEvent) says which: its `button`
  /// (`Left` or `Middle`; `Left` for Enter), its modifiers (`shift`, `ctrl`,
  /// `alt`, and `meta`: Cmd on macOS, the Windows key on Windows) and
  /// `from_keyboard` for Enter. An app follows browser rules with it: a
  /// middle click or Ctrl+click (Cmd+click on macOS) opens the link in the
  /// background, Ctrl+Shift+click opens it in front.
  ///
  /// A middle click is a press and a release of the middle button on the
  /// same link, like a left click. Its press starts no text selection or
  /// drag. A press released off the link activates nothing, and neither
  /// does a press on one link released on another more than a few pixels
  /// away; closer, the link under the release activates, as with a left
  /// click.
  /// The callback runs inside the input's dispatch, so
  /// [`is_synthetic_input`](crate::app::events::is_synthetic_input) tells a
  /// person from an MCP client here too.
  ///
  /// It runs before [`on_link_select`](Self::on_link_select), which still
  /// gets left clicks and Enter when both are set. With it, a link to an
  /// in-app path no longer navigates the router by itself.
  pub fn on_link_activate(
    mut self,
    f: impl Fn(&MarkdownLink, &crate::app::events::MouseEvent) + Send + Sync + 'static,
  ) -> Self {
    self.on_link_activate = Some(Arc::new(f));
    self
  }

  /// Decides how each link is shown: as a link with its own looks, trailing
  /// runs and tooltip, or as text ([`MarkdownLinkPresentation`]). It is
  /// called once per link and render with the exact link the document
  /// follows, so what a link shows always comes from the destination it
  /// opens. Without it, every link is a link in the theme's looks.
  ///
  /// A link stays inside its paragraph's text, which wraps and selects as
  /// one. A link is a Tab stop (in document order), Enter and a click follow
  /// it, and the pointer over it shows a pointer cursor. Its hovered and
  /// focused looks recolor its words, underline and trailing runs and can
  /// fill behind its words; keyboard focus also draws a ring around them. A
  /// link presented with [`MarkdownLinkPresentation::pointer_only`] is no
  /// Tab stop and takes no focus.
  ///
  /// Keep the props (or the closure's `Arc`) across renders where you can:
  /// props with a new closure compare unequal and render the document again.
  pub fn link_presentation(
    mut self,
    f: impl Fn(&MarkdownLink) -> MarkdownLinkPresentation + Send + Sync + 'static,
  ) -> Self {
    self.link_presentation = Some(Arc::new(f));
    self
  }

  /// Lets the app draw a list block itself: called for every list of the
  /// document, it returns the element to draw instead, or `None` to draw the
  /// list as usual. Decide from [`MarkdownList::items`] before drawing
  /// anything with the list's `inlines` or `blocks`, since drawing numbers
  /// the links it draws.
  pub fn list_view(mut self, f: impl Fn(&MarkdownList<'_>) -> Option<Element> + Send + Sync + 'static) -> Self {
    self.list_view = Some(Arc::new(f));
    self
  }
}

fn same_callback<T: ?Sized>(left: &Option<Arc<T>>, right: &Option<Arc<T>>) -> bool {
  match (left, right) {
    (None, None) => true,
    (Some(left), Some(right)) => Arc::ptr_eq(left, right),
    _ => false,
  }
}

#[cfg(feature = "devtools")]
impl crate::app::component::DevtoolsInspectable for MarkdownProps {
  fn write_info(&self, buffer: &mut Vec<crate::app::component::ComponentInfo>) {
    buffer.push(crate::app::component::ComponentInfo::with_value(
      "source",
      std::any::type_name::<Arc<str>>(),
      format!("len={}", self.source.len()),
    ));
  }
}

/// The document once, in the theme of what is being rendered, with nothing kept ([`MarkdownProps::build`]).
impl From<MarkdownProps> for Element {
  fn from(props: MarkdownProps) -> Self {
    props.build(&crate::ui::current_theme())
  }
}

pub struct Markdown {
  source: Signal<Arc<str>>,
  document: Memo<MarkdownDocument>,
  hover: TooltipHover,
  /// The `<details>` a person turned from how they began, by ordinal.
  details: Signal<Vec<usize>>,
  /// A jump asked for (a footnote, the way back from one): the anchor, and the render that takes it.
  pending_jump: Arc<Mutex<Option<String>>>,
  jumps: Signal<u64>,
}

struct MarkdownRenderContext<'a> {
  width: BlockWidth,
  links: &'a LinkSession,
  list_view: Option<&'a MarkdownListView>,
  /// The bar of a code block that overflows sideways: the Markdown theme's,
  /// or else the app theme's, always drawn over the box's padding so a
  /// fitting block keeps its size whatever the app's bars reserve.
  code_block_scrollbar: ScrollBarStyle,
  /// How many code blocks this render has drawn: the next one's ordinal.
  code_blocks: Cell<usize>,
  /// The rendered text so far, and the highlights its leaves take.
  text: &'a DocumentText<'a>,
  /// The theme the system's parts in the document are built in: a task's box, an admonition.
  ui: Theme,
  /// How many lists deep the block being drawn is.
  depth: Cell<usize>,
  /// How many tasks this render has drawn: the next one's index.
  tasks: Cell<usize>,
  on_task_change: Option<&'a TaskCallback>,
  /// How many `<details>` this render has drawn: the next one's ordinal.
  details: Cell<usize>,
  /// The `<details>` a person turned from how they began.
  details_turned: Vec<usize>,
  turn_details: Option<Signal<Vec<usize>>>,
}

impl Markdown {
  pub fn mount(ctx: &mut Ctx, props: MarkdownProps) -> Element {
    ctx.mount::<Self>(props)
  }
}

impl Component for Markdown {
  type Props = MarkdownProps;

  fn create(ctx: &mut Ctx) -> Self {
    let source = ctx.signal(ctx.props::<Self::Props>().source.clone());
    let memo_source = source.clone();
    let document = ctx.memo(move || parse_markdown(&memo_source.get()));
    let hover = TooltipHover::new(|fire| ctx.create_timeout(std::time::Duration::ZERO, fire));
    Self {
      source,
      document,
      hover,
      details: ctx.signal(Vec::new()),
      pending_jump: Arc::new(Mutex::new(None)),
      jumps: ctx.signal(0),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let props = ctx.props::<Self::Props>().clone();
    if self.source.get_untracked() != props.source {
      self.source.set(props.source.clone());
    }
    let ui = ctx.theme().clone();
    // The document's colors are the palette's roles: drawn in the theme it is in.
    let theme = props
      .theme
      .clone()
      .unwrap_or_else(|| ui.markdown().clone())
      .resolve(&ui.palette());
    self.hover.set_delay(theme.link_tooltip_delay);
    let document = self.document.get();
    // A jump to a footnote or back: the render it asks for scrolls to the anchor.
    let _ = self.jumps.get();
    let jump: MarkdownJump = {
      let (pending, jumps) = (self.pending_jump.clone(), self.jumps.clone());
      Arc::new(move |target: &str| {
        *pending.lock().unwrap() = Some(target.to_owned());
        jumps.update(|count| *count += 1);
      })
    };
    let (code_fill, kbd_fill) = mark_fills(&ui);
    let links = LinkSession::new(
      props.link_presentation.clone(),
      props.on_link_select.clone(),
      props.on_link_activate.clone(),
      #[cfg(feature = "router")]
      ctx.use_context::<crate::router::Navigator>(),
      self.hover.clone(),
    )
    .with_jump(jump)
    .with_fills(code_fill, kbd_fill);
    links.set_footnotes(footnote_numbers(&document));
    let code_block_scrollbar = ScrollBarStyle {
      placement: ScrollBarPlacement::Overlay,
      ..theme
        .code_block_scrollbar
        .clone()
        .unwrap_or_else(|| ctx.theme().scrollbar().clone())
    };
    let text = DocumentText::new(&props.highlights);
    let render_ctx = MarkdownRenderContext {
      width: BlockWidth::new(props.fit_content),
      links: &links,
      list_view: props.list_view.as_ref(),
      code_block_scrollbar,
      code_blocks: Cell::new(0),
      text: &text,
      ui: ui.clone(),
      depth: Cell::new(0),
      tasks: Cell::new(0),
      on_task_change: props.on_task_change.as_ref(),
      details: Cell::new(0),
      details_turned: self.details.get(),
      turn_details: Some(self.details.clone()),
    };
    // A style that keeps the default text color names none: the document is drawn in the theme's foreground.
    let base_style = document_style(&props.style, &ui);
    let mut root = render_document(&document, &base_style, &theme, &render_ctx);
    let target = self.pending_jump.lock().unwrap().take();
    if let Some(anchor) = target.and_then(|target| links.anchor_of(&target)) {
      ctx.scroll_into_view(&anchor);
    }
    let tooltips = links.take_tooltips();
    if !tooltips.is_empty() {
      root = root.child(ctx.mount::<LinkTooltips>(LinkTooltipsProps {
        tooltips: tooltips.into(),
        shown: self.hover.shown(),
        style: base_style,
        text: theme.link_tooltip.clone(),
        panel: theme.link_tooltip_box.clone(),
      }));
    }
    let mut element: Element = root.into();

    if let Some(width) = props.width {
      element = element.map_node(|node| node.width(width));
    }
    if props.selectable {
      element.node.selectable_recursive(true);
    }
    element
  }
}

/// The document's own containers (the document, flows, lists, quotes,
/// footnotes) let what their blocks draw past them show, as CSS blocks do: a
/// link's focus ring reaches outside its words. Code blocks and tables keep
/// clipping to their boxes.
fn render_document(
  document: &MarkdownDocument,
  base_style: &TextStyle,
  theme: &ThemeMarkdown,
  render_ctx: &MarkdownRenderContext<'_>,
) -> Column {
  let mut root = render_ctx
    .width
    .blocks(Column::new().overflow_visible().spacing(theme.document_spacing));
  let mut footnotes = Vec::new();
  for block in &document.blocks {
    // A footnote the text refers to goes to the end, by its number.
    if let MarkdownBlock::FootnoteDefinition { label, blocks } = block
      && let Some(number) = render_ctx.links.footnote_number(label)
    {
      footnotes.push((number, label.as_str(), blocks.as_slice()));
      continue;
    }
    root = root.child(render_block(block, base_style, theme, render_ctx));
  }
  if !footnotes.is_empty() {
    footnotes.sort_by_key(|(number, ..)| *number);
    root = root.child(blocks::render_footnotes(&footnotes, base_style, theme, render_ctx));
  }
  root
}

/// The number of each footnote the text refers to, by label: in the order
/// of the first reference to each, as GitHub numbers them. A footnote with
/// no definition has none.
fn footnote_numbers(document: &MarkdownDocument) -> HashMap<String, usize> {
  fn definitions<'d>(blocks: &'d [MarkdownBlock], found: &mut HashSet<&'d str>) {
    for block in blocks {
      if let MarkdownBlock::FootnoteDefinition { label, .. } = block {
        found.insert(label);
      }
    }
  }
  fn references(inlines: &[MarkdownInline], order: &mut Vec<String>) {
    for inline in inlines {
      match inline {
        MarkdownInline::FootnoteReference(label) if !order.contains(label) => order.push(label.clone()),
        MarkdownInline::Emphasis(children)
        | MarkdownInline::Strong(children)
        | MarkdownInline::Strikethrough(children)
        | MarkdownInline::Link { children, .. } => references(children, order),
        _ => {}
      }
    }
  }
  fn walk(blocks: &[MarkdownBlock], order: &mut Vec<String>) {
    for block in blocks {
      match block {
        MarkdownBlock::Paragraph(inlines) | MarkdownBlock::Heading { children: inlines, .. } => {
          references(inlines, order)
        }
        MarkdownBlock::BlockQuote(blocks)
        | MarkdownBlock::Admonition { blocks, .. }
        | MarkdownBlock::Details { blocks, .. }
        | MarkdownBlock::FootnoteDefinition { blocks, .. } => walk(blocks, order),
        MarkdownBlock::List { items, .. } => {
          for item in items {
            walk(&item.blocks, order);
          }
        }
        MarkdownBlock::Table { rows, .. } => {
          for row in rows {
            for cell in &row.cells {
              references(cell, order);
            }
          }
        }
        _ => {}
      }
    }
  }
  let mut defined = HashSet::new();
  definitions(&document.blocks, &mut defined);
  let mut order = Vec::new();
  walk(&document.blocks, &mut order);
  order
    .into_iter()
    .filter(|label| defined.contains(label.as_str()))
    .enumerate()
    .map(|(index, label)| (label, index + 1))
    .collect()
}

/// The fills behind code and keys in a sentence: `ui::InlineCode`'s box and
/// `ui::Kbd`'s, as the theme draws them. A fill's vertical padding is the
/// part's; the paragraph sets it to the height of the part's box.
fn mark_fills(ui: &Theme) -> (Option<TextHighlight>, Option<TextHighlight>) {
  let code = ui.resolve::<InlineCode>(&Case::new(), None).part(InlineCodePart::Root);
  let key = ui.resolve::<Kbd>(&Case::new(), None).part(KbdPart::Key);
  let code_fill = crate::ui::fill_of(&code, 0..0);
  // A key's box is as high as the part says: its padding is that height.
  let key_height = match key.rest().height.and_then(|height| height.resolve(ui)) {
    Some(Dimension::Px(px)) => px,
    _ => 20.0,
  };
  let kbd_fill = crate::ui::fill_of(&key, 0..0).map(|fill| {
    let across = fill.padding.0;
    fill.padding(across, key_height)
  });
  let _ = Length::Fill;
  (code_fill, kbd_fill)
}

fn render_block(
  block: &MarkdownBlock,
  base_style: &TextStyle,
  theme: &ThemeMarkdown,
  render_ctx: &MarkdownRenderContext<'_>,
) -> Element {
  match block {
    MarkdownBlock::Paragraph(inlines) => render_inline_block(inlines, base_style.clone(), theme, render_ctx),
    MarkdownBlock::Heading { level, children } => {
      let heading = render_inline_block(
        children,
        heading_style(theme, *level, base_style, &render_ctx.ui),
        theme,
        render_ctx,
      );
      // A heading of the second level has a line under it.
      match (level, theme.heading_2_rule.clone()) {
        (MarkdownHeadingLevel::H2, Some(line)) => render_ctx
          .width
          .blocks(Column::new().overflow_visible().padding_top(8.0).padding_bottom(6.0))
          .border_bottom(crate::node::border::Border::inside(1.0, line))
          .child(heading)
          .into(),
        _ => heading,
      }
    }
    MarkdownBlock::BlockQuote(blocks) => render_blockquote(blocks, base_style, theme, render_ctx),
    MarkdownBlock::Admonition { kind, blocks } => {
      blocks::render_admonition(*kind, blocks, base_style, theme, render_ctx)
    }
    MarkdownBlock::Details { summary, open, blocks } => {
      blocks::render_details(summary, *open, blocks, base_style, theme, render_ctx)
    }
    MarkdownBlock::List { ordered, start, items } => {
      render_list(*ordered, *start, items, base_style, theme, render_ctx)
    }
    MarkdownBlock::Table { alignments, rows } => render_table(alignments, rows, base_style, theme, render_ctx),
    MarkdownBlock::CodeBlock { kind, text } => render_code_block(kind, text, base_style, theme, render_ctx),
    MarkdownBlock::Math { text } => render_math_block(text, base_style, theme, render_ctx),
    MarkdownBlock::FootnoteDefinition { label, blocks } => {
      render_footnote_definition(label, blocks, base_style, theme, render_ctx)
    }
    MarkdownBlock::Html(text) => render_ctx.width.leaf(render_ctx.text.leaf(Text::styled(
      &markdown_html_text(text.trim_end_matches('\n')),
      base_style.clone(),
    ))),
    MarkdownBlock::ThematicBreak => render_ctx.width.leaf(
      Rect::new(Dimension::Auto, 1.0).background(
        theme
          .rule
          .background
          .clone()
          .unwrap_or_else(|| theme.table_marker.apply(base_style).color.into()),
      ),
    ),
  }
}

/// A paragraph, a heading or a table cell: one rich-text node, links in it.
/// Images other than a lone one break it into a wrapping row of texts and
/// images.
fn render_inline_block(
  inlines: &[MarkdownInline],
  style: TextStyle,
  theme: &ThemeMarkdown,
  render_ctx: &MarkdownRenderContext<'_>,
) -> Element {
  if let Some(image @ MarkdownInline::Image { title, alt, .. }) = single_image(inlines) {
    let picture = render_picture(image, &style, theme, render_ctx);
    return blocks::render_figure(picture, Some(alt), title, &style, theme, render_ctx);
  }

  if let [MarkdownInline::Text(text)] = inlines {
    return render_ctx.width.leaf(render_ctx.text.leaf(Text::styled(text, style)));
  }

  if has_image(inlines) {
    return render_inline_flow(inlines, style, theme, render_ctx);
  }

  let mut paragraph = Paragraph::new(inlines, false, theme, render_ctx.links);
  paragraph.push_inlines(inlines, &style, &mut None);
  render_ctx
    .width
    .leaf(render_ctx.text.leaf(Element::from_node(paragraph.take_node())))
}

fn render_inline_flow(
  inlines: &[MarkdownInline],
  style: TextStyle,
  theme: &ThemeMarkdown,
  render_ctx: &MarkdownRenderContext<'_>,
) -> Element {
  let mut hard_break_segments = inlines.split(|inline| matches!(inline, MarkdownInline::HardBreak));
  let Some(first_segment) = hard_break_segments.next() else {
    return render_inline_flow_row(inlines, &style, theme, render_ctx);
  };

  let remaining_segments = hard_break_segments.collect::<Vec<_>>();
  if remaining_segments.is_empty() {
    return render_inline_flow_row(first_segment, &style, theme, render_ctx);
  }

  let mut column = render_ctx
    .width
    .blocks(Column::new().overflow_visible().spacing(0.0).min_width(0.0));
  column = column.child(render_inline_flow_row(first_segment, &style, theme, render_ctx));
  for segment in remaining_segments {
    column = column.child(render_inline_flow_row(segment, &style, theme, render_ctx));
  }
  column.into()
}

/// A wrapping row of the texts between images, and the images.
fn render_inline_flow_row(
  inlines: &[MarkdownInline],
  style: &TextStyle,
  theme: &ThemeMarkdown,
  render_ctx: &MarkdownRenderContext<'_>,
) -> Element {
  let mut flow = Vec::new();
  let mut paragraph = Paragraph::new(inlines, true, theme, render_ctx.links);
  paragraph.push_inlines(inlines, style, &mut Some(&mut flow));
  if !paragraph.is_empty() {
    flow.push(FlowItem::Text(Box::new(paragraph.take_node())));
  }
  let mut row = render_ctx.width.row(
    Row::new()
      .overflow_visible()
      .wrap()
      .spacing(0.0)
      .align_items(Alignment::Center)
      .min_width(0.0),
  );
  for item in flow {
    row = row.child(match item {
      FlowItem::Text(mut node) => {
        render_ctx.text.append(&mut node);
        Element::from_node((*node).max_width(FILL_WIDTH).min_width(0.0))
      }
      FlowItem::Image(image, style) => render_markdown_image(image, &style, theme),
    });
  }
  row.into()
}

/// Whether an image outside any link is among `inlines`: an image in a link
/// is part of the link's words (its alt text).
fn has_image(inlines: &[MarkdownInline]) -> bool {
  inlines.iter().any(|inline| match inline {
    MarkdownInline::Image { .. } => true,
    MarkdownInline::Emphasis(children) | MarkdownInline::Strong(children) | MarkdownInline::Strikethrough(children) => {
      has_image(children)
    }
    _ => false,
  })
}

fn single_image(inlines: &[MarkdownInline]) -> Option<&MarkdownInline> {
  let mut image = None;
  for inline in inlines {
    match inline {
      MarkdownInline::Image { .. } if image.is_none() => image = Some(inline),
      MarkdownInline::Text(text) if text.trim().is_empty() => {}
      MarkdownInline::SoftBreak | MarkdownInline::HardBreak => {}
      _ => return None,
    }
  }
  image
}

/// A lone image: the image where the app draws images, or else the box of its place.
fn render_picture(
  inline: &MarkdownInline,
  base_style: &TextStyle,
  theme: &ThemeMarkdown,
  render_ctx: &MarkdownRenderContext<'_>,
) -> Element {
  #[cfg(all(feature = "image", feature = "resources"))]
  {
    let _ = (base_style, theme, render_ctx);
    render_markdown_image(inline, base_style, theme)
  }
  #[cfg(not(all(feature = "image", feature = "resources")))]
  {
    let _ = inline;
    blocks::image_place(base_style, theme, render_ctx)
  }
}

fn render_markdown_image(inline: &MarkdownInline, base_style: &TextStyle, theme: &ThemeMarkdown) -> Element {
  let MarkdownInline::Image {
    destination,
    title: _,
    alt,
  } = inline
  else {
    return Element::from_node(Node::rich_text(markdown_inline_rich_text(
      std::slice::from_ref(inline),
      base_style,
      theme,
    )));
  };
  let _ = destination;
  let _ = alt;

  #[cfg(all(feature = "image", feature = "resources"))]
  {
    crate::components::Image::from_resource(destination)
      .max_width(FILL_WIDTH)
      .into()
  }

  #[cfg(not(all(feature = "image", feature = "resources")))]
  {
    let style = theme.image_caption.apply(base_style);
    Element::from_node(Node::rich_text(markdown_inline_rich_text(alt, &style, theme)))
  }
}

fn render_blockquote(
  blocks: &[MarkdownBlock],
  base_style: &TextStyle,
  theme: &ThemeMarkdown,
  render_ctx: &MarkdownRenderContext<'_>,
) -> Element {
  let mut body = render_ctx.width.row_body(
    Column::new()
      .overflow_visible()
      .spacing(theme.nested_block_spacing)
      .min_width(0.0),
  );
  for block in blocks {
    body = body.child(render_block(block, base_style, theme, render_ctx));
  }

  let bar_color = theme
    .blockquote_box
    .border_color
    .clone()
    .unwrap_or_else(|| theme.blockquote_marker.apply(base_style).color.into());
  let bar_width = theme.blockquote_box.border_width.unwrap_or(3.0);

  let mut row = render_ctx.width.row(
    Row::new()
      .overflow_visible()
      .spacing(theme.blockquote_gap)
      .align_items(Alignment::Stretch)
      .child(Rect::new(bar_width, FILL_WIDTH).background(bar_color))
      .child(body),
  );
  row = apply_row_box(row, &theme.blockquote_box, false);
  row.into()
}

fn render_list(
  ordered: bool,
  start: Option<u64>,
  items: &[MarkdownListItem],
  base_style: &TextStyle,
  theme: &ThemeMarkdown,
  render_ctx: &MarkdownRenderContext<'_>,
) -> Element {
  if let Some(list_view) = render_ctx.list_view {
    let list = MarkdownList {
      ordered,
      start,
      items,
      style: base_style,
      theme,
      render_ctx,
    };
    // A view that declines draws nothing that stays: forget the links it
    // drew and the code blocks it counted to decide, so the list's own are
    // numbered as if it had not.
    let checkpoint = render_ctx.links.checkpoint();
    let code_blocks = render_ctx.code_blocks.get();
    if let Some(element) = list_view(&list) {
      return element;
    }
    render_ctx.links.restore(checkpoint);
    render_ctx.code_blocks.set(code_blocks);
  }
  blocks::render_items(ordered, start.unwrap_or(1), items, base_style, theme, render_ctx)
}

fn render_code_block(
  kind: &MarkdownCodeBlockKind,
  text: &str,
  base_style: &TextStyle,
  theme: &ThemeMarkdown,
  render_ctx: &MarkdownRenderContext<'_>,
) -> Element {
  let width = render_ctx.width;
  let code_style = theme.code_block.apply(base_style);

  // The box's padding goes inside the lines' scroll container, so a line
  // wider than the box scrolls under the padding and is clipped at the
  // box's edge, like a CSS `pre` with `overflow-x: auto`, and the scrollbar
  // sits in the bottom padding. The label stays above, outside the scroll.
  let padding = theme.code_block_box.padding.unwrap_or_default();
  let mut lines_padding = padding;
  let ordinal = render_ctx.code_blocks.get();
  render_ctx.code_blocks.set(ordinal + 1);
  let key = code_block_key(ordinal, kind);
  let mut column = width.blocks(Column::new().spacing(0.0).key(key.clone()));
  if let MarkdownCodeBlockKind::Fenced {
    language: Some(language),
  } = kind
  {
    let label_style = theme.code_block_label.apply(base_style);
    let label = width.blocks(
      Column::new()
        .overflow_visible()
        .padding(padding.bottom(0.0))
        .child(width.leaf(Text::styled(language, label_style))),
    );
    column = column.child(label);
    lines_padding = lines_padding.top(theme.code_block_spacing);
  }
  let lines = Column::new().overflow_visible().padding(lines_padding).child(
    render_ctx.text.leaf(Element::from_node(
      Node::rich_text(highlight_code_spans(
        text.trim_end_matches('\n'),
        kind,
        &code_style,
        &render_ctx.ui.palette(),
      ))
      .text_wrap(false),
    )),
  );
  let scroll = ScrollHorizontal::new(lines)
    .key(key)
    .scrollbar(render_ctx.code_block_scrollbar.clone());
  column = column.child(width.leaf(scroll));
  let box_style = MarkdownBlockStyle {
    padding: None,
    ..theme.code_block_box.clone()
  };
  column = apply_column_box(column, &box_style, true);
  column.into()
}

/// Keys a code block's box and its scroll node by the block's ordinal among
/// the document's code blocks and its language: the key holds while the
/// block's code streams in, token by token, and while paragraphs are added
/// or removed around it, so a scrolled block keeps its scroll and a
/// selection in it, and its node ids and MCP refs stay. A code block
/// inserted above it or removed from above it shifts the ordinals, and the
/// blocks after it take the scroll of the block that had their ordinal.
fn code_block_key(ordinal: usize, kind: &MarkdownCodeBlockKind) -> Arc<str> {
  match kind {
    MarkdownCodeBlockKind::Fenced {
      language: Some(language),
    } => format!("markdown-code:{ordinal}:{language}").into(),
    _ => format!("markdown-code:{ordinal}").into(),
  }
}

fn render_math_block(
  text: &str,
  base_style: &TextStyle,
  theme: &ThemeMarkdown,
  render_ctx: &MarkdownRenderContext<'_>,
) -> Element {
  let kind = MarkdownCodeBlockKind::Fenced {
    language: Some("math".to_owned()),
  };
  render_code_block(&kind, text, base_style, theme, render_ctx)
}

fn render_footnote_definition(
  label: &str,
  blocks: &[MarkdownBlock],
  base_style: &TextStyle,
  theme: &ThemeMarkdown,
  render_ctx: &MarkdownRenderContext<'_>,
) -> Element {
  let marker_style = theme.link.apply(base_style);
  let mut body = render_ctx
    .width
    .row_body(Column::new().overflow_visible().spacing(theme.nested_block_spacing));
  for block in blocks {
    body = body.child(render_block(block, base_style, theme, render_ctx));
  }

  render_ctx
    .width
    .row(
      Row::new()
        .overflow_visible()
        .spacing(theme.list_marker_gap)
        .align_items(Alignment::Start)
        .child(Text::styled(&format!("[^{label}]"), marker_style).width(theme.ordered_list_marker_width))
        .child(body),
    )
    .into()
}

fn highlight_code_spans(
  text: &str,
  kind: &MarkdownCodeBlockKind,
  base_style: &TextStyle,
  palette: &ThemePalette,
) -> Vec<RichTextSpan> {
  if !is_rust_like_code(kind) {
    return vec![RichTextSpan {
      text: text.to_owned(),
      style: base_style.clone(),
    }];
  }

  let keyword_style = code_style(base_style, palette.get(PaletteColor::Info));
  let string_style = code_style(base_style, palette.get(PaletteColor::Success));
  let comment_style = code_style(base_style, palette.get(PaletteColor::MutedForeground));
  let number_style = code_style(base_style, palette.get(PaletteColor::Destructive));
  let mut spans = Vec::new();
  let mut rest = text;
  while !rest.is_empty() {
    if let Some(comment) = rest.strip_prefix("//") {
      let line_len = comment.find('\n').map(|index| index + 2).unwrap_or(rest.len());
      push_code_span(&mut spans, &rest[..line_len], comment_style.clone());
      rest = &rest[line_len..];
      continue;
    }
    let Some(ch) = rest.chars().next() else {
      break;
    };
    if ch == '"' {
      let len = quoted_len(rest);
      push_code_span(&mut spans, &rest[..len], string_style.clone());
      rest = &rest[len..];
      continue;
    }
    if ch.is_ascii_digit() {
      let len = rest
        .char_indices()
        .find(|(_, ch)| !ch.is_ascii_digit() && *ch != '.')
        .map(|(index, _)| index)
        .unwrap_or(rest.len());
      push_code_span(&mut spans, &rest[..len], number_style.clone());
      rest = &rest[len..];
      continue;
    }
    if ch == '_' || ch.is_ascii_alphabetic() {
      let len = rest
        .char_indices()
        .find(|(_, ch)| *ch != '_' && !ch.is_ascii_alphanumeric())
        .map(|(index, _)| index)
        .unwrap_or(rest.len());
      let token = &rest[..len];
      let style = if is_rust_keyword(token) {
        keyword_style.clone()
      } else {
        base_style.clone()
      };
      push_code_span(&mut spans, token, style);
      rest = &rest[len..];
      continue;
    }
    push_code_span(&mut spans, &rest[..ch.len_utf8()], base_style.clone());
    rest = &rest[ch.len_utf8()..];
  }
  spans
}

fn is_rust_like_code(kind: &MarkdownCodeBlockKind) -> bool {
  matches!(
    kind,
    MarkdownCodeBlockKind::Fenced {
      language: Some(language)
    } if matches!(language.as_str(), "rust" | "rs")
  )
}

fn quoted_len(text: &str) -> usize {
  let mut escaped = false;
  for (index, ch) in text.char_indices().skip(1) {
    if escaped {
      escaped = false;
      continue;
    }
    if ch == '\\' {
      escaped = true;
      continue;
    }
    if ch == '"' {
      return index + ch.len_utf8();
    }
  }
  text.len()
}

fn is_rust_keyword(token: &str) -> bool {
  matches!(
    token,
    "as"
      | "async"
      | "await"
      | "break"
      | "const"
      | "continue"
      | "crate"
      | "else"
      | "enum"
      | "false"
      | "fn"
      | "for"
      | "if"
      | "impl"
      | "in"
      | "let"
      | "loop"
      | "match"
      | "mod"
      | "move"
      | "mut"
      | "pub"
      | "ref"
      | "return"
      | "self"
      | "Self"
      | "static"
      | "struct"
      | "super"
      | "trait"
      | "true"
      | "type"
      | "unsafe"
      | "use"
      | "where"
      | "while"
  )
}

fn code_style(base_style: &TextStyle, color: Color) -> TextStyle {
  let mut style = base_style.clone();
  style.color = color;
  style
}

fn push_code_span(spans: &mut Vec<RichTextSpan>, text: &str, style: TextStyle) {
  if text.is_empty() {
    return;
  }
  if let Some(last) = spans.last_mut()
    && last.style == style
  {
    last.text.push_str(text);
    return;
  }
  spans.push(RichTextSpan {
    text: text.to_owned(),
    style,
  });
}

fn render_table(
  alignments: &[MarkdownTableAlignment],
  rows: &[MarkdownTableRow],
  base_style: &TextStyle,
  theme: &ThemeMarkdown,
  render_ctx: &MarkdownRenderContext<'_>,
) -> Element {
  // The design's table: its rows one under another with a line between two,
  // the header on its fill and every other body row striped, the columns of
  // one width with a line between two.
  let mut table = Column::new().width(FILL_WIDTH);
  let line_color = table_line_color(theme);
  let mut body_rows = 0;
  for (row_index, row) in rows.iter().enumerate() {
    let mut row_node = Row::new().align_items(Alignment::Stretch).width(FILL_WIDTH);
    // The header's fill is its cells'; a body row's stripe is the row's.
    let fill = if row.header {
      None
    } else {
      body_rows += 1;
      theme.table_stripe.clone().filter(|_| body_rows % 2 == 0)
    };
    if let Some(fill) = fill {
      row_node = row_node.background(fill);
    }
    if row_index + 1 < rows.len()
      && let Some(line) = line_color.clone()
    {
      row_node = row_node.border_bottom(crate::node::border::Border::inside(1.0, line));
    }
    let count = row.cells.len();
    for (cell_index, cell) in row.cells.iter().enumerate() {
      let alignment = alignments.get(cell_index).copied().unwrap_or_default();
      let mut element = render_table_cell(row.header, cell, alignment, base_style, theme, render_ctx);
      if cell_index + 1 < count
        && let Some(line) = line_color.clone()
      {
        crate::node::NodeUpdate::border_right(&mut *element.node, crate::node::border::Border::inside(1.0, line));
      }
      row_node = row_node.child(element);
    }
    table = table.child(row_node);
  }
  table = apply_column_box(table, &theme.table_box, true);
  table.into()
}

fn table_line_color(theme: &ThemeMarkdown) -> Option<crate::node::TextColor> {
  theme
    .table_box
    .border_color
    .clone()
    .or_else(|| theme.table_box.background.clone())
}

fn render_table_cell(
  header: bool,
  cell: &[MarkdownInline],
  alignment: MarkdownTableAlignment,
  base_style: &TextStyle,
  theme: &ThemeMarkdown,
  render_ctx: &MarkdownRenderContext<'_>,
) -> Element {
  let mut style = if header {
    theme.table_header.apply(base_style)
  } else {
    theme.table_cell.apply(base_style)
  };
  style.text_align = table_text_align(alignment);

  let box_style = if header {
    theme.table_header_box.clone()
  } else {
    theme.table_cell_box.clone()
  };
  // The words are the cell's content, not the cell (see `apply_column_box`),
  // centered in its height as a stretched text leaf centers its words.
  let content = Column::new()
    .overflow_visible()
    .align_items(table_cell_alignment(alignment))
    .justify(Justify::Center)
    .child(render_inline_block(cell, style, theme, render_ctx));
  // The columns share the table's width alike, as the design's cells fill it.
  apply_column_box(content, &box_style, true)
    .min_width(theme.table_cell_min_width.min(40.0))
    .flex_full(1.0, 1.0, Some(0.0))
    .into()
}

fn table_text_align(alignment: MarkdownTableAlignment) -> TextAlign {
  match alignment {
    MarkdownTableAlignment::None | MarkdownTableAlignment::Left => TextAlign::Left,
    MarkdownTableAlignment::Center => TextAlign::Center,
    MarkdownTableAlignment::Right => TextAlign::Right,
  }
}

/// Where a cell puts content narrower than the cell (an image, a fitting
/// text).
fn table_cell_alignment(alignment: MarkdownTableAlignment) -> Alignment {
  match alignment {
    MarkdownTableAlignment::None | MarkdownTableAlignment::Left => Alignment::Start,
    MarkdownTableAlignment::Center => Alignment::Center,
    MarkdownTableAlignment::Right => Alignment::End,
  }
}

/// Gives `column` the box `style`. A Markdown box is always a container
/// around its content, never the content itself: a `Text` or `RichText` leaf
/// with padding grows by it, but lurq paints the leaf's words over its whole
/// rect, padding included, so the words would start at the box's edge.
fn apply_column_box(mut column: Column, style: &MarkdownBlockStyle, include_border: bool) -> Column {
  if let Some(padding) = style.padding {
    column = column.padding(padding);
  }
  if let Some(background) = style.background.clone() {
    column = column.background(background);
  }
  if include_border && let (Some(width), Some(color)) = (style.border_width, style.border_color.clone()) {
    column = column.border_inside(width, color);
  }
  if let Some(radius) = style.radius {
    column = column.rounded(radius);
  }
  column
}

/// Gives `row` the box `style`; like [`apply_column_box`], never a text leaf.
fn apply_row_box(mut row: Row, style: &MarkdownBlockStyle, include_border: bool) -> Row {
  if let Some(padding) = style.padding {
    row = row.padding(padding);
  }
  if let Some(background) = style.background.clone() {
    row = row.background(background);
  }
  if include_border && let (Some(width), Some(color)) = (style.border_width, style.border_color.clone()) {
    row = row.border_inside(width, color);
  }
  if let Some(radius) = style.radius {
    row = row.rounded(radius);
  }
  row
}

/// A heading's style: the theme's type of its level (`TypographyStyle::Heading1`
/// to `Heading6`, the sixth in capitals) in the document's color, with what
/// the Markdown theme says of headings and of its level over it.
fn heading_style(theme: &ThemeMarkdown, level: MarkdownHeadingLevel, base_style: &TextStyle, ui: &Theme) -> TextStyle {
  use crate::app::theme::TypographyStyle;
  let role = match level {
    MarkdownHeadingLevel::H1 => TypographyStyle::Heading1,
    MarkdownHeadingLevel::H2 => TypographyStyle::Heading2,
    MarkdownHeadingLevel::H3 => TypographyStyle::Heading3,
    MarkdownHeadingLevel::H4 => TypographyStyle::Heading4,
    MarkdownHeadingLevel::H5 => TypographyStyle::Heading5,
    MarkdownHeadingLevel::H6 => TypographyStyle::Heading6,
  };
  let mut scale = ui.typography_style(role);
  scale.color = base_style.color;
  let shared = theme.heading.apply(&scale);
  match level {
    MarkdownHeadingLevel::H1 => theme.heading_1.apply(&shared),
    MarkdownHeadingLevel::H2 => theme.heading_2.apply(&shared),
    MarkdownHeadingLevel::H3 => theme.heading_3.apply(&shared),
    MarkdownHeadingLevel::H4 => theme.heading_4.apply(&shared),
    MarkdownHeadingLevel::H5 => theme.heading_5.apply(&shared),
    MarkdownHeadingLevel::H6 => theme.heading_6.apply(&shared),
  }
}

/// The document's text: the style the app gives, or the theme's `Article`
/// (15 on lines of 1.65) where it gives none, in the theme's foreground where
/// it names no color.
fn document_style(style: &TextStyle, ui: &Theme) -> TextStyle {
  let mut style = if *style == TextStyle::default() {
    ui.typography_style(crate::app::theme::TypographyStyle::Article)
  } else {
    style.clone()
  };
  style.color = ui.palette().text_color(&style);
  style
}
