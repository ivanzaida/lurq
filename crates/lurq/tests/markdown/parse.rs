use lurq::markdown::{
  MarkdownBlock, MarkdownCodeBlockKind, MarkdownHeadingLevel, MarkdownInline, MarkdownListItem, MarkdownTableRow,
  parse_markdown,
};

#[test]
fn parses_heading_and_inline_styles() {
  let doc = parse_markdown("# Hello **bold** and *soft*");

  assert_eq!(
    doc.blocks,
    vec![MarkdownBlock::Heading {
      level: MarkdownHeadingLevel::H1,
      children: vec![
        MarkdownInline::Text("Hello ".to_owned()),
        MarkdownInline::Strong(vec![MarkdownInline::Text("bold".to_owned())]),
        MarkdownInline::Text(" and ".to_owned()),
        MarkdownInline::Emphasis(vec![MarkdownInline::Text("soft".to_owned())]),
      ],
    }]
  );
}

#[test]
fn parses_paragraph_code_link_and_strikethrough() {
  let doc = parse_markdown("Use `Text` with [docs](https://example.test) and ~~old~~ text.");

  assert_eq!(
    doc.blocks,
    vec![MarkdownBlock::Paragraph(vec![
      MarkdownInline::Text("Use ".to_owned()),
      MarkdownInline::Code("Text".to_owned()),
      MarkdownInline::Text(" with ".to_owned()),
      MarkdownInline::Link {
        destination: "https://example.test".to_owned(),
        title: String::new(),
        children: vec![MarkdownInline::Text("docs".to_owned())],
      },
      MarkdownInline::Text(" and ".to_owned()),
      MarkdownInline::Strikethrough(vec![MarkdownInline::Text("old".to_owned())]),
      MarkdownInline::Text(" text.".to_owned()),
    ])]
  );
}

#[test]
fn parses_lists_and_nested_blocks() {
  let doc = parse_markdown("1. first\n2. second\n\n> quoted");

  assert_eq!(
    doc.blocks,
    vec![
      MarkdownBlock::List {
        ordered: true,
        start: Some(1),
        items: vec![
          MarkdownListItem::new(vec![MarkdownBlock::Paragraph(vec![MarkdownInline::Text(
            "first".to_owned()
          )])]),
          MarkdownListItem::new(vec![MarkdownBlock::Paragraph(vec![MarkdownInline::Text(
            "second".to_owned()
          )])]),
        ],
      },
      MarkdownBlock::BlockQuote(vec![MarkdownBlock::Paragraph(vec![MarkdownInline::Text(
        "quoted".to_owned()
      )])]),
    ]
  );
}

#[test]
fn parses_fenced_code_blocks() {
  let doc = parse_markdown("```rust\nfn main() {}\n```");

  assert_eq!(
    doc.blocks,
    vec![MarkdownBlock::CodeBlock {
      kind: MarkdownCodeBlockKind::Fenced {
        language: Some("rust".to_owned()),
      },
      text: "fn main() {}\n".to_owned(),
    }]
  );
}

#[test]
fn parses_tables() {
  let doc = parse_markdown("| Feature | Purpose |\n|---------|---------|\n| `winit` | Window shell |");

  assert_eq!(
    doc.blocks,
    vec![MarkdownBlock::Table {
      rows: vec![
        MarkdownTableRow::new(
          true,
          vec![
            vec![MarkdownInline::Text("Feature".to_owned())],
            vec![MarkdownInline::Text("Purpose".to_owned())],
          ],
        ),
        MarkdownTableRow::new(
          false,
          vec![
            vec![MarkdownInline::Code("winit".to_owned())],
            vec![MarkdownInline::Text("Window shell".to_owned())],
          ],
        ),
      ],
    }]
  );
}
