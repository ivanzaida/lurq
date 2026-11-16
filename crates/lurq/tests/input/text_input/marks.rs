//! What an input draws beside its text: wavy lines under the ranges an app marks, and a suggestion after the
//! end of what is typed while the caret is there.
use lurq::{
  app::{App, Tree},
  components::{InputGhost, InputSquiggles, TextInput, TextInputController},
  core::Signal,
  layout::text_style::TextStyle,
  node::color::Color,
};

use crate::support::{GlyphSnapshot, RenderSnapshot, render_pass_with_app};

const ID: &str = "field";
const INK: Color = Color::new(10, 20, 30, 255);
const WAVE: Color = Color::new(200, 30, 160, 255);
const GHOST: Color = Color::new(20, 200, 90, 255);

/// A focused input of `text`, made by `configure`, and its controller.
struct Field {
  tree: Tree,
  app: App,
  value: Signal<String>,
  controller: TextInputController,
}

impl Field {
  fn new(text: &str, configure: impl Fn(TextInput) -> TextInput) -> Self {
    let value = Signal::new(text.to_owned());
    let controller = TextInputController::new();
    let input = TextInput::styled(
      value.clone(),
      TextStyle {
        font_size: 14.0,
        line_height: 1.4,
        color: INK,
        ..TextStyle::default()
      },
    )
    .id(ID)
    .controller(controller.clone())
    .width(320.0);
    let mut tree = Tree::new();
    tree.resize(800, 600);
    tree.set_root(configure(input));
    let mut field = Self {
      tree,
      app: App::new(),
      value,
      controller,
    };
    field.render();
    field.tree.get_element_by_id_mut(ID).unwrap().focus();
    field.render();
    field
  }

  fn render(&mut self) -> RenderSnapshot {
    render_pass_with_app(&mut self.tree, &mut self.app)
  }
}

/// The glyphs of a color, from left to right.
fn glyphs(snapshot: &RenderSnapshot, color: Color) -> Vec<&GlyphSnapshot> {
  let mut found: Vec<_> = snapshot
    .glyphs
    .iter()
    .filter(|glyph| glyph.color[..3] == color.to_linear_f32_array()[..3])
    .collect();
  found.sort_by(|a, b| a.x.total_cmp(&b.x));
  found
}

#[test]
fn squiggles_are_a_wave_under_each_range_in_their_color() {
  let mut field = Field::new("A mispeled word", |input| {
    input.single_line().squiggles(InputSquiggles::new([2..10]).color(WAVE))
  });
  let snapshot = field.render();
  let wave: Vec<_> = snapshot.rects.iter().filter(|rect| rect.color == WAVE).collect();
  let letters = glyphs(&snapshot, INK);
  // "A", then the eight letters of the marked word.
  let (first, last) = (letters[1], letters[8]);
  let (left, right) = (first.x, last.x + last.width);
  assert!(wave.len() >= 40, "a column to a pixel under the word: {}", wave.len());
  assert!(wave.iter().all(|column| column.width == 1.0 && column.height < 2.0));
  let (wave_left, wave_right) = wave.iter().fold((f32::MAX, f32::MIN), |(low, high), column| {
    (low.min(column.x), high.max(column.x + column.width))
  });
  assert!(
    wave_left >= left - 2.0 && wave_right <= right + 2.0,
    "under the word only: {wave_left}..{wave_right} of {left}..{right}"
  );
  // It goes up and down by a pixel and a half, under the letters.
  let (top, bottom) = wave.iter().fold((f32::MAX, f32::MIN), |(low, high), column| {
    (low.min(column.y), high.max(column.y))
  });
  assert!((bottom - top - 1.5).abs() < 0.01, "{}", bottom - top);
  let baseline = first.y + first.height;
  assert!(top > baseline - 2.0 && top < baseline + 6.0, "{top} under {baseline}");
}

#[test]
fn squiggles_follow_a_range_onto_every_row_it_wraps_to_and_a_masked_input_draws_none() {
  let text = "one two three four five six seven eight nine ten eleven twelve";
  let mut field = Field::new(text, |input| {
    input
      .multiline()
      .width(120.0)
      .squiggles(InputSquiggles::new([0..text.len()]).color(WAVE))
  });
  let snapshot = field.render();
  let mut rows: Vec<f32> = snapshot
    .rects
    .iter()
    .filter(|rect| rect.color == WAVE)
    .map(|rect| rect.y)
    .collect();
  rows.sort_by(f32::total_cmp);
  rows.dedup_by(|a, b| (*a - *b).abs() < 4.0);
  assert!(rows.len() >= 3, "a wave on each row: {}", rows.len());

  let mut masked = Field::new("secret", |input| {
    input
      .single_line()
      .mask()
      .squiggles(InputSquiggles::new([0..6]).color(WAVE))
  });
  assert!(masked.render().rects.iter().all(|rect| rect.color != WAVE));
}

#[test]
fn a_ghost_is_drawn_after_the_text_on_its_baseline_while_the_caret_is_at_the_end() {
  let mut field = Field::new("cargo n", |input| {
    input
      .single_line()
      .ghost(InputGhost::new("n --release").color(GHOST).opacity(0.7))
  });
  let snapshot = field.render();
  let typed = glyphs(&snapshot, INK);
  let ghost = glyphs(&snapshot, GHOST);
  assert_eq!(ghost.len(), 10, "the ten letters of the suggestion");
  let end = typed.last().unwrap();
  assert!(
    ghost[0].x >= end.x + end.width - 2.5 && ghost[0].x < end.x + end.width + 4.0,
    "right after what is typed: {} after {}",
    ghost[0].x,
    end.x + end.width
  );
  // The last letter typed and the first of the suggestion are both an "n": they stand on one line.
  assert!(
    (ghost[0].y - end.y).abs() < 0.51 && (ghost[0].height - end.height).abs() < 0.51,
    "on the baseline of the text: {} and {}",
    ghost[0].y,
    end.y
  );
  // It is no part of the value.
  assert_eq!(field.value.get(), "cargo n");

  // With the caret elsewhere, or a selection, it is not drawn.
  field.controller.select(3, 3).unwrap();
  assert!(glyphs(&field.render(), GHOST).is_empty());
  field.controller.select(0, 7).unwrap();
  assert!(glyphs(&field.render(), GHOST).is_empty());
  field.controller.select(7, 7).unwrap();
  assert_eq!(glyphs(&field.render(), GHOST).len(), 10);
}

#[test]
fn a_ghost_is_not_drawn_in_an_input_that_is_empty_or_has_not_the_focus() {
  let mut empty = Field::new("", |input| {
    input.single_line().ghost(InputGhost::new("cargo run").color(GHOST))
  });
  assert!(glyphs(&empty.render(), GHOST).is_empty());

  let value = Signal::new("cargo ru".to_owned());
  let mut tree = Tree::new();
  tree.resize(800, 600);
  tree.set_root(
    TextInput::new(value)
      .single_line()
      .ghost(InputGhost::new("n").color(GHOST)),
  );
  let mut app = App::new();
  assert!(glyphs(&render_pass_with_app(&mut tree, &mut app), GHOST).is_empty());
}

#[test]
fn an_input_told_so_has_its_text_selected_as_it_takes_the_focus() {
  let value = Signal::new("Project Alpha".to_owned());
  let mut tree = Tree::new();
  tree.resize(800, 600);
  tree.set_root(TextInput::new(value.clone()).single_line().select_on_focus(true).id(ID));
  let mut app = App::new();
  render_pass_with_app(&mut tree, &mut app);
  tree.get_element_by_id_mut(ID).unwrap().focus();
  render_pass_with_app(&mut tree, &mut app);
  let selected = tree
    .get_element_by_id_mut(ID)
    .unwrap()
    .as_text_input()
    .unwrap()
    .selected_text();
  assert_eq!(selected.as_deref(), Some("Project Alpha"));
  // What is typed takes its place.
  tree.key_down("B".into(), "KeyB".into(), false, false, false);
  assert_eq!(value.get(), "B");
}
