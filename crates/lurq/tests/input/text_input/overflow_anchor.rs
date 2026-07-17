use lurq::{
  app::{Tree, events::MouseButton},
  components::{TextInput, TextInputOverflowAnchor},
  core::Signal,
  layout::quad::QuadContent,
};

use crate::support::{pointer_click, run_pass};

fn text_quad_x(runtime: &Tree) -> f32 {
  runtime
    .resolve_quads(runtime.last_layout().expect("text input should be laid out"))
    .into_iter()
    .find(|quad| matches!(quad.content, QuadContent::Text { .. }))
    .expect("text input should produce a text quad")
    .x
}

#[test]
fn unfocused_overflow_anchor_selects_visible_edge() {
  let value = "0.123123131234567890".to_owned();

  let mut start = Tree::new();
  start.set_root(
    TextInput::new(Signal::new(value.clone()))
      .width(64.0)
      .unfocused_overflow_anchor(TextInputOverflowAnchor::Start),
  );
  run_pass(&mut start);

  let mut end = Tree::new();
  end.set_root(
    TextInput::new(Signal::new(value))
      .width(64.0)
      .unfocused_overflow_anchor(TextInputOverflowAnchor::End),
  );
  run_pass(&mut end);

  assert!(
    text_quad_x(&start).abs() < 0.01,
    "start-anchored input should show its prefix"
  );
  assert!(
    text_quad_x(&end) < -1.0,
    "end-anchored input should scroll to its suffix"
  );
}

#[test]
fn focused_typing_scrolls_to_follow_caret_then_restores_unfocused_anchor() {
  let value = Signal::new(String::new());
  let mut runtime = Tree::new();
  runtime.set_root(
    TextInput::new(value.clone())
      .width(64.0)
      .unfocused_overflow_anchor(TextInputOverflowAnchor::Start),
  );
  run_pass(&mut runtime);

  let bounds = runtime
    .find_element(|_| true)
    .expect("text input should exist")
    .bounds();
  pointer_click(
    &mut runtime,
    bounds.x + bounds.width / 2.0,
    bounds.y + bounds.height / 2.0,
    MouseButton::Left,
  );
  for ch in "0.123123131234567890".chars() {
    runtime.key_down(ch.to_string(), "Unidentified".to_owned(), false, false, false);
  }
  run_pass(&mut runtime);

  assert_eq!(value.get(), "0.123123131234567890");
  assert!(
    text_quad_x(&runtime) < -1.0,
    "focused input should scroll as the caret advances"
  );

  runtime.key_down("Escape".to_owned(), "Escape".to_owned(), false, false, false);
  run_pass(&mut runtime);

  assert!(
    text_quad_x(&runtime).abs() < 0.01,
    "blurred input should return to its unfocused start anchor"
  );
}
