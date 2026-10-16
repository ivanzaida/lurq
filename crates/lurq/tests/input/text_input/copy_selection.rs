//! Selecting in a text input with the pointer, checked against the drawn
//! glyphs (`support::selection`): a single-line and a multiline input, and
//! what Ctrl+C would copy from them.

use lurq::{
  app::{
    App, Tree,
    events::{ImeEvent, MouseButton, ScrollPhase},
  },
  components::{Column, TextInput},
  core::Signal,
};

use crate::support::{
  TestSurface, render_pass_with_app,
  selection::{GLYPH_PADDING, Page, pairs, rows_of},
};

const INPUT_ID: &str = "input";
const SINGLE: &str = "a single line of input text with digits 2423 in it";
const MULTI: &str = "a multiline input whose text wraps onto several rows at the width of the test, with a second \
paragraph after a line break\nthat starts here and goes on a little.";

fn page(input: TextInput, width: f32) -> Page {
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.resize(800, 600);
  tree.set_root(Column::new().width(width).child(input.id(INPUT_ID).width(width)));
  tree.pass(&mut app, &TestSurface);
  Page::new(app, tree, 1.0)
}

fn selected(page: &mut Page) -> Option<String> {
  page
    .tree
    .get_element_by_id_mut(INPUT_ID)
    .and_then(|element| element.as_text_input())
    .expect("the input")
    .selected_text()
}

fn assert_drags_select_their_glyphs(page: &mut Page, step: usize) {
  let count = page.inks.len();
  let mut failures = Vec::new();
  for (from, to) in pairs(count, step) {
    page.drag_inks(from, to);
    let expected = page.text_between(from, to).to_owned();
    let got = selected(page).unwrap_or_default();
    if got != expected {
      failures.push(format!("{from}..={to}: selected {got:?}, drawn {expected:?}"));
    }
  }
  assert!(
    failures.is_empty(),
    "{} drags selected other text:\n{}",
    failures.len(),
    failures.join("\n")
  );
}

#[test]
fn every_drag_over_a_single_line_input_selects_the_glyphs_it_covers() {
  let mut page = page(TextInput::new(Signal::new(SINGLE.to_owned())), 500.0);
  assert_eq!(page.text, SINGLE);
  assert_drags_select_their_glyphs(&mut page, 3);
  let count = page.inks.len();
  for (from, to) in pairs(count, 9).chain([(0, count - 1)]) {
    page.drag_inks(from, to);
    page.assert_highlight_covers(from, to);
  }
}

#[test]
fn every_drag_over_a_multiline_input_selects_the_glyphs_it_covers() {
  let mut page = page(TextInput::new(Signal::new(MULTI.to_owned())).multiline(), 300.0);
  assert!(
    page.inks.iter().any(|ink| ink.row > 2),
    "the text wraps onto several rows"
  );
  assert_drags_select_their_glyphs(&mut page, 5);
  let count = page.inks.len();
  for (from, to) in pairs(count, 15).chain([(0, count - 1)]) {
    page.drag_inks(from, to);
    page.assert_highlight_covers(from, to);
  }
}

#[test]
fn a_new_drag_in_an_input_replaces_the_selection() {
  let mut page = page(TextInput::new(Signal::new(MULTI.to_owned())).multiline(), 300.0);
  let (from, to) = page.inks_of("several rows", 0);
  page.drag_inks(from, to);
  assert_eq!(selected(&mut page).as_deref(), Some("several rows"));
  let (from, to) = page.inks_of("starts here", 0);
  page.drag_inks(from, to);
  assert_eq!(selected(&mut page).as_deref(), Some("starts here"));
  page.assert_highlight_covers(from, to);
}

const HEBREW: &str = "שלום עולם, זהו משפט בעברית עם עוד כמה מילים";

/// A right-to-left value runs right to left: a drag from the right edge of
/// its first letter to the left edge of its last selects it all, and a drag
/// from the right edge to the middle selects what is written first.
#[test]
fn drags_over_a_right_to_left_input_select_what_they_cover() {
  let mut page = page(TextInput::new(Signal::new(HEBREW.to_owned())), 400.0);
  let left = page.inks.iter().map(|ink| ink.left).fold(f32::MAX, f32::min);
  let right = page.inks.iter().map(|ink| ink.right).fold(f32::MIN, f32::max);
  let y = page.inks[0].y;

  page.drag((right, y), (left, y));
  assert_eq!(selected(&mut page).as_deref(), Some(HEBREW));

  page.drag((right, y), ((left + right) / 2.0, y));
  let prefix = selected(&mut page).expect("a prefix is selected");
  assert!(
    HEBREW.starts_with(&prefix) && !prefix.is_empty() && prefix.len() < HEBREW.len(),
    "{prefix:?} starts the value"
  );
}

const LONG: &str = "one two three four five six seven eight nine ten eleven twelve thirteen fourteen fifteen sixteen \
seventeen eighteen nineteen twenty twentyone twentytwo twentythree twentyfour twentyfive twentysix twentyseven \
twentyeight twentynine thirty";

/// A multiline input scrolled by the wheel draws rows from the middle of its
/// value; a drag over each drawn row selects the words drawn on it, row
/// after row.
#[test]
fn drags_over_the_rows_of_a_scrolled_multiline_input_select_the_words_drawn_on_them() {
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.resize(800, 600);
  tree.set_root(
    Column::new().width(240.0).child(
      TextInput::new(Signal::new(LONG.to_owned()))
        .multiline()
        .rows(3, 3)
        .id(INPUT_ID)
        .width(240.0),
    ),
  );
  tree.pass(&mut app, &TestSurface);
  let mut page = Page::without_inks(app, tree, 1.0);
  let bounds = page
    .tree
    .get_element_by_id_mut(INPUT_ID)
    .expect("the input")
    .bounds()
    .expect("laid out");
  let (x, y) = (bounds.x + bounds.width / 2.0, bounds.y + bounds.height / 2.0);
  for _ in 0..3 {
    page.tree.scroll(x, y, 0.0, -40.0, ScrollPhase::Scroll);
    page.pass();
  }
  let glyphs = render_pass_with_app(&mut page.tree, &mut page.app)
    .glyphs
    .into_iter()
    .filter(|glyph| {
      glyph.width > 0.0
        && glyph.y + glyph.height / 2.0 > bounds.y
        && glyph.y + glyph.height / 2.0 < bounds.y + bounds.height
    })
    .collect::<Vec<_>>();
  let rows = rows_of(glyphs);
  let row_count = rows.iter().map(|(row, _)| row.index).max().map_or(0, |last| last + 1);
  assert!(row_count >= 2, "several rows are drawn, not {row_count}");

  let mut after = 0;
  for index in 0..row_count {
    let on_row: Vec<_> = rows.iter().filter(|(row, _)| row.index == index).collect();
    let (row, first) = on_row[0];
    let (_, last) = on_row[on_row.len() - 1];
    page
      .tree
      .mouse_down(first.x + GLYPH_PADDING, row.center, MouseButton::Left);
    page.tree.mouse_move(last.x + last.width - GLYPH_PADDING, row.center);
    page
      .tree
      .mouse_up(last.x + last.width - GLYPH_PADDING, row.center, MouseButton::Left);
    page.pass();
    let words = selected(&mut page).unwrap_or_default();
    let start = LONG[after..].find(&words).map(|at| after + at);
    assert!(
      start.is_some() && words.chars().filter(|ch| !ch.is_whitespace()).count() == on_row.len(),
      "row {index} selects the {} words drawn on it, after byte {after}: {words:?}",
      on_row.len()
    );
    if index == 0 {
      assert!(
        start > Some(0),
        "the first drawn row is not the value's first: the input scrolled"
      );
    }
    after = start.unwrap_or(after) + words.len();
  }
}

/// A space composed by an input method before a line break is underlined:
/// the composed text sits in the displayed text, whose row ends with the
/// break, and selected whitespace in an input stays visible anyway.
#[test]
fn a_composed_space_before_a_line_break_is_underlined() {
  let mut page = page(TextInput::new(Signal::new("ab\ncd".to_owned())).multiline(), 200.0);
  let bounds = page
    .tree
    .get_element_by_id_mut(INPUT_ID)
    .expect("the input")
    .bounds()
    .expect("laid out");
  let (first_row_end, y) = (page.inks[1].right + 20.0, page.inks[1].y);
  page.click(first_row_end, y);
  page.tree.ime(ImeEvent::Preedit {
    text: " ".to_owned(),
    cursor: Some((1, 1)),
  });
  page.pass();
  let underlines: Vec<_> = render_pass_with_app(&mut page.tree, &mut page.app)
    .rects
    .into_iter()
    .filter(|rect| {
      rect.height <= 1.5
        && rect.width >= 2.0
        && rect.x >= bounds.x
        && rect.x + rect.width <= bounds.x + bounds.width
        && rect.y > bounds.y
        && rect.y < bounds.y + bounds.height
    })
    .collect();
  assert!(
    !underlines.is_empty(),
    "the composed space is underlined at the end of the first row"
  );
  assert!(
    underlines.iter().all(|rect| rect.x >= page.inks[1].right - 1.0),
    "the underline lies after \"ab\": {underlines:?}"
  );
}
