use lurq::{
  app::{App, Tree, component::Component, ctx::Ctx, events::MouseButton},
  components::{Column, TextInput},
  core::Signal,
  layout::{
    Constraints, Size,
    text_style::{FontWeight, TextAlign, TextStyle},
  },
  node::{Element, Style, color::Color, dimension::Dimension},
};

use crate::support::{pointer_click, render_pass, run_pass};

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

#[test]
fn text_align_centers_value_glyphs_in_single_line_input() {
  let left_value = Signal::new("A".to_owned());
  let centered_value = Signal::new("A".to_owned());
  let mut left_runtime = Tree::new();
  let mut centered_runtime = Tree::new();

  left_runtime.set_root(TextInput::new(left_value).single_line().width(200.0).height(40.0));
  centered_runtime.set_root(
    TextInput::new(centered_value)
      .single_line()
      .width(200.0)
      .height(40.0)
      .text_align(TextAlign::Center),
  );

  let left = render_pass(&mut left_runtime);
  let centered = render_pass(&mut centered_runtime);
  let left_glyph = left.glyphs.first().expect("left input should render a glyph");
  let centered_glyph = centered.glyphs.first().expect("centered input should render a glyph");

  assert!(
    centered_glyph.x > left_glyph.x + 70.0,
    "centered glyph should move toward the middle of the input; left={}, centered={}",
    left_glyph.x,
    centered_glyph.x
  );
}

#[test]
fn text_align_centers_placeholder_glyphs_in_single_line_input() {
  let left_value = Signal::new(String::new());
  let centered_value = Signal::new(String::new());
  let mut left_runtime = Tree::new();
  let mut centered_runtime = Tree::new();

  left_runtime.set_root(
    TextInput::new(left_value)
      .placeholder("Name")
      .single_line()
      .width(200.0)
      .height(40.0),
  );
  centered_runtime.set_root(
    TextInput::new(centered_value)
      .placeholder("Name")
      .single_line()
      .width(200.0)
      .height(40.0)
      .text_align(TextAlign::Center),
  );

  let left = render_pass(&mut left_runtime);
  let centered = render_pass(&mut centered_runtime);
  let left_glyph = left.glyphs.first().expect("left placeholder should render a glyph");
  let centered_glyph = centered
    .glyphs
    .first()
    .expect("centered placeholder should render a glyph");

  assert!(
    centered_glyph.x > left_glyph.x + 50.0,
    "centered placeholder should move toward the middle of the input; left={}, centered={}",
    left_glyph.x,
    centered_glyph.x
  );
}

#[test]
fn single_line_input_centers_glyphs_with_tall_line_height() {
  let value = Signal::new("Text".to_owned());
  let mut runtime = Tree::new();

  runtime.set_root(
    TextInput::styled(
      value,
      TextStyle {
        font_size: 20.0,
        line_height: 1.6,
        color: Color::from_hex("#1e293b"),
        ..TextStyle::default()
      },
    )
    .single_line()
    .width(200.0)
    .height(64.0),
  );

  let snapshot = render_pass(&mut runtime);
  let top = snapshot
    .glyphs
    .iter()
    .map(|glyph| glyph.y)
    .fold(f32::INFINITY, f32::min);
  let bottom = snapshot
    .glyphs
    .iter()
    .map(|glyph| glyph.y + glyph.height)
    .fold(f32::NEG_INFINITY, f32::max);
  let center = (top + bottom) * 0.5;
  let input_center = 32.0;

  assert!(
    (center - input_center).abs() <= 2.0,
    "single-line input glyph center should match input center: glyph={center}, input={input_center}",
  );
}

#[derive(Clone, Debug, lurq::DevtoolsInspectable)]
struct ErrorStyledTextInputProps {
  error: Signal<bool>,
}

impl PartialEq for ErrorStyledTextInputProps {
  fn eq(&self, _other: &Self) -> bool {
    true
  }
}

struct ErrorStyledTextInput;

impl Component for ErrorStyledTextInput {
  type Props = ErrorStyledTextInputProps;

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let props = ctx.props::<Self::Props>().clone();
    let has_error = props.error.get();
    let background = if has_error { "#33191b" } else { "#111827" };
    let border = if has_error { "#f05d5e" } else { "#30343a" };

    Column::new().width(240.0).child(
      TextInput::styled(
        Signal::new("a3f1".to_owned()),
        TextStyle {
          color: Color::from_hex("#f4f4f2"),
          font_size: 12.0,
          ..TextStyle::default()
        },
      )
      .width(Dimension::Pct(100.0))
      .height(40.0)
      .padding_horizontal(10.0)
      .rounded(5.0)
      .background(background)
      .border_inside(1.0, Color::from_hex(border))
      .placeholder("a3f1b2c4d5e691cc...")
      .placeholder_style(TextStyle {
        color: Color::from_hex("#7d766c"),
        ..TextStyle::default()
      })
      .single_line(),
    )
  }
}

#[test]
fn text_input_wrapper_visuals_update_after_component_rerender() {
  let error = Signal::new(false);
  let mut runtime = Tree::new();
  runtime.mount_root::<ErrorStyledTextInput>(&mut App::new(), ErrorStyledTextInputProps { error: error.clone() });

  let ok = render_pass(&mut runtime);
  assert_input_visuals(&ok.rects, Color::from_hex("#111827"), Color::from_hex("#30343a"));

  error.set(true);

  let invalid = render_pass(&mut runtime);
  assert_input_visuals(&invalid.rects, Color::from_hex("#33191b"), Color::from_hex("#f05d5e"));
}

#[test]
fn focused_text_input_style_draws_on_input_frame_not_padded_text_content() {
  let value = Signal::new("a3f1".to_owned());
  let mut runtime = Tree::new();
  runtime.set_root(
    TextInput::new(value)
      .width(240.0)
      .height(40.0)
      .padding_horizontal(10.0)
      .background("#111827")
      .border_inside(1.0, "#30343a")
      .focused_style(Style::new().border_inside(1.0, "#60a5fa"))
      .single_line(),
  );
  run_pass(&mut runtime);
  let rect = runtime.find_element(|_| true).unwrap().bounds();
  let (x, y) = rect.center();
  pointer_click(&mut runtime, x, y, MouseButton::Left);

  let focused = render_pass(&mut runtime);
  let focused_borders = focused
    .rects
    .iter()
    .filter(|rect| rect.stroke_color == Color::from_hex("#60a5fa"))
    .collect::<Vec<_>>();

  assert_eq!(focused_borders.len(), 1, "focused border should be emitted once");
  assert_eq!(focused_borders[0].width, 240.0);
  assert_eq!(focused_borders[0].height, 40.0);
}

#[test]
fn mouse_down_focused_text_input_style_stays_on_input_frame() {
  let value = Signal::new("abandon ability able about above\nabsent ...".to_owned());
  let mut runtime = Tree::new();
  runtime.set_root(
    TextInput::new(value)
      .width(600.0)
      .height(120.0)
      .padding(15.0)
      .background("#111827")
      .border_inside(1.0, "#30343a")
      .focused_style(Style::new().background("#111827").border_inside(1.0, "#60a5fa"))
      .multiline(),
  );
  run_pass(&mut runtime);
  let rect = runtime.find_element(|_| true).unwrap().bounds();
  let (x, y) = rect.center();

  runtime.mouse_down(x, y, MouseButton::Left);
  let active = render_pass(&mut runtime);
  let focused_borders = active
    .rects
    .iter()
    .filter(|rect| rect.stroke_color == Color::from_hex("#60a5fa"))
    .collect::<Vec<_>>();

  assert_eq!(focused_borders.len(), 1, "active focused border should be emitted once");
  assert_eq!(focused_borders[0].width, 600.0);
  assert_eq!(focused_borders[0].height, 120.0);
}

fn assert_input_visuals(rects: &[crate::support::RectSnapshot], fill: Color, border: Color) {
  let background = rects
    .iter()
    .find(|rect| rect.width == 240.0 && rect.height == 40.0 && rect.color == fill)
    .expect("expected rendered text input wrapper fill rect");
  assert_eq!(background.radii, [5.0; 4]);

  let border_rect = rects
    .iter()
    .find(|rect| {
      rect.width == 240.0
        && rect.height == 40.0
        && rect.color == Color::from_hex("#00000000")
        && rect.stroke == [1.0; 4]
    })
    .expect("expected rendered text input wrapper border rect");
  assert_eq!(border_rect.stroke_color, border);
}

fn assert_color_close(actual: [f32; 4], expected: [f32; 4]) {
  for (actual, expected) in actual.into_iter().zip(expected) {
    assert!(
      (actual - expected).abs() < 0.0001,
      "glyph color channel should match style: actual={actual}, expected={expected}"
    );
  }
}
