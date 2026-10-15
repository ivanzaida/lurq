//! ArrowUp and ArrowDown in a multiline input move the caret one laid-out
//! row: a soft-wrapped paragraph is several rows, not one line.

use lurq::{
  app::{App, Tree},
  components::{Column, TextInput},
  core::Signal,
  layout::text_style::TextStyle,
};

use crate::support::{RenderSnapshot, render_pass_with_app};

const ID: &str = "rows-input";

fn paragraph() -> String {
  (0..30)
    .map(|word| format!("word{word:02}"))
    .collect::<Vec<_>>()
    .join(" ")
}

struct Editor {
  tree: Tree,
  app: App,
  value: Signal<String>,
}

impl Editor {
  /// A 200 px wide multiline input that grows to 20 rows, focused with the
  /// caret at the end of `text`.
  fn new(text: &str) -> Self {
    let value = Signal::new(text.to_owned());
    let mut tree = Tree::new();
    tree.resize(400, 600);
    tree.set_root(
      Column::new().child(
        TextInput::styled(
          value.clone(),
          TextStyle {
            font_size: 13.0,
            line_height: 1.4,
            ..TextStyle::default()
          },
        )
        .id(ID)
        .rows(1, 20)
        .width(200.0),
      ),
    );
    let mut editor = Self {
      tree,
      app: App::new(),
      value,
    };
    editor.render();
    editor.tree.get_element_by_id_mut(ID).unwrap().focus();
    editor.render();
    editor
  }

  fn render(&mut self) -> RenderSnapshot {
    render_pass_with_app(&mut self.tree, &mut self.app)
  }

  fn key(&mut self, key: &str, shift: bool) {
    self.tree.key_down(key.into(), key.into(), shift, false, false);
    self.tree.key_up(key.into(), key.into(), shift, false, false);
    self.render();
  }

  /// The number of laid-out rows that show glyphs.
  fn rows(&mut self) -> usize {
    let snapshot = self.render();
    let mut rows: Vec<f32> = snapshot.glyphs.iter().map(|glyph| glyph.y).collect();
    rows.sort_by(f32::total_cmp);
    rows.dedup_by(|a, b| (*a - *b).abs() < 4.0);
    rows.len()
  }
}

#[test]
fn text_input_shift_arrow_up_selects_wrapped_rows_not_the_paragraph() {
  let text = paragraph();
  let mut editor = Editor::new(&text);
  let rows = editor.rows();
  assert!(rows >= 6, "the paragraph wraps into several rows: {rows}");

  for _ in 0..3 {
    editor.key("ArrowUp", true);
  }
  editor.key("Backspace", false);

  let left = editor.value.get();
  assert!(text.starts_with(&left), "only the end is deleted: {left:?}");
  assert_eq!(
    editor.rows(),
    rows - 3,
    "three rows up from the end, the selection kept the first rows: {left:?}"
  );
}

#[test]
fn text_input_arrow_down_moves_one_wrapped_row_at_a_time() {
  let text = paragraph();
  let mut editor = Editor::new(&text);
  let rows = editor.rows();
  editor.key("Home", false);

  for _ in 0..2 {
    editor.key("ArrowDown", false);
  }
  editor.key("Home", true);
  editor.key("Backspace", false);

  let left = editor.value.get();
  assert!(
    text.ends_with(&left) && left.starts_with("word"),
    "only the start is deleted, up to the start of the third row: {left:?}"
  );
  assert_eq!(
    editor.rows(),
    rows - 2,
    "two rows down from the start, the selection took the first two rows: {left:?}"
  );
}

#[test]
fn text_input_arrow_up_and_down_return_to_the_same_place() {
  let text = paragraph();
  let mut editor = Editor::new(&text);
  for _ in 0..4 {
    editor.key("ArrowUp", false);
  }
  for _ in 0..4 {
    editor.key("ArrowDown", false);
  }
  editor.key("Backspace", false);

  assert_eq!(
    editor.value.get(),
    text[..text.len() - 1],
    "four rows up and four down is the end again"
  );
}
