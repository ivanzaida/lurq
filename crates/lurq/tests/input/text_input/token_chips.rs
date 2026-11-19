use lurq::{
  app::{App, Tree, events::MouseButton},
  components::{InputToken, InputTokenStyle, InputTokens, TextInput, TextInputController},
  core::Signal,
  layout::text_style::TextStyle,
  node::color::Color,
};

use crate::support::{RenderSnapshot, pointer_click, render_pass_with_app};

const SOURCE: &str = "ab wgpu cd";
const TOKEN: std::ops::Range<usize> = 3..7;
const FILL: Color = Color::new(22, 44, 99, 255);
const PICKED: Color = Color::new(200, 30, 60, 255);
const INK: Color = Color::new(11, 222, 111, 255);
const CARET: Color = Color::new(255, 11, 33, 255);
const SELECTION: Color = Color::new(7, 77, 177, 255);
/// The room a chip keeps before its text, and after it for its icon.
const START: f32 = 4.0;
const END: f32 = 18.0;

fn chip() -> InputTokenStyle {
  InputTokenStyle::new(INK, FILL)
    .padding(START)
    .padding_end(END)
    .radius(3.0)
}

struct Editor {
  tree: Tree,
  app: App,
  controller: TextInputController,
}

impl Editor {
  fn new(token: InputToken) -> Self {
    let source = Signal::new(SOURCE.to_owned());
    let controller = TextInputController::new();
    let mut tree = Tree::new();
    tree.set_root(
      TextInput::styled(
        source,
        TextStyle {
          font_size: 13.0,
          line_height: 1.4,
          ..TextStyle::default()
        },
      )
      .id("editor")
      .controller(controller.clone())
      .tokens(InputTokens::new(SOURCE, vec![token]).unwrap())
      .caret_color(CARET)
      .selection_color(SELECTION)
      .width(300.0),
    );
    let mut editor = Self {
      tree,
      app: App::new(),
      controller,
    };
    editor.render();
    editor.tree.get_element_by_id_mut("editor").unwrap().focus();
    editor
  }

  fn render(&mut self) -> RenderSnapshot {
    render_pass_with_app(&mut self.tree, &mut self.app)
  }

  /// Where the caret is drawn with the caret at `byte`.
  fn caret_at(&mut self, byte: usize) -> f32 {
    self.controller.select(byte, byte).unwrap();
    self
      .render()
      .rects
      .into_iter()
      .find(|rect| rect.color == CARET)
      .unwrap()
      .x
  }
}

#[test]
fn the_caret_at_the_ends_of_an_atomic_token_stands_outside_its_chip() {
  let mut editor = Editor::new(InputToken::new(TOKEN, chip()).atomic(true));
  let snapshot = editor.render();
  let pill = snapshot.rects.iter().find(|rect| rect.color == FILL).unwrap().clone();
  let (left, right) = (pill.x, pill.x + pill.width);
  // After the token the caret is past the room the chip keeps at its end, not between its text and its icon.
  let after = editor.caret_at(TOKEN.end);
  assert!((after - right).abs() < 0.1, "caret {after}, the chip ends at {right}");
  let before = editor.caret_at(TOKEN.start);
  assert!((before - left).abs() < 0.1, "caret {before}, the chip starts at {left}");
  // A press beside the chip puts the caret at that end of it, where the caret is then drawn.
  pointer_click(&mut editor.tree, right + 1.0, pill.y + 4.0, MouseButton::Left);
  assert_eq!(editor.controller.selection().unwrap().caret, TOKEN.end);
  pointer_click(&mut editor.tree, left - 1.0, pill.y + 4.0, MouseButton::Left);
  assert_eq!(editor.controller.selection().unwrap().caret, TOKEN.start);
  // The highlight of the selected token is the chip's box: it starts and ends where the carets stand.
  editor.controller.select(TOKEN.start, TOKEN.end).unwrap();
  let highlight = editor
    .render()
    .rects
    .into_iter()
    .find(|rect| rect.color == SELECTION)
    .unwrap();
  assert!((highlight.x - left).abs() < 0.1 && (highlight.x + highlight.width - right).abs() < 0.1);
}

#[test]
fn the_caret_at_the_end_of_a_token_that_is_typed_in_stands_where_its_text_ends() {
  let mut editor = Editor::new(InputToken::new(TOKEN, chip()));
  let snapshot = editor.render();
  let pill = snapshot.rects.iter().find(|rect| rect.color == FILL).unwrap().clone();
  // What is typed after its last letter is the token's: the caret there is inside its fill, as it always was.
  let after = editor.caret_at(TOKEN.end);
  assert!((after - (pill.x + pill.width - END)).abs() < 0.1);
}

#[test]
fn a_token_in_its_selected_look_is_drawn_over_the_selection() {
  let style = chip().selected(InputTokenStyle::new(INK, PICKED).padding(START).padding_end(END));
  let mut editor = Editor::new(InputToken::new(TOKEN, style).atomic(true));
  editor.controller.select(TOKEN.start, TOKEN.end).unwrap();
  let snapshot = editor.render();
  let pill = snapshot.rects.iter().find(|rect| rect.color == PICKED).unwrap();
  let highlight = snapshot.rects.iter().find(|rect| rect.color == SELECTION).unwrap();
  assert!(
    snapshot.rects.iter().all(|rect| rect.color != FILL),
    "the token is in one look"
  );
  assert!(
    pill.order > highlight.order,
    "the selected chip is over the highlight, which would cover its fill in the band of the text"
  );
  // Its text is over both.
  let glyphs: Vec<_> = snapshot
    .glyphs
    .iter()
    .filter(|glyph| glyph.color == INK.to_linear_f32_array())
    .collect();
  assert_eq!(glyphs.len(), TOKEN.len());
  assert!(glyphs.iter().all(|glyph| glyph.order > pill.order));
  // A selection that covers part of what is around it leaves the token as it was, under nothing.
  editor.controller.select(0, 2).unwrap();
  let snapshot = editor.render();
  assert!(snapshot.rects.iter().any(|rect| rect.color == FILL));
  assert!(snapshot.rects.iter().all(|rect| rect.color != PICKED));
}

#[test]
fn a_selected_token_with_no_look_for_it_keeps_the_highlight_over_it() {
  let mut editor = Editor::new(InputToken::new(TOKEN, chip()).atomic(true));
  editor.controller.select(TOKEN.start, TOKEN.end).unwrap();
  let snapshot = editor.render();
  let pill = snapshot.rects.iter().find(|rect| rect.color == FILL).unwrap();
  let highlight = snapshot.rects.iter().find(|rect| rect.color == SELECTION).unwrap();
  assert!(highlight.order > pill.order);
}
