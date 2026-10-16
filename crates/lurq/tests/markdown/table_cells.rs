//! A Markdown table cell's box: its padding sits around the cell's words,
//! whatever the column's alignment.

use lurq::{
  app::theme::{MarkdownBlockStyle, MarkdownFocusRing, ThemeMarkdown},
  components::MarkdownProps,
  layout::{
    quad::{Quad, QuadContent},
    text_style::TextAlign,
  },
  node::{color::Color, padding::Padding},
};

use super::links::{mount, pass, quads, rects_of};

const CELL: &str = "#123456";

/// A quad's left, top, right and bottom edges.
type Edges = [f32; 4];

fn edges(quad: &Quad) -> Edges {
  [quad.x, quad.y, quad.x + quad.width, quad.y + quad.height]
}

/// A cell's words: their edges and their alignment.
struct Words {
  edges: Edges,
  align: TextAlign,
}

/// The cells of a table's header row, or of its body.
#[derive(Clone, Copy)]
enum Part {
  Header,
  Body,
}

/// The edges of the body cells and of their words, for a table whose body
/// cells have the background `CELL` and the padding `[4, 8]`.
fn body_cells(source: &str) -> (Vec<Edges>, Vec<Words>) {
  cells_of(MarkdownProps::new(source), Part::Body)
}

fn body_cells_of(props: MarkdownProps) -> (Vec<Edges>, Vec<Words>) {
  cells_of(props, Part::Body)
}

/// The edges of `part`'s cells and of their words: its cells, and only they,
/// have the background `CELL` and the padding `[4, 8]`.
fn cells_of(props: MarkdownProps, part: Part) -> (Vec<Edges>, Vec<Words>) {
  let marked = MarkdownBlockStyle {
    background: Some(Color::from_hex(CELL)),
    padding: Some(Padding::symmetric(8.0, 4.0)),
    ..MarkdownBlockStyle::new()
  };
  let mut theme = ThemeMarkdown::default();
  match part {
    Part::Header => theme.table_header_box = marked,
    Part::Body => theme.table_cell_box = marked,
  }
  let (_app, tree) = mount(props.theme(theme).width(600.0));
  let quads = quads(&tree);
  let cells = rects_of(&quads, Color::from_hex(CELL))
    .into_iter()
    .map(edges)
    .collect::<Vec<_>>();
  let words = quads
    .iter()
    .filter_map(|quad| {
      let align = match &quad.content {
        QuadContent::Text { style, .. } => style.text_align,
        QuadContent::RichText { spans, .. } => spans[0].style.text_align,
        _ => return None,
      };
      let edges = edges(quad);
      cells
        .iter()
        .any(|cell| inside(edges, *cell))
        .then_some(Words { edges, align })
    })
    .collect();
  (cells, words)
}

fn inside(inner: Edges, outer: Edges) -> bool {
  inner[0] >= outer[0] && inner[1] >= outer[1] && inner[2] <= outer[2] && inner[3] <= outer[3]
}

/// Asserts the gaps between the cell's edges and its words': left, top,
/// right, bottom.
fn assert_insets(cell: Edges, words: Edges, expected: [f32; 4]) {
  let actual = [
    words[0] - cell[0],
    words[1] - cell[1],
    cell[2] - words[2],
    cell[3] - words[3],
  ];
  assert!(
    actual
      .iter()
      .zip(expected)
      .all(|(actual, expected)| (actual - expected).abs() < 0.01),
    "left, top, right, bottom: {actual:?}, expected {expected:?}"
  );
}

/// Asserts each cell's words sit inside its padding, in its column's
/// alignment.
fn assert_cells((cells, words): (Vec<Edges>, Vec<Words>), aligns: &[TextAlign]) {
  assert_eq!(
    (cells.len(), words.len()),
    (aligns.len(), aligns.len()),
    "every body cell, each with its words"
  );
  for ((cell, words), align) in cells.iter().zip(&words).zip(aligns) {
    assert_insets(*cell, words.edges, [8.0, 4.0, 8.0, 4.0]);
    assert!(words.align == *align, "the column keeps its alignment");
  }
}

#[test]
fn markdown_table_cell_words_sit_inside_the_cell_padding_in_every_column_alignment() {
  assert_cells(
    body_cells("| a | b | c |\n|:--|:-:|--:|\n| left | middle | right |"),
    &[TextAlign::Left, TextAlign::Center, TextAlign::Right],
  );
}

#[test]
fn markdown_table_cell_with_a_link_keeps_its_words_inside_the_cell_padding() {
  assert_cells(
    body_cells("| a | b |\n|:--|--:|\n| see [docs](/docs) | [right](/r) |"),
    &[TextAlign::Left, TextAlign::Right],
  );
}

/// Fitting its content, a cell's words are as wide as their text, and the
/// column's alignment places them in the cell, inside its padding.
#[test]
fn markdown_fitting_table_cell_places_its_words_inside_the_padding_by_column_alignment() {
  let (cells, words) = body_cells_of(
    MarkdownProps::new("| a | b |\n|:--|--:|\n| left | right |\n| a much wider left cell | a much wider right cell |")
      .fit_content(true),
  );
  assert_eq!(
    (cells.len(), words.len()),
    (4, 4),
    "every body cell, each with its words"
  );
  let (left, right) = ((cells[0], &words[0]), (cells[1], &words[1]));
  assert!(
    right.1.edges[2] - right.1.edges[0] < right.0[2] - right.0[0] - 16.0,
    "the words are narrower than the cell"
  );
  assert!(
    (left.1.edges[0] - left.0[0] - 8.0).abs() < 0.01,
    "left column: 8 px at the left"
  );
  assert!(
    left.0[2] - left.1.edges[2] > 8.0,
    "left column: the room is at the right"
  );
  assert!(
    (right.0[2] - right.1.edges[2] - 8.0).abs() < 0.01,
    "right column: 8 px at the right"
  );
  assert!(
    right.1.edges[0] - right.0[0] > 8.0,
    "right column: the room is at the left"
  );
}

#[test]
fn markdown_table_header_cell_words_sit_inside_the_header_padding() {
  let (cells, words) = cells_of(
    MarkdownProps::new("| left | middle | right |\n|:--|:-:|--:|\n| a | b | c |"),
    Part::Header,
  );
  assert_eq!(
    (cells.len(), words.len()),
    (3, 3),
    "every header cell, each with its words"
  );
  for ((cell, words), align) in cells
    .iter()
    .zip(&words)
    .zip([TextAlign::Left, TextAlign::Center, TextAlign::Right])
  {
    assert_insets(*cell, words.edges, [8.0, 4.0, 8.0, 4.0]);
    assert!(words.align == align, "the column keeps its alignment");
  }
}

#[test]
fn markdown_short_table_cell_centers_its_words_in_a_taller_row() {
  const WRAPPING: &str = "two two two two two two two two two two two two two two two two two two two two";
  let (cells, words) = body_cells(&format!("| a | b |\n|---|---|\n| one | {WRAPPING} |"));
  let (short, words) = (cells[0], words[0].edges);
  let (above, below) = (words[1] - short[1], short[3] - words[3]);
  assert!(above > 4.0 + 5.0, "the words sit lower than the 4 px padding: {above}");
  assert!((above - below).abs() < 0.01, "centered: {above} above, {below} below");
}

const RING: &str = "#ff00ff";

/// A link at the very start of a cell with no padding, focused from the
/// keyboard: its ring, 3 px outside the words, reaches past the cell.
#[test]
fn markdown_table_cell_does_not_clip_a_link_focus_ring() {
  let theme = ThemeMarkdown {
    link_focus_ring: Some(MarkdownFocusRing::new(2.0, Color::from_hex(RING)).offset(1.0)),
    table_cell_box: MarkdownBlockStyle {
      padding: None,
      ..MarkdownBlockStyle::new()
    },
    ..ThemeMarkdown::default()
  };
  let (mut app, mut tree) = mount(
    MarkdownProps::new("| a | b |\n|---|---|\n| x | [Docs](/docs) |")
      .theme(theme)
      .width(400.0),
  );
  tree.key_down("Tab".into(), "Tab".into(), false, false, false);
  tree.key_up("Tab".into(), "Tab".into(), false, false, false);
  pass(&mut app, &mut tree);
  let quads = quads(&tree);
  let ring = quads
    .iter()
    .find(|quad| {
      quad
        .border
        .and_then(|border| border.top)
        .is_some_and(|top| top.color == Color::from_hex(RING))
    })
    .expect("the focused link's ring");
  let clip = ring.clip;
  assert!(
    !clip.active || (clip.x <= ring.x && clip.x + clip.width >= ring.x + ring.width),
    "the ring [{}, {}] is not clipped at its sides: clip [{}, {}]",
    ring.x,
    ring.x + ring.width,
    clip.x,
    clip.x + clip.width
  );
}
