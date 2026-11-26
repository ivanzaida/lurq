//! Wiki links in a document: read from Markdown as links that remember they are wiki links, shown with the title
//! the app gives their target, and written back as the wiki links they were.
use super::*;
use crate::components::styled_text::{SpanStyle, WikiLink, ops};

/// The titles of the app's pages.
fn title(target: &str) -> Option<String> {
  match target {
    "/roadmap" => Some("Roadmap 2027".to_string()),
    "/onboarding" => Some("Onboarding".to_string()),
    _ => None,
  }
}

fn read(source: &str) -> RichText {
  RichText::from_markdown_with_wiki_links(source, title)
}

/// The style of the link that covers byte `at`.
fn style_at(document: &RichText, at: usize) -> SpanStyle {
  document.styled().style_at(at)
}

#[test]
fn a_wiki_link_without_words_shows_its_title_and_is_written_back_as_its_target() {
  let page = read("See [[/roadmap]] now.");
  assert_eq!(page.text, "See Roadmap 2027 now.");
  let style = style_at(&page, 4);
  assert_eq!(style.link.as_deref(), Some("/roadmap"));
  assert_eq!(style.wiki, Some(WikiLink::Titled("Roadmap 2027".into())));
  assert_eq!(page.spans.len(), 1);
  assert_eq!(page.spans[0].range, 4..16);
  assert_eq!(page.to_markdown(), "See [[/roadmap]] now.");
}

#[test]
fn a_wiki_link_with_words_shows_them_and_is_written_back_with_them() {
  let page = read("Read [[/onboarding|the first steps]] and [[/roadmap|**the** plan]].");
  assert_eq!(page.text, "Read the first steps and the plan.");
  assert_eq!(style_at(&page, 5).wiki, Some(WikiLink::Worded));
  assert_eq!(style_at(&page, 5).link.as_deref(), Some("/onboarding"));
  // Marks in the words are kept inside the link.
  let bold = style_at(&page, page.text.find("the plan").unwrap());
  assert!(bold.bold);
  assert_eq!(bold.link.as_deref(), Some("/roadmap"));
  assert_eq!(
    page.to_markdown(),
    "Read [[/onboarding|the first steps]] and [[/roadmap|**the** plan]]."
  );
}

#[test]
fn a_target_with_no_title_shows_the_target_and_an_empty_title_counts_as_none() {
  let page = read("A [[/unknown]] page.");
  assert_eq!(page.text, "A /unknown page.");
  assert_eq!(page.to_markdown(), "A [[/unknown]] page.");
  let blank = RichText::from_markdown_with_wiki_links("[[/x]]", |_| Some("  ".to_string()));
  assert_eq!(blank.text, "/x");
}

#[test]
fn the_round_trip_reads_back_the_same_document() {
  for source in [
    "## [[/roadmap]]\n\n- an item to [[/onboarding|start]]\n- [[/unknown]]",
    "Two in a row: [[/roadmap]][[/onboarding]].",
    "Wiki and plain: [[/roadmap]] and [the docs](https://example.com).",
    "Words with marks: [[/roadmap|a *b* \\] c]] done",
  ] {
    let document = read(source);
    let written = document.to_markdown();
    assert_eq!(read(&written), document, "{source:?} was written {written:?}");
  }
}

#[test]
fn a_title_changed_by_a_person_is_written_as_the_words_it_is() {
  let page = read("See [[/roadmap]].");
  // The person types inside the link: its words are no longer the title.
  let at = page.text.find("2027").unwrap();
  let mut text = page.text.clone();
  text.insert_str(at, "for ");
  let spans = ops::splice(&page.spans, at..at, 4, &style_at(&page, at));
  let changed = RichText::new(text, spans, page.paragraphs.clone());
  assert_eq!(changed.text, "See Roadmap for 2027.");
  assert_eq!(changed.to_markdown(), "See [[/roadmap|Roadmap for 2027]].");
}

#[test]
fn a_link_given_another_target_or_taken_away_is_a_wiki_link_no_more() {
  let page = read("See [[/roadmap]].");
  let range = page.spans[0].range.clone();
  let retargeted = ops::restyle(&page.spans, page.text.len(), range.clone(), |style| {
    style.link = Some("https://example.com".into())
  });
  assert_eq!(retargeted[0].style.wiki, None);
  let document = RichText::new(page.text.clone(), retargeted, Vec::new());
  assert_eq!(document.to_markdown(), "See [Roadmap 2027](https://example.com).");
  let unlinked = ops::restyle(&page.spans, page.text.len(), range, |style| style.link = None);
  assert!(unlinked.is_empty(), "nothing is left of it: {unlinked:?}");
}

#[test]
fn text_typed_after_a_wiki_link_is_plain() {
  let page = read("See [[/roadmap]]");
  let end = page.text.len();
  assert!(ops::insertion_style(&page.text, &page.spans, end, end).is_plain());
}

#[test]
fn a_target_markdown_cannot_hold_in_a_wiki_link_is_written_as_a_plain_link() {
  let style = SpanStyle::new().wiki_link("a|b", WikiLink::Worded);
  let document = RichText::new("words", vec![StyledSpan::new(0..5, style)], Vec::new());
  assert_eq!(document.to_markdown(), "[words](a|b)");
}

#[test]
fn a_bang_before_a_wiki_link_does_not_make_it_a_picture() {
  let page = read("Wow\\![[/roadmap]]");
  assert_eq!(page.text, "Wow!Roadmap 2027");
  let written = page.to_markdown();
  assert_eq!(written, "Wow\\![[/roadmap]]");
  assert_eq!(read(&written), page);
}

#[test]
fn without_wiki_links_brackets_are_text_and_a_wiki_link_is_written_as_one() {
  let plain = RichText::from_markdown("See [[/roadmap]].");
  assert_eq!(plain.text, "See [[/roadmap]].");
  assert!(plain.spans.is_empty());
  assert!(!plain.has_wiki_links());
  assert!(read("See [[/roadmap]].").has_wiki_links());
}
