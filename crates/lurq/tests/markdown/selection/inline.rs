//! Paragraphs whose spans differ: links in the app and on the web with the
//! host drawn after them, emphasis and strong text, and inline code at the
//! start, in the middle and at the end.

use lurq::{
  app::theme::MarkdownInlineStyle,
  components::{MarkdownLinkPresentation, MarkdownProps, MarkdownRun},
  node::color::Color,
};

use super::page;
use crate::support::selection::{Page, assert_drags_copy_their_glyphs, pairs};

/// The color of a link's trailing runs, which are presentation: the page
/// leaves their glyphs out.
const HOST: &str = "#00ff00";

const LINKS: &str = "Open [the task](/tasks/42) in the app, or [PR 42](https://github.com/acme/pull/42) on the web, \
then [mail the team](mailto:team@example.com) and read [the docs](https://docs.rs/lurq) when the line has wrapped.";
const EMPHASIS: &str = "Some *emphasized words* and **strong ones** and ***both at once*** with ~~struck~~ text in a \
paragraph long enough to wrap at the width of the test.";
const CODE_START: &str = "`fn main()` starts this paragraph, and it goes on with plain words until it wraps at the \
width of the test.";
const CODE_MIDDLE: &str = "This paragraph has `fn main()` in the middle of it, and it goes on with plain words until it \
wraps at the width.";
const CODE_END: &str = "This paragraph goes on with plain words until it wraps at the width of the test and ends with \
`fn main()`";
const CODE_ONLY: &str = "`a paragraph that is one code span and nothing else, long enough to wrap`";

fn with_links(source: &str, width: f32) -> Page {
  let host = MarkdownInlineStyle {
    text: lurq::app::theme::MarkdownTextStyle {
      color: Some(Color::from_hex(HOST)),
      ..Default::default()
    },
  };
  let props = MarkdownProps::new(source).link_presentation(move |link| {
    let destination = link.destination();
    if destination.starts_with("mailto:") {
      MarkdownLinkPresentation::text().trailing([MarkdownRun::new(
        format!(" ({})", destination.trim_start_matches("mailto:")),
        host.clone(),
      )])
    } else if let Some(rest) = destination.strip_prefix("https://") {
      let host_name = rest.split('/').next().unwrap_or_default().to_owned();
      MarkdownLinkPresentation::link().trailing([MarkdownRun::new(format!(" {host_name}"), host.clone())])
    } else {
      MarkdownLinkPresentation::link()
    }
  });
  page(props, width).ignore_color(Color::from_hex(HOST))
}

fn sweep(mut page: Page, step: usize) -> Page {
  assert!(page.inks.iter().any(|ink| ink.row > 0), "the paragraph wraps");
  let count = page.inks.len();
  assert_drags_copy_their_glyphs(&mut page, pairs(count, step));
  for (from, to) in pairs(count, step * 3).chain([(0, count - 1)]) {
    page.drag_inks(from, to);
    page.assert_highlight_covers(from, to);
  }
  page
}

#[test]
fn every_drag_over_a_paragraph_with_links_copies_the_glyphs_it_covers_without_the_hosts() {
  let mut page = sweep(with_links(LINKS, 420.0), 5);
  let (from, to) = page.inks_of("PR 42", 0);
  page.drag_inks(from, to + 2);
  assert_eq!(
    page.selected().as_deref(),
    Some("PR 42 on"),
    "the host after the link is not copied"
  );
  let (from, _) = page.inks_of("mail", 0);
  let (_, to) = page.inks_of("read", 0);
  page.drag_inks(from, to);
  assert_eq!(page.selected().as_deref(), Some("mail the team and read"));
}

#[test]
fn every_drag_over_emphasis_copies_the_glyphs_it_covers() {
  sweep(page(MarkdownProps::new(EMPHASIS), 360.0), 5);
}

#[test]
fn every_drag_over_inline_code_at_the_start_copies_the_glyphs_it_covers() {
  sweep(page(MarkdownProps::new(CODE_START), 300.0), 5);
}

#[test]
fn every_drag_over_inline_code_in_the_middle_copies_the_glyphs_it_covers() {
  sweep(page(MarkdownProps::new(CODE_MIDDLE), 300.0), 5);
}

#[test]
fn every_drag_over_inline_code_at_the_end_copies_the_glyphs_it_covers() {
  sweep(page(MarkdownProps::new(CODE_END), 300.0), 5);
}

#[test]
fn every_drag_over_a_paragraph_that_is_one_code_span_copies_the_glyphs_it_covers() {
  sweep(page(MarkdownProps::new(CODE_ONLY), 300.0), 5);
}
