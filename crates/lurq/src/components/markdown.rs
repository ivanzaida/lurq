use std::sync::Arc;

use crate::{
  app::{
    component::Component,
    ctx::Ctx,
    theme::{MarkdownBlockStyle, ThemeMarkdown},
  },
  components::{Column, Rect, Row, Text},
  core::{Memo, Signal},
  layout::{
    Alignment,
    quad::RichTextSpan,
    text_style::{FontWeight, TextAlign, TextStyle},
  },
  markdown::{
    MarkdownBlock, MarkdownCodeBlockKind, MarkdownDocument, MarkdownHeadingLevel, MarkdownInline, MarkdownListItem,
    MarkdownTableAlignment, MarkdownTableRow, markdown_html_text, markdown_inline_rich_text, parse_markdown,
  },
  node::{CursorIcon, Element, Node, color::Color, dimension::Dimension},
};

const FILL_WIDTH: Dimension = Dimension::Pct(100.0);

type MarkdownLinkCallback = Arc<dyn Fn(&MarkdownLink) + Send + Sync>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MarkdownLink {
  destination: Arc<str>,
  title: Arc<str>,
  text: Arc<str>,
}

impl MarkdownLink {
  pub fn destination(&self) -> &str {
    &self.destination
  }

  pub fn title(&self) -> &str {
    &self.title
  }

  pub fn text(&self) -> &str {
    &self.text
  }
}

#[derive(Clone)]
pub struct MarkdownProps {
  source: Arc<str>,
  style: TextStyle,
  theme: Option<ThemeMarkdown>,
  selectable: bool,
  width: Option<Dimension>,
  on_link_click: Option<MarkdownLinkCallback>,
}

impl PartialEq for MarkdownProps {
  fn eq(&self, other: &Self) -> bool {
    self.source == other.source
      && self.style == other.style
      && self.theme == other.theme
      && self.selectable == other.selectable
      && self.width == other.width
      && same_link_callback(&self.on_link_click, &other.on_link_click)
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
      on_link_click: None,
    }
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

  pub fn selectable(mut self, selectable: bool) -> Self {
    self.selectable = selectable;
    self
  }

  pub fn width(mut self, width: impl Into<Dimension>) -> Self {
    self.width = Some(width.into());
    self
  }

  pub fn on_link_click(mut self, f: impl Fn(&MarkdownLink) + Send + Sync + 'static) -> Self {
    self.on_link_click = Some(Arc::new(f));
    self
  }
}

fn same_link_callback(left: &Option<MarkdownLinkCallback>, right: &Option<MarkdownLinkCallback>) -> bool {
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

pub struct Markdown {
  source: Signal<Arc<str>>,
  document: Memo<MarkdownDocument>,
}

#[derive(Clone)]
struct MarkdownRenderContext {
  on_link_click: Option<MarkdownLinkCallback>,
  #[cfg(feature = "router")]
  navigator: Option<crate::router::Navigator>,
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
    Self { source, document }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let props = ctx.props::<Self::Props>().clone();
    if self.source.get_untracked() != props.source {
      self.source.set(props.source.clone());
    }
    let theme = props.theme.clone().unwrap_or_else(|| ctx.theme().markdown().clone());
    let document = self.document.get();
    let render_ctx = MarkdownRenderContext {
      on_link_click: props.on_link_click.clone(),
      #[cfg(feature = "router")]
      navigator: ctx.use_context::<crate::router::Navigator>(),
    };
    let mut element = render_document(&document, &props.style, &theme, &render_ctx);

    if let Some(width) = props.width {
      element.node = element.node.width(width);
    }
    if props.selectable {
      element.node.selectable_recursive(true);
    }
    element
  }
}

fn render_document(
  document: &MarkdownDocument,
  base_style: &TextStyle,
  theme: &ThemeMarkdown,
  render_ctx: &MarkdownRenderContext,
) -> Element {
  let mut root = Column::new().spacing(theme.document_spacing).width(FILL_WIDTH);
  for block in &document.blocks {
    root = root.child(render_block(block, base_style, theme, render_ctx));
  }
  root.into()
}

fn render_block(
  block: &MarkdownBlock,
  base_style: &TextStyle,
  theme: &ThemeMarkdown,
  render_ctx: &MarkdownRenderContext,
) -> Element {
  match block {
    MarkdownBlock::Paragraph(inlines) => render_inline_block(inlines, base_style.clone(), theme, render_ctx),
    MarkdownBlock::Heading { level, children } => {
      render_inline_block(children, heading_style(theme, *level, base_style), theme, render_ctx)
    }
    MarkdownBlock::BlockQuote(blocks) => render_blockquote(blocks, base_style, theme, render_ctx),
    MarkdownBlock::List { ordered, start, items } => {
      render_list(*ordered, *start, items, base_style, theme, render_ctx)
    }
    MarkdownBlock::Table { alignments, rows } => render_table(alignments, rows, base_style, theme, render_ctx),
    MarkdownBlock::CodeBlock { kind, text } => render_code_block(kind, text, base_style, theme),
    MarkdownBlock::Math { text } => render_math_block(text, base_style, theme),
    MarkdownBlock::FootnoteDefinition { label, blocks } => {
      render_footnote_definition(label, blocks, base_style, theme, render_ctx)
    }
    MarkdownBlock::Html(text) => Text::styled(&markdown_html_text(text.trim_end_matches('\n')), base_style.clone())
      .width(FILL_WIDTH)
      .into(),
    MarkdownBlock::ThematicBreak => Rect::new(FILL_WIDTH, 1.0)
      .background(theme.table_marker.apply(base_style).color)
      .into(),
  }
}

fn render_inline_block(
  inlines: &[MarkdownInline],
  style: TextStyle,
  theme: &ThemeMarkdown,
  render_ctx: &MarkdownRenderContext,
) -> Element {
  if let Some(image) = single_image(inlines) {
    return render_markdown_image(image, &style, theme);
  }

  if let [MarkdownInline::Text(text)] = inlines {
    return Text::styled(text, style).width(FILL_WIDTH).into();
  }

  if inline_requires_flow(inlines) {
    return render_inline_flow(inlines, style, theme, render_ctx);
  }

  Element::from_node(Node::rich_text(markdown_inline_rich_text(inlines, &style, theme)).width(FILL_WIDTH))
}

fn render_inline_flow(
  inlines: &[MarkdownInline],
  style: TextStyle,
  theme: &ThemeMarkdown,
  render_ctx: &MarkdownRenderContext,
) -> Element {
  let mut hard_break_segments = inlines.split(|inline| matches!(inline, MarkdownInline::HardBreak));
  let Some(first_segment) = hard_break_segments.next() else {
    return render_inline_flow_row(inlines, style, theme, render_ctx);
  };

  let remaining_segments = hard_break_segments.collect::<Vec<_>>();
  if remaining_segments.is_empty() {
    return render_inline_flow_row(first_segment, style, theme, render_ctx);
  }

  let mut column = Column::new().spacing(0.0).min_width(0.0).width(FILL_WIDTH);
  column = column.child(render_inline_flow_row(first_segment, style.clone(), theme, render_ctx));
  for segment in remaining_segments {
    column = column.child(render_inline_flow_row(segment, style.clone(), theme, render_ctx));
  }
  column.into()
}

fn render_inline_flow_row(
  inlines: &[MarkdownInline],
  style: TextStyle,
  theme: &ThemeMarkdown,
  render_ctx: &MarkdownRenderContext,
) -> Element {
  let mut row = Row::new()
    .wrap()
    .spacing(0.0)
    .align_items(Alignment::Center)
    .min_width(0.0)
    .width(FILL_WIDTH);
  let mut spans = Vec::new();
  push_inline_flow_children(&mut row, &mut spans, inlines, &style, theme, render_ctx);
  flush_inline_spans(&mut row, &mut spans);
  row.into()
}

fn push_inline_flow_children(
  row: &mut Row,
  spans: &mut Vec<RichTextSpan>,
  inlines: &[MarkdownInline],
  style: &TextStyle,
  theme: &ThemeMarkdown,
  render_ctx: &MarkdownRenderContext,
) {
  for inline in inlines {
    match inline {
      MarkdownInline::Link {
        destination,
        title,
        children,
      } => {
        flush_inline_spans(row, spans);
        let link_style = theme.link.apply(style);
        let text = plain_inline_text(children);
        let link = MarkdownLink {
          destination: Arc::from(destination.as_str()),
          title: Arc::from(title.as_str()),
          text: Arc::from(text),
        };
        let node = Node::rich_text(markdown_inline_rich_text(children, &link_style, theme))
          .max_width(FILL_WIDTH)
          .min_width(0.0)
          .cursor(CursorIcon::Pointer)
          .on_click(link_click_handler(link, render_ctx));
        *row = take_row(row).child(Element::from_node(node));
      }
      MarkdownInline::Image { .. } => {
        flush_inline_spans(row, spans);
        *row = take_row(row).child(render_markdown_image(inline, style, theme));
      }
      MarkdownInline::Emphasis(children) => {
        let child_style = theme.emphasis.apply(style);
        push_inline_flow_children(row, spans, children, &child_style, theme, render_ctx);
      }
      MarkdownInline::Strong(children) => {
        let child_style = theme.strong.apply(style);
        push_inline_flow_children(row, spans, children, &child_style, theme, render_ctx);
      }
      MarkdownInline::Strikethrough(children) => {
        let child_style = theme.strikethrough.apply(style);
        push_inline_flow_children(row, spans, children, &child_style, theme, render_ctx);
      }
      MarkdownInline::HardBreak => {
        flush_inline_spans(row, spans);
        *row = take_row(row).child(Text::new("").width(FILL_WIDTH));
      }
      _ => spans.extend(markdown_inline_rich_text(std::slice::from_ref(inline), style, theme)),
    }
  }
}

fn flush_inline_spans(row: &mut Row, spans: &mut Vec<RichTextSpan>) {
  if spans.is_empty() {
    return;
  }
  let node = Node::rich_text(std::mem::take(spans))
    .max_width(FILL_WIDTH)
    .min_width(0.0);
  *row = take_row(row).child(Element::from_node(node));
}

fn take_row(row: &mut Row) -> Row {
  std::mem::take(row)
}

fn link_click_handler(
  link: MarkdownLink,
  render_ctx: &MarkdownRenderContext,
) -> impl Fn(crate::app::events::MouseEvent) + Send + Sync + 'static {
  let on_link_click = render_ctx.on_link_click.clone();
  #[cfg(feature = "router")]
  let navigator = render_ctx.navigator.clone();
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

#[cfg(feature = "router")]
fn is_router_link(destination: &str) -> bool {
  destination.starts_with('/') || destination.starts_with('#') || destination.starts_with('?')
}

fn inline_requires_flow(inlines: &[MarkdownInline]) -> bool {
  inlines.iter().any(|inline| match inline {
    MarkdownInline::Link { .. } | MarkdownInline::Image { .. } => true,
    MarkdownInline::Emphasis(children) | MarkdownInline::Strong(children) | MarkdownInline::Strikethrough(children) => {
      inline_requires_flow(children)
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
    return crate::components::Image::from_resource(destination)
      .max_width(FILL_WIDTH)
      .into();
  }

  #[cfg(not(all(feature = "image", feature = "resources")))]
  {
    let mut style = theme.inline_code.apply(base_style);
    if theme.inline_code.text.color.is_none() {
      style.color = Color::from_hex("#64748b");
    }
    Element::from_node(Node::rich_text(markdown_inline_rich_text(alt, &style, theme)))
  }
}

fn plain_inline_text(inlines: &[MarkdownInline]) -> String {
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
      | MarkdownInline::Strikethrough(children) => {
        text.push_str(&plain_inline_text(children));
      }
      MarkdownInline::Link { children, .. } => text.push_str(&plain_inline_text(children)),
      MarkdownInline::Image { alt, .. } => text.push_str(&plain_inline_text(alt)),
      MarkdownInline::SoftBreak | MarkdownInline::HardBreak => text.push(' '),
      MarkdownInline::TaskListMarker(checked) => text.push_str(if *checked { "[x] " } else { "[ ] " }),
    }
  }
  text
}

fn render_blockquote(
  blocks: &[MarkdownBlock],
  base_style: &TextStyle,
  theme: &ThemeMarkdown,
  render_ctx: &MarkdownRenderContext,
) -> Element {
  let mut body = Column::new()
    .spacing(theme.nested_block_spacing)
    .min_width(0.0)
    .flex(1.0);
  for block in blocks {
    body = body.child(render_block(block, base_style, theme, render_ctx));
  }

  let bar_color = theme
    .blockquote_box
    .border_color
    .unwrap_or_else(|| theme.blockquote_marker.apply(base_style).color);
  let bar_width = theme.blockquote_box.border_width.unwrap_or(3.0);

  let mut row = Row::new()
    .spacing(theme.blockquote_gap)
    .align_items(Alignment::Stretch)
    .child(Rect::new(bar_width, FILL_WIDTH).background(bar_color))
    .child(body)
    .width(FILL_WIDTH);
  row = apply_row_box(row, &theme.blockquote_box, false);
  row.into()
}

fn render_list(
  ordered: bool,
  start: Option<u64>,
  items: &[MarkdownListItem],
  base_style: &TextStyle,
  theme: &ThemeMarkdown,
  render_ctx: &MarkdownRenderContext,
) -> Element {
  let mut list = Column::new().spacing(theme.list_item_spacing).width(FILL_WIDTH);
  let start = start.unwrap_or(1);
  for (index, item) in items.iter().enumerate() {
    let marker = if ordered {
      format!("{}.", start + index as u64)
    } else {
      "-".to_owned()
    };
    let marker_style = theme.list_marker.apply(base_style);
    let marker_width = if ordered {
      theme.ordered_list_marker_width
    } else {
      theme.unordered_list_marker_width
    };
    let mut body = Column::new()
      .spacing(theme.nested_block_spacing)
      .min_width(0.0)
      .flex(1.0);
    for block in &item.blocks {
      body = body.child(render_block(block, base_style, theme, render_ctx));
    }
    list = list.child(
      Row::new()
        .spacing(theme.list_marker_gap)
        .align_items(Alignment::Start)
        .child(Text::styled(&marker, marker_style).width(marker_width))
        .child(body)
        .width(FILL_WIDTH),
    );
  }
  list.into()
}

fn render_code_block(
  kind: &MarkdownCodeBlockKind,
  text: &str,
  base_style: &TextStyle,
  theme: &ThemeMarkdown,
) -> Element {
  let mut code_style = theme.code_block.apply(base_style);
  if theme.code_block.text.color.is_none() {
    code_style.color = Color::from_hex("#e2e8f0");
  }

  let mut column = Column::new().spacing(theme.code_block_spacing).width(FILL_WIDTH);
  if let MarkdownCodeBlockKind::Fenced {
    language: Some(language),
  } = kind
  {
    let mut label_style = theme.code_block_label.apply(base_style);
    if theme.code_block_label.text.color.is_none() {
      label_style.color = Color::from_hex("#94a3b8");
    }
    column = column.child(Text::styled(language, label_style).width(FILL_WIDTH));
  }
  column = column.child(Element::from_node(
    Node::rich_text(highlight_code_spans(text.trim_end_matches('\n'), kind, &code_style))
      .text_wrap(false)
      .width(FILL_WIDTH),
  ));
  column = apply_column_box(column, &theme.code_block_box, true);
  column.into()
}

fn render_math_block(text: &str, base_style: &TextStyle, theme: &ThemeMarkdown) -> Element {
  let kind = MarkdownCodeBlockKind::Fenced {
    language: Some("math".to_owned()),
  };
  render_code_block(&kind, text, base_style, theme)
}

fn render_footnote_definition(
  label: &str,
  blocks: &[MarkdownBlock],
  base_style: &TextStyle,
  theme: &ThemeMarkdown,
  render_ctx: &MarkdownRenderContext,
) -> Element {
  let marker_style = theme.link.apply(base_style);
  let mut body = Column::new().spacing(theme.nested_block_spacing).flex(1.0);
  for block in blocks {
    body = body.child(render_block(block, base_style, theme, render_ctx));
  }

  Row::new()
    .spacing(theme.list_marker_gap)
    .align_items(Alignment::Start)
    .child(Text::styled(&format!("[^{label}]"), marker_style).width(theme.ordered_list_marker_width))
    .child(body)
    .width(FILL_WIDTH)
    .into()
}

fn highlight_code_spans(text: &str, kind: &MarkdownCodeBlockKind, base_style: &TextStyle) -> Vec<RichTextSpan> {
  if !is_rust_like_code(kind) {
    return vec![RichTextSpan {
      text: text.to_owned(),
      style: base_style.clone(),
    }];
  }

  let keyword_style = code_style(base_style, Color::from_hex("#93c5fd"));
  let string_style = code_style(base_style, Color::from_hex("#86efac"));
  let comment_style = code_style(base_style, Color::from_hex("#94a3b8"));
  let number_style = code_style(base_style, Color::from_hex("#fca5a5"));
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
  render_ctx: &MarkdownRenderContext,
) -> Element {
  let mut table = Column::new().width(FILL_WIDTH);
  let line_color = table_line_color(theme);
  for (row_index, row) in rows.iter().enumerate() {
    if row_index > 0
      && theme.table_row_spacing > 0.0
      && let Some(line_color) = line_color
    {
      table = table.child(Rect::new(FILL_WIDTH, theme.table_row_spacing).background(line_color));
    }
    let mut row_node = Row::new().align_items(Alignment::Stretch).width(FILL_WIDTH);
    for (cell_index, cell) in row.cells.iter().enumerate() {
      if cell_index > 0
        && theme.table_column_spacing > 0.0
        && let Some(line_color) = line_color
      {
        row_node = row_node.child(Rect::new(theme.table_column_spacing, FILL_WIDTH).background(line_color));
      }
      let alignment = alignments.get(cell_index).copied().unwrap_or_default();
      row_node = row_node.child(render_table_cell(
        row.header, cell, alignment, base_style, theme, render_ctx,
      ));
    }
    table = table.child(row_node);
  }
  table = apply_column_box(table, &theme.table_box, true);
  table.into()
}

fn table_line_color(theme: &ThemeMarkdown) -> Option<Color> {
  theme.table_box.border_color.or(theme.table_box.background)
}

fn render_table_cell(
  header: bool,
  cell: &[MarkdownInline],
  alignment: MarkdownTableAlignment,
  base_style: &TextStyle,
  theme: &ThemeMarkdown,
  render_ctx: &MarkdownRenderContext,
) -> Element {
  let mut style = if header {
    theme.table_header.apply(base_style)
  } else {
    theme.table_cell.apply(base_style)
  };
  if header {
    style.weight = FontWeight::Bold;
  }
  style.text_align = table_text_align(alignment);

  let box_style = if header {
    &theme.table_header_box
  } else {
    &theme.table_cell_box
  };
  if box_style.background.is_some() && style.color == base_style.color {
    style.color = Color::from_hex("#0f172a");
  }
  let mut element = render_inline_block(cell, style, theme, render_ctx);
  element.node = apply_node_box(element.node, box_style)
    .min_width(theme.table_cell_min_width)
    .flex(1.0);
  element
}

fn table_text_align(alignment: MarkdownTableAlignment) -> TextAlign {
  match alignment {
    MarkdownTableAlignment::None | MarkdownTableAlignment::Left => TextAlign::Left,
    MarkdownTableAlignment::Center => TextAlign::Center,
    MarkdownTableAlignment::Right => TextAlign::Right,
  }
}

fn apply_column_box(mut column: Column, style: &MarkdownBlockStyle, include_border: bool) -> Column {
  if let Some(padding) = style.padding {
    column = column.padding(padding);
  }
  if let Some(background) = style.background {
    column = column.background(background);
  }
  if include_border && let (Some(width), Some(color)) = (style.border_width, style.border_color) {
    column = column.border_inside(width, color);
  }
  if let Some(radius) = style.radius {
    column = column.rounded(radius);
  }
  column
}

fn apply_row_box(mut row: Row, style: &MarkdownBlockStyle, include_border: bool) -> Row {
  if let Some(padding) = style.padding {
    row = row.padding(padding);
  }
  if let Some(background) = style.background {
    row = row.background(background);
  }
  if include_border && let (Some(width), Some(color)) = (style.border_width, style.border_color) {
    row = row.border_inside(width, color);
  }
  if let Some(radius) = style.radius {
    row = row.rounded(radius);
  }
  row
}

fn apply_node_box(mut node: Node, style: &MarkdownBlockStyle) -> Node {
  if let Some(padding) = style.padding {
    node = node.padding(padding);
  }
  if let Some(background) = style.background {
    node = node.background(background);
  }
  if let (Some(width), Some(color)) = (style.border_width, style.border_color) {
    node = node.border_inside(width, color);
  }
  if let Some(radius) = style.radius {
    node = node.rounded(radius);
  }
  node
}

fn heading_style(theme: &ThemeMarkdown, level: MarkdownHeadingLevel, base_style: &TextStyle) -> TextStyle {
  let shared = theme.heading.apply(base_style);
  match level {
    MarkdownHeadingLevel::H1 => theme.heading_1.apply(&shared),
    MarkdownHeadingLevel::H2 => theme.heading_2.apply(&shared),
    MarkdownHeadingLevel::H3 => theme.heading_3.apply(&shared),
    MarkdownHeadingLevel::H4 => theme.heading_4.apply(&shared),
    MarkdownHeadingLevel::H5 => theme.heading_5.apply(&shared),
    MarkdownHeadingLevel::H6 => theme.heading_6.apply(&shared),
  }
}
