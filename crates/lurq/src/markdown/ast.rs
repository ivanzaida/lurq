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
  List {
    ordered: bool,
    start: Option<u64>,
    items: Vec<MarkdownListItem>,
  },
  Table {
    rows: Vec<MarkdownTableRow>,
  },
  CodeBlock {
    kind: MarkdownCodeBlockKind,
    text: String,
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

#[derive(Clone, Debug, PartialEq)]
pub enum MarkdownInline {
  Text(String),
  Code(String),
  Emphasis(Vec<MarkdownInline>),
  Strong(Vec<MarkdownInline>),
  Strikethrough(Vec<MarkdownInline>),
  Link {
    destination: String,
    title: String,
    children: Vec<MarkdownInline>,
  },
  Image {
    destination: String,
    title: String,
    alt: Vec<MarkdownInline>,
  },
  Html(String),
  FootnoteReference(String),
  SoftBreak,
  HardBreak,
  TaskListMarker(bool),
}
