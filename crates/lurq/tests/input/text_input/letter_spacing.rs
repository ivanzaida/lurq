//! Carets, hit testing and selection follow spaced advances. With the
//! weight-probe Regular face at 10px an "a" is 5px, so 5px letter spacing puts
//! glyph boundaries every 10px (every 5px without spacing).

use lurq::{
  app::{App, Tree, events::MouseButton},
  components::{Text, TextInput},
  core::Signal,
  layout::text_style::TextStyle,
  node::color::Color,
};

use crate::support::{RenderSnapshot, pointer_click, render_pass_with_app};

const FAMILY: &str = "Lurq Weight Probe";
const REGULAR: &[u8] = include_bytes!("../../assets/weight_probe/LurqWeightProbe-Regular.ttf");
const SPACING: f32 = 5.0;
const STEP: f32 = 10.0;

fn probe_app() -> App {
  let mut app = App::new();
  app.install_fonts([REGULAR.to_vec()], std::iter::empty::<(&str, &str)>());
  app
}

fn probe_style() -> TextStyle {
  TextStyle {
    font_family: FAMILY.into(),
    font_size: 10.0,
    ..TextStyle::default()
  }
}

fn caret_x(snapshot: &RenderSnapshot, color: Color) -> f32 {
  snapshot
    .rects
    .iter()
    .find(|rect| rect.width == 1.0 && rect.height > 0.0 && rect.color == color)
    .expect("focused input should draw its caret")
    .x
}

#[test]
fn text_input_carets_and_hit_testing_use_spaced_advances() {
  let caret = Color::from_hex("#ff00ff");
  let mut app = probe_app();
  let value = Signal::new("aaaa".to_owned());
  let mut tree = Tree::new();
  tree.set_root(
    TextInput::styled(value.clone(), probe_style())
      .letter_spacing(SPACING)
      .caret_color(caret)
      .width(200.0),
  );
  render_pass_with_app(&mut tree, &mut app);
  let rect = tree.find_element(|_| true).unwrap().bounds();
  let y = rect.y + rect.height / 2.0;

  pointer_click(&mut tree, rect.x + 1.0, y, MouseButton::Left);
  let origin = caret_x(&render_pass_with_app(&mut tree, &mut app), caret);

  // The end of the text sits after all four spaced glyphs, trailing spacing included.
  pointer_click(&mut tree, rect.x + rect.width - 1.0, y, MouseButton::Left);
  let end = caret_x(&render_pass_with_app(&mut tree, &mut app), caret);
  assert!(
    (end - (origin + 4.0 * STEP)).abs() <= 1.0,
    "end caret at {end}, expected {}",
    origin + 4.0 * STEP
  );

  // Just past the second boundary: index 2 with spacing (index 4 without).
  pointer_click(&mut tree, origin + 2.0 * STEP + 1.0, y, MouseButton::Left);
  let middle = caret_x(&render_pass_with_app(&mut tree, &mut app), caret);
  assert!(
    (middle - (origin + 2.0 * STEP)).abs() <= 1.0,
    "caret at {middle}, expected two spaced glyphs from the start"
  );
  tree.key_down("X".to_owned(), "KeyX".to_owned(), false, false, false);
  render_pass_with_app(&mut tree, &mut app);
  assert_eq!(value.get(), "aaXaa");
}

#[test]
fn selectable_text_selection_covers_spaced_glyphs() {
  let selection = Color::from_hex("#00ff88");
  let mut app = probe_app();
  let mut tree = Tree::new();
  tree.set_root(
    Text::styled("aaaa", probe_style())
      .letter_spacing(SPACING)
      .selectable(true)
      .selection_color(selection),
  );
  render_pass_with_app(&mut tree, &mut app);
  let rect = tree.find_element(|_| true).unwrap().bounds();
  let y = rect.y + rect.height / 2.0;

  tree.mouse_down(rect.x, y, MouseButton::Left);
  // Unspaced, this point would lie past the end of the 20px text.
  tree.mouse_move(rect.x + 3.0 * STEP + 1.0, y);
  tree.mouse_up(rect.x + 3.0 * STEP + 1.0, y, MouseButton::Left);
  let snapshot = render_pass_with_app(&mut tree, &mut app);
  let width: f32 = snapshot
    .rects
    .iter()
    .filter(|rect| rect.color == selection && rect.height > 0.0)
    .map(|rect| rect.width)
    .sum();
  assert!(
    (width - 3.0 * STEP).abs() <= 1.0,
    "selection over three spaced glyphs should be {}px wide, got {width}",
    3.0 * STEP
  );
}
