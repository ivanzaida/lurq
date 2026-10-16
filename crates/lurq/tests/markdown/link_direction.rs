//! A link's trailing runs follow its words, left to right, whatever direction
//! its paragraph takes: neither a directional mark (U+200E, U+200F, U+061C)
//! nor a right-to-left paragraph can draw a host on the other side of the
//! words, and a right-to-left paragraph still reads right to left around the
//! link.

use lurq::components::{MarkdownLinkPresentation, MarkdownProps, MarkdownRun};

use super::links::{colored, mount};
use crate::support::{GlyphSnapshot, render_pass_with_app};

/// What a drawn glyph belongs to, told apart by its color.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Piece {
  /// A link's words, pure red.
  Words,
  /// The trailing host of a link to `evil.example`, pure green.
  Host,
  /// The trailing host of any other link, pure blue.
  OtherHost,
  /// The paragraph's own text.
  Text,
}

fn piece(glyph: &GlyphSnapshot) -> Piece {
  match glyph.color.map(|channel| channel > 0.9) {
    [true, false, false, _] => Piece::Words,
    [false, true, false, _] => Piece::Host,
    [false, false, true, _] => Piece::OtherHost,
    _ => Piece::Text,
  }
}

/// The pieces of `source` as drawn, left to right, with each run of glyphs
/// of one piece and its glyph count.
fn drawn_pieces(source: &str) -> Vec<(Piece, usize)> {
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
        let host_color = if host == "evil.example" { "#00ff00" } else { "#0000ff" };
        MarkdownLinkPresentation::link().style(colored("#ff0000")).trailing([
          MarkdownRun::new(" ", colored(host_color)),
          MarkdownRun::new(host, colored(host_color)),
        ])
      })
      .width(1600.0),
  );
  let mut glyphs = render_pass_with_app(&mut tree, &mut app).glyphs;
  glyphs.sort_by(|a, b| a.x.total_cmp(&b.x));
  let mut pieces: Vec<(Piece, usize)> = Vec::new();
  for glyph in &glyphs {
    match pieces.last_mut() {
      Some((last, count)) if *last == piece(glyph) => *count += 1,
      _ => pieces.push((piece(glyph), 1)),
    }
  }
  pieces
}

fn kinds(pieces: &[(Piece, usize)]) -> Vec<Piece> {
  pieces.iter().map(|(piece, _)| *piece).collect()
}

/// Whether the host glyphs come right after the words, to their right.
fn host_follows_words(pieces: &[(Piece, usize)]) -> bool {
  kinds(pieces).windows(2).any(|pair| pair == [Piece::Words, Piece::Host])
    && !kinds(pieces).windows(2).any(|pair| pair == [Piece::Host, Piece::Words])
}

/// Asserts that each of `sources` draws its host right after its words,
/// naming every source that does not.
fn assert_hosts_follow_words(sources: &[&str]) {
  let reversed = sources
    .iter()
    .map(|source| (*source, drawn_pieces(source)))
    .filter(|(_, drawn)| !host_follows_words(drawn))
    .collect::<Vec<_>>();
  assert!(reversed.is_empty(), "{reversed:?}");
}

#[test]
fn markdown_link_host_follows_its_words_after_a_right_to_left_mark_and_a_forged_host() {
  // The measured case: drew `evil.example PR 412 github.com`, a forged host
  // right after the words and the real one on their other side.
  let drawn = drawn_pieces(
    "\u{200F}\u{200E}[](https://github.com/acme/app/pull/412)\u{200E}[PR 412](https://evil.example/pull/412)",
  );
  assert_eq!(
    kinds(&drawn),
    vec![Piece::OtherHost, Piece::Words, Piece::Host],
    "{drawn:?}"
  );
}

#[test]
fn markdown_link_host_follows_its_words_in_a_paragraph_that_starts_right_to_left() {
  assert_hosts_follow_words(&[
    "\u{061C}Pull request: [PR 412](https://evil.example/pull/412)",
    "\u{200F}Pull request: [PR 412](https://evil.example/pull/412)",
    "\u{05E9}\u{05DC}\u{05D5}\u{05DD} [PR 412](https://evil.example/pull/412)",
    "\u{0645}\u{0631}\u{062D}\u{0628}\u{0627} [PR 412](https://evil.example/pull/412)",
  ]);
}

#[test]
fn markdown_link_host_follows_right_to_left_words() {
  assert_hosts_follow_words(&[
    "See [\u{05E9}\u{05DC}\u{05D5}\u{05DD}](https://evil.example/pull/412) now",
    "\u{05D0}\u{05D1}\u{05D2} [\u{05E9}\u{05DC}\u{05D5}\u{05DD}](https://evil.example/pull/412) \u{05D3}\u{05D4}",
  ]);
}

#[test]
fn markdown_right_to_left_paragraph_reads_right_to_left_around_a_link() {
  // Three letters, the link, then six letters: right to left, the three
  // are drawn right of the link and the six left of it.
  let drawn = drawn_pieces(
    "\u{05D0}\u{05D1}\u{05D2} [PR 412](https://evil.example/pull/412) \u{05D3}\u{05D4}\u{05D5}\u{05D6}\u{05D7}\u{05D8}",
  );
  assert_eq!(
    kinds(&drawn),
    vec![Piece::Text, Piece::Words, Piece::Host, Piece::Text],
    "{drawn:?}"
  );
  assert_eq!((drawn[0].1, drawn[3].1), (6, 3), "{drawn:?}");
}

#[test]
fn markdown_left_to_right_paragraph_with_a_link_reads_left_to_right() {
  let drawn = drawn_pieces("Merged [PR 412](https://evil.example/pull/412) today");
  assert_eq!(
    kinds(&drawn),
    vec![Piece::Text, Piece::Words, Piece::Host, Piece::Text],
    "{drawn:?}"
  );
  assert_eq!((drawn[0].1, drawn[3].1), (6, 5), "{drawn:?}");
}
