//! A document is read from Markdown and written as Markdown that reads back as the same document.
use super::*;
use crate::components::styled_text::{ParagraphKind::*, SpanStyle};

fn span(range: std::ops::Range<usize>, style: SpanStyle) -> StyledSpan {
  StyledSpan::new(range, style)
}

/// The document a source says, checked to be written back as a source that says it again.
fn read(source: &str) -> RichText {
  let document = RichText::from_markdown(source);
  let written = document.to_markdown();
  assert_eq!(
    RichText::from_markdown(&written),
    document,
    "what was written reads back as it was: {written:?}"
  );
  document
}

#[test]
fn rich_text_reads_the_blocks_of_markdown_as_kinds_of_paragraphs() {
  let document = read(
    "## Release notes\n\nBackdrop blur.\n\n- Faster shaping\n- Fixes\n\n1. one\n2. two\n\n> said\n\n```\nlet a = 1;\nlet b = 2;\n```",
  );
  assert_eq!(
    document.text,
    "Release notes\nBackdrop blur.\nFaster shaping\nFixes\none\ntwo\nsaid\nlet a = 1;\nlet b = 2;"
  );
  assert_eq!(
    document.paragraphs,
    vec![Heading(2), Body, Bullet, Bullet, Ordered, Ordered, Quote, Code, Code]
  );
  assert!(document.spans.is_empty());
}

#[test]
fn rich_text_reads_the_marks_of_markdown_as_spans() {
  let document = read("Blur **follows *rounded* clips** and ~~moves~~ `code`, [docs](https://lurq.dev), <u>under</u>.");
  assert_eq!(document.text, "Blur follows rounded clips and moves code, docs, under.");
  assert_eq!(
    document.spans,
    vec![
      span(5..13, SpanStyle::new().bold()),
      span(13..20, SpanStyle::new().bold().italic()),
      span(20..26, SpanStyle::new().bold()),
      span(31..36, SpanStyle::new().strikethrough()),
      span(37..41, SpanStyle::new().code()),
      span(43..47, SpanStyle::new().link("https://lurq.dev")),
      span(49..54, SpanStyle::new().underline()),
    ]
  );
}

#[test]
fn rich_text_writes_markdown_with_marks_nested_and_lists_numbered() {
  let document = RichText::new(
    "Title\nplain bold both\none\ntwo\nquoted\nmore",
    vec![
      span(12..17, SpanStyle::new().bold()),
      span(17..21, SpanStyle::new().bold().italic()),
    ],
    vec![Heading(1), Body, Ordered, Ordered, Quote, Quote],
  );
  assert_eq!(
    document.to_markdown(),
    "# Title\n\nplain **bold *both***\n\n1. one\n2. two\n\n> quoted\n>\n> more"
  );
  assert_eq!(RichText::from_markdown(&document.to_markdown()), document);
}

#[test]
fn rich_text_escapes_what_markdown_would_read_as_markup() {
  let document = RichText::plain("# not a heading\n1. not an item\na * star and a [bracket]\n- not a bullet");
  let written = document.to_markdown();
  assert_eq!(
    written,
    "\\# not a heading\n\n1\\. not an item\n\na \\* star and a \\[bracket\\]\n\n\\- not a bullet"
  );
  assert_eq!(RichText::from_markdown(&written), document);
}

#[test]
fn rich_text_keeps_the_space_at_the_edge_of_a_mark_outside_it() {
  // Emphasis does not take in the space at its edges: the mark is written around the words.
  let document = RichText::new("a bold b", vec![span(1..7, SpanStyle::new().bold())], vec![]);
  assert_eq!(document.to_markdown(), "a **bold** b");
}

#[test]
fn rich_text_is_canonical_and_counts_its_words() {
  let document = RichText::new("one two\nthree", vec![span(0..3, SpanStyle::default())], vec![Bullet]);
  assert!(document.spans.is_empty(), "a plain span is no span");
  assert_eq!(document.paragraphs, vec![Bullet, Body], "one kind for each paragraph");
  assert_eq!(document.word_count(), 3);
  assert!(RichText::default().is_empty());
  let blocks = document.blocks();
  assert_eq!(blocks.len(), 2);
  assert_eq!((blocks[1].0, blocks[1].1.text.as_str()), (Body, "three"));
}

#[test]
fn a_text_reads_as_markdown_where_a_line_of_it_has_a_block_mark_or_markdown_reads_a_mark_in_it() {
  for source in [
    "# Title",
    "text\n\n###### Small",
    "- item",
    "* item",
    "+ item",
    "1. first",
    "12) twelfth",
    "> quoted",
    "```\ncode\n```",
    "~~~\ncode\n~~~",
    "   - three spaces in",
    "2 * 3 = 6, **really**",
    "*emphasis* at the start",
    "a `call()` in a line",
    "~~struck~~",
    "see [the docs](https://lurq.dev)",
    "<u>under</u> it",
  ] {
    assert!(RichText::reads_as_markdown(source), "{source:?}");
  }
  for source in [
    "",
    "Plain words.",
    "2 * 3 = 6",
    "snake_case_names and a_b_c",
    "a < b and [x]",
    "#hashtag and #another",
    "####### seven",
    "-dash",
    "- ",
    "1.5 is a number",
    "a > b",
    "    - four spaces in is code",
  ] {
    assert!(!RichText::reads_as_markdown(source), "{source:?}");
  }
}
