//! The marks of text editing as the board draws them (Desktop Tools, Text Editing), on an input and a textarea
//! of the design system: the selection and the caret in the theme's roles, the line under a composition, the
//! wavy line under a range the app marks, and the completion after what is typed, which Tab takes.

use lurq::{
  app::{
    events::ImeEvent,
    theme::{PaletteColor, ThemePalette},
  },
  components::Column,
  core::Signal,
  layout::quad::{Quad, QuadContent},
  node::color::Color,
  ui::{Input, InputPart, Textarea},
};

use super::stage::Stage;

const ID: &str = "field";

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

/// An input made by `adjust` over a button, and the signal of its text.
fn scene(text: &str, adjust: impl Fn(Input) -> Input + Send + Sync + 'static) -> (Stage, Signal<String>) {
  let value = Signal::new(text.to_owned());
  let held = value.clone();
  let stage = Stage::new(move |_| {
    Column::new()
      .size(400.0, 300.0)
      .child(adjust(Input::new(held.clone()).id(ID)))
      .child(lurq::components::Button::new("After").id("after").tab_index(0))
      .into()
  });
  (stage, value)
}

fn focus(stage: &mut Stage) {
  stage.tree.get_element_by_id_mut(ID).unwrap().focus();
  stage.pass();
}

/// The filled rectangles of a color.
fn rects(stage: &Stage, color: Color) -> Vec<Quad> {
  stage
    .quads()
    .into_iter()
    .filter(|quad| matches!(&quad.content, QuadContent::Rect { color: fill, .. } if *fill == color))
    .collect()
}

#[test]
fn a_selection_is_filled_with_the_themes_selection_role_and_the_caret_is_the_texts_color() {
  let (mut stage, _) = scene("The quick brown fox jumps", |input| input);
  focus(&mut stage);
  let (x, y, width, height) = stage.bounds(ID);
  // The caret: a line in the foreground inside the box, the board's 1.5 wide and 18 high in type of 14.
  let carets: Vec<_> = rects(&stage, light(PaletteColor::Foreground))
    .into_iter()
    .filter(|quad| quad.width <= 2.0 && quad.height >= 12.0)
    .collect();
  assert_eq!(carets.len(), 1, "one caret");
  assert_eq!(carets[0].width, 1.5);
  assert!((carets[0].height - 18.0).abs() < 0.01, "{}", carets[0].height);
  assert!(
    carets[0].x > x && carets[0].x < x + width && carets[0].y >= y && carets[0].y + carets[0].height <= y + height
  );
  // Shift+Home selects to the start: the selection's fill is the role, behind the text.
  stage
    .tree
    .key_down("Home".to_owned(), "Home".to_owned(), true, false, false);
  stage.pass();
  let selection = rects(&stage, light(PaletteColor::Selection));
  assert_eq!(selection.len(), 1);
  assert!(selection[0].width > 100.0 && selection[0].height >= 14.0);
  // Out of the focus the input draws neither.
  stage.click("after");
  assert!(rects(&stage, light(PaletteColor::Selection)).is_empty());
}

#[test]
fn a_composition_is_underlined_in_the_color_of_the_text() {
  let (mut stage, value) = scene("Typing ", |input| input);
  focus(&mut stage);
  stage.tree.ime(ImeEvent::Preedit {
    text: "nihongo".into(),
    cursor: Some((7, 7)),
  });
  stage.pass();
  assert_eq!(value.get(), "Typing ", "a composition is not text yet");
  let lines: Vec<_> = rects(&stage, light(PaletteColor::Foreground))
    .into_iter()
    .filter(|quad| quad.height <= 2.0 && quad.width > 20.0)
    .collect();
  assert_eq!(lines.len(), 1, "a line under what is being composed");
  assert_eq!(lines[0].height, 1.5, "the board's line");
}

#[test]
fn squiggles_are_drawn_in_the_destructive_color_under_the_ranges_the_app_marks() {
  let (mut stage, _) = scene("A mispeled word", |input| input.squiggles([2..10]));
  let wave = rects(&stage, light(PaletteColor::Destructive));
  assert!(wave.len() >= 40, "a wave under the word: {}", wave.len());
  let (x, y, _, height) = stage.bounds(ID);
  assert!(
    wave
      .iter()
      .all(|column| column.x > x + 12.0 && column.y > y && column.y < y + height)
  );
  // A rule of the theme gives them another color.
  let (stage, _) = scene("A mispeled word", |input| {
    input
      .squiggles([2..10])
      .style(InputPart::Squiggle, |s| s.foreground(PaletteColor::Warning))
  });
  assert!(rects(&stage, light(PaletteColor::Destructive)).is_empty());
  assert!(!rects(&stage, light(PaletteColor::Warning)).is_empty());
  // A textarea has them too.
  let text = Signal::new("A mispeled word".to_owned());
  let stage = Stage::new(move |_| {
    Column::new()
      .size(400.0, 300.0)
      .child(Textarea::new(text.clone()).squiggles([2..10]))
      .into()
  });
  assert!(rects(&stage, light(PaletteColor::Destructive)).len() >= 40);
}

#[test]
fn a_completion_is_drawn_faint_after_what_is_typed_and_tab_takes_it() {
  let (mut stage, value) = scene("cargo ru", |input| input.completion("n --release"));
  let ghost = |stage: &Stage| {
    stage
      .quads()
      .into_iter()
      .find(|quad| matches!(&quad.content, QuadContent::Text { text, .. } if text == "n --release"))
  };
  assert!(ghost(&stage).is_none(), "not while the input has not the focus");
  focus(&mut stage);
  let shown = ghost(&stage).expect("the completion");
  let QuadContent::Text { style, .. } = &shown.content else {
    unreachable!()
  };
  assert_eq!(style.color, light(PaletteColor::MutedForeground));
  assert!((shown.opacity - 0.7).abs() < 0.01, "faint: {}", shown.opacity);
  let (x, ..) = stage.bounds(ID);
  assert!(shown.x > x + 12.0 + 40.0, "after what is typed: {}", shown.x);
  // Tab takes it: it is typed, and the focus stays in the field.
  stage.key("Tab");
  assert_eq!(value.get(), "cargo run --release");
  let focused = stage
    .tree
    .focused_element()
    .and_then(|element| element.id().map(str::to_owned));
  assert_eq!(focused.as_deref(), Some(ID));
}

#[test]
fn tab_goes_on_from_an_input_whose_caret_is_not_at_the_end_or_that_offers_nothing() {
  let (mut stage, value) = scene("cargo ru", |input| input.completion("n --release"));
  focus(&mut stage);
  stage
    .tree
    .key_down("Home".to_owned(), "Home".to_owned(), false, false, false);
  stage.key("Tab");
  assert_eq!(value.get(), "cargo ru");
  let focused = stage
    .tree
    .focused_element()
    .and_then(|element| element.id().map(str::to_owned));
  assert_eq!(focused.as_deref(), Some("after"), "Tab is Tab");

  let (mut stage, value) = scene("cargo ru", |input| input.completion(""));
  focus(&mut stage);
  stage.key("Tab");
  assert_eq!(value.get(), "cargo ru");
}
