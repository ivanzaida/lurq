//! A multi-line text input places its lines, caret and selections by the
//! font's metrics, not by the ink of the glyphs it shows.

use lurq::{
  app::{App, Tree},
  components::{Column, TextInput},
  core::Signal,
  layout::text_style::TextStyle,
  node::color::Color,
};

use crate::support::{GlyphSnapshot, RectSnapshot, RenderSnapshot, render_pass_with_app};

const FONT_SIZE: f32 = 13.0;
const LINE_HEIGHT: f32 = 1.7;
const CARET: Color = Color::new(255, 0, 0, 255);
const SELECTION: Color = Color::new(0, 0, 255, 255);
/// Transparent space around each glyph bitmap in the atlas.
const GLYPH_PADDING: f32 = 2.0;

struct Field {
  app: App,
  tree: Tree,
}

impl Field {
  /// A textarea like Orchester's task brief: 13px text with 1.7 leading, a
  /// placeholder, and no padding of its own.
  fn new() -> Self {
    let style = TextStyle {
      font_size: FONT_SIZE,
      line_height: LINE_HEIGHT,
      color: Color::from_hex("#EDEDED"),
      ..TextStyle::default()
    };
    let placeholder = TextStyle {
      color: Color::from_hex("#888888"),
      ..style.clone()
    };
    let mut tree = Tree::new();
    tree.set_root(
      Column::new().padding(16.0).child(
        TextInput::styled(Signal::new(String::new()), style)
          .id("brief")
          .placeholder_style(placeholder)
          .caret_color(CARET)
          .selection_color(SELECTION)
          .textarea()
          .rows(3, 8)
          .width(400.0)
          .placeholder("What should change, and why?"),
      ),
    );
    let mut field = Self { app: App::new(), tree };
    field.render();
    field.tree.get_element_by_id_mut("brief").expect("field").focus();
    field
  }

  fn render(&mut self) -> RenderSnapshot {
    render_pass_with_app(&mut self.tree, &mut self.app)
  }

  fn key(&mut self, key: &str, code: &str, shift: bool) {
    self.tree.key_down(key.to_owned(), code.to_owned(), shift, false, false);
  }

  fn type_text(&mut self, text: &str) {
    for ch in text.chars() {
      self.key(&ch.to_string(), &format!("Key{}", ch.to_ascii_uppercase()), false);
    }
  }

  fn left(&mut self) -> f32 {
    self
      .tree
      .get_element_by_id_mut("brief")
      .and_then(|element| element.bounds())
      .expect("field laid out")
      .x
  }
}

fn rect(snapshot: &RenderSnapshot, color: Color) -> RectSnapshot {
  *snapshot
    .rects
    .iter()
    .find(|rect| rect.color == color)
    .unwrap_or_else(|| panic!("no rect in {color:?}"))
}

fn ink_left(glyph: &GlyphSnapshot) -> f32 {
  glyph.x + GLYPH_PADDING
}

#[test]
fn caret_in_an_empty_field_sits_before_the_placeholder() {
  let mut field = Field::new();
  let snapshot = field.render();
  let caret = rect(&snapshot, CARET);
  let first = snapshot.glyphs.first().expect("placeholder glyphs");

  assert!(
    caret.x + caret.width <= ink_left(first),
    "the caret ({}..{}) must not cover the placeholder's first glyph (ink from {})",
    caret.x,
    caret.x + caret.width,
    ink_left(first)
  );
  assert_eq!(caret.x + caret.width, field.left(), "it ends where the line starts");
}

#[test]
fn the_line_keeps_its_baseline_when_a_taller_glyph_is_typed() {
  let mut field = Field::new();
  field.type_text("as");
  let short = field.render();
  field.type_text("d");
  let tall = field.render();

  assert_eq!(short.glyphs.len(), 2);
  assert_eq!(tall.glyphs.len(), 3);
  for index in 0..2 {
    assert_eq!(
      short.glyphs[index].y, tall.glyphs[index].y,
      "glyph {index} moved when \"d\" was typed"
    );
  }
  assert_eq!(rect(&short, CARET).y, rect(&tall, CARET).y);
}

#[test]
fn a_selection_covers_the_selected_glyphs_not_the_leading() {
  let mut field = Field::new();
  field.type_text("asd");
  field.key("ArrowLeft", "ArrowLeft", false);
  let caret_after_s = rect(&field.render(), CARET).x;
  field.key("ArrowLeft", "ArrowLeft", true);
  field.key("ArrowLeft", "ArrowLeft", true);
  let snapshot = field.render();
  let selection = rect(&snapshot, SELECTION);

  assert_eq!(selection.x, field.left(), "the selection starts at \"a\"");
  assert!(
    (selection.x + selection.width - caret_after_s).abs() < 0.01,
    "the selection ends after \"s\" ({} vs {caret_after_s})",
    selection.x + selection.width
  );
  let line_box = FONT_SIZE * LINE_HEIGHT;
  assert!(
    selection.height < line_box - 1.0 && selection.height >= FONT_SIZE,
    "the selection is the text's height, not the {line_box}px line box: {}",
    selection.height
  );
  for glyph in &snapshot.glyphs {
    let (top, bottom) = (glyph.y + GLYPH_PADDING, glyph.y + glyph.height - GLYPH_PADDING);
    assert!(
      top >= selection.y && bottom <= selection.y + selection.height,
      "glyph ink {top}..{bottom} lies in the selection's band {}..{}",
      selection.y,
      selection.y + selection.height
    );
  }
}
