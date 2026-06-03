use lurq::{
  app::{Tree, events::MouseButton},
  core::Signal,
  layout::{quad::QuadContent, text_style::TextStyle},
  node::color::Color,
};

use crate::support::{render_pass, run_pass};

#[test]
fn renders_caret_after_text_input_is_focused() {
  let value = Signal::new("A".to_owned());
  let mut runtime = Tree::new();

  runtime.set_root(lurq::components::TextInput::new(value));
  run_pass(&mut runtime);
  let rect = runtime.find_element(|_| true).unwrap().bounds();
  let (x, y) = rect.center();

  runtime.click(x, y, MouseButton::Left);
  let snapshot = render_pass(&mut runtime);

  assert!(snapshot.rects.iter().any(|rect| rect.width == 1.0 && rect.height > 0.0));
}

#[test]
fn caret_uses_text_line_height_in_tall_text_input() {
  let value = Signal::new("A".to_owned());
  let mut runtime = Tree::new();

  runtime.set_root(lurq::components::TextInput::new(value).height(80.0));
  run_pass(&mut runtime);
  let rect = runtime.find_element(|_| true).unwrap().bounds();
  let (x, y) = rect.center();

  runtime.click(x, y, MouseButton::Left);
  let snapshot = render_pass(&mut runtime);
  let caret = snapshot
    .rects
    .iter()
    .find(|rect| rect.width == 1.0 && rect.height > 0.0)
    .expect("focused text input should render a caret");

  assert!(
    caret.height < 40.0,
    "caret should follow text line height, not the 80px input container; got {}",
    caret.height
  );
  let expected_y = (80.0 - caret.height) * 0.5;
  assert!(
    (caret.y - expected_y).abs() < 0.01,
    "caret should be vertically centered; got {}, expected {}",
    caret.y,
    expected_y
  );
}

#[test]
fn single_line_text_input_centers_text_quad_in_tall_input() {
  let value = Signal::new("qweqweq".to_owned());
  let mut runtime = Tree::new();

  runtime.set_root(lurq::components::TextInput::new(value).single_line().height(40.0));
  run_pass(&mut runtime);
  let quads = runtime.resolve_quads(runtime.last_layout().unwrap());
  let text_quad = quads
    .iter()
    .find(|quad| matches!(quad.content, QuadContent::Text { .. }))
    .expect("text input should produce a text quad");

  assert!(
    (text_quad.y - 10.4).abs() < 0.01,
    "40px single-line input should center 19.2px line-height text; got y {}",
    text_quad.y
  );
}

#[test]
fn caret_color_sets_rendered_caret_color() {
  let expected = Color::from_hex("#e5e7eb");
  let value = Signal::new("A".to_owned());
  let mut runtime = Tree::new();

  runtime.set_root(
    lurq::components::TextInput::new(value)
      .height(40.0)
      .caret_color("#e5e7eb"),
  );
  run_pass(&mut runtime);
  let rect = runtime.find_element(|_| true).unwrap().bounds();
  let (x, y) = rect.center();

  runtime.click(x, y, MouseButton::Left);
  let snapshot = render_pass(&mut runtime);
  let caret = snapshot
    .rects
    .iter()
    .find(|rect| rect.width == 1.0 && rect.height > 0.0)
    .expect("focused text input should render a caret");

  assert_eq!(caret.color, expected);
}

#[test]
fn text_style_caret_color_sets_rendered_caret_color() {
  let expected = Color::from_hex("#38bdf8");
  let value = Signal::new("A".to_owned());
  let mut runtime = Tree::new();

  runtime.set_root(
    lurq::components::TextInput::styled(
      value,
      TextStyle {
        caret_color: Some(expected),
        ..TextStyle::default()
      },
    )
    .height(40.0),
  );
  run_pass(&mut runtime);
  let rect = runtime.find_element(|_| true).unwrap().bounds();
  let (x, y) = rect.center();

  runtime.click(x, y, MouseButton::Left);
  let snapshot = render_pass(&mut runtime);
  let caret = snapshot
    .rects
    .iter()
    .find(|rect| rect.width == 1.0 && rect.height > 0.0)
    .expect("focused text input should render a caret");

  assert_eq!(caret.color, expected);
}
