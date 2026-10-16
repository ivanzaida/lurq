//! A link's trailing runs follow its words, left to right, whatever direction
//! its paragraph takes: neither a directional mark (U+200E, U+200F, U+061C)
//! nor a right-to-left paragraph can draw a host on the other side of the
//! words, and a right-to-left paragraph still reads right to left around the
//! link.

use std::sync::{Arc, Mutex};

use lurq::{
  components::{MarkdownLinkPresentation, MarkdownProps, MarkdownRun},
  node::color::Color,
};

use super::links::{MarkdownRoot, colored, mount};
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
  // right after the words and the real one on their other side. A link
  // with no words draws nothing, not even its host.
  let drawn = drawn_pieces(
    "\u{200F}\u{200E}[](https://github.com/acme/app/pull/412)\u{200E}[PR 412](https://evil.example/pull/412)",
  );
  assert_eq!(kinds(&drawn), vec![Piece::Words, Piece::Host], "{drawn:?}");
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

/// The words and the trailing runs of link `index`, or the paragraph's own
/// text, told apart by color: words are red and runs green, with the link's
/// index in the blue channel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Ink {
  Words(usize),
  Run(usize),
  Text,
}

const LINK_BLUES: [&str; 5] = ["00", "40", "80", "c0", "ff"];

fn words_color(index: usize) -> String {
  format!("#ff00{}", LINK_BLUES[index])
}

fn run_color(index: usize) -> String {
  format!("#00ff{}", LINK_BLUES[index])
}

fn ink(glyph: &GlyphSnapshot) -> Ink {
  let [red, green, blue, _] = glyph.color;
  let index = || {
    (0..LINK_BLUES.len())
      .min_by(|a, b| {
        let distance = |index: usize| (Color::from_hex(&words_color(index)).to_linear_f32_array()[2] - blue).abs();
        distance(*a).total_cmp(&distance(*b))
      })
      .unwrap_or_default()
  };
  match (red > 0.9 && green < 0.1, green > 0.9 && red < 0.1) {
    (true, _) => Ink::Words(index()),
    (_, true) => Ink::Run(index()),
    _ => Ink::Text,
  }
}

/// `source` at `width`, its links showing their words, then a space, their
/// host, a space and an arrow, as an app does.
fn sweep_props(source: &str, width: f32) -> MarkdownProps {
  MarkdownProps::new(source)
    .link_presentation(|link| {
      let host = link
        .destination()
        .trim_start_matches("https://")
        .split('/')
        .next()
        .unwrap_or_default()
        .to_owned();
      let run = colored(&run_color(link.index()));
      MarkdownLinkPresentation::link()
        .style(colored(&words_color(link.index())))
        .trailing([
          MarkdownRun::new(" ", run.clone()),
          MarkdownRun::new(host, run.clone()),
          MarkdownRun::new(" ", run.clone()),
          MarkdownRun::new("\u{2197}", run),
        ])
    })
    .width(width)
}

/// Each laid-out line of `glyphs`, left to right, as runs of glyphs of one
/// ink.
fn lines(mut glyphs: Vec<GlyphSnapshot>) -> Vec<Vec<Ink>> {
  let center = |glyph: &GlyphSnapshot| glyph.y + glyph.height / 2.0;
  glyphs.sort_by(|a, b| center(a).total_cmp(&center(b)));
  let mut lines: Vec<Vec<GlyphSnapshot>> = Vec::new();
  let mut last_center = f32::NEG_INFINITY;
  for glyph in glyphs {
    if center(&glyph) - last_center > 7.0 {
      lines.push(Vec::new());
    }
    last_center = center(&glyph);
    lines.last_mut().expect("a line").push(glyph);
  }
  lines
    .into_iter()
    .map(|mut line| {
      line.sort_by(|a, b| a.x.total_cmp(&b.x));
      let mut inks: Vec<Ink> = line.iter().map(ink).collect();
      inks.dedup();
      inks
    })
    .collect()
}

/// Whether `line` puts anything but a link's own trailing runs right after
/// its words.
fn misattributes(line: &[Ink]) -> bool {
  line.windows(2).any(|pair| match pair {
    [Ink::Words(link), next] => *next != Ink::Run(*link),
    _ => false,
  })
}

/// Asserts, at every width from 60 to 1600 px in steps of 2, that no line of
/// any of `corpus` puts anything but a link's own trailing runs right after
/// its words.
fn assert_runs_follow_words_at_every_width(corpus: &[(&str, &str)]) {
  let mut failures = Vec::new();
  for (label, source) in corpus {
    let (mut app, mut tree) = mount(sweep_props(source, 60.0));
    for width in (60..=1600).step_by(2) {
      tree.update_root_props::<MarkdownRoot>(sweep_props(source, width as f32));
      let wrong = lines(render_pass_with_app(&mut tree, &mut app).glyphs)
        .into_iter()
        .filter(|line| misattributes(line))
        .collect::<Vec<_>>();
      if !wrong.is_empty() {
        failures.push(format!("{label} at {width}px: {wrong:?}"));
      }
    }
  }
  assert!(
    failures.is_empty(),
    "{} failures, first: {:#?}",
    failures.len(),
    &failures[..failures.len().min(8)]
  );
}

const SOF_PASUQ: &str = "\u{05C3}";
const SHALOM: &str = "\u{05E9}\u{05DC}\u{05D5}\u{05DD}";
const OLAM: &str = "\u{05E2}\u{05D5}\u{05DC}\u{05DD}";
const MARHABA: &str = "\u{0645}\u{0631}\u{062D}\u{0628}\u{0627}";
const ALAM: &str = "\u{0639}\u{0627}\u{0644}\u{0645}";
const REAL: &str = "https://github.com/acme/app/pull/412";
const EVIL: &str = "https://evil.example/pull/412";

#[test]
fn markdown_link_runs_follow_words_at_every_width_after_a_forged_host() {
  let empty = format!("[]({REAL})");
  let invisible = format!("[\u{200B}]({REAL})");
  assert_runs_follow_words_at_every_width(&[
    ("soft wrap", &format!("{SOF_PASUQ} {empty}[PR 412]({EVIL})")),
    ("hebrew first", &format!("{SHALOM} {empty}[PR 412]({EVIL})")),
    ("zwsp words", &format!("{SOF_PASUQ} {invisible}[PR 412]({EVIL})")),
    (
      "zwsp words, hebrew first",
      &format!("{SHALOM} {invisible}[PR 412]({EVIL})"),
    ),
    (
      "joiner words",
      &format!("{SOF_PASUQ} [\u{2060}\u{FEFF}\u{AD}]({REAL})[PR 412]({EVIL})"),
    ),
    ("plain host", &format!("{SOF_PASUQ} github.com [PR 412]({EVIL})")),
    ("dot words", &format!("{SHALOM} [.]({REAL})[PR 412]({EVIL})")),
  ]);
}

#[test]
fn markdown_link_runs_follow_words_at_every_width_with_breaks_in_the_words() {
  let empty = format!("[]({REAL})");
  assert_runs_follow_words_at_every_width(&[
    ("backslash break", &format!("{SOF_PASUQ} {empty}[PR 412\\nx]({EVIL})")),
    ("two-space break", &format!("{SOF_PASUQ} {empty}[PR 412  \nx]({EVIL})")),
    ("U+2029", &format!("{SOF_PASUQ} {empty}[PR 412\u{2029}x]({EVIL})")),
    ("U+0085", &format!("{SOF_PASUQ} {empty}[PR 412\u{85}x]({EVIL})")),
    ("U+2028", &format!("{SOF_PASUQ} {empty}[PR 412\u{2028}x]({EVIL})")),
    ("U+001E", &format!("{SOF_PASUQ} {empty}[PR 412\u{1E}x]({EVIL})")),
    ("html break", &format!("{SOF_PASUQ} {empty}[PR 412<br>x]({EVIL})")),
    ("break before", &format!("{SHALOM}\\n{empty}[PR 412]({EVIL}) {OLAM}")),
  ]);
}

#[test]
fn markdown_link_runs_follow_words_at_every_width_in_blocks() {
  let empty = format!("[]({REAL})");
  assert_runs_follow_words_at_every_width(&[
    ("heading", &format!("# {SOF_PASUQ} {empty}[PR 412\u{2029}x]({EVIL})")),
    ("list item", &format!("- {SOF_PASUQ} {empty}[PR 412\\nx]({EVIL})")),
    ("quote", &format!("> {SOF_PASUQ} {empty}[PR 412\\nx]({EVIL})")),
    (
      "table cell",
      &format!("| a |\n|---|\n| {SOF_PASUQ} {empty}[PR 412<br>x]({EVIL}) |"),
    ),
    ("emphasis", &format!("{SOF_PASUQ} *{empty}*[**PR 412\\nx**]({EVIL})")),
  ]);
}

#[test]
fn markdown_link_runs_follow_words_at_every_width_with_several_links() {
  assert_runs_follow_words_at_every_width(&[
    (
      "hebrew paragraph",
      &format!("{SHALOM} [docs]({REAL}) {OLAM} [PR 412]({EVIL}) {SHALOM} [{OLAM}]({REAL}) {OLAM}"),
    ),
    (
      "arabic paragraph",
      &format!("{MARHABA} [docs]({REAL}) {ALAM} [PR 412 and more words]({EVIL}) {MARHABA}"),
    ),
    (
      "hebrew phrase in english",
      &format!("Merged {SHALOM} [a]({REAL}) [PR 412 with a long title]({EVIL}) {OLAM} today"),
    ),
    (
      "english paragraph",
      &format!("See [docs]({REAL}) and [PR 412]({EVIL}) and [{SHALOM} {OLAM}]({REAL}), then [x]({EVIL})."),
    ),
    (
      "long words",
      &format!("{SOF_PASUQ} [a]({REAL}) [PR 412 with a very long title that wraps over lines]({EVIL})"),
    ),
  ]);
}

#[test]
fn markdown_link_moves_to_the_next_line_whole_where_it_fits() {
  let source = format!("Merged the fix in [PR 412 now]({EVIL}) today, with more words after it");
  let is_link = |ink: &Ink| matches!(ink, Ink::Words(0) | Ink::Run(0));
  let (mut app, mut tree) = mount(sweep_props(&source, 1600.0));
  let glyphs = render_pass_with_app(&mut tree, &mut app).glyphs;
  let link = glyphs.iter().filter(|glyph| is_link(&ink(glyph)));
  let left = link.clone().map(|glyph| glyph.x).fold(f32::INFINITY, f32::min);
  let right = link
    .map(|glyph| glyph.x + glyph.width)
    .fold(f32::NEG_INFINITY, f32::max);
  let link_width = right - left;
  let mut split = Vec::new();
  for width in (60..=1600).step_by(2) {
    if (width as f32) < link_width + 4.0 {
      continue;
    }
    tree.update_root_props::<MarkdownRoot>(sweep_props(&source, width as f32));
    let lines = lines(render_pass_with_app(&mut tree, &mut app).glyphs);
    if lines.iter().filter(|line| line.iter().any(is_link)).count() > 1 {
      split.push(width);
    }
  }
  assert!(split.is_empty(), "a {link_width}px link splits at {split:?}");
}

#[test]
fn markdown_link_whose_words_draw_nothing_is_not_presented_or_pressable() {
  let presented = Arc::new(Mutex::new(Vec::new()));
  let (mut app, mut tree) = mount(
    MarkdownProps::new(format!(
      "See []({REAL}) [\u{200B}\u{2060}]({REAL}) [ ]({REAL}) [\u{AD}\u{FEFF}]({REAL}) [PR 412\u{2029}x]({EVIL})."
    ))
    .link_presentation({
      let presented = presented.clone();
      move |link| {
        presented.lock().unwrap().push((link.index(), link.text().to_owned()));
        MarkdownLinkPresentation::link().trailing([MarkdownRun::new(" github.com", colored("#00ff00"))])
      }
    })
    .width(800.0),
  );
  assert_eq!(
    *presented.lock().unwrap(),
    vec![(4, "PR 412 x".to_owned())],
    "only the link with words to see is presented, its separator a space"
  );
  let glyphs = render_pass_with_app(&mut tree, &mut app).glyphs;
  let hosts = glyphs
    .iter()
    .filter(|glyph| glyph.color[1] > 0.9 && glyph.color[0] < 0.1)
    .count();
  assert_eq!(hosts, "github.com".len(), "one host, the visible link's");
  assert!(
    tree
      .find_element(|element| element.text_content() == Some("PR 412 x"))
      .is_some(),
    "the visible link is a link node"
  );
  assert!(
    tree
      .find_element(|element| element.text_content() == Some("\u{200B}\u{2060}"))
      .is_none(),
    "a link that draws nothing is no link node"
  );
}
