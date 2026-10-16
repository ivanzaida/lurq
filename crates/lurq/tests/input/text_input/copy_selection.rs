//! Selecting in a text input with the pointer, checked against the drawn
//! glyphs (`support::selection`): a single-line and a multiline input, and
//! what Ctrl+C would copy from them.

use lurq::{
  app::{App, Tree},
  components::{Column, TextInput},
  core::Signal,
};

use crate::support::{
  TestSurface,
  selection::{Page, pairs},
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
