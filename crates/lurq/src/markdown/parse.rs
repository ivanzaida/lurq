use pulldown_cmark::{
  Alignment as CmarkAlignment, BlockQuoteKind, CodeBlockKind, Event, HeadingLevel, LinkType, Options, Parser, Tag,
  TagEnd,
};

use super::{
  MarkdownAdmonitionKind, MarkdownBlock, MarkdownCodeBlockKind, MarkdownDocument, MarkdownHeadingLevel, MarkdownInline,
  MarkdownListItem, MarkdownTableAlignment, MarkdownTableRow, rich_text::markdown_html_text,
};

pub fn parse_markdown(source: &str) -> MarkdownDocument {
  parse_markdown_with(source, MarkdownOptions::new())
}

/// What Markdown reads beside CommonMark and GitHub's tables, strikethrough, task lists, footnotes, math and
/// admonitions, which it always reads.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MarkdownOptions {
  wiki_links: bool,
}

impl MarkdownOptions {
  pub fn new() -> Self {
    Self::default()
  }

  /// Whether `[[target]]` and `[[target|words]]` are wiki links ([`MarkdownInline::WikiLink`]) and not text.
  pub fn wiki_links(mut self, wiki_links: bool) -> Self {
    self.wiki_links = wiki_links;
    self
  }
}

/// `source` read as [`parse_markdown`] reads it, with what `options` adds.
pub fn parse_markdown_with(source: &str, options: MarkdownOptions) -> MarkdownDocument {
  MarkdownParser::new().parse(source, options)
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
  Admonition {
    kind: MarkdownAdmonitionKind,
    blocks: Vec<MarkdownBlock>,
  },
  /// A `<details>` element, open from its HTML block to the one that closes it.
  Details {
    summary: Option<String>,
    open: bool,
    blocks: Vec<MarkdownBlock>,
  },
  /// A `<kbd>` element, from its opening tag to its closing one: its words.
  Kbd {
    text: String,
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
  /// A wiki link, and whether it was given its words after a `|`.
  WikiLink {
    target: String,
    piped: bool,
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

  fn parse(mut self, source: &str, read: MarkdownOptions) -> MarkdownDocument {
    let mut options = Options::empty();
    if read.wiki_links {
      options.insert(Options::ENABLE_WIKILINKS);
    }
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_MATH);
    // GitHub's admonitions: `> [!NOTE]` and the rest.
    options.insert(Options::ENABLE_GFM);

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
      Tag::BlockQuote(Some(kind)) => self.frames.push(Frame::Admonition {
        kind: admonition_kind(kind),
        blocks: Vec::new(),
      }),
      Tag::BlockQuote(None) => self.frames.push(Frame::BlockQuote { blocks: Vec::new() }),
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
      Tag::Link {
        link_type: LinkType::WikiLink { has_pothole },
        dest_url,
        ..
      } => self.frames.push(Frame::WikiLink {
        target: dest_url.to_string(),
        piped: has_pothole,
        inlines: Vec::new(),
      }),
      Tag::Link {
        link_type,
        dest_url,
        title,
        ..
      } => self.frames.push(Frame::Link {
        // An email autolink (`<a@b.c>`) is a `mailto:` link, as in HTML.
        destination: if link_type == LinkType::Email && !dest_url.starts_with("mailto:") {
          format!("mailto:{dest_url}")
        } else {
          dest_url.to_string()
        },
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
    match self.frames.pop() {
      Some(Frame::BlockQuote { blocks }) => self.push_block(MarkdownBlock::BlockQuote(blocks)),
      Some(Frame::Admonition { kind, blocks }) => self.push_block(MarkdownBlock::Admonition { kind, blocks }),
      Some(other) => self.frames.push(other),
      None => {}
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

  /// An HTML block: a `<details>` opens what the blocks after it go into, a
  /// `</details>` closes it, and a `<summary>` names it; any other is a block
  /// of its own.
  fn close_html_block(&mut self) {
    let Some(Frame::HtmlBlock { text }) = self.frames.pop() else {
      return;
    };
    let mut rest = text.as_str();
    loop {
      let trimmed = rest.trim_start();
      if let Some(after) = strip_tag(trimmed, "details") {
        let (open, after) = details_open(after);
        self.frames.push(Frame::Details {
          summary: None,
          open,
          blocks: Vec::new(),
        });
        rest = after;
      } else if let Some(after) = strip_tag(trimmed, "summary") {
        // Past the tag's own attributes and its `>`.
        let after = after.find('>').map_or("", |end| &after[end + 1..]);
        let (summary, after) = match after.find("</summary>") {
          Some(end) => (&after[..end], &after[end + "</summary>".len()..]),
          None => (after, ""),
        };
        if let Some(Frame::Details { summary: named, .. }) = self.frames.last_mut() {
          *named = Some(markdown_html_text(summary).trim().to_owned());
        }
        rest = after;
      } else if let Some(after) = trimmed.strip_prefix("</details>") {
        if let Some(position) = self
          .frames
          .iter()
          .rposition(|frame| matches!(frame, Frame::Details { .. }))
        {
          // A block left open inside it ends with it.
          while self.frames.len() > position + 1 {
            self.end_open_frame();
          }
          if let Some(Frame::Details { summary, open, blocks }) = self.frames.pop() {
            self.push_block(MarkdownBlock::Details {
              summary: summary.unwrap_or_else(|| "Details".to_owned()),
              open,
              blocks,
            });
          }
        }
        rest = after;
      } else {
        break;
      }
    }
    if !rest.trim().is_empty() {
      self.push_block(MarkdownBlock::Html(rest.to_owned()));
    }
  }

  /// Ends the frame on top as its closing event would.
  fn end_open_frame(&mut self) {
    match self.frames.last() {
      Some(Frame::Paragraph { .. }) => self.close_paragraph(),
      Some(Frame::Details { .. }) => {
        if let Some(Frame::Details { summary, open, blocks }) = self.frames.pop() {
          self.push_block(MarkdownBlock::Details {
            summary: summary.unwrap_or_else(|| "Details".to_owned()),
            open,
            blocks,
          });
        }
      }
      _ => {
        self.frames.pop();
      }
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
    match self.frames.pop() {
      Some(Frame::Link {
        destination,
        title,
        inlines,
      }) => self.push_inline(MarkdownInline::Link {
        destination,
        title,
        children: inlines,
      }),
      // Without a `|` its words are its target, which the app's title may stand for.
      Some(Frame::WikiLink { target, piped, inlines }) => self.push_inline(MarkdownInline::WikiLink {
        target,
        words: if piped { inlines } else { Vec::new() },
      }),
      _ => {}
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
      Some(Frame::CodeBlock { text: code, .. })
      | Some(Frame::HtmlBlock { text: code })
      | Some(Frame::Kbd { text: code }) => code.push_str(text),
      _ => self.push_inline(MarkdownInline::Text(text.to_owned())),
    }
  }

  /// HTML in a block goes into it; in a sentence, `<kbd>` opens a key and
  /// `</kbd>` closes it, and any other tag is kept as it is.
  fn push_html(&mut self, html: &str) {
    if let Some(Frame::HtmlBlock { text }) = self.frames.last_mut() {
      text.push_str(html);
      return;
    }
    let tag = html.trim().to_ascii_lowercase();
    if tag == "<kbd>" {
      self.frames.push(Frame::Kbd { text: String::new() });
      return;
    }
    if tag == "</kbd>"
      && let Some(Frame::Kbd { .. }) = self.frames.last()
    {
      if let Some(Frame::Kbd { text }) = self.frames.pop() {
        self.push_inline(MarkdownInline::Kbd(text));
      }
      return;
    }
    self.push_inline(MarkdownInline::Html(html.to_owned()));
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
      | Some(Frame::Admonition { blocks, .. })
      | Some(Frame::Details { blocks, .. })
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
      | Some(Frame::Admonition { blocks, .. })
      | Some(Frame::Details { blocks, .. })
      | Some(Frame::FootnoteDefinition { blocks, .. }) => push_implicit_paragraph_inline(blocks, inline),
      // A key in a key: its words go into the key's.
      Some(Frame::Kbd { text }) => {
        if let MarkdownInline::Text(words) = inline {
          text.push_str(&words);
        }
      }
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

/// What follows `<tag` (or `<tag>`) at the start of `text`, where the tag is that one.
fn strip_tag<'t>(text: &'t str, tag: &str) -> Option<&'t str> {
  let rest = text.strip_prefix('<')?;
  let name = rest.get(..tag.len())?;
  if !name.eq_ignore_ascii_case(tag) {
    return None;
  }
  let after = &rest[tag.len()..];
  after
    .starts_with(|c: char| c == '>' || c.is_whitespace())
    .then_some(after)
}

/// Whether a `<details` tag says `open`, and what follows the tag.
fn details_open(after_name: &str) -> (bool, &str) {
  match after_name.find('>') {
    Some(end) => {
      let attributes = &after_name[..end];
      let open = attributes
        .split_whitespace()
        .any(|attribute| attribute.eq_ignore_ascii_case("open") || attribute.to_ascii_lowercase().starts_with("open="));
      (open, &after_name[end + 1..])
    }
    None => (false, ""),
  }
}

fn admonition_kind(kind: BlockQuoteKind) -> MarkdownAdmonitionKind {
  match kind {
    BlockQuoteKind::Note => MarkdownAdmonitionKind::Note,
    BlockQuoteKind::Tip => MarkdownAdmonitionKind::Tip,
    BlockQuoteKind::Important => MarkdownAdmonitionKind::Important,
    BlockQuoteKind::Warning => MarkdownAdmonitionKind::Warning,
    BlockQuoteKind::Caution => MarkdownAdmonitionKind::Caution,
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
