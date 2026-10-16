//! A paragraph wrapped onto several lines at a fractional display scale:
//! the pointer and the glyphs are in physical pixels, the layout in logical
//! ones, and every drag still copies the glyphs it covers.

use lurq::components::MarkdownProps;

use super::{paragraph::REPORTED, scaled};
use crate::support::selection::{assert_drags_copy_their_glyphs, pairs};

#[test]
fn every_drag_over_a_paragraph_wrapped_at_a_fractional_scale_copies_the_glyphs_it_covers() {
  let mut page = scaled(MarkdownProps::new(REPORTED), 420.0, 1.5);
  let rows = page.inks.iter().map(|ink| ink.row).max().unwrap_or(0) + 1;
  assert!(rows >= 4, "the paragraph wraps onto at least four lines, not {rows}");
  let count = page.inks.len();
  assert_drags_copy_their_glyphs(&mut page, pairs(count, 9));
}

#[test]
fn the_highlight_of_a_paragraph_wrapped_at_a_fractional_scale_stays_on_its_glyphs() {
  let mut page = scaled(MarkdownProps::new(REPORTED), 420.0, 1.5);
  let count = page.inks.len();
  for (from, to) in pairs(count, 29).chain([(0, count - 1)]) {
    page.drag_inks(from, to);
    page.assert_highlight_covers(from, to);
  }
}

#[test]
fn a_selection_can_be_started_again_on_every_line_at_a_fractional_scale() {
  let mut page = scaled(MarkdownProps::new(REPORTED), 420.0, 1.5);
  let rows = page.inks.iter().map(|ink| ink.row).max().unwrap_or(0) + 1;
  let count = page.inks.len();
  page.drag_inks(0, count - 1);
  for row in 0..rows {
    let first = page
      .inks
      .iter()
      .position(|ink| ink.row == row)
      .expect("a glyph on the row");
    let last = page
      .inks
      .iter()
      .rposition(|ink| ink.row == row)
      .expect("a glyph on the row");
    page.drag_inks(first, last);
    assert_eq!(
      page.selected().as_deref(),
      Some(&*page.text_between(first, last)),
      "row {row}"
    );
  }
}
