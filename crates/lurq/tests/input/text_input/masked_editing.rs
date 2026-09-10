use lurq::{
  app::{Tree, events::MouseButton},
  components::{Text, TextInput},
  core::Signal,
  layout::{Constraints, Size, quad::QuadContent, text_style::TextStyle},
  node::color::Color,
};

use crate::support::{pointer_click, render_pass, run_pass};

fn bullet_advance() -> f32 {
  let mut runtime = Tree::new();
  runtime.set_layout_constraints_override(Some(Constraints::loose(Size::new(400.0, 400.0))));
  runtime.set_root(Text::styled("•", TextStyle::default()));
  run_pass(&mut runtime);
  let advance = runtime.find_element(|_| true).unwrap().bounds().width;
  assert!(advance > 0.0);
  advance
}

#[test]
fn default_mask_clicks_map_to_each_value_character_boundary() {
  let advance = bullet_advance();
  for text in ["abc", "aé🦀"] {
    for index in 0..=3 {
      let value = Signal::new(String::new());
      let mut runtime = Tree::new();
      runtime.set_root(
        TextInput::styled(value.clone(), TextStyle::default())
          .single_line()
          .width(200.0)
          .height(52.0)
          .padding_horizontal(14.0)
          .mask(),
      );
      run_pass(&mut runtime);
      let bounds = runtime.find_element(|_| true).unwrap().bounds();
      // Focus far from the text so the later boundary click is not a double-click.
      pointer_click(&mut runtime, bounds.x + 180.0, bounds.center().1, MouseButton::Left);
      for ch in text.chars() {
        runtime.key_down(ch.to_string(), String::new(), false, false, false);
      }
      run_pass(&mut runtime);
      assert_eq!(value.get(), text);

      pointer_click(
        &mut runtime,
        bounds.x + 14.0 + advance * index as f32,
        bounds.center().1,
        MouseButton::Left,
      );
      runtime.key_down("X".to_owned(), "KeyX".to_owned(), false, false, false);

      let byte_index = text
        .char_indices()
        .map(|(byte, _)| byte)
        .nth(index)
        .unwrap_or(text.len());
      let mut expected = text.to_owned();
      expected.insert(byte_index, 'X');
      assert_eq!(value.get(), expected, "click at masked boundary {index} in {text:?}");
    }
  }
}

#[test]
fn default_mask_drag_selection_backspace_removes_underlying_characters() {
  let advance = bullet_advance();
  for text in ["abc", "aé🦀"] {
    let value = Signal::new(text.to_owned());
    let mut runtime = Tree::new();
    let selection_color = Color::from_hex("#ff00ff");
    runtime.set_root(
      TextInput::styled(value.clone(), TextStyle::default())
        .single_line()
        .width(200.0)
        .height(52.0)
        .padding_horizontal(14.0)
        .selection_color(selection_color)
        .mask(),
    );
    run_pass(&mut runtime);
    let bounds = runtime.find_element(|_| true).unwrap().bounds();
    let start_x = bounds.x + 14.0 + advance;
    let end_x = bounds.x + 14.0 + advance * 3.0;
    let y = bounds.center().1;
    runtime.mouse_down(start_x, y, MouseButton::Left);
    runtime.mouse_move(end_x, y);
    runtime.mouse_up(end_x, y, MouseButton::Left);

    let snapshot = render_pass(&mut runtime);
    let selection = snapshot
      .rects
      .iter()
      .find(|rect| rect.color == selection_color)
      .unwrap();
    assert!((selection.width - 2.0 * advance).abs() < 0.01);

    runtime.key_down("Backspace".to_owned(), "Backspace".to_owned(), false, false, false);
    run_pass(&mut runtime);
    assert_eq!(value.get(), "a");
    let quads = runtime.resolve_quads(runtime.last_layout().unwrap());
    assert!(
      quads
        .iter()
        .any(|quad| matches!(&quad.content, QuadContent::Text { text, .. } if text == "•"))
    );
  }
}

#[test]
fn default_mask_width_and_end_caret_equal_three_bullet_advances() {
  let advance = bullet_advance();
  let mut runtime = Tree::new();
  let caret_color = Color::from_hex("#ff00ff");
  runtime.set_root(
    TextInput::styled(Signal::new("abc".to_owned()), TextStyle::default())
      .single_line()
      .width(200.0)
      .height(52.0)
      .padding_horizontal(14.0)
      .caret_color(caret_color)
      .mask(),
  );
  run_pass(&mut runtime);
  let bounds = runtime.find_element(|_| true).unwrap().bounds();
  pointer_click(&mut runtime, bounds.x + 180.0, bounds.center().1, MouseButton::Left);
  let snapshot = render_pass(&mut runtime);
  assert_eq!(snapshot.glyph_count, 3);
  let caret = snapshot.rects.iter().find(|rect| rect.color == caret_color).unwrap();
  assert!(
    (caret.x - (bounds.x + 14.0) - 3.0 * advance).abs() < 0.01,
    "masked text advance should equal three bullets; caret x={}, content x={}, bullet advance={advance}",
    caret.x,
    bounds.x + 14.0,
  );
}
