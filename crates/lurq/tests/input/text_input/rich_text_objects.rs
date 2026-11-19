//! A picture in a document is one character of its text and a record beside it: read from a Markdown image and
//! written back as one that reads as the same document.
use super::*;
use crate::components::styled_text::{ParagraphKind::*, SpanStyle};

const O: char = OBJECT_CHARACTER;

fn object(at: usize, source: &str) -> InlineObject {
  InlineObject::new(at, source)
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
fn a_picture_alone_in_its_paragraph_is_a_paragraph_of_one_character() {
  let document = read("Before.\n\n![The lake at dusk](photos/lake.jpg)\n\nAfter.");
  assert_eq!(document.text, format!("Before.\n{O}\nAfter."));
  assert_eq!(document.paragraphs, [Body, Body, Body]);
  assert_eq!(
    document.objects,
    [object(8, "photos/lake.jpg").label("The lake at dusk")]
  );
  assert!(document.spans.is_empty(), "a picture is not a link to its source");
  assert_eq!(
    document.to_markdown(),
    "Before.\n\n![The lake at dusk](photos/lake.jpg)\n\nAfter."
  );
}

#[test]
fn a_picture_among_words_stays_among_them() {
  let document = read("The lake ![at dusk](lake.jpg) and the hill ![](hill.png).");
  assert_eq!(document.text, format!("The lake {O} and the hill {O}."));
  assert_eq!(
    document.objects,
    [object(9, "lake.jpg").label("at dusk"), object(26, "hill.png")]
  );
  assert_eq!(
    document.to_markdown(),
    "The lake ![at dusk](lake.jpg) and the hill ![](hill.png)."
  );
}

#[test]
fn a_picture_with_a_width_is_written_as_a_tag_and_read_from_one() {
  let document = read("See <img src=\"lake.jpg\" alt=\"The lake\" width=\"320\"> here.");
  assert_eq!(document.text, format!("See {O} here."));
  assert_eq!(document.objects, [object(4, "lake.jpg").label("The lake").width(320)]);
  assert_eq!(
    document.to_markdown(),
    "See <img src=\"lake.jpg\" alt=\"The lake\" width=\"320\"> here."
  );
}

#[test]
fn a_picture_with_a_width_alone_on_its_line_is_a_paragraph_of_it() {
  // Markdown reads a tag alone on its line as a block of HTML.
  let document = read("Before.\n\n<img src=\"lake.jpg\" alt=\"\" width=\"480\">\n\n- <img src='hill.png' width=64 />");
  assert_eq!(document.text, format!("Before.\n{O}\n{O}"));
  assert_eq!(document.paragraphs, [Body, Body, Bullet]);
  assert_eq!(
    document.objects,
    [object(8, "lake.jpg").width(480), object(12, "hill.png").width(64)]
  );
}

#[test]
fn a_tag_that_is_no_image_and_an_image_with_no_source_are_left_out() {
  let document = RichText::from_markdown("a <span>b</span> <img alt=\"nothing\"> c");
  assert_eq!(document.text, "a b  c");
  assert!(document.objects.is_empty());
}

#[test]
fn a_picture_keeps_the_marks_and_the_link_it_stands_in() {
  let document = read("**bold ![x](a.png) words** and [![logo](logo.svg)](https://lurq.dev)");
  assert_eq!(document.text, format!("bold {O} words and {O}"));
  assert_eq!(
    document.objects,
    [object(5, "a.png").label("x"), object(19, "logo.svg").label("logo")]
  );
  assert_eq!(
    document.spans,
    [
      StyledSpan::new(0..14, SpanStyle::new().bold()),
      StyledSpan::new(19..22, SpanStyle::new().link("https://lurq.dev")),
    ]
  );
  assert_eq!(
    document.to_markdown(),
    "**bold ![x](a.png) words** and [![logo](logo.svg)](https://lurq.dev)"
  );
}

#[test]
fn the_kind_of_a_paragraph_holds_for_its_pictures() {
  let document = read("## Title ![mark](m.png)\n\n- ![one](1.png) first\n- second\n\n> ![q](q.png)");
  assert_eq!(document.paragraphs, [Heading(2), Bullet, Bullet, Quote]);
  assert_eq!(document.objects.len(), 3);
  assert_eq!(
    document.to_markdown(),
    "## Title ![mark](m.png)\n\n- ![one](1.png) first\n- second\n\n> ![q](q.png)"
  );
}

#[test]
fn a_source_and_a_label_with_what_markdown_would_read_are_written_so_that_they_read_back() {
  for (source, label) in [
    ("photos/my lake (1).jpg", "a [bracket] and a *star*"),
    ("C:\\photos\\_lake_.jpg", "back\\slash"),
    ("https://example.com/a?b=1&copy;=2&c=<3>", "\"quoted\" & <angled>"),
    ("", ""),
    ("data:image/png;base64,AAAA", "!"),
  ] {
    for width in [None, Some(200)] {
      let text = format!("a {O} b");
      let mut picture = object(2, source).label(label);
      picture.width = width;
      let document = RichText::plain(text).with_objects(vec![picture]);
      let written = document.to_markdown();
      assert_eq!(RichText::from_markdown(&written), document, "{written:?}");
    }
  }
}

#[test]
fn an_exclamation_mark_before_a_link_does_not_make_a_picture_of_it() {
  let document = RichText::new(
    "wow!docs",
    vec![StyledSpan::new(4..8, SpanStyle::new().link("https://lurq.dev"))],
    vec![],
  );
  let written = document.to_markdown();
  assert_eq!(written, "wow\\![docs](https://lurq.dev)");
  assert_eq!(RichText::from_markdown(&written), document);
}

#[test]
fn a_picture_in_code_is_written_as_its_label() {
  // Markdown reads no image in code: what is written says what the picture was.
  let text = format!("let a = {O};\nrun {O} now");
  let document = RichText::new(
    text,
    vec![StyledSpan::new(17..20, SpanStyle::new().code())],
    vec![Code, Body],
  )
  .with_objects(vec![object(8, "a.png").label("one"), object(17, "b.png").label("two")]);
  assert_eq!(document.to_markdown(), "```\nlet a = one;\n```\n\nrun `two` now");
}

#[test]
fn an_object_character_with_no_record_is_a_character() {
  let document = read(&format!("a {O} b"));
  assert_eq!(document.text, format!("a {O} b"));
  assert!(document.objects.is_empty());
  assert_eq!(document.to_markdown(), format!("a {O} b"));
  // A record that points at no object character is dropped.
  assert!(
    RichText::plain("abc")
      .with_objects(vec![object(1, "a.png")])
      .objects
      .is_empty()
  );
}

#[test]
fn plain_text_and_the_count_of_words_leave_the_picture_out() {
  let document = RichText::from_markdown("one ![the lake](lake.jpg) two![](x.png)three");
  assert_eq!(document.plain_text(), "one the lake twothree");
  assert_eq!(document.word_count(), 3, "a picture is no word and stands between two");
  let blocks = document.blocks();
  assert_eq!(blocks[0].1.text, format!("one {O} two{O}three"));
  // A document of a picture alone has no words.
  assert_eq!(RichText::from_markdown("![x](y.png)").word_count(), 0);
}

#[test]
fn a_text_with_an_image_reads_as_markdown() {
  assert!(RichText::reads_as_markdown("see ![the lake](lake.jpg)"));
  assert!(RichText::reads_as_markdown("<img src=\"lake.jpg\" width=\"200\">"));
  assert!(!RichText::reads_as_markdown("wow! [not a link]"));
}
