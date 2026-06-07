use std::{thread, time::Duration};

use lurq::{
  app::{
    App, Tree,
    events::MouseButton,
    theme::{CaretMode, PaletteColor},
  },
  core::Signal,
  layout::{quad::QuadContent, text_style::TextStyle},
  node::color::Color,
};

use crate::support::{TestSurface, pointer_click, render_pass, run_pass};

#[test]
fn renders_caret_after_text_input_is_focused() {
  let value = Signal::new("A".to_owned());
  let mut runtime = Tree::new();

  runtime.set_root(lurq::components::TextInput::new(value));
  run_pass(&mut runtime);
  let rect = runtime.find_element(|_| true).unwrap().bounds();
  let (x, y) = rect.center();

  pointer_click(&mut runtime, x, y, MouseButton::Left);
  let snapshot = render_pass(&mut runtime);

  assert!(snapshot.rects.iter().any(|rect| rect.width == 1.0 && rect.height > 0.0));
}

#[test]
fn focused_text_input_caret_blinks_and_resets_after_typing() {
  let caret_color = Color::from_hex("#ff00ff");
  let value = Signal::new("A".to_owned());
  let mut runtime = Tree::new();

  runtime.set_root(lurq::components::TextInput::new(value).caret_color(caret_color));
  run_pass(&mut runtime);
  let rect = runtime.find_element(|_| true).unwrap().bounds();
  let (x, y) = rect.center();

  pointer_click(&mut runtime, x, y, MouseButton::Left);
  let visible = render_pass(&mut runtime);
  assert!(has_caret(&visible, caret_color));

  thread::sleep(Duration::from_millis(600));
  let hidden = render_pass(&mut runtime);
  assert!(!has_caret(&hidden, caret_color));

  runtime.key_down("B".to_owned(), "KeyB".to_owned(), false, false, false);
  let reset = render_pass(&mut runtime);
  assert!(has_caret(&reset, caret_color));
}

#[test]
fn caret_mode_uses_theme_and_text_input_override() {
  let persistent_color = Color::from_hex("#ff00ff");
  let override_color = Color::from_hex("#00ffff");
  let mut persistent_app = App::new();
  let mut override_app = App::new();
  let mut persistent_runtime = Tree::new();
  let mut override_runtime = Tree::new();

  persistent_app.theme().set_caret_mode(CaretMode::Persistent);
  override_app.theme().set_caret_mode(CaretMode::Persistent);

  persistent_runtime
    .set_root(lurq::components::TextInput::new(Signal::new("A".to_owned())).caret_color(persistent_color));
  override_runtime.set_root(
    lurq::components::TextInput::new(Signal::new("A".to_owned()))
      .caret_color(override_color)
      .caret_mode(CaretMode::Blinking),
  );

  focus_text_input(&mut persistent_runtime, &mut persistent_app);
  focus_text_input(&mut override_runtime, &mut override_app);
  assert!(has_caret_quad(
    &mut persistent_runtime,
    &mut persistent_app,
    persistent_color
  ));
  assert!(has_caret_quad(&mut override_runtime, &mut override_app, override_color));

  thread::sleep(Duration::from_millis(600));

  assert!(has_caret_quad(
    &mut persistent_runtime,
    &mut persistent_app,
    persistent_color
  ));
  assert!(!has_caret_quad(
    &mut override_runtime,
    &mut override_app,
    override_color
  ));
}

#[test]
fn caret_uses_text_line_height_in_tall_text_input() {
  let value = Signal::new("A".to_owned());
  let mut runtime = Tree::new();

  runtime.set_root(lurq::components::TextInput::new(value).height(80.0));
  run_pass(&mut runtime);
  let rect = runtime.find_element(|_| true).unwrap().bounds();
  let (x, y) = rect.center();

  pointer_click(&mut runtime, x, y, MouseButton::Left);
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
fn text_input_padding_offsets_text_quad() {
  let value = Signal::new("qweqweq".to_owned());
  let mut runtime = Tree::new();

  runtime.set_root(
    lurq::components::TextInput::new(value)
      .single_line()
      .width(120.0)
      .height(40.0)
      .padding_horizontal(10.0),
  );
  run_pass(&mut runtime);
  let quads = runtime.resolve_quads(runtime.last_layout().unwrap());
  let text_quad = quads
    .iter()
    .find(|quad| matches!(quad.content, QuadContent::Text { .. }))
    .expect("text input should produce a text quad");

  assert!(
    (text_quad.x - 10.0).abs() < 0.01,
    "text input text should start after horizontal padding; got x {}",
    text_quad.x
  );
}

#[test]
fn padded_scroll_overflow_keeps_end_caret_inside_content_clip() {
  let value = Signal::new("Mira Cqwewqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq".to_owned());
  let mut runtime = Tree::new();

  runtime.set_root(
    lurq::components::TextInput::new(value)
      .width(80.0)
      .height(40.0)
      .padding_horizontal(10.0),
  );
  run_pass(&mut runtime);
  let rect = runtime.find_element(|_| true).unwrap().bounds();

  pointer_click(
    &mut runtime,
    rect.x + 10.0,
    rect.y + rect.height / 2.0,
    MouseButton::Left,
  );
  runtime.key_down("End".to_owned(), "End".to_owned(), false, false, false);

  let snapshot = render_pass(&mut runtime);
  let caret = snapshot
    .rects
    .iter()
    .find(|rect| rect.width == 1.0 && rect.height > 0.0)
    .expect("focused text input should render a caret");

  assert!(caret.clip.active, "overflowing text input caret should be clipped");
  assert!(
    caret.x >= caret.clip.x && caret.x + caret.width <= caret.clip.x + caret.clip.width + 0.01,
    "caret should stay inside the padded content clip; caret x={} width={}, clip x={} width={}",
    caret.x,
    caret.width,
    caret.clip.x,
    caret.clip.width
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

  pointer_click(&mut runtime, x, y, MouseButton::Left);
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
        caret_color: Some(expected.into()),
        ..TextStyle::default()
      },
    )
    .height(40.0),
  );
  run_pass(&mut runtime);
  let rect = runtime.find_element(|_| true).unwrap().bounds();
  let (x, y) = rect.center();

  pointer_click(&mut runtime, x, y, MouseButton::Left);
  let snapshot = render_pass(&mut runtime);
  let caret = snapshot
    .rects
    .iter()
    .find(|rect| rect.width == 1.0 && rect.height > 0.0)
    .expect("focused text input should render a caret");

  assert_eq!(caret.color, expected);
}

#[test]
fn fixed_height_multiline_caret_stays_on_new_line_after_typing() {
  let value = Signal::new("A".to_owned());
  let mut runtime = Tree::new();

  runtime.set_root(
    lurq::components::TextInput::styled(
      value.clone(),
      TextStyle {
        font_size: 13.0,
        caret_color: Some(Color::from_hex("#38bdf8").into()),
        ..TextStyle::default()
      },
    )
    .width(240.0)
    .height(82.0)
    .padding_horizontal(10.0)
    .padding_vertical(10.0)
    .background("#101215")
    .border_inside(1.0, "#334155")
    .multiline(),
  );
  run_pass(&mut runtime);
  let input_rect = runtime
    .find_element(|el| el.text_content() == Some("A"))
    .unwrap()
    .bounds();
  let (x, y) = input_rect.center();

  pointer_click(&mut runtime, x, y, MouseButton::Left);
  runtime.key_down("End".to_owned(), "End".to_owned(), false, false, false);
  runtime.key_down("Enter".to_owned(), "Enter".to_owned(), false, false, false);
  let after_enter_y = focused_caret_y(&mut runtime);

  runtime.key_down("B".to_owned(), "KeyB".to_owned(), false, false, false);
  let after_typing_y = focused_caret_y(&mut runtime);

  assert_eq!(value.get(), "A\nB");
  assert!(
    (after_typing_y - after_enter_y).abs() < 0.01,
    "typing on a new fixed-height multiline row should not move the caret to another row; after enter={after_enter_y}, after typing={after_typing_y}"
  );
}

#[test]
fn caret_color_accepts_palette_color() {
  let expected = Color::from_hex("#123456");
  let value = Signal::new("A".to_owned());
  let mut app = App::new();
  let mut runtime = Tree::new();
  app.theme().set_palette_color(PaletteColor::Accent, expected);

  runtime.set_root(
    lurq::components::TextInput::new(value)
      .height(40.0)
      .caret_color(PaletteColor::Accent),
  );

  assert_eq!(focused_caret_color(&mut runtime, &mut app), expected);
}

#[test]
fn text_style_caret_color_accepts_palette_color() {
  let expected = Color::from_hex("#123456");
  let value = Signal::new("A".to_owned());
  let mut app = App::new();
  let mut runtime = Tree::new();
  app.theme().set_palette_color(PaletteColor::Accent, expected);

  runtime.set_root(
    lurq::components::TextInput::styled(
      value,
      TextStyle {
        caret_color: Some(PaletteColor::Accent.into()),
        ..TextStyle::default()
      },
    )
    .height(40.0),
  );

  assert_eq!(focused_caret_color(&mut runtime, &mut app), expected);
}

fn focused_caret_color(runtime: &mut Tree, app: &mut App) -> Color {
  runtime.pass(app, &TestSurface);
  let rect = runtime.find_element(|_| true).unwrap().bounds();
  let (x, y) = rect.center();

  pointer_click(runtime, x, y, MouseButton::Left);
  runtime.pass(app, &TestSurface);
  let quads = runtime.resolve_quads(runtime.last_layout().unwrap());
  quads
    .iter()
    .find_map(|quad| match quad.content {
      QuadContent::Rect { color, .. } if quad.width == 1.0 && quad.height > 0.0 => Some(color),
      _ => None,
    })
    .expect("focused text input should render a caret")
}

fn focused_caret_y(runtime: &mut Tree) -> f32 {
  render_pass(runtime)
    .rects
    .iter()
    .find(|rect| rect.width == 1.0 && rect.height > 0.0)
    .expect("focused text input should render a caret")
    .y
}

fn focus_text_input(runtime: &mut Tree, app: &mut App) {
  runtime.pass(app, &TestSurface);
  let rect = runtime.find_element(|_| true).unwrap().bounds();
  let (x, y) = rect.center();
  pointer_click(runtime, x, y, MouseButton::Left);
}

fn has_caret_quad(runtime: &mut Tree, app: &mut App, color: Color) -> bool {
  runtime.pass(app, &TestSurface);
  runtime
    .resolve_quads(runtime.last_layout().unwrap())
    .iter()
    .any(|quad| match quad.content {
      QuadContent::Rect { color: rect_color, .. } => quad.width == 1.0 && quad.height > 0.0 && rect_color == color,
      _ => false,
    })
}

fn has_caret(snapshot: &crate::support::RenderSnapshot, color: Color) -> bool {
  snapshot
    .rects
    .iter()
    .any(|rect| rect.width == 1.0 && rect.height > 0.0 && rect.color == color)
}
