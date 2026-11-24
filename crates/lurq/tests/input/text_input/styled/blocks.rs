//! The room a kind of paragraph keeps between its items and round its run, the type of a number of its own, the
//! room a mark keeps round its text, and the blocks of a checklist and a divider.
use lurq::{
  app::theme::ThemeTextMarks,
  components::{
    ParagraphKind::{self, Body, Bullet, Checklist, Divider, Ordered, Quote},
    ParagraphMarker, ParagraphStyle, ParagraphStyles, SpanStyle,
  },
  core::Signal,
  node::color::Color,
};

use super::{Editor, span};
use crate::support::RenderSnapshot;

/// A row of the input's type: 14 on a line of 1.4.
const ROW: f32 = 14.0 * 1.4;
const GAP: f32 = 10.0;
const BAR: Color = Color::new(20, 200, 90, 255);
const NUMBER: Color = Color::new(30, 90, 220, 255);
const FILL: Color = Color::new(40, 40, 200, 255);
const EDGE: Color = Color::new(200, 40, 40, 255);
const LINE: Color = Color::new(30, 160, 30, 255);

const DONE: ParagraphKind = Checklist { checked: true };
const TODO: ParagraphKind = Checklist { checked: false };

fn looks() -> ParagraphStyles {
  let check = ParagraphMarker::Check {
    size: 14.0,
    gap: 8.0,
    radius: 3.0,
    edge_width: 1.0,
    edge: EDGE.into(),
    fill: FILL.into(),
    tick: Color::new(255, 255, 255, 255).into(),
    icon: None,
  };
  ParagraphStyles::new()
    .gap(GAP)
    .with(Bullet, ParagraphStyle::new().indent(24.0).item_gap(4.0))
    .with(
      Ordered,
      ParagraphStyle::new()
        .indent(40.0)
        .marker(ParagraphMarker::Number {
          gap: 6.0,
          color: NUMBER.into(),
        })
        .marker_font_size(24.0),
    )
    .with(
      Quote,
      ParagraphStyle::new()
        .indent(16.0)
        .padding_y(6.0, 8.0)
        .marker(ParagraphMarker::Bar {
          width: 3.0,
          color: BAR.into(),
        }),
    )
    .with(TODO, ParagraphStyle::new().indent(24.0).marker(check))
    .with(
      Divider,
      ParagraphStyle::new().marker(ParagraphMarker::Rule {
        thickness: 1.0,
        color: LINE.into(),
      }),
    )
}

/// An editor of `text` whose paragraphs are `kinds`, a press on a box checking it, and the signal of the kinds.
fn document(text: &str, kinds: Vec<ParagraphKind>) -> (Editor, Signal<Vec<ParagraphKind>>) {
  let kinds = Signal::new(kinds);
  let bound = kinds.clone();
  let editor = Editor::with(text, vec![], move |input| {
    input.paragraphs(bound.clone(), looks()).checks_on_press(true)
  });
  (editor, kinds)
}

/// The top of the row of the caret at `byte`.
fn top(editor: &Editor, byte: usize) -> f32 {
  editor.controller.caret_point(byte).unwrap().y
}

fn near(a: f32, b: f32) -> bool {
  (a - b).abs() < 0.6
}

/// The boxes of the items, from the top: where each is, and whether it is filled. A box's edge is drawn as a
/// rectangle of its own over its fill.
fn boxes(snapshot: &RenderSnapshot) -> Vec<(f32, f32, bool)> {
  let mut boxes: Vec<_> = snapshot
    .rects
    .iter()
    .filter(|rect| rect.stroke_color == EDGE && rect.stroke.iter().any(|side| *side > 0.0))
    .map(|edge| {
      let filled = snapshot
        .rects
        .iter()
        .any(|rect| rect.color == FILL && near(rect.x, edge.x) && near(rect.y, edge.y));
      (edge.x, edge.y, filled)
    })
    .collect();
  boxes.sort_by(|a, b| a.1.total_cmp(&b.1));
  boxes
}

#[test]
fn styled_items_of_a_list_stand_closer_than_the_blocks_round_them() {
  let (editor, _) = document("a\nb\nc\nd", vec![Body, Bullet, Bullet, Body]);
  let pitch = |from: usize, to: usize| top(&editor, to) - top(&editor, from);
  assert!(near(pitch(0, 2), ROW + GAP), "{}", pitch(0, 2));
  assert!(near(pitch(2, 4), ROW + 4.0), "between two items: {}", pitch(2, 4));
  assert!(near(pitch(4, 6), ROW + GAP), "{}", pitch(4, 6));
}

#[test]
fn styled_a_run_keeps_room_above_and_under_it_inside_its_bar() {
  let (mut editor, _) = document("a\nq1\nq2\nb", vec![Body, Quote, Quote, Body]);
  let pitch = |editor: &Editor, from: usize, to: usize| top(editor, to) - top(editor, from);
  assert!(near(pitch(&editor, 0, 2), ROW + GAP + 6.0), "{}", pitch(&editor, 0, 2));
  assert!(
    near(pitch(&editor, 2, 5), ROW + GAP),
    "none between two quotes: {}",
    pitch(&editor, 2, 5)
  );
  assert!(near(pitch(&editor, 5, 8), ROW + 8.0 + GAP), "{}", pitch(&editor, 5, 8));
  let snapshot = editor.render();
  let bar = snapshot
    .rects
    .iter()
    .find(|rect| rect.color == BAR)
    .expect("the quote's bar");
  let height = 6.0 + ROW + GAP + ROW + 8.0;
  assert!(near(bar.height, height), "the bar holds its room: {}", bar.height);
}

#[test]
fn styled_a_number_in_a_type_of_its_own_stands_on_the_items_baseline() {
  let (mut editor, _) = document("ab", vec![Ordered]);
  let snapshot = editor.render();
  let number = snapshot
    .glyphs
    .iter()
    .filter(|glyph| glyph.color == NUMBER.to_linear_f32_array())
    .min_by(|a, b| a.x.total_cmp(&b.x))
    .expect("the number");
  let letter = snapshot
    .glyphs
    .iter()
    .filter(|glyph| glyph.color != NUMBER.to_linear_f32_array())
    .max_by(|a, b| a.x.total_cmp(&b.x))
    .expect("the item's text");
  assert!(
    number.height > letter.height * 1.3,
    "the number is the larger type: {} against {}",
    number.height,
    letter.height
  );
  let (number_foot, letter_foot) = (number.y + number.height, letter.y + letter.height);
  assert!(
    (number_foot - letter_foot).abs() < 1.5,
    "on one baseline: {number_foot} against {letter_foot}"
  );
}

#[test]
fn styled_a_press_on_the_box_of_an_item_checks_it_as_one_undo_step() {
  let (mut editor, kinds) = document("todo\ndone", vec![TODO, DONE]);
  let snapshot = editor.render();
  let drawn = boxes(&snapshot);
  assert_eq!(drawn.len(), 2, "a box before each item");
  assert!(!drawn[0].2, "an item not checked is not filled");
  assert!(drawn[1].2, "a checked one is");
  let (x, y) = (drawn[0].0 + 7.0, drawn[0].1 + 7.0);
  editor.select(7, 7);
  crate::support::pointer_click(&mut editor.tree, x, y, lurq::app::events::MouseButton::Left);
  assert_eq!(kinds.get(), vec![DONE, DONE]);
  assert_eq!(
    editor.controller.selection().unwrap().caret,
    7,
    "the caret stays where it was"
  );
  assert!(boxes(&editor.render())[0].2, "it is filled now");
  assert!(editor.controller.undo().unwrap());
  assert_eq!(kinds.get(), vec![TODO, DONE]);
  // From the controller: every item the selection touches is checked, then unchecked where all are.
  editor.select(0, 9);
  assert!(editor.controller.toggle_checks().unwrap());
  assert_eq!(kinds.get(), vec![DONE, DONE]);
  assert!(editor.controller.toggle_checks().unwrap());
  assert_eq!(kinds.get(), vec![TODO, TODO]);
}

#[test]
fn styled_a_box_is_not_checked_by_a_press_unless_the_input_says_so() {
  let kinds = Signal::new(vec![TODO]);
  let bound = kinds.clone();
  let mut editor = Editor::with("todo", vec![], move |input| input.paragraphs(bound.clone(), looks()));
  let snapshot = editor.render();
  let (x, y, _) = boxes(&snapshot)[0];
  crate::support::pointer_click(&mut editor.tree, x + 7.0, y + 7.0, lurq::app::events::MouseButton::Left);
  assert_eq!(kinds.get(), vec![TODO]);
}

#[test]
fn styled_what_an_edit_splits_off_a_checked_item_is_not_checked() {
  let (mut editor, kinds) = document("done", vec![DONE]);
  editor.select(4, 4);
  editor.key("Enter");
  assert_eq!(kinds.get(), vec![DONE, TODO]);
}

#[test]
fn styled_a_divider_is_a_line_across_the_input_and_text_typed_in_it_makes_it_body_text() {
  let (mut editor, kinds) = document("a\n\nb", vec![Body, Divider, Body]);
  let snapshot = editor.render();
  let line = snapshot
    .rects
    .iter()
    .find(|rect| rect.color == LINE)
    .expect("the divider's line");
  assert!(line.width > 300.0, "across the input: {}", line.width);
  assert!(near(line.height, 1.0));
  editor.select(2, 2);
  editor.type_text("x");
  assert_eq!(kinds.get(), vec![Body, Body, Body]);
  assert_eq!(editor.value.get(), "a\nx\nb");
}

#[test]
fn styled_a_divider_is_put_in_after_the_caret_paragraph_as_one_undo_step() {
  let (mut editor, kinds) = document("one", vec![Body]);
  editor.select(3, 3);
  assert!(editor.controller.insert_divider().unwrap());
  assert_eq!(editor.value.get(), "one\n\n");
  assert_eq!(kinds.get(), vec![Body, Divider, Body]);
  assert_eq!(
    editor.controller.selection().unwrap().caret,
    5,
    "in the paragraph after it"
  );
  assert!(editor.controller.undo().unwrap());
  assert_eq!((editor.value.get().as_str(), kinds.get()), ("one", vec![Body]));
  // An empty paragraph is made the divider.
  let (mut editor, kinds) = document("", vec![Body]);
  assert!(editor.controller.insert_divider().unwrap());
  assert_eq!(editor.value.get(), "\n");
  assert_eq!(kinds.get(), vec![Divider, Body]);
}

#[test]
fn styled_a_marks_padding_moves_the_text_beside_it() {
  let code = vec![span(0..2, SpanStyle::new().code())];
  let after = |editor: &Editor| editor.controller.caret_point(3).unwrap().x;
  let plain = Editor::new("ab cd", code.clone());
  let mut marks = ThemeTextMarks::default();
  marks.code.padding = 5.0;
  let padded = Editor::with("ab cd", code, move |input| input.text_marks(marks.clone()));
  assert!(
    near(after(&padded) - after(&plain), 10.0),
    "the room before and after the code: {}",
    after(&padded) - after(&plain)
  );
}
