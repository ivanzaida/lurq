//! Wiki links (`[[target]]`, `[[target|words]]`) where a document reads them, and a tag in a sentence
//! (`<tag>changed later</tag>`): issue #258.

use lurq::{
  app::theme::PaletteColor,
  components::MarkdownProps,
  layout::quad::QuadContent,
  markdown::{MarkdownBlock, MarkdownInline, MarkdownOptions, parse_markdown, parse_markdown_with},
};

use super::links::{has_element_with_text, mount, quads};

fn wiki(source: &str) -> Vec<MarkdownInline> {
  match parse_markdown_with(source, MarkdownOptions::new().wiki_links(true))
    .blocks
    .into_iter()
    .next()
  {
    Some(MarkdownBlock::Paragraph(inlines)) => inlines,
    other => panic!("a paragraph: {other:?}"),
  }
}

#[test]
fn wiki_links_are_read_only_where_asked_for() {
  assert_eq!(
    wiki("See [[/roadmap]] and [[/team|the team]]."),
    vec![
      MarkdownInline::Text("See ".to_owned()),
      MarkdownInline::WikiLink {
        target: "/roadmap".to_owned(),
        words: Vec::new(),
      },
      MarkdownInline::Text(" and ".to_owned()),
      MarkdownInline::WikiLink {
        target: "/team".to_owned(),
        words: vec![MarkdownInline::Text("the team".to_owned())],
      },
      MarkdownInline::Text(".".to_owned()),
    ]
  );
  // Without the option a wiki link is text, as before.
  let plain = parse_markdown("See [[/roadmap]].");
  assert!(!plain.has_wiki_links());
  // In code it is code.
  assert_eq!(
    wiki("`[[/roadmap]]`"),
    vec![MarkdownInline::Code("[[/roadmap]]".to_owned())]
  );
}

#[test]
fn a_wiki_link_resolved_is_a_link_titled_by_the_app() {
  let mut document = parse_markdown_with(
    "- [[/roadmap]], [[/gone]] and [[/team|the team]]",
    MarkdownOptions::new().wiki_links(true),
  );
  assert!(document.has_wiki_links());
  document.resolve_wiki_links(&|target| (target == "/roadmap").then(|| "Roadmap 2027".to_owned()));
  assert!(!document.has_wiki_links());
  let MarkdownBlock::List { items, .. } = &document.blocks[0] else {
    panic!("a list");
  };
  let MarkdownBlock::Paragraph(inlines) = &items[0].blocks[0] else {
    panic!("a paragraph in the item");
  };
  let link = |target: &str, words: &str| MarkdownInline::Link {
    destination: target.to_owned(),
    title: String::new(),
    children: vec![MarkdownInline::Text(words.to_owned())],
  };
  assert_eq!(inlines[0], link("/roadmap", "Roadmap 2027"));
  // A target the app has no title for shows itself.
  assert_eq!(inlines[2], link("/gone", "/gone"));
  // Words given are kept.
  assert_eq!(inlines[4], link("/team", "the team"));
}

#[test]
fn a_document_with_wiki_links_draws_them_as_links_with_the_apps_titles() {
  let (_app, mut tree) = mount(
    MarkdownProps::new("See [[/roadmap]] and [[/team|the team]].")
      .wiki_links(|target| (target == "/roadmap").then(|| "Roadmap 2027".to_owned()))
      .width(500.0),
  );
  assert!(has_element_with_text(&mut tree, "Roadmap 2027"));
  assert!(has_element_with_text(&mut tree, "the team"));
  let (_app, mut tree) = mount(MarkdownProps::new("See [[/roadmap]].").width(500.0));
  assert!(!has_element_with_text(&mut tree, "Roadmap 2027"));
}

#[test]
fn the_parser_reads_a_tag() {
  assert_eq!(
    parse_markdown("A ==changed== range <tag>changed later</tag>.").blocks,
    vec![MarkdownBlock::Paragraph(vec![
      MarkdownInline::Text("A ==changed== range ".to_owned()),
      MarkdownInline::Tag("changed later".to_owned()),
      MarkdownInline::Text(".".to_owned()),
    ])]
  );
}

#[test]
fn a_tag_is_on_a_badges_box() {
  let props = MarkdownProps::new("The limit is 40 <tag>changed later</tag> now.").width(500.0);
  assert_eq!(props.rendered_text(), "The limit is 40 changed later now.");
  let (app, tree) = mount(props);
  let secondary = app.theme().palette().get(PaletteColor::Secondary);
  let boxes = quads(&tree)
    .into_iter()
    .filter(|quad| matches!(&quad.content, QuadContent::Rect { color, .. } if *color == secondary))
    .count();
  let rects: Vec<_> = quads(&tree)
    .into_iter()
    .filter_map(|quad| match quad.content {
      QuadContent::Rect { color, .. } => Some((color, quad.width, quad.height)),
      _ => None,
    })
    .collect();
  assert_eq!(boxes, 1, "the tag's box, of {secondary:?}: {rects:?}");
  let drawn = quads(&tree).into_iter().any(|quad| match quad.content {
    QuadContent::RichText { spans, .. } => spans.iter().any(|span| span.text == "changed later"),
    _ => false,
  });
  assert!(drawn, "its words are drawn");
}
