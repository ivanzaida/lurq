//! Carets and selections inside a ligature. The ligature-probe face is
//! monospaced (5px cells at 10px) and its `liga` feature draws "--" as one
//! glyph of one cell, so in "a--a" both hyphens share the cell 5..10 and the
//! caret between them sits in its middle, as between the "f" and "i" of
//! DejaVu Sans' "fi".

use lurq::{
  app::{App, Tree, events::MouseButton},
  components::{Text, TextInput},
  core::Signal,
  layout::text_style::TextStyle,
  node::color::Color,
};

use crate::support::{RenderSnapshot, pointer_click, render_pass_with_app};

const FAMILY: &str = "Lurq Ligature Probe";
const PROBE: &[u8] = include_bytes!("../../assets/ligature_probe/LurqLigatureProbe-Regular.ttf");
const CELL: f32 = 5.0;
const TEXT: &str = "a--a";
/// How far a painted edge may sit from its unsnapped position.
const SNAP: f32 = 1.0;

fn probe_app() -> App {
  let mut app = App::new();
  app.install_fonts([PROBE.to_vec()], std::iter::empty::<(&str, &str)>());
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

/// The left and right edge of the one highlight drawn in `color`.
fn highlight(snapshot: &RenderSnapshot, color: Color) -> (f32, f32) {
  let rects: Vec<_> = snapshot
    .rects
    .iter()
    .filter(|rect| rect.color == color && rect.width > 0.0 && rect.height > 0.0)
    .collect();
  assert_eq!(rects.len(), 1, "one highlight: {rects:?}");
  (rects[0].x, rects[0].x + rects[0].width)
}

fn assert_near(actual: f32, expected: f32, what: &str) {
  assert!(
    (actual - expected).abs() <= SNAP,
    "{what} at {actual}, expected {expected}"
  );
}

fn key(tree: &mut Tree, key: &str, shift: bool) {
  tree.key_down(key.to_owned(), key.to_owned(), shift, false, false);
}

#[test]
fn a_caret_and_a_selection_that_end_inside_a_ligature_are_drawn_inside_it() {
  let caret = Color::from_hex("#ff00ff");
  let selection = Color::from_hex("#00ff88");
  let mut app = probe_app();
  let value = Signal::new(TEXT.to_owned());
  let mut tree = Tree::new();
  tree.set_root(
    TextInput::styled(value.clone(), probe_style())
      .caret_color(caret)
      .selection_color(selection)
      .width(200.0),
  );
  render_pass_with_app(&mut tree, &mut app);
  let rect = tree.find_element(|_| true).unwrap().bounds();
  let y = rect.y + rect.height / 2.0;
  pointer_click(&mut tree, rect.x + 1.0, y, MouseButton::Left);
  let origin = caret_x(&render_pass_with_app(&mut tree, &mut app), caret);

  for (index, x) in [(1, CELL), (2, 1.5 * CELL), (3, 2.0 * CELL), (4, 3.0 * CELL)] {
    key(&mut tree, "ArrowRight", false);
    let painted = caret_x(&render_pass_with_app(&mut tree, &mut app), caret);
    assert_near(painted - origin, x, &format!("the caret at byte {index}"));
  }

  key(&mut tree, "Home", false);
  key(&mut tree, "ArrowRight", false);
  key(&mut tree, "ArrowRight", true);
  let (left, right) = highlight(&render_pass_with_app(&mut tree, &mut app), selection);
  assert_near(left - origin, CELL, "the highlight of the first hyphen starts");
  assert_near(right - origin, 1.5 * CELL, "the highlight of the first hyphen ends");

  key(&mut tree, "ArrowRight", false);
  key(&mut tree, "X", false);
  assert_eq!(value.get(), "a-X-a");
}

#[test]
fn a_drag_to_the_middle_of_a_ligature_copies_and_highlights_its_first_character() {
  let selection = Color::from_hex("#00ff88");
  let mut app = probe_app();
  let mut tree = Tree::new();
  tree.set_root(
    Text::styled(TEXT, probe_style())
      .selectable(true)
      .selection_color(selection),
  );
  render_pass_with_app(&mut tree, &mut app);
  let rect = tree.find_element(|_| true).unwrap().bounds();
  let y = rect.y + rect.height / 2.0;

  // From the ligature's left edge to just past its middle.
  tree.mouse_down(rect.x + CELL, y, MouseButton::Left);
  tree.mouse_move(rect.x + 1.5 * CELL + 0.5, y);
  tree.mouse_up(rect.x + 1.5 * CELL + 0.5, y, MouseButton::Left);
  let (left, right) = highlight(&render_pass_with_app(&mut tree, &mut app), selection);
  assert_eq!(tree.selected_text().as_deref(), Some("-"));
  assert_near(left - rect.x, CELL, "the highlight starts");
  assert_near(right - rect.x, 1.5 * CELL, "the highlight ends");
}
