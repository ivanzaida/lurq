//! Kinds of paragraphs of a styled input: kept with the text through edits
//! and undo, set from the controller, and drawn in each kind's type, with its
//! indent, its marker and the room between paragraphs.
use lurq::{
  components::{
    ParagraphKind::{self, Body, Bullet, Heading, Ordered, Quote},
    ParagraphMarker, ParagraphStyle, ParagraphStyles, RichText, SpanStyle, StyledEdit, TextMark,
  },
  core::Signal,
  layout::text_style::FontWeight,
  node::color::Color,
};

use super::{Editor, ID, bold, span};
use crate::support::{GlyphSnapshot, RenderSnapshot};

const DOT: Color = Color::new(200, 30, 160, 255);
const BAR: Color = Color::new(20, 200, 90, 255);
const NUMBER: Color = Color::new(30, 90, 220, 255);
const HEADING: Color = Color::new(180, 60, 20, 255);
const INDENT: f32 = 24.0;
const GAP: f32 = 10.0;

fn looks() -> ParagraphStyles {
  ParagraphStyles::new()
    .gap(GAP)
    .with(
      Heading(2),
      ParagraphStyle::new()
        .font_size(22.0)
        .weight(FontWeight::SemiBold)
        .color(HEADING),
    )
    .with(
      Bullet,
      ParagraphStyle::new().indent(INDENT).marker(ParagraphMarker::Dot {
        size: 5.0,
        gap: 10.0,
        color: DOT.into(),
      }),
    )
    .with(
      Ordered,
      ParagraphStyle::new().indent(INDENT).marker(ParagraphMarker::Number {
        gap: 6.0,
        color: NUMBER.into(),
      }),
    )
    .with(
      Quote,
      ParagraphStyle::new().indent(INDENT).marker(ParagraphMarker::Bar {
        width: 3.0,
        color: BAR.into(),
      }),
    )
}

/// An editor of `text` whose paragraphs are `kinds`, and the signal the kinds are kept in.
fn document(text: &str, kinds: Vec<ParagraphKind>) -> (Editor, Signal<Vec<ParagraphKind>>) {
  let kinds = Signal::new(kinds);
  let bound = kinds.clone();
  let editor = Editor::with(text, vec![], move |input| input.paragraphs(bound.clone(), looks()));
  (editor, kinds)
}

fn kinds_of(editor: &Editor) -> Vec<ParagraphKind> {
  editor
    .controller
    .rich_text()
    .unwrap()
    .expect("a styled input")
    .paragraphs
}

/// The glyphs of a color, from left to right.
fn glyphs(snapshot: &RenderSnapshot, color: Color) -> Vec<&GlyphSnapshot> {
  let mut found: Vec<_> = snapshot
    .glyphs
    .iter()
    .filter(|glyph| glyph.color == color.to_linear_f32_array())
    .collect();
  found.sort_by(|a, b| a.x.total_cmp(&b.x));
  found
}

fn origin(editor: &mut Editor) -> (f32, f32) {
  let rect = editor.tree.get_element_by_id_mut(ID).unwrap().bounds().unwrap();
  (rect.x, rect.y)
}

#[test]
fn styled_paragraphs_split_and_join_with_the_text() {
  let (mut editor, kinds) = document("Title\nitem\nend", vec![Heading(2), Bullet, Body]);
  editor.select(8, 8);
  editor.key("Enter");
  assert_eq!(editor.value.get(), "Title\nit\nem\nend");
  assert_eq!(
    kinds.get(),
    vec![Heading(2), Bullet, Bullet, Body],
    "an item split is two items"
  );
  // Backspace at the start of the second item joins it to the first.
  editor.key("Backspace");
  assert_eq!(kinds.get(), vec![Heading(2), Bullet, Body]);
  // The body joined to the item becomes part of the item.
  editor.select(11, 11);
  editor.key("Backspace");
  assert_eq!(editor.value.get(), "Title\nitemend");
  assert_eq!(kinds.get(), vec![Heading(2), Bullet]);
  assert_eq!(
    kinds_of(&editor),
    kinds.get(),
    "the controller reads what the signal holds"
  );
}

#[test]
fn styled_paragraphs_are_set_over_the_selection_as_one_undo_step() {
  let (mut editor, kinds) = document("one\ntwo\nthree", vec![]);
  assert_eq!(kinds_of(&editor), vec![Body, Body, Body], "one kind for each paragraph");
  // The caret alone: its paragraph.
  editor.select(5, 5);
  assert!(editor.controller.set_paragraphs(Heading(2)).unwrap());
  assert_eq!(kinds.get(), vec![Body, Heading(2), Body]);
  assert_eq!(editor.controller.caret_paragraph().unwrap(), Some((Heading(2), 4..7)));
  assert!(
    !editor.controller.set_paragraphs(Heading(2)).unwrap(),
    "it is one already"
  );
  // A selection: every paragraph it touches.
  editor.select(1, 9);
  assert_eq!(
    editor.controller.selected_paragraphs().unwrap(),
    Some(vec![Body, Heading(2), Body])
  );
  assert!(editor.controller.set_paragraphs(Bullet).unwrap());
  assert_eq!(kinds.get(), vec![Bullet, Bullet, Bullet]);
  let changes = editor.changes.lock().unwrap().clone();
  assert_eq!(changes.last().unwrap().1, StyledEdit::Paragraphs { range: 0..3 });
  drop(changes);
  // Undo takes back one setting at a time, with the text as it was.
  assert!(editor.controller.can_undo());
  assert!(editor.controller.undo().unwrap());
  assert_eq!(kinds.get(), vec![Body, Heading(2), Body]);
  assert!(editor.controller.undo().unwrap());
  assert_eq!(kinds.get(), vec![Body, Body, Body]);
  assert!(!editor.controller.can_undo());
  assert!(editor.controller.can_redo());
  assert!(editor.controller.redo().unwrap());
  assert_eq!(kinds.get(), vec![Body, Heading(2), Body]);
  assert_eq!(editor.value.get(), "one\ntwo\nthree");
}

#[test]
fn styled_undo_of_a_text_edit_puts_the_kinds_back_with_the_text() {
  let (mut editor, kinds) = document("Title\nbody", vec![Heading(2), Body]);
  editor.select(5, 6);
  editor.key("Backspace");
  assert_eq!(
    (editor.value.get().as_str(), kinds.get()),
    ("Titlebody", vec![Heading(2)])
  );
  editor.shortcut("z", "KeyZ", false);
  assert_eq!(editor.value.get(), "Title\nbody");
  assert_eq!(kinds.get(), vec![Heading(2), Body]);
}

#[test]
fn styled_change_reports_the_kinds_with_the_text() {
  let kinds = Signal::new(vec![Bullet]);
  let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::<RichText>::new()));
  let (bound, log) = (kinds.clone(), seen.clone());
  let mut editor = Editor::with("item", vec![], move |input| {
    let log = log.clone();
    input
      .paragraphs(bound.clone(), looks())
      .on_styled_change(move |change| log.lock().unwrap().push(change.to_rich_text()))
  });
  editor.key("Enter");
  let seen = seen.lock().unwrap();
  assert_eq!(seen.len(), 1);
  assert_eq!(seen[0], RichText::new("item\n", vec![], vec![Bullet, Bullet]));
}

#[test]
fn styled_an_app_write_of_the_kinds_is_drawn_and_not_reported() {
  let (mut editor, kinds) = document("one\ntwo", vec![]);
  let before = editor.render();
  kinds.set(vec![Body, Bullet]);
  let after = editor.render();
  assert!(before.rects.iter().all(|rect| rect.color != DOT));
  assert_eq!(after.rects.iter().filter(|rect| rect.color == DOT).count(), 1);
  assert!(editor.changes.lock().unwrap().is_empty());
}

#[test]
fn styled_paragraphs_draw_each_kind_in_its_type_with_room_between() {
  let (mut editor, _) = document("Hg\nHg", vec![Heading(2), Body]);
  let snapshot = editor.render();
  let (_, top) = origin(&mut editor);
  let heading = glyphs(&snapshot, HEADING);
  assert_eq!(heading.len(), 2, "the heading's letters take its color");
  let body: Vec<_> = snapshot
    .glyphs
    .iter()
    .filter(|glyph| glyph.color != HEADING.to_linear_f32_array())
    .collect();
  assert_eq!(body.len(), 2);
  assert!(
    heading[0].height > body[0].height * 1.3,
    "a heading of 22 is drawn larger than a body of 14: {} and {}",
    heading[0].height,
    body[0].height
  );
  // The heading's row is 22 * 1.4 high, the room between is 10, and the body's row follows.
  let body_top = body.iter().map(|glyph| glyph.y).fold(f32::MAX, f32::min);
  let heading_row = 22.0 * 1.4;
  assert!(
    body_top > top + heading_row + GAP - 1.0 && body_top < top + heading_row + GAP + 14.0 * 1.4,
    "the body starts under the heading's row and the gap: {body_top} from {top}"
  );
}

#[test]
fn styled_paragraphs_indent_an_item_and_draw_its_dot_before_it() {
  let (mut editor, _) = document("ab\nab", vec![Body, Bullet]);
  let snapshot = editor.render();
  let (left, top) = origin(&mut editor);
  let mut letters: Vec<_> = snapshot.glyphs.iter().collect();
  letters.sort_by(|a, b| a.y.total_cmp(&b.y).then(a.x.total_cmp(&b.x)));
  let (plain, item) = (letters[0], letters[2]);
  assert!(
    (item.x - plain.x - INDENT).abs() < 0.6,
    "the item's text starts {INDENT} after the body's: {} and {}",
    item.x,
    plain.x
  );
  let dot = snapshot
    .rects
    .iter()
    .find(|rect| rect.color == DOT)
    .expect("the item's dot");
  assert_eq!((dot.width, dot.height), (5.0, 5.0));
  assert_eq!(dot.radii, [2.5; 4], "a dot is round");
  // It ends 10 before the text and is centred on the item's row, the second: 14 * 1.4 high, 10 under the first.
  assert!((dot.x - (left + INDENT - 10.0 - 5.0)).abs() < 0.6, "{}", dot.x);
  let row = 14.0 * 1.4;
  let middle = top + row + GAP + row / 2.0;
  assert!((dot.y + 2.5 - middle).abs() < 0.6, "{} and {middle}", dot.y);
}

#[test]
fn styled_paragraphs_number_the_items_of_a_run_and_start_again_after_it() {
  let (mut editor, _) = document("a\na\nb\na", vec![Ordered, Ordered, Body, Ordered]);
  let snapshot = editor.render();
  let (left, _) = origin(&mut editor);
  let digits = glyphs(&snapshot, NUMBER);
  // "1." "2." and, after the body, "1." again: six glyphs, each number ending 6 before the text.
  assert_eq!(digits.len(), 6);
  // A digit and its full stop stand on one baseline.
  let mut rows: Vec<f32> = digits.iter().map(|glyph| glyph.y + glyph.height).collect();
  rows.sort_by(f32::total_cmp);
  rows.dedup_by(|a, b| (*a - *b).abs() < 3.0);
  assert_eq!(rows.len(), 3, "a number for each item");
  let right = digits
    .iter()
    .map(|glyph| glyph.x + glyph.width)
    .fold(f32::MIN, f32::max);
  assert!(
    // A glyph's box reaches a pixel or two past its advance.
    right <= left + INDENT - 6.0 + 2.5 && right > left + INDENT - 6.0 - 4.0,
    "the numbers end before the text: {right} from {left}"
  );
}

#[test]
fn styled_paragraphs_draw_one_bar_down_a_run_of_quotes_and_the_room_between_them() {
  let (mut editor, _) = document("one\ntwo\nend\nlast", vec![Quote, Quote, Body, Quote]);
  let snapshot = editor.render();
  let (left, top) = origin(&mut editor);
  let mut bars: Vec<_> = snapshot.rects.iter().filter(|rect| rect.color == BAR).collect();
  bars.sort_by(|a, b| a.y.total_cmp(&b.y));
  assert_eq!(
    bars.len(),
    2,
    "one for the two quotes, one for the quote after the body"
  );
  let row = 14.0 * 1.4;
  assert!((bars[0].x - left).abs() < 0.6 && bars[0].width == 3.0);
  assert!((bars[0].y - top).abs() < 0.6);
  assert!((bars[0].height - (row + GAP + row)).abs() < 0.6, "{}", bars[0].height);
  assert!((bars[1].y - (top + 3.0 * (row + GAP))).abs() < 0.6, "{}", bars[1].y);
  assert!((bars[1].height - row).abs() < 0.6);
}

#[test]
fn styled_paragraphs_fill_a_run_of_code_lines_as_one_block_with_no_room_between() {
  let kinds = Signal::new(vec![Body, ParagraphKind::Code, ParagraphKind::Code, Body]);
  let bound = kinds.clone();
  let mut editor = Editor::with("a\nb\nc\nd", vec![], move |input| {
    input.paragraphs(
      bound.clone(),
      looks().with(
        ParagraphKind::Code,
        ParagraphStyle::new().indent(12.0).background(BAR, 6.0).joined(true),
      ),
    )
  });
  let snapshot = editor.render();
  let (left, top) = origin(&mut editor);
  let fills: Vec<_> = snapshot.rects.iter().filter(|rect| rect.color == BAR).collect();
  assert_eq!(fills.len(), 1, "one fill behind both lines");
  let row = 14.0 * 1.4;
  assert!((fills[0].x - left).abs() < 0.6);
  assert!(
    (fills[0].width - 420.0).abs() < 0.6,
    "across the input: {}",
    fills[0].width
  );
  assert!((fills[0].y - (top + row + GAP)).abs() < 0.6);
  assert!((fills[0].height - 2.0 * row).abs() < 0.6, "{}", fills[0].height);
  assert_eq!(fills[0].radii, [6.0; 4]);
}

#[test]
fn styled_caret_and_pointer_follow_an_indented_paragraph() {
  let (mut editor, _) = document("ab\nab", vec![Body, Bullet]);
  editor.render();
  let plain = editor.controller.caret_point(0).unwrap();
  let item = editor.controller.caret_point(3).unwrap();
  assert!(
    (item.x - plain.x - INDENT).abs() < 0.6,
    "the caret stands after the indent"
  );
  assert!(item.y > plain.y + GAP, "under the first row and the room between");
  // A press in the room before the item's text puts the caret at its start.
  let (left, top) = origin(&mut editor);
  let y = top + 14.0 * 1.4 + GAP + 8.0;
  crate::support::pointer_click(&mut editor.tree, left + 4.0, y, lurq::app::events::MouseButton::Left);
  assert_eq!(editor.controller.selection().unwrap().caret, 3);
  // ArrowUp from the item goes to the body over the room between them.
  editor.key("ArrowUp");
  assert!(editor.controller.selection().unwrap().caret <= 2);
}

#[test]
fn styled_a_mark_in_a_heading_is_drawn_over_the_headings_type() {
  let kinds = Signal::new(vec![Heading(2), Body]);
  let bound = kinds.clone();
  let mut editor = Editor::with("ab\nab", vec![span(0..5, bold())], move |input| {
    input.paragraphs(bound.clone(), looks())
  });
  let snapshot = editor.render();
  let heading = glyphs(&snapshot, HEADING);
  assert_eq!(heading.len(), 2, "a bold span keeps the heading's color and size");
  let body: Vec<_> = snapshot
    .glyphs
    .iter()
    .filter(|glyph| glyph.color != HEADING.to_linear_f32_array())
    .collect();
  assert!(heading[0].height > body[0].height * 1.3);
  assert_eq!(editor.spans(), vec![span(0..5, bold())], "the span itself is one");
}

#[test]
fn styled_underline_is_a_mark_toggled_and_drawn_as_a_line_under_its_text() {
  let mut editor = Editor::new("under it", vec![]);
  editor.select(0, 5);
  assert!(editor.controller.toggle_mark(TextMark::Underline).unwrap());
  assert_eq!(editor.spans(), vec![span(0..5, SpanStyle::new().underline())]);
  let style = editor.controller.selection_style().unwrap().unwrap();
  assert_eq!(style.underline, lurq::components::MarkPresence::All);
  let snapshot = editor.render();
  let glyph_bottom = snapshot
    .glyphs
    .iter()
    .map(|glyph| glyph.y + glyph.height)
    .fold(f32::MIN, f32::max);
  let line = snapshot
    .rects
    .iter()
    .find(|rect| rect.height <= 2.5 && rect.width > 20.0 && rect.y > glyph_bottom - 6.0)
    .expect("a line under the underlined text");
  assert!(line.width < 60.0, "under its five letters only: {}", line.width);
  assert!(editor.controller.toggle_mark(TextMark::Underline).unwrap());
  assert!(editor.spans().is_empty());
}

#[test]
fn styled_kinds_are_drawn_where_the_lines_wrap_and_are_not_justified() {
  use lurq::{components::TextInput, layout::text_style::TextAlign};
  let dots = |snapshot: &RenderSnapshot| snapshot.rects.iter().filter(|rect| rect.color == DOT).count();
  let build = |configure: fn(TextInput) -> TextInput| {
    let kinds = Signal::new(vec![Heading(2), Bullet, Bullet]);
    let bound = kinds.clone();
    let editor = Editor::with("Title\none\ntwo", vec![], move |input| {
      configure(input.paragraphs(bound.clone(), looks()))
    });
    (editor, kinds)
  };
  // Where the lines wrap, each kind is drawn: two dots.
  let (mut wrapped, _) = build(|input| input);
  assert_eq!(dots(&wrapped.render()), 2);
  // A justified input and one whose lines do not wrap draw every paragraph as body text.
  for configure in [
    (|input| input.text_align(TextAlign::Justified)) as fn(TextInput) -> TextInput,
    |input| input.wrap(false),
  ] {
    let (mut plain, kinds) = build(configure);
    let snapshot = plain.render();
    assert_eq!(dots(&snapshot), 0);
    assert!(
      glyphs(&snapshot, HEADING).is_empty(),
      "the heading is in the input's own type"
    );
    // The kinds are still kept, moved by an edit and set from the controller.
    plain.select(5, 5);
    plain.key("Enter");
    assert_eq!(kinds.get(), [Heading(2), Heading(2), Bullet, Bullet]);
    assert!(plain.controller.set_paragraphs(Quote).unwrap());
    assert_eq!(kinds.get(), [Heading(2), Quote, Bullet, Bullet]);
  }
}

#[test]
fn styled_a_row_of_a_kind_is_as_high_as_its_line_says_whatever_the_font() {
  // A line lower than the font's own: 14 on a line of 1, where the font's ascent and descent are more.
  let kinds = Signal::new(vec![Body, Body, Body]);
  let bound = kinds.clone();
  let looks = ParagraphStyles::new().with(Body, ParagraphStyle::new().line_height(1.0));
  let editor = Editor::with("one\ntwo\nthree", vec![], move |input| {
    input.paragraphs(bound.clone(), looks.clone())
  });
  let rows: Vec<f32> = [0, 4, 8]
    .into_iter()
    .map(|byte| editor.controller.caret_point(byte).unwrap().y)
    .collect();
  assert!((rows[1] - rows[0] - 14.0).abs() < 0.01, "{rows:?}");
  assert!((rows[2] - rows[1] - 14.0).abs() < 0.01, "{rows:?}");
}

#[test]
fn styled_an_empty_input_with_kinds_draws_its_placeholder_as_its_first_paragraph() {
  let kinds = Signal::new(vec![Bullet]);
  let bound = kinds.clone();
  let mut editor = Editor::with("", vec![], move |input| {
    input.paragraphs(bound.clone(), looks()).placeholder("An item")
  });
  let snapshot = editor.render();
  // The item's dot is drawn, and the caret stands after its indent.
  assert_eq!(snapshot.rects.iter().filter(|rect| rect.color == DOT).count(), 1);
  assert!((editor.controller.caret_point(0).unwrap().x - INDENT).abs() < 0.01);
  // The placeholder starts where the item's text would: its first glyph where that of the same words typed is (a
  // glyph's box hangs a little before its pen, so the two are compared with each other, not with the indent).
  let (x, _) = origin(&mut editor);
  let first =
    |snapshot: &crate::support::RenderSnapshot| snapshot.glyphs.iter().map(|glyph| glyph.x).fold(f32::MAX, f32::min);
  let placeholder = first(&snapshot);
  let kinds = Signal::new(vec![Bullet]);
  let mut typed = Editor::with("An item", vec![], move |input| input.paragraphs(kinds.clone(), looks()));
  let (typed_x, _) = origin(&mut typed);
  let text = first(&typed.render());
  assert!(
    ((placeholder - x) - (text - typed_x)).abs() < 0.5,
    "the placeholder at {placeholder}, the text at {text}"
  );
  assert!(placeholder - x > INDENT - 3.0, "after the indent: {placeholder}");
}
