use pulldown_cmark::{Alignment as CmarkAlignment, CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};

use super::{
  MarkdownBlock, MarkdownCodeBlockKind, MarkdownDocument, MarkdownHeadingLevel, MarkdownInline, MarkdownListItem,
  MarkdownTableAlignment, MarkdownTableRow,
};

pub fn parse_markdown(source: &str) -> MarkdownDocument {
  MarkdownParser::new().parse(source)
}

struct MarkdownParser {
  frames: Vec<Frame>,
}

enum Frame {
  Document {
    blocks: Vec<MarkdownBlock>,
  },
  Paragraph {
    inlines: Vec<MarkdownInline>,
  },
  Heading {
    level: MarkdownHeadingLevel,
    inlines: Vec<MarkdownInline>,
  },
  BlockQuote {
    blocks: Vec<MarkdownBlock>,
  },
  FootnoteDefinition {
    label: String,
    blocks: Vec<MarkdownBlock>,
  },
  List {
    ordered: bool,
    start: Option<u64>,
    items: Vec<MarkdownListItem>,
  },
  Table {
    alignments: Vec<MarkdownTableAlignment>,
    rows: Vec<MarkdownTableRow>,
  },
  TableHead {
    cells: Vec<Vec<MarkdownInline>>,
    rows: Vec<MarkdownTableRow>,
  },
  TableRow {
    cells: Vec<Vec<MarkdownInline>>,
  },
  TableCell {
    inlines: Vec<MarkdownInline>,
  },
  Item {
    blocks: Vec<MarkdownBlock>,
  },
  CodeBlock {
    kind: MarkdownCodeBlockKind,
    text: String,
  },
  HtmlBlock {
    text: String,
  },
  Emphasis {
    inlines: Vec<MarkdownInline>,
  },
  Strong {
    inlines: Vec<MarkdownInline>,
  },
  Strikethrough {
    inlines: Vec<MarkdownInline>,
  },
  Link {
    destination: String,
    title: String,
    inlines: Vec<MarkdownInline>,
  },
  Image {
    destination: String,
    title: String,
    inlines: Vec<MarkdownInline>,
  },
}

impl MarkdownParser {
  fn new() -> Self {
    Self {
      frames: vec![Frame::Document { blocks: Vec::new() }],
    }
  }

  fn parse(mut self, source: &str) -> MarkdownDocument {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_MATH);

    for event in Parser::new_ext(source, options) {
      self.push_event(event);
    }

    match self.frames.pop() {
      Some(Frame::Document { blocks }) => MarkdownDocument::new(blocks),
      _ => MarkdownDocument::default(),
    }
  }

  fn push_event(&mut self, event: Event<'_>) {
    match event {
      Event::Start(tag) => self.start_tag(tag),
      Event::End(tag) => self.end_tag(tag),
      Event::Text(text) => self.push_text(&text),
      Event::Code(code) => self.push_inline(MarkdownInline::Code(code.to_string())),
      Event::Html(html) | Event::InlineHtml(html) => self.push_html(&html),
      Event::FootnoteReference(label) => self.push_inline(MarkdownInline::FootnoteReference(label.to_string())),
      Event::SoftBreak => self.push_inline(MarkdownInline::SoftBreak),
      Event::HardBreak => self.push_inline(MarkdownInline::HardBreak),
      Event::Rule => self.push_block(MarkdownBlock::ThematicBreak),
      Event::TaskListMarker(checked) => self.push_inline(MarkdownInline::TaskListMarker(checked)),
      Event::InlineMath(math) => self.push_inline(MarkdownInline::Math(math.to_string())),
      Event::DisplayMath(math) => self.push_display_math(&math),
    }
  }

  fn start_tag(&mut self, tag: Tag<'_>) {
    match tag {
      Tag::Paragraph => self.frames.push(Frame::Paragraph { inlines: Vec::new() }),
      Tag::Heading { level, .. } => self.frames.push(Frame::Heading {
        level: heading_level(level),
        inlines: Vec::new(),
      }),
      Tag::BlockQuote(_) => self.frames.push(Frame::BlockQuote { blocks: Vec::new() }),
      Tag::FootnoteDefinition(label) => self.frames.push(Frame::FootnoteDefinition {
        label: label.to_string(),
        blocks: Vec::new(),
      }),
      Tag::CodeBlock(kind) => self.frames.push(Frame::CodeBlock {
        kind: code_block_kind(kind),
        text: String::new(),
      }),
      Tag::HtmlBlock => self.frames.push(Frame::HtmlBlock { text: String::new() }),
      Tag::Table(alignments) => self.frames.push(Frame::Table {
        alignments: alignments.into_iter().map(table_alignment).collect(),
        rows: Vec::new(),
      }),
      Tag::TableHead => self.frames.push(Frame::TableHead {
        cells: Vec::new(),
        rows: Vec::new(),
      }),
      Tag::TableRow => self.frames.push(Frame::TableRow { cells: Vec::new() }),
      Tag::TableCell => self.frames.push(Frame::TableCell { inlines: Vec::new() }),
      Tag::List(start) => self.frames.push(Frame::List {
        ordered: start.is_some(),
        start,
        items: Vec::new(),
      }),
      Tag::Item => self.frames.push(Frame::Item { blocks: Vec::new() }),
      Tag::Emphasis => self.frames.push(Frame::Emphasis { inlines: Vec::new() }),
      Tag::Strong => self.frames.push(Frame::Strong { inlines: Vec::new() }),
      Tag::Strikethrough => self.frames.push(Frame::Strikethrough { inlines: Vec::new() }),
      Tag::Link { dest_url, title, .. } => self.frames.push(Frame::Link {
        destination: dest_url.to_string(),
        title: title.to_string(),
        inlines: Vec::new(),
      }),
      Tag::Image { dest_url, title, .. } => self.frames.push(Frame::Image {
        destination: dest_url.to_string(),
        title: title.to_string(),
        inlines: Vec::new(),
      }),
      _ => {}
    }
  }

  fn end_tag(&mut self, tag: TagEnd) {
    match tag {
      TagEnd::Paragraph => self.close_paragraph(),
      TagEnd::Heading(_) => self.close_heading(),
      TagEnd::BlockQuote(_) => self.close_blockquote(),
      TagEnd::FootnoteDefinition => self.close_footnote_definition(),
      TagEnd::CodeBlock => self.close_code_block(),
      TagEnd::HtmlBlock => self.close_html_block(),
      TagEnd::Table => self.close_table(),
      TagEnd::TableHead => self.close_table_head(),
      TagEnd::TableRow => self.close_table_row(),
      TagEnd::TableCell => self.close_table_cell(),
      TagEnd::List(_) => self.close_list(),
      TagEnd::Item => self.close_item(),
      TagEnd::Emphasis => self.close_emphasis(),
      TagEnd::Strong => self.close_strong(),
      TagEnd::Strikethrough => self.close_strikethrough(),
      TagEnd::Link => self.close_link(),
      TagEnd::Image => self.close_image(),
      _ => {}
    }
  }

  fn close_paragraph(&mut self) {
    if let Some(Frame::Paragraph { inlines }) = self.frames.pop() {
      if inlines.is_empty() {
        return;
      }
      self.push_block(MarkdownBlock::Paragraph(inlines));
    }
  }

  fn close_heading(&mut self) {
    if let Some(Frame::Heading { level, inlines }) = self.frames.pop() {
      self.push_block(MarkdownBlock::Heading {
        level,
        children: inlines,
      });
    }
  }

  fn close_blockquote(&mut self) {
    if let Some(Frame::BlockQuote { blocks }) = self.frames.pop() {
      self.push_block(MarkdownBlock::BlockQuote(blocks));
    }
  }

  fn close_footnote_definition(&mut self) {
    if let Some(Frame::FootnoteDefinition { label, blocks }) = self.frames.pop() {
      self.push_block(MarkdownBlock::FootnoteDefinition { label, blocks });
    }
  }

  fn close_code_block(&mut self) {
    if let Some(Frame::CodeBlock { kind, text }) = self.frames.pop() {
      self.push_block(MarkdownBlock::CodeBlock { kind, text });
    }
  }

  fn close_html_block(&mut self) {
    if let Some(Frame::HtmlBlock { text }) = self.frames.pop() {
      self.push_block(MarkdownBlock::Html(text));
    }
  }

  fn close_list(&mut self) {
    if let Some(Frame::List { ordered, start, items }) = self.frames.pop() {
      self.push_block(MarkdownBlock::List { ordered, start, items });
    }
  }

  fn close_table(&mut self) {
    if let Some(Frame::Table { alignments, rows }) = self.frames.pop() {
      self.push_block(MarkdownBlock::Table { alignments, rows });
    }
  }

  fn close_table_head(&mut self) {
    if let Some(Frame::TableHead { mut cells, mut rows }) = self.frames.pop()
      && let Some(Frame::Table { rows: table_rows, .. }) = self.frames.last_mut()
    {
      if !cells.is_empty() {
        rows.insert(0, MarkdownTableRow::new(true, std::mem::take(&mut cells)));
      }
      table_rows.extend(rows);
    }
  }

  fn close_table_row(&mut self) {
    let Some(Frame::TableRow { cells }) = self.frames.pop() else {
      return;
    };
    match self.frames.last_mut() {
      Some(Frame::TableHead { rows, .. }) => rows.push(MarkdownTableRow::new(true, cells)),
      Some(Frame::Table { rows, .. }) => rows.push(MarkdownTableRow::new(false, cells)),
      _ => {}
    }
  }

  fn close_table_cell(&mut self) {
    if let Some(Frame::TableCell { inlines }) = self.frames.pop()
      && let Some(parent) = self.frames.last_mut()
    {
      match parent {
        Frame::TableRow { cells } | Frame::TableHead { cells, .. } => cells.push(inlines),
        _ => {}
      }
    }
  }

  fn close_item(&mut self) {
    if let Some(Frame::Item { blocks }) = self.frames.pop()
      && let Some(Frame::List { items, .. }) = self.frames.last_mut()
    {
      items.push(MarkdownListItem::new(blocks));
    }
  }

  fn close_emphasis(&mut self) {
    if let Some(Frame::Emphasis { inlines }) = self.frames.pop() {
      self.push_inline(MarkdownInline::Emphasis(inlines));
    }
  }

  fn close_strong(&mut self) {
    if let Some(Frame::Strong { inlines }) = self.frames.pop() {
      self.push_inline(MarkdownInline::Strong(inlines));
    }
  }

  fn close_strikethrough(&mut self) {
    if let Some(Frame::Strikethrough { inlines }) = self.frames.pop() {
      self.push_inline(MarkdownInline::Strikethrough(inlines));
    }
  }

  fn close_link(&mut self) {
    if let Some(Frame::Link {
      destination,
      title,
      inlines,
    }) = self.frames.pop()
    {
      self.push_inline(MarkdownInline::Link {
        destination,
        title,
        children: inlines,
      });
    }
  }

  fn close_image(&mut self) {
    if let Some(Frame::Image {
      destination,
      title,
      inlines,
    }) = self.frames.pop()
    {
      self.push_inline(MarkdownInline::Image {
        destination,
        title,
        alt: inlines,
      });
    }
  }

  fn push_text(&mut self, text: &str) {
    match self.frames.last_mut() {
      Some(Frame::CodeBlock { text: code, .. }) | Some(Frame::HtmlBlock { text: code }) => code.push_str(text),
      _ => self.push_inline(MarkdownInline::Text(text.to_owned())),
    }
  }

  fn push_html(&mut self, html: &str) {
    match self.frames.last_mut() {
      Some(Frame::HtmlBlock { text }) => text.push_str(html),
      _ => self.push_inline(MarkdownInline::Html(html.to_owned())),
    }
  }

  fn push_display_math(&mut self, text: &str) {
    let text = text.trim_matches('\n');
    let Some(Frame::Paragraph { inlines }) = self.frames.last() else {
      self.push_block(MarkdownBlock::Math { text: text.to_owned() });
      return;
    };
    if !inlines.is_empty() {
      self.push_inline(MarkdownInline::Math(text.to_owned()));
      return;
    }

    let empty_paragraph = self.frames.pop();
    self.push_block(MarkdownBlock::Math { text: text.to_owned() });
    if empty_paragraph.is_some() {
      self.frames.push(Frame::Paragraph { inlines: Vec::new() });
    }
  }

  fn push_block(&mut self, block: MarkdownBlock) {
    match self.frames.last_mut() {
      Some(Frame::Document { blocks })
      | Some(Frame::BlockQuote { blocks })
      | Some(Frame::FootnoteDefinition { blocks, .. })
      | Some(Frame::Item { blocks }) => blocks.push(block),
      _ => {}
    }
  }

  fn push_inline(&mut self, inline: MarkdownInline) {
    match self.frames.last_mut() {
      Some(Frame::Paragraph { inlines })
      | Some(Frame::Heading { inlines, .. })
      | Some(Frame::TableCell { inlines })
      | Some(Frame::Emphasis { inlines })
      | Some(Frame::Strong { inlines })
      | Some(Frame::Strikethrough { inlines })
      | Some(Frame::Link { inlines, .. })
      | Some(Frame::Image { inlines, .. }) => inlines.push(inline),
      Some(Frame::Item { blocks })
      | Some(Frame::BlockQuote { blocks })
      | Some(Frame::FootnoteDefinition { blocks, .. }) => push_implicit_paragraph_inline(blocks, inline),
      _ => {}
    }
  }
}

fn push_implicit_paragraph_inline(blocks: &mut Vec<MarkdownBlock>, inline: MarkdownInline) {
  if let Some(MarkdownBlock::Paragraph(inlines)) = blocks.last_mut() {
    inlines.push(inline);
  } else {
    blocks.push(MarkdownBlock::Paragraph(vec![inline]));
  }
}

fn heading_level(level: HeadingLevel) -> MarkdownHeadingLevel {
  match level {
    HeadingLevel::H1 => MarkdownHeadingLevel::H1,
    HeadingLevel::H2 => MarkdownHeadingLevel::H2,
    HeadingLevel::H3 => MarkdownHeadingLevel::H3,
    HeadingLevel::H4 => MarkdownHeadingLevel::H4,
    HeadingLevel::H5 => MarkdownHeadingLevel::H5,
    HeadingLevel::H6 => MarkdownHeadingLevel::H6,
  }
}

fn code_block_kind(kind: CodeBlockKind<'_>) -> MarkdownCodeBlockKind {
  match kind {
    CodeBlockKind::Indented => MarkdownCodeBlockKind::Indented,
    CodeBlockKind::Fenced(language) => {
      let language = language
        .split(' ')
        .next()
        .filter(|language| !language.is_empty())
        .map(str::to_owned);
      MarkdownCodeBlockKind::Fenced { language }
    }
  }
}

fn table_alignment(alignment: CmarkAlignment) -> MarkdownTableAlignment {
  match alignment {
    CmarkAlignment::None => MarkdownTableAlignment::None,
    CmarkAlignment::Left => MarkdownTableAlignment::Left,
    CmarkAlignment::Center => MarkdownTableAlignment::Center,
    CmarkAlignment::Right => MarkdownTableAlignment::Right,
  }
}
