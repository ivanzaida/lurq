//! After a selection, the next one: a new drag anywhere starts over, a
//! Shift press or drag extends from where the selection started, a Ctrl
//! drag adds a selection in another text, and a double or triple click
//! selects the word or the line under the pointer.

use lurq::components::MarkdownProps;

use super::{page, paragraph::REPORTED};
use crate::support::selection::pairs;

const TWO_PARAGRAPHS: &str = "The first paragraph has a `code span` and a few more words to wrap it onto two lines at \
the width of the test.\n\nThe second paragraph has *emphasis* in it and ends here.";

#[test]
fn a_new_drag_after_a_selection_starts_over_anywhere() {
  let mut page = page(MarkdownProps::new(REPORTED), 560.0);
  let count = page.inks.len();
  // Each drag follows another one, wherever it ended.
  let mut previous = None;
  for (from, to) in pairs(count, 31).chain([(count - 1, count - 1), (0, 0)]) {
    page.drag_inks(from, to);
    let expected = page.text_between(from, to);
    assert_eq!(
      page.selected().as_deref(),
      Some(expected.as_str()),
      "the drag {from}..={to} after {previous:?} selects its own glyphs"
    );
    previous = Some((from, to));
  }
}

#[test]
fn a_shift_click_extends_the_selection_from_where_it_started() {
  let mut page = page(MarkdownProps::new(REPORTED), 560.0);
  let (from, to) = page.inks_of("design marks", 0);
  page.drag_inks(from, to);
  assert_eq!(page.selected().as_deref(), Some("design marks"));

  let (_, further) = page.inks_of("planned", 0);
  let after = page.after(further);
  page.shift_click(after.0, after.1);
  assert_eq!(page.selected().as_deref(), Some(&*page.text_between(from, further)));

  // Back before the anchor: the selection runs from the pressed point to it.
  let (earlier, _) = page.inks_of("Nothing", 0);
  let before = page.before(earlier);
  page.shift_click(before.0, before.1);
  let anchor = page.inks[from].start;
  assert_eq!(
    page.selected().as_deref(),
    Some(&page.text[page.inks[earlier].start..anchor])
  );
}

#[test]
fn a_shift_drag_extends_the_selection_from_where_it_started() {
  let mut page = page(MarkdownProps::new(REPORTED), 560.0);
  let (from, to) = page.inks_of("design marks", 0);
  page.drag_inks(from, to);

  let (_, planned) = page.inks_of("planned", 0);
  let (_, variable) = page.inks_of("variable", 0);
  let start = page.after(planned);
  let end = page.after(variable);
  page.shift_press(start.0, start.1);
  page.tree.mouse_move_with_modifiers(end.0, end.1, true, false, false);
  page.shift_release(end.0, end.1);
  assert_eq!(page.selected().as_deref(), Some(&*page.text_between(from, variable)));
}

#[test]
fn a_shift_click_extends_across_paragraphs() {
  let mut page = page(MarkdownProps::new(TWO_PARAGRAPHS), 420.0);
  let (from, to) = page.inks_of("code span", 0);
  page.drag_inks(from, to);
  let (_, further) = page.inks_of("emphasis", 0);
  let after = page.after(further);
  page.shift_click(after.0, after.1);
  assert_eq!(page.selected().as_deref(), Some(&*page.text_between(from, further)));
}

#[test]
fn a_shift_click_without_a_selection_selects_from_the_pressed_point() {
  let mut page = page(MarkdownProps::new(REPORTED), 560.0);
  let (from, to) = page.inks_of("design marks", 0);
  let (before, after) = (page.before(from), page.after(to));
  page.shift_press(before.0, before.1);
  page
    .tree
    .mouse_move_with_modifiers(after.0, after.1, true, false, false);
  page.shift_release(after.0, after.1);
  assert_eq!(page.selected().as_deref(), Some("design marks"));
}

#[test]
fn a_ctrl_drag_adds_a_selection_in_another_paragraph() {
  let mut page = page(MarkdownProps::new(TWO_PARAGRAPHS), 420.0);
  let (from, to) = page.inks_of("code span", 0);
  page.drag_inks(from, to);
  let (from, to) = page.inks_of("emphasis", 0);
  let (before, after) = (page.before(from), page.after(to));
  page.tree.mouse_down_with_modifiers(
    before.0,
    before.1,
    lurq::app::events::MouseButton::Left,
    false,
    true,
    false,
  );
  page
    .tree
    .mouse_move_with_modifiers(after.0, after.1, false, true, false);
  page.tree.mouse_up_with_modifiers(
    after.0,
    after.1,
    lurq::app::events::MouseButton::Left,
    false,
    true,
    false,
  );
  page.pass();
  assert_eq!(page.selected().as_deref(), Some("code span\nemphasis"));
}

#[test]
fn a_double_click_selects_the_word_under_the_pointer_and_a_triple_click_its_line() {
  let mut page = page(MarkdownProps::new(TWO_PARAGRAPHS), 420.0);
  let (from, to) = page.inks_of("emphasis", 0);
  let (x, y) = page.before(from + 2);
  page.click(x, y);
  page.click(x, y);
  assert_eq!(page.selected().as_deref(), Some("emphasis"));
  page.assert_highlight_covers(from, to);

  page.click(x, y);
  let (start, _) = page.inks_of("The second", 0);
  let end = page.inks.len() - 1;
  assert_eq!(page.selected().as_deref(), Some(&*page.text_between(start, end)));
  page.assert_highlight_covers(start, end);
}

#[test]
fn a_double_click_in_inline_code_selects_the_code_word() {
  let mut page = page(MarkdownProps::new(REPORTED), 560.0);
  let (from, to) = page.inks_of("storage", 0);
  let (x, y) = page.before(from + 1);
  page.click(x, y);
  page.click(x, y);
  assert_eq!(page.selected().as_deref(), Some("storage"));
  page.assert_highlight_covers(from, to);
}

#[test]
fn a_click_after_a_selection_clears_it_and_a_drag_from_there_selects_again() {
  let mut page = page(MarkdownProps::new(REPORTED), 560.0);
  let (from, to) = page.inks_of("design marks", 0);
  page.drag_inks(from, to);
  let (_, planned) = page.inks_of("planned", 0);
  let (x, y) = page.after(planned);
  page.click(x, y);
  assert_eq!(page.selected(), None, "a click collapses the selection");
  let (_, variable) = page.inks_of("variable", 0);
  page.drag_inks(planned + 1, variable);
  assert_eq!(
    page.selected().as_deref(),
    Some(&*page.text_between(planned + 1, variable))
  );
}
