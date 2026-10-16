//! Selecting in the other blocks: headings, lists, quotes and tables, each
//! with inline code in it, and drags across blocks.

use lurq::components::MarkdownProps;

use super::page;
use crate::support::selection::{Page, assert_drags_copy_their_glyphs, pairs};

const HEADING: &str = "## A heading with `inline code` and a few words that go on long enough to wrap the line";
const LIST: &str = "- the first item has `code` in it and enough words to wrap onto a second line of its own\n\
- the second item\n\
1. a numbered item with `more code`\n\
2. and the last one";
const QUOTE: &str = "> A quote with `code` in it and enough words to wrap it onto a second line at this width.\n>\n\
> Its second paragraph.";
const TABLE: &str = "| Name | Value |\n|---|---|\n| alpha | `one` |\n| beta with words that wrap | two |";

fn sweep(source: &str, width: f32, step: usize) -> Page {
  let mut page = page(MarkdownProps::new(source), width);
  let count = page.inks.len();
  assert_drags_copy_their_glyphs(&mut page, pairs(count, step));
  for (from, to) in pairs(count, step * 3) {
    page.drag_inks(from, to);
    page.assert_highlight_covers(from, to);
  }
  page
}

#[test]
fn every_drag_over_a_heading_copies_the_glyphs_it_covers() {
  let page = sweep(HEADING, 420.0, 5);
  assert!(page.inks.iter().any(|ink| ink.row > 0), "the heading wraps");
}

#[test]
fn every_drag_over_list_items_copies_the_glyphs_it_covers() {
  let page = sweep(LIST, 360.0, 5);
  assert!(page.inks.iter().any(|ink| ink.row > 0), "the first item wraps");
}

#[test]
fn every_drag_over_a_quote_copies_the_glyphs_it_covers() {
  let page = sweep(QUOTE, 360.0, 5);
  assert!(page.inks.iter().any(|ink| ink.row > 0), "the quote wraps");
}

#[test]
fn every_drag_over_a_table_copies_the_glyphs_it_covers() {
  sweep(TABLE, 420.0, 3);
}

#[test]
fn a_drag_across_blocks_copies_each_block_on_its_own_line() {
  let mut page = page(
    MarkdownProps::new("# Title with `code`\n\nA paragraph.\n\n- item one\n- item `two`\n\n> quoted"),
    420.0,
  );
  let (from, _) = page.inks_of("with", 0);
  let (_, to) = page.inks_of("quoted", 0);
  page.drag_inks(from, to);
  assert_eq!(
    page.selected().as_deref(),
    Some("with code\nA paragraph.\n- item one\n- item two\nquoted"),
    "a list item's marker is drawn beside its text and copied with it"
  );
}

#[test]
fn a_drag_across_table_rows_copies_each_row_on_its_own_line() {
  let mut page = page(MarkdownProps::new(TABLE), 420.0);
  let (from, _) = page.inks_of("alpha", 0);
  let (_, to) = page.inks_of("two", 0);
  page.drag_inks(from, to);
  assert_eq!(
    page.selected().as_deref(),
    Some("alpha one\nbeta with words that wrap two")
  );
}
