use lurq::{
  app::Tree,
  components::TextInput,
  core::Signal,
  layout::{
    Constraints, Size,
    text_style::{FontWeight, TextStyle},
  },
  node::color::Color,
};

use crate::support::{render_pass, run_pass};

#[test]
fn styled_constructor_applies_text_color_to_glyphs() {
  let expected = Color::from_hex("#a855f7").to_linear_f32_array();
  let value = Signal::new("Styled".to_owned());
  let mut runtime = Tree::new();

  runtime.set_root(TextInput::styled(
    value,
    TextStyle {
      color: Color::from_hex("#a855f7"),
      ..TextStyle::default()
    },
  ));

  let snapshot = render_pass(&mut runtime);
  let glyph = snapshot.glyphs.first().expect("styled input should render glyphs");
  assert_color_close(glyph.color, expected);
}

#[test]
fn text_style_setter_applies_font_metrics_to_layout() {
  let value = Signal::new("A".to_owned());
  let mut runtime = Tree::new();

  runtime.set_layout_constraints_override(Some(Constraints::loose(Size::new(400.0, 400.0))));
  runtime.set_root(TextInput::new(value).text_style(TextStyle {
    font_size: 24.0,
    line_height: 1.5,
    ..TextStyle::default()
  }));
  run_pass(&mut runtime);

  let bounds = runtime.find_element(|_| true).unwrap().bounds();
  assert!(
    (bounds.height - 36.0).abs() < 0.01,
    "text input height should use styled font metrics; got {}",
    bounds.height
  );
}

#[test]
fn placeholder_uses_configured_text_style() {
  let expected = Color::from_hex("#22c55e").to_linear_f32_array();
  let value = Signal::new(String::new());
  let mut runtime = Tree::new();

  runtime.set_root(TextInput::new(value).placeholder("Name").text_style(TextStyle {
    color: Color::from_hex("#22c55e"),
    weight: FontWeight::Bold,
    ..TextStyle::default()
  }));

  let snapshot = render_pass(&mut runtime);
  let glyph = snapshot
    .glyphs
    .first()
    .expect("styled placeholder should render glyphs");
  assert_color_close(glyph.color, expected);
}

#[test]
fn placeholder_style_overrides_text_style_only_for_placeholder() {
  let text_color = Color::from_hex("#22c55e").to_linear_f32_array();
  let placeholder_color = Color::from_hex("#a855f7").to_linear_f32_array();
  let value = Signal::new(String::new());
  let mut runtime = Tree::new();

  runtime.set_root(
    TextInput::new(value.clone())
      .placeholder("Name")
      .text_style(TextStyle {
        color: Color::from_hex("#22c55e"),
        ..TextStyle::default()
      })
      .placeholder_style(TextStyle {
        color: Color::from_hex("#a855f7"),
        ..TextStyle::default()
      }),
  );

  let placeholder_snapshot = render_pass(&mut runtime);
  let placeholder_glyph = placeholder_snapshot
    .glyphs
    .first()
    .expect("placeholder should render glyphs");
  assert_color_close(placeholder_glyph.color, placeholder_color);

  value.set("Ada".to_owned());
  let text_snapshot = render_pass(&mut runtime);
  let text_glyph = text_snapshot.glyphs.first().expect("input value should render glyphs");
  assert_color_close(text_glyph.color, text_color);
}

fn assert_color_close(actual: [f32; 4], expected: [f32; 4]) {
  for (actual, expected) in actual.into_iter().zip(expected) {
    assert!(
      (actual - expected).abs() < 0.0001,
      "glyph color channel should match style: actual={actual}, expected={expected}"
    );
  }
}
