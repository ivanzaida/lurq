//! A Markdown link's tooltip box: its padding sits around the words.

use std::time::Duration;

use lurq::{
  app::{
    Tree,
    theme::{MarkdownBlockStyle, ThemeMarkdown},
  },
  components::{MarkdownLinkPresentation, MarkdownProps},
  layout::quad::{Quad, QuadContent},
  node::{color::Color, padding::Padding},
};

use super::links::{bounds_of, mount, pass, quads, rects_of};

const BOX: &str = "#123456";
const WORDS: &str = "Opens /tasks/1";
/// The window the tests lay out in is 800 px wide.
const WINDOW_WIDTH: f32 = 800.0;

/// A quad's left, top, right and bottom edges.
type Edges = [f32; 4];

fn edges(quad: &Quad) -> Edges {
  [quad.x, quad.y, quad.x + quad.width, quad.y + quad.height]
}

/// The edges of the tooltip's box and of its words, with the pointer resting
/// on the link.
fn tooltip_edges(tooltip_box: MarkdownBlockStyle) -> (Edges, Edges) {
  tooltip_edges_saying(WORDS, tooltip_box)
}

fn tooltip_edges_saying(words: &'static str, tooltip_box: MarkdownBlockStyle) -> (Edges, Edges) {
  let theme = ThemeMarkdown {
    link_tooltip_box: MarkdownBlockStyle {
      background: Some(Color::from_hex(BOX)),
      ..tooltip_box
    },
    link_tooltip_delay: Duration::ZERO,
    ..ThemeMarkdown::default()
  };
  let (mut app, mut tree) = mount(
    MarkdownProps::new("See [KON-1](/tasks/1) for details.")
      .theme(theme)
      .link_presentation(move |_| MarkdownLinkPresentation::link().tooltip(words))
      .width(400.0),
  );
  let link = bounds_of(&mut tree, "KON-1");
  tree.mouse_move(link.x + 2.0, link.y + 2.0);
  pass(&mut app, &mut tree);
  pass(&mut app, &mut tree);
  shown(&tree, words)
}

fn shown(tree: &Tree, words: &str) -> (Edges, Edges) {
  let quads = quads(tree);
  let boxes = rects_of(&quads, Color::from_hex(BOX));
  assert_eq!(boxes.len(), 1, "one tooltip box");
  let words_quad = quads
    .iter()
    .find(|quad| matches!(&quad.content, QuadContent::Text { text, .. } if text == words))
    .expect("the tooltip's words");
  (edges(boxes[0]), edges(words_quad))
}

/// Asserts the gaps between the box's edges and the words': left, top,
/// right, bottom.
fn assert_insets((tooltip_box, words): (Edges, Edges), expected: [f32; 4]) {
  let actual = [
    words[0] - tooltip_box[0],
    words[1] - tooltip_box[1],
    tooltip_box[2] - words[2],
    tooltip_box[3] - words[3],
  ];
  assert!(
    actual
      .iter()
      .zip(expected)
      .all(|(actual, expected)| (actual - expected).abs() < 0.01),
    "left, top, right, bottom: {actual:?}, expected {expected:?}"
  );
}

#[test]
fn markdown_link_tooltip_words_sit_inside_the_box_padding_on_every_side() {
  let shown = tooltip_edges(MarkdownBlockStyle {
    padding: Some(Padding::all(4.0)),
    ..MarkdownBlockStyle::new()
  });
  assert_insets(shown, [4.0; 4]);
}

#[test]
fn markdown_link_tooltip_box_takes_its_horizontal_and_vertical_padding_apart() {
  let shown = tooltip_edges(MarkdownBlockStyle {
    padding: Some(Padding::symmetric(8.0, 4.0)),
    ..MarkdownBlockStyle::new()
  });
  assert_insets(shown, [8.0, 4.0, 8.0, 4.0]);
}

#[test]
fn markdown_link_tooltip_longer_than_the_window_wraps_inside_its_padding() {
  const LONG: &str = concat!(
    "Opens a page whose name is far too long to fit on one line of the window at all, ",
    "so the words wrap inside the tooltip's box instead of running past the window's right edge",
  );
  let tooltip_box = MarkdownBlockStyle {
    padding: Some(Padding::symmetric(8.0, 4.0)),
    ..MarkdownBlockStyle::new()
  };
  let (_, one_line) = tooltip_edges(tooltip_box.clone());
  let (tooltip_box, words) = tooltip_edges_saying(LONG, tooltip_box);
  assert!(
    tooltip_box[2] <= WINDOW_WIDTH,
    "the box stays in the window: {tooltip_box:?}"
  );
  assert!(
    words[3] - words[1] > 1.5 * (one_line[3] - one_line[1]),
    "the words wrap: {words:?}"
  );
  assert_insets((tooltip_box, words), [8.0, 4.0, 8.0, 4.0]);
}
