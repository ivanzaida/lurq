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
    text_style::{FontWeight, TextStyle},
  },
  markdown::{
    MarkdownBlock, MarkdownCodeBlockKind, MarkdownDocument, MarkdownHeadingLevel, MarkdownInline, MarkdownListItem,
    MarkdownTableRow, markdown_inline_rich_text, parse_markdown,
  },
  node::{Element, Node, color::Color, dimension::Dimension},
};

const FILL_WIDTH: Dimension = Dimension::Pct(100.0);

#[derive(Clone, PartialEq)]
pub struct MarkdownProps {
  source: Arc<str>,
  style: TextStyle,
  selectable: bool,
  width: Option<Dimension>,
}

impl MarkdownProps {
  pub fn new(source: impl Into<Arc<str>>) -> Self {
    Self {
      source: source.into(),
      style: TextStyle::default(),
      selectable: false,
      width: None,
    }
  }

  pub fn styled(source: impl Into<Arc<str>>, style: TextStyle) -> Self {
    Self::new(source).style(style)
  }

  pub fn style(mut self, style: TextStyle) -> Self {
    self.style = style;
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
    let theme = ctx.theme().markdown().clone();
    let document = self.document.get();
    let mut element = render_document(&document, &props.style, &theme);

    if let Some(width) = props.width {
      element.node = element.node.width(width);
    }
    if props.selectable {
      element.node.selectable_recursive(true);
    }
    element
  }
}

fn render_document(document: &MarkdownDocument, base_style: &TextStyle, theme: &ThemeMarkdown) -> Element {
  let mut root = Column::new().spacing(theme.document_spacing).width(FILL_WIDTH);
  for block in &document.blocks {
    root = root.child(render_block(block, base_style, theme));
  }
  root.into()
}

fn render_block(block: &MarkdownBlock, base_style: &TextStyle, theme: &ThemeMarkdown) -> Element {
  match block {
    MarkdownBlock::Paragraph(inlines) => render_inline_block(inlines, base_style.clone(), theme),
    MarkdownBlock::Heading { level, children } => {
      render_inline_block(children, heading_style(theme, *level, base_style), theme)
    }
    MarkdownBlock::BlockQuote(blocks) => render_blockquote(blocks, base_style, theme),
    MarkdownBlock::List { ordered, start, items } => render_list(*ordered, *start, items, base_style, theme),
    MarkdownBlock::Table { rows } => render_table(rows, base_style, theme),
    MarkdownBlock::CodeBlock { kind, text } => render_code_block(kind, text, base_style, theme),
    MarkdownBlock::Html(text) => Text::styled(text.trim_end_matches('\n'), base_style.clone())
      .width(FILL_WIDTH)
      .into(),
    MarkdownBlock::ThematicBreak => Rect::new(FILL_WIDTH, 1.0)
      .background(theme.table_marker.apply(base_style).color)
      .into(),
  }
}

fn render_inline_block(inlines: &[MarkdownInline], style: TextStyle, theme: &ThemeMarkdown) -> Element {
  Element::from_node(Node::rich_text(markdown_inline_rich_text(inlines, &style, theme)).width(FILL_WIDTH))
}

fn render_blockquote(blocks: &[MarkdownBlock], base_style: &TextStyle, theme: &ThemeMarkdown) -> Element {
  let mut body = Column::new().spacing(theme.nested_block_spacing).flex(1.0);
  for block in blocks {
    body = body.child(render_block(block, base_style, theme));
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
    let mut body = Column::new().spacing(theme.nested_block_spacing).flex(1.0);
    for block in &item.blocks {
      body = body.child(render_block(block, base_style, theme));
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
  column = column.child(
    Text::styled(text.trim_end_matches('\n'), code_style)
      .nowrap()
      .width(FILL_WIDTH),
  );
  column = apply_column_box(column, &theme.code_block_box, true);
  column.into()
}

fn render_table(rows: &[MarkdownTableRow], base_style: &TextStyle, theme: &ThemeMarkdown) -> Element {
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
      row_node = row_node.child(render_table_cell(row.header, cell, base_style, theme));
    }
    table = table.child(row_node);
  }
  table = apply_column_box(table, &theme.table_box, true);
  table.into()
}

fn table_line_color(theme: &ThemeMarkdown) -> Option<Color> {
  theme.table_box.border_color.or(theme.table_box.background)
}

fn render_table_cell(header: bool, cell: &[MarkdownInline], base_style: &TextStyle, theme: &ThemeMarkdown) -> Element {
  let mut style = if header {
    theme.table_header.apply(base_style)
  } else {
    theme.table_cell.apply(base_style)
  };
  if header {
    style.weight = FontWeight::Bold;
  }

  let box_style = if header {
    &theme.table_header_box
  } else {
    &theme.table_cell_box
  };
  if box_style.background.is_some() && style.color == base_style.color {
    style.color = Color::from_hex("#0f172a");
  }
  let node = Node::rich_text(markdown_inline_rich_text(cell, &style, theme))
    .min_width(theme.table_cell_min_width)
    .flex(1.0);
  Element::from_node(apply_node_box(node, box_style))
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
