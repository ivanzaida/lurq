//! Where a paragraph paints around its links: underlines on the painted
//! glyphs at fractional scales, a focus ring per laid-out line, and rings
//! inside lists and quotes.

use std::time::Duration;

use lurq::{
  app::{
    App, Tree,
    theme::{MarkdownFocusRing, ThemeMarkdown},
  },
  components::{MarkdownLinkPresentation, MarkdownProps},
  layout::quad::{Quad, QuadContent},
  node::color::Color,
};

use super::links::{MarkdownRoot, bounds_of, mount, pass, quads, rects_of, underlined};
use crate::support::{GlyphSnapshot, TestSurface, render_pass_with_app};

const RING: &str = "#ff00ff";
/// Linear red of the link's words (`#aa0000`).
const WORDS_RED: f32 = 0.402;

fn rings(quads: &[Quad]) -> Vec<&Quad> {
  quads
    .iter()
    .filter(|quad| {
      quad
        .border
        .and_then(|border| border.top)
        .is_some_and(|top| top.color == Color::from_hex(RING))
    })
    .collect()
}

fn ring_theme() -> ThemeMarkdown {
  ThemeMarkdown {
    link_focus_ring: Some(MarkdownFocusRing::new(2.0, Color::from_hex(RING)).offset(1.0)),
    link_tooltip_delay: Duration::ZERO,
    ..ThemeMarkdown::default()
  }
}

fn tab(app: &mut App, tree: &mut Tree) {
  tree.key_down("Tab".into(), "Tab".into(), false, false, false);
  tree.key_up("Tab".into(), "Tab".into(), false, false, false);
  pass(app, tree);
}

fn is_word_glyph(glyph: &GlyphSnapshot) -> bool {
  (glyph.color[0] - WORDS_RED).abs() < 0.02 && glyph.color[1] < 0.01 && glyph.color[2] < 0.01
}

#[test]
fn markdown_link_underline_lies_under_its_painted_glyphs_at_a_fractional_scale() {
  let lead = "The quick brown fox jumps over the lazy dog and keeps on running far away from here, ".repeat(3);
  underline_lies_under_glyphs(&format!("{lead}[the linked words at the end](/end)"));
}

#[test]
fn markdown_link_only_paragraph_underline_lies_under_its_painted_glyphs_at_a_fractional_scale() {
  underline_lies_under_glyphs("[a paragraph that is only a link](/only)");
}

fn underline_lies_under_glyphs(source: &str) {
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.resize(2400, 300);
  tree.set_scale_factor(1.5);
  tree.mount_root::<MarkdownRoot>(
    &mut app,
    MarkdownProps::new(source)
      .link_presentation(|_| MarkdownLinkPresentation::link().style(underlined("#aa0000", "#aa0001")))
      .width(1500.0),
  );
  tree.pass(&mut app, &TestSurface);

  let glyphs = render_pass_with_app(&mut tree, &mut app)
    .glyphs
    .into_iter()
    .filter(is_word_glyph)
    .collect::<Vec<_>>();
  assert!(!glyphs.is_empty(), "the link's glyphs are painted");
  let left = glyphs.iter().map(|glyph| glyph.x).fold(f32::INFINITY, f32::min);
  let right = glyphs
    .iter()
    .map(|glyph| glyph.x + glyph.width)
    .fold(f32::NEG_INFINITY, f32::max);

  let quads = quads(&tree);
  let underlines = rects_of(&quads, Color::from_hex("#aa0001"));
  assert_eq!(underlines.len(), 1);
  let (start, end) = (underlines[0].x * 1.5, (underlines[0].x + underlines[0].width) * 1.5);
  // A glyph bitmap carries a pixel or two of antialiasing padding around its
  // ink: the laid-out words start and end within three device pixels of the
  // painted bitmaps, on the same line.
  assert!(
    (start - left).abs() <= 3.0,
    "underline starts at {start}, glyphs at {left}"
  );
  assert!((end - right).abs() <= 3.0, "underline ends at {end}, glyphs at {right}");
  let top = glyphs.iter().map(|glyph| glyph.y).fold(f32::INFINITY, f32::min);
  let bottom = glyphs
    .iter()
    .map(|glyph| glyph.y + glyph.height)
    .fold(f32::NEG_INFINITY, f32::max);
  let line = underlines[0].y * 1.5;
  assert!(
    line > top && line < bottom + 3.0,
    "underline at {line}, glyphs {top}..{bottom}"
  );
}

#[test]
fn markdown_wrapped_link_has_a_focus_ring_on_each_line() {
  let (mut app, mut tree) = mount(
    MarkdownProps::new("Start [a link whose words wrap onto the next lines here](/wrapped) end.")
      .theme(ring_theme())
      .width(170.0),
  );
  tab(&mut app, &mut tree);
  let quads = quads(&tree);
  let rings = rings(&quads);
  assert!(rings.len() >= 2, "one ring per line: {}", rings.len());
  let mut tops = rings.iter().map(|ring| ring.y.round() as i32).collect::<Vec<_>>();
  tops.dedup();
  assert_eq!(tops.len(), rings.len(), "the rings are on different lines");
  let first = bounds_of(&mut tree, "a link whose words wrap onto the next lines here");
  assert!(
    (rings[0].x - (first.x - 3.0)).abs() < 0.01,
    "the first ring frames the first line"
  );
}

#[test]
fn markdown_link_ring_inside_a_list_or_quote_is_not_clipped_by_its_containers() {
  for source in ["- [Docs](/docs) for more.", "> [Docs](/docs) for more."] {
    let (mut app, mut tree) = mount(MarkdownProps::new(source).theme(ring_theme()).width(400.0));
    tab(&mut app, &mut tree);
    let link = bounds_of(&mut tree, "Docs");
    let quads = quads(&tree);
    let ring = rings(&quads);
    assert_eq!(ring.len(), 1, "{source}");
    let ring = ring[0];
    assert!((ring.x - (link.x - 3.0)).abs() < 0.01, "{source}");
    let paragraph = quads
      .iter()
      .find(|quad| matches!(quad.content, QuadContent::RichText { .. }))
      .unwrap();
    assert!(ring.x < paragraph.x, "{source}: the ring reaches past the item's text");
    assert!(
      !ring.clip.active || (ring.clip.x <= ring.x && ring.clip.x + ring.clip.width >= ring.x + ring.width),
      "{source}: clipped to {:?}",
      (ring.clip.x, ring.clip.y, ring.clip.width, ring.clip.height)
    );
  }
}

/// The lines the link's glyphs are painted on, and the lines its underlines
/// are laid out on, in device pixels, at `width`.
fn painted_and_laid_out_lines(width: f32) -> (usize, usize) {
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.resize(600, 600);
  tree.set_scale_factor(1.5);
  tree.mount_root::<MarkdownRoot>(
    &mut app,
    MarkdownProps::new("Some words before [the link words that wrap somewhere](/x) and some after.")
      .link_presentation(|_| MarkdownLinkPresentation::link().style(underlined("#aa0000", "#aa0001")))
      .width(width),
  );
  tree.pass(&mut app, &TestSurface);
  let glyphs = render_pass_with_app(&mut tree, &mut app)
    .glyphs
    .into_iter()
    .filter(is_word_glyph)
    .collect::<Vec<_>>();
  let mut baselines = glyphs
    .iter()
    .map(|glyph| (glyph.y + glyph.height).round() as i32)
    .collect::<Vec<_>>();
  baselines.sort_unstable();
  baselines.dedup_by(|a, b| (*a - *b).abs() <= 6);
  let underlines = rects_of(&quads(&tree), Color::from_hex("#aa0001")).len();
  (baselines.len(), underlines)
}

#[test]
fn markdown_link_wraps_where_its_glyphs_wrap_at_a_fractional_scale() {
  // Widths whose device-pixel width is fractional: the paint wraps at
  // `ceil(width * 1.5)` device pixels, which a logical layout at
  // `ceil(width)` misses for a word ending in between.
  for step in 0..120 {
    let width = 150.05 + step as f32 * 0.37;
    let (painted, laid_out) = painted_and_laid_out_lines(width);
    assert_eq!(painted, laid_out, "at width {width}");
  }
}
