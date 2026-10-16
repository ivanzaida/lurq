//! Selecting and copying plain `Text` outside Markdown, checked against the
//! drawn glyphs (`support::selection`): single-line, wrapped and
//! multi-paragraph texts, texts stacked in a column, and a selection
//! extended with Shift.

use lurq::{
  app::{App, Tree},
  components::{Column, Text},
};

use crate::support::{
  TestSurface,
  selection::{Page, assert_drags_copy_their_glyphs, pairs},
};

const WRAPPED: &str = "A plain text with enough words in it to wrap onto a few lines at the width of the test, with \
accents (déjà vu), digits 2423 and punctuation; nothing else.";
const PARAGRAPHS: &str = "First line of the text.\nSecond line, a bit longer than the first one.\n\nFourth line after an \
empty one.";

fn page(root: impl Into<lurq::node::Element>, scale: f32) -> Page {
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.set_scale_factor(scale);
  tree.resize((800.0 * scale) as u32, (600.0 * scale) as u32);
  tree.set_root(root);
  tree.pass(&mut app, &TestSurface);
  Page::new(app, tree, scale)
}

fn sweep(mut page: Page, step: usize) -> Page {
  let count = page.inks.len();
  assert_drags_copy_their_glyphs(&mut page, pairs(count, step));
  for (from, to) in pairs(count, step * 3).chain([(0, count - 1)]) {
    page.drag_inks(from, to);
    page.assert_highlight_covers(from, to);
  }
  page
}

#[test]
fn every_drag_over_a_wrapped_text_copies_the_glyphs_it_covers() {
  let page = sweep(
    page(
      Column::new().width(320.0).child(Text::new(WRAPPED).selectable(true)),
      1.0,
    ),
    5,
  );
  assert!(
    page.inks.iter().any(|ink| ink.row > 1),
    "the text wraps onto three lines"
  );
}

#[test]
fn every_drag_over_a_wrapped_text_at_a_fractional_scale_copies_the_glyphs_it_covers() {
  let page = sweep(
    page(
      Column::new().width(320.0).child(Text::new(WRAPPED).selectable(true)),
      1.5,
    ),
    5,
  );
  assert!(
    page.inks.iter().any(|ink| ink.row > 1),
    "the text wraps onto three lines"
  );
}

#[test]
fn every_drag_over_a_text_with_line_breaks_copies_the_glyphs_it_covers() {
  let mut page = page(Text::new(PARAGRAPHS).selectable(true), 1.0);
  let count = page.inks.len();
  assert_drags_copy_their_glyphs(&mut page, pairs(count, 3));
  for (from, to) in pairs(count, 9).chain([(0, count - 1)]) {
    page.drag_inks(from, to);
    page.assert_highlight_covers(from, to);
  }
}

#[test]
fn a_drag_across_stacked_texts_copies_each_on_its_own_line() {
  let mut page = page(
    Column::new()
      .spacing(8.0)
      .child(Text::new("the first text").selectable(true))
      .child(Text::new("the second text").selectable(true))
      .child(Text::new("the third text").selectable(true)),
    1.0,
  );
  let (from, _) = page.inks_of("first", 0);
  let (_, to) = page.inks_of("third", 0);
  page.drag_inks(from, to);
  assert_eq!(
    page.selected().as_deref(),
    Some("first text\nthe second text\nthe third")
  );
  page.assert_highlight_covers(from, to);
}

#[test]
fn a_shift_click_extends_a_plain_text_selection_and_a_new_drag_replaces_it() {
  let mut page = page(
    Column::new().width(320.0).child(Text::new(WRAPPED).selectable(true)),
    1.0,
  );
  let (from, to) = page.inks_of("enough words", 0);
  page.drag_inks(from, to);
  assert_eq!(page.selected().as_deref(), Some("enough words"));

  let (_, digits) = page.inks_of("2423", 0);
  let after = page.after(digits);
  page.shift_click(after.0, after.1);
  assert_eq!(page.selected().as_deref(), Some(&*page.text_between(from, digits)));

  let (from, to) = page.inks_of("nothing else", 0);
  page.drag_inks(from, to);
  assert_eq!(page.selected().as_deref(), Some("nothing else"));
  page.assert_highlight_covers(from, to);
}

#[test]
fn a_double_click_selects_a_word_and_a_triple_click_its_line() {
  let mut page = page(Text::new(PARAGRAPHS).selectable(true), 1.0);
  let (from, to) = page.inks_of("longer", 0);
  let (x, y) = page.before(from + 1);
  page.click(x, y);
  page.click(x, y);
  assert_eq!(page.selected().as_deref(), Some("longer"));
  page.assert_highlight_covers(from, to);
  page.click(x, y);
  assert_eq!(
    page.selected().as_deref(),
    Some("Second line, a bit longer than the first one.")
  );
}
