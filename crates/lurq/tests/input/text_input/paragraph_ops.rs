//! The kinds of a styled input's paragraphs follow an edit: a paragraph that
//! is split gives its kind to both halves, paragraphs that are joined take
//! the kind of the first.
use ParagraphKind::{Body, Bullet, Heading, Ordered, Quote};

use super::*;

/// The kinds after replacing `replaced` of `before` with `inserted`.
fn edit(kinds: &[ParagraphKind], before: &str, replaced: Range<usize>, inserted: &str) -> Vec<ParagraphKind> {
  let mut after = before.to_owned();
  after.replace_range(replaced.clone(), inserted);
  splice_kinds(
    kinds,
    &paragraph_ranges(before),
    &paragraph_ranges(&after),
    replaced,
    inserted.len(),
  )
}

#[test]
fn paragraphs_are_the_lines_of_a_text_and_one_after_a_last_break() {
  assert_eq!(paragraph_ranges(""), vec![0..0]);
  assert_eq!(paragraph_ranges("ab"), vec![0..2]);
  assert_eq!(paragraph_ranges("ab\ncd"), vec![0..2, 3..5]);
  assert_eq!(paragraph_ranges("ab\n"), vec![0..2, 3..3]);
  assert_eq!(paragraph_ranges("ab\r\ncd"), vec![0..2, 4..6]);
  let ranges = paragraph_ranges("ab\ncd\n\nef");
  assert_eq!(ranges, vec![0..2, 3..5, 6..6, 7..9]);
  // The caret at the end of a paragraph is in it, and at the start of the next in the next.
  assert_eq!(paragraph_of(&ranges, 2), 0);
  assert_eq!(paragraph_of(&ranges, 3), 1);
  assert_eq!(paragraph_of(&ranges, 6), 2);
  assert_eq!(paragraph_of(&ranges, 9), 3);
}

#[test]
fn paragraph_kinds_are_one_for_each_paragraph_with_a_level_a_heading_has() {
  assert_eq!(
    normalize_kinds(vec![Heading(9), Bullet, Quote], 2),
    vec![Heading(6), Bullet]
  );
  assert_eq!(normalize_kinds(vec![Heading(0)], 3), vec![Heading(1), Body, Body]);
}

#[test]
fn paragraph_split_gives_its_kind_to_both_halves() {
  let kinds = [Heading(2), Bullet, Body];
  let text = "Title\nitem\nend";
  // Enter in the middle of the item: two items.
  assert_eq!(edit(&kinds, text, 8..8, "\n"), vec![Heading(2), Bullet, Bullet, Body]);
  // Enter at the start of the heading: an empty heading above it.
  assert_eq!(
    edit(&kinds, text, 0..0, "\n"),
    vec![Heading(2), Heading(2), Bullet, Body]
  );
  // Pasting three lines into the last paragraph: all of its kind.
  assert_eq!(
    edit(&kinds, text, 14..14, "a\nb\nc"),
    vec![Heading(2), Bullet, Body, Body, Body]
  );
}

#[test]
fn paragraphs_joined_take_the_kind_of_the_first() {
  let kinds = [Heading(1), Body, Ordered, Quote];
  let text = "Title\nbody\none\nquote";
  // Backspace at the start of the body: it joins the heading.
  assert_eq!(edit(&kinds, text, 5..6, ""), vec![Heading(1), Ordered, Quote]);
  // A selection from the body into the quote, deleted: the body's kind stays.
  assert_eq!(edit(&kinds, text, 8..17, ""), vec![Heading(1), Body]);
  // Typed over: what is typed is of the first paragraph's kind, its lines too.
  assert_eq!(edit(&kinds, text, 8..17, "x\ny"), vec![Heading(1), Body, Body]);
  // Everything replaced: the first paragraph's kind is what is left.
  assert_eq!(edit(&kinds, text, 0..text.len(), "new"), vec![Heading(1)]);
}

#[test]
fn paragraph_kinds_do_not_change_with_an_edit_inside_a_paragraph() {
  let kinds = [Bullet, Quote];
  assert_eq!(edit(&kinds, "one\ntwo", 1..2, "xyz"), vec![Bullet, Quote]);
  assert_eq!(edit(&kinds, "one\ntwo", 7..7, "!"), vec![Bullet, Quote]);
}

#[test]
fn paragraph_styles_give_each_kind_its_type_over_the_inputs() {
  let base = TextStyle {
    font_size: 14.0,
    line_height: 1.5,
    ..TextStyle::default()
  };
  let styles = ParagraphStyles::new()
    .gap(10.0)
    .with(
      Heading(2),
      ParagraphStyle::new().font_size(22.0).weight(FontWeight::SemiBold),
    )
    .with(Bullet, ParagraphStyle::new().indent(21.0));
  let heading = styles.of(Heading(2)).text(&base);
  assert_eq!((heading.font_size, heading.weight), (22.0, FontWeight::SemiBold));
  assert_eq!(heading.line_height, 1.5, "what a kind does not say is the input's");
  assert_eq!(styles.of(Heading(3)).text(&base).font_size, 14.0);
  assert_eq!(styles.of(Bullet).indent, 21.0);
  assert_eq!(ParagraphStyle::new().font_size_scale(1.5).text(&base).font_size, 21.0);
}

/// The kinds after `pasted`, with a kind for each of its lines, is inserted at `at` of `before`.
fn paste(
  kinds: &[ParagraphKind],
  before: &str,
  at: usize,
  pasted: &str,
  pasted_kinds: &[ParagraphKind],
) -> Vec<ParagraphKind> {
  let moved = edit(kinds, before, at..at, pasted);
  let mut after = before.to_owned();
  after.insert_str(at, pasted);
  place_kinds(moved, &paragraph_ranges(&after), at, pasted.len(), pasted_kinds)
}

#[test]
fn text_pasted_with_kinds_of_its_own_brings_them_where_it_does_not_go_on_from_a_line() {
  // One paragraph goes into the line it is put in, and gives its kind only to a line that holds nothing else.
  assert_eq!(paste(&[Body], "abcd", 2, "x", &[Heading(1)]), [Body]);
  assert_eq!(paste(&[Body], "cd", 0, "x", &[Heading(1)]), [Body]);
  assert_eq!(paste(&[Body, Body], "ab\n", 3, "x", &[Heading(1)]), [Body, Heading(1)]);
  // Of several, the first goes on from the text before it, the last gives its kind to the text after it.
  assert_eq!(
    paste(&[Body], "abcd", 2, "x\ny\nz", &[Heading(1), Quote, Bullet]),
    [Body, Quote, Bullet]
  );
  // With nothing before it the first is what it was copied as.
  assert_eq!(
    paste(&[Body], "cd", 0, "x\ny", &[Heading(1), Bullet]),
    [Heading(1), Bullet]
  );
  // A last one that is empty, of text that ended in a line break, leaves the text after it what it was.
  assert_eq!(
    paste(&[Quote], "cd", 0, "x\n", &[Heading(1), Body]),
    [Heading(1), Quote]
  );
  // Kinds that are not one for each pasted line are left out.
  assert_eq!(paste(&[Quote], "cd", 0, "x\ny", &[Heading(1)]), [Quote, Quote]);
  // A heading's level is kept within 1 to 6.
  assert_eq!(paste(&[Body], "", 0, "x", &[Heading(9)]), [Heading(6)]);
}
