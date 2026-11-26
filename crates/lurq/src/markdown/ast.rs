#[derive(Clone, Debug, Default, PartialEq)]
pub struct MarkdownDocument {
  pub blocks: Vec<MarkdownBlock>,
}

impl MarkdownDocument {
  pub fn new(blocks: Vec<MarkdownBlock>) -> Self {
    Self { blocks }
  }

  pub fn is_empty(&self) -> bool {
    self.blocks.is_empty()
  }

  /// Whether a wiki link is anywhere in it.
  pub fn has_wiki_links(&self) -> bool {
    let mut found = false;
    self.visit_inlines(&mut |inlines| {
      found |= inlines
        .iter()
        .any(|inline| matches!(inline, MarkdownInline::WikiLink { .. }))
    });
    found
  }

  /// The document with each wiki link a link to its target: its words, or where it has none the title `title`
  /// gives its target, else the target itself.
  pub fn resolve_wiki_links(&mut self, title: &dyn Fn(&str) -> Option<String>) {
    self.visit_inlines_mut(&mut |inlines| resolve_wiki_links(inlines, title));
  }

  fn visit_inlines(&self, visit: &mut dyn FnMut(&[MarkdownInline])) {
    fn inline(inlines: &[MarkdownInline], visit: &mut dyn FnMut(&[MarkdownInline])) {
      visit(inlines);
      for held in inlines {
        match held {
          MarkdownInline::Emphasis(children)
          | MarkdownInline::Strong(children)
          | MarkdownInline::Strikethrough(children)
          | MarkdownInline::Link { children, .. }
          | MarkdownInline::WikiLink { words: children, .. }
          | MarkdownInline::Image { alt: children, .. } => inline(children, visit),
          _ => {}
        }
      }
    }
    fn blocks(held: &[MarkdownBlock], visit: &mut dyn FnMut(&[MarkdownInline])) {
      for block in held {
        match block {
          MarkdownBlock::Paragraph(inlines) | MarkdownBlock::Heading { children: inlines, .. } => {
            inline(inlines, visit)
          }
          MarkdownBlock::BlockQuote(held)
          | MarkdownBlock::Admonition { blocks: held, .. }
          | MarkdownBlock::Details { blocks: held, .. }
          | MarkdownBlock::FootnoteDefinition { blocks: held, .. } => blocks(held, visit),
          MarkdownBlock::List { items, .. } => {
            for item in items {
              blocks(&item.blocks, visit);
            }
          }
          MarkdownBlock::Table { rows, .. } => {
            for cell in rows.iter().flat_map(|row| &row.cells) {
              inline(cell, visit);
            }
          }
          _ => {}
        }
      }
    }
    blocks(&self.blocks, visit);
  }

  fn visit_inlines_mut(&mut self, visit: &mut dyn FnMut(&mut Vec<MarkdownInline>)) {
    fn blocks(held: &mut [MarkdownBlock], visit: &mut dyn FnMut(&mut Vec<MarkdownInline>)) {
      for block in held {
        match block {
          MarkdownBlock::Paragraph(inlines) | MarkdownBlock::Heading { children: inlines, .. } => visit(inlines),
          MarkdownBlock::BlockQuote(held)
          | MarkdownBlock::Admonition { blocks: held, .. }
          | MarkdownBlock::Details { blocks: held, .. }
          | MarkdownBlock::FootnoteDefinition { blocks: held, .. } => blocks(held, visit),
          MarkdownBlock::List { items, .. } => {
            for item in items {
              blocks(&mut item.blocks, visit);
            }
          }
          MarkdownBlock::Table { rows, .. } => {
            for cell in rows.iter_mut().flat_map(|row| &mut row.cells) {
              visit(cell);
            }
          }
          _ => {}
        }
      }
    }
    blocks(&mut self.blocks, visit);
  }
}

/// `inlines` with each wiki link in them, and in what they hold, a link to its target.
fn resolve_wiki_links(inlines: &mut [MarkdownInline], title: &dyn Fn(&str) -> Option<String>) {
  for inline in inlines {
    match inline {
      MarkdownInline::WikiLink { target, words } => {
        let mut children = std::mem::take(words);
        resolve_wiki_links(&mut children, title);
        if children.is_empty() {
          children.push(MarkdownInline::Text(title(target).unwrap_or_else(|| target.clone())));
        }
        *inline = MarkdownInline::Link {
          destination: std::mem::take(target),
          title: String::new(),
          children,
        };
      }
      MarkdownInline::Emphasis(children)
      | MarkdownInline::Strong(children)
      | MarkdownInline::Strikethrough(children)
      | MarkdownInline::Link { children, .. } => resolve_wiki_links(children, title),
      _ => {}
    }
  }
}

#[cfg(feature = "devtools")]
impl crate::app::component::DevtoolsInspectable for MarkdownDocument {
  fn write_info(&self, buffer: &mut Vec<crate::app::component::ComponentInfo>) {
    buffer.push(crate::app::component::ComponentInfo::with_value(
      "blocks",
      std::any::type_name::<usize>(),
      self.blocks.len().to_string(),
    ));
  }
}

#[derive(Clone, Debug, PartialEq)]
pub enum MarkdownBlock {
  Paragraph(Vec<MarkdownInline>),
  Heading {
    level: MarkdownHeadingLevel,
    children: Vec<MarkdownInline>,
  },
  BlockQuote(Vec<MarkdownBlock>),
  /// A quote GitHub draws as a note, a tip, a warning: `> [!NOTE]` on its first line.
  Admonition {
    kind: MarkdownAdmonitionKind,
    blocks: Vec<MarkdownBlock>,
  },
  /// What a `<details>` element holds, under its `<summary>`, which opens and closes it.
  Details {
    summary: String,
    /// Whether it is open to begin with: `<details open>`.
    open: bool,
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
  CodeBlock {
    kind: MarkdownCodeBlockKind,
    text: String,
  },
  Math {
    text: String,
  },
  FootnoteDefinition {
    label: String,
    blocks: Vec<MarkdownBlock>,
  },
  Html(String),
  ThematicBreak,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct MarkdownListItem {
  pub blocks: Vec<MarkdownBlock>,
}

impl MarkdownListItem {
  pub fn new(blocks: Vec<MarkdownBlock>) -> Self {
    Self { blocks }
  }
}

#[derive(Clone, Debug, PartialEq)]
pub struct MarkdownTableRow {
  pub header: bool,
  pub cells: Vec<Vec<MarkdownInline>>,
}

impl MarkdownTableRow {
  pub fn new(header: bool, cells: Vec<Vec<MarkdownInline>>) -> Self {
    Self { header, cells }
  }
}

/// The kind of an admonition, as GitHub writes it: `> [!NOTE]`, `> [!TIP]`,
/// `> [!IMPORTANT]`, `> [!WARNING]`, `> [!CAUTION]`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MarkdownAdmonitionKind {
  Note,
  Tip,
  Important,
  Warning,
  Caution,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MarkdownHeadingLevel {
  H1,
  H2,
  H3,
  H4,
  H5,
  H6,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MarkdownCodeBlockKind {
  Indented,
  Fenced { language: Option<String> },
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MarkdownTableAlignment {
  #[default]
  None,
  Left,
  Center,
  Right,
}

#[derive(Clone, Debug, PartialEq)]
pub enum MarkdownInline {
  Text(String),
  Code(String),
  Math(String),
  Emphasis(Vec<MarkdownInline>),
  Strong(Vec<MarkdownInline>),
  Strikethrough(Vec<MarkdownInline>),
  Link {
    destination: String,
    title: String,
    children: Vec<MarkdownInline>,
  },
  /// A wiki link, `[[/slug]]` or `[[/slug|words]]`, read where the document is parsed with wiki links
  /// ([`MarkdownOptions::wiki_links`]): its target and its words, none where it has no `|` (the target's title is
  /// then shown, as the app resolves it, else the target).
  WikiLink {
    target: String,
    words: Vec<MarkdownInline>,
  },
  Image {
    destination: String,
    title: String,
    alt: Vec<MarkdownInline>,
  },
  Html(String),
  FootnoteReference(String),
  /// A key of the keyboard, `<kbd>Ctrl</kbd>`: its words.
  Kbd(String),
  SoftBreak,
  HardBreak,
  TaskListMarker(bool),
}
