//! A link's words, trailing runs and shown address cannot be reordered by
//! directional controls (U+202A–U+202E, U+2066–U+2069) in or before them, and
//! the isolate marks that protect them are not drawn.

use std::sync::{Arc, Mutex};

use lurq::{
  app::Tree,
  components::{MarkdownLinkPresentation, MarkdownProps, MarkdownRun, Text},
  layout::text_style::TextStyle,
  markdown::{is_bidi_control, isolate_bidi},
};

use super::links::{colored, has_element_with_text, mount, quads, rich_text_quads, texts};
use crate::support::{GlyphSnapshot, render_pass, render_pass_with_app};

const HOST: &str = "github.com";
/// The host's color, pure green, so its glyphs can be told apart.
const HOST_COLOR: &str = "#00ff00";

fn is_host_glyph(glyph: &GlyphSnapshot) -> bool {
  glyph.color[1] > 0.9 && glyph.color[0] < 0.1 && glyph.color[2] < 0.1
}

/// The widths of `text`'s glyphs, left to right, drawn alone in the default
/// style: the order a reader sees.
fn reading_widths(text: &str) -> Vec<f32> {
  let mut tree = Tree::new();
  tree.set_root(Text::styled(text, TextStyle::default()));
  let mut glyphs = render_pass(&mut tree).glyphs;
  glyphs.sort_by(|a, b| a.x.total_cmp(&b.x));
  glyphs.iter().map(|glyph| glyph.width).collect()
}

/// The widths of the host's glyphs in the paragraph, left to right.
fn drawn_host_widths(source: &str) -> Vec<f32> {
  let (mut app, mut tree) = mount(
    MarkdownProps::new(source)
      .link_presentation(|link| {
        let host = link
          .destination()
          .trim_start_matches("https://")
          .split('/')
          .next()
          .unwrap_or_default()
          .to_owned();
        MarkdownLinkPresentation::link().trailing([MarkdownRun::new(format!(" {host}"), colored(HOST_COLOR))])
      })
      .width(800.0),
  );
  let mut glyphs = render_pass_with_app(&mut tree, &mut app)
    .glyphs
    .into_iter()
    .filter(is_host_glyph)
    .collect::<Vec<_>>();
  glyphs.sort_by(|a, b| a.x.total_cmp(&b.x));
  glyphs.iter().map(|glyph| glyph.width).collect()
}

/// Whether `drawn` widths run in `expected`'s order rather than backwards.
/// A glyph bitmap's width differs by a pixel with its subpixel position, so
/// the orders are matched by distance.
fn reads_forward(drawn: &[f32], expected: &[f32]) -> bool {
  let distance = |order: &[f32]| -> f32 { drawn.iter().zip(order).map(|(drawn, want)| (drawn - want).abs()).sum() };
  let backwards = expected.iter().rev().copied().collect::<Vec<_>>();
  drawn.len() == expected.len() && distance(expected) < distance(&backwards)
}

#[test]
fn markdown_link_host_reads_forward_after_an_unclosed_override_before_the_link() {
  let expected = reading_widths(HOST);
  let backwards = expected.iter().rev().copied().collect::<Vec<_>>();
  assert!(
    !reads_forward(&backwards, &expected),
    "the measured host reads differently backwards"
  );
  // The measured case: an unclosed RIGHT-TO-LEFT OVERRIDE earlier in the
  // paragraph drew the host as `moc.buhtig`.
  let drawn = drawn_host_widths("Merged \u{202E}see [PR 42](https://github.com/acme/pull/42) today.");
  assert!(reads_forward(&drawn, &expected), "{drawn:?} vs {expected:?}");
}

#[test]
fn markdown_link_host_reads_forward_after_an_override_in_the_words() {
  let expected = reading_widths(HOST);
  // As a character and as an entity reference.
  for source in [
    "[PR \u{202E}42](https://github.com/acme/pull/42)",
    "[PR &#x202E;42](https://github.com/acme/pull/42)",
  ] {
    let drawn = drawn_host_widths(source);
    assert!(reads_forward(&drawn, &expected), "{source}: {drawn:?} vs {expected:?}");
  }
}

#[test]
fn markdown_link_words_runs_and_address_drop_directional_controls() {
  let (_app, mut tree) = mount(
    MarkdownProps::new("A [w\u{202E}or\u{2066}ds](https://github.com/x) and [mail](mailto:a\u{202E}b@c.d) end.")
      .link_presentation(|link| {
        if link.destination().starts_with("mailto:") {
          MarkdownLinkPresentation::text().trailing([MarkdownRun::plain(format!(" ({})", link.destination()))])
        } else {
          MarkdownLinkPresentation::link().trailing([MarkdownRun::plain(" gith\u{202D}ub.com")])
        }
      })
      .width(800.0),
  );

  let quads = quads(&tree);
  let spans = rich_text_quads(&quads)[0];
  let shown = texts(spans);
  assert_eq!(
    shown,
    vec![
      "A ",
      "words",
      " github.com",
      " and ",
      "mail",
      " (mailto:ab@c.d)",
      " end."
    ]
  );
  assert!(shown.iter().all(|text| !text.chars().any(is_bidi_control)), "{shown:?}");
  // Each piece sits between its own isolate marks.
  let all = spans.iter().map(|span| span.text.as_str()).collect::<Vec<_>>();
  for piece in ["words", " github.com", "mail", " (mailto:ab@c.d)"] {
    let at = all.iter().position(|text| *text == piece).unwrap();
    assert_eq!((all[at - 1], all[at + 1]), ("\u{2068}", "\u{2069}"), "{piece}");
  }
  assert!(
    has_element_with_text(&mut tree, "words"),
    "the link is named by its words, without controls"
  );
}

#[test]
fn markdown_isolate_marks_are_not_drawn() {
  let (mut app, mut tree) = mount(MarkdownProps::new("See [docs](/docs).").width(400.0));
  let glyphs = render_pass_with_app(&mut tree, &mut app).glyphs;
  assert_eq!(glyphs.len(), "Seedocs.".chars().count(), "only the words' glyphs");
  // And the public helper's marks take no room either.
  assert_eq!(reading_widths(&isolate_bidi("doc\u{202E}s")), reading_widths("docs"));
}

#[test]
fn markdown_link_name_is_the_drawn_words_and_email_autolinks_are_mailto() {
  let presented = Arc::new(Mutex::new(Vec::new()));
  let (_app, mut tree) = mount(
    MarkdownProps::new("Read [**a**&amp;<b>b</b>](/x) or write <team@example.com>.")
      .link_presentation({
        let presented = presented.clone();
        move |link| {
          presented
            .lock()
            .unwrap()
            .push((link.destination().to_owned(), link.text().to_owned()));
          MarkdownLinkPresentation::link()
        }
      })
      .width(600.0),
  );
  assert_eq!(
    *presented.lock().unwrap(),
    vec![
      ("/x".to_owned(), "a&b".to_owned()),
      ("mailto:team@example.com".to_owned(), "team@example.com".to_owned())
    ]
  );
  assert!(
    has_element_with_text(&mut tree, "a&b"),
    "the link node is named by its drawn words"
  );
}

#[test]
fn markdown_isolate_bidi_strips_and_wraps() {
  assert_eq!(isolate_bidi("a\u{202E}b\u{2069}c"), "\u{2068}abc\u{2069}");
}
