use lurq::{
  app::{Tree, events::MouseButton},
  components::TextInputOverflow,
  core::Signal,
  layout::{Constraints, Size},
};

use crate::support::{render_pass, run_pass};

#[test]
fn renders_current_value_and_can_be_found_by_rendered_text() {
  let value = Signal::new("Ada".to_owned());
  let mut runtime = Tree::new();

  runtime.set_root(lurq::components::TextInput::new(value).placeholder("Name"));
  run_pass(&mut runtime);

  let found = runtime.find_element(|el| el.text_content() == Some("Ada"));
  assert!(found.is_some(), "text input should render its current signal value");
}

#[test]
fn renders_placeholder_when_value_is_empty() {
  let value = Signal::new(String::new());
  let mut runtime = Tree::new();

  runtime.set_root(lurq::components::TextInput::new(value).placeholder("Name"));
  run_pass(&mut runtime);

  let found = runtime.find_element(|el| el.text_content() == Some("Name"));
  assert!(found.is_some(), "empty text input should render its placeholder");
}

#[test]
fn scroll_overflow_keeps_single_line_height_for_long_text() {
  let value = Signal::new("This is a long text input value that should scroll".to_owned());
  let mut runtime = Tree::new();

  runtime.set_layout_constraints_override(Some(Constraints::loose(Size::new(400.0, 400.0))));
  runtime.set_root(lurq::components::TextInput::new(value));
  run_pass(&mut runtime);

  let bounds = runtime.find_element(|_| true).unwrap().bounds();
  assert_eq!(bounds.width, 120.0);
  assert_eq!(bounds.height, 28.0);
}

#[test]
fn scroll_overflow_keeps_glyphs_visible_when_text_is_scrolled() {
  let value = Signal::new("Mira Cqwewqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq".to_owned());
  let mut runtime = Tree::new();

  runtime.set_layout_constraints_override(Some(Constraints::loose(Size::new(400.0, 400.0))));
  runtime.set_root(lurq::components::TextInput::new(value).width(80.0));
  run_pass(&mut runtime);

  let bounds = runtime.find_element(|_| true).unwrap().bounds();
  runtime.click(bounds.x + 1.0, bounds.y + bounds.height / 2.0, MouseButton::Left);
  runtime.key_down("End".to_owned(), "End".to_owned(), false, false, false);

  let snapshot = render_pass(&mut runtime);
  assert!(
    snapshot.glyph_count > 0,
    "scrolled text input should still rasterize visible glyphs"
  );
}

#[test]
fn multiline_overflow_grows_after_enter() {
  let value = Signal::new("A".to_owned());
  let mut runtime = Tree::new();

  runtime.set_layout_constraints_override(Some(Constraints::loose(Size::new(400.0, 400.0))));
  runtime.set_root(lurq::components::TextInput::new(value.clone()).overflow(TextInputOverflow::Multiline));
  run_pass(&mut runtime);
  let initial = runtime.find_element(|_| true).unwrap().bounds();
  let (x, y) = initial.center();

  runtime.click(x, y, MouseButton::Left);
  runtime.key_down("Enter".to_owned(), "Enter".to_owned(), false, false, false);
  runtime.key_down("B".to_owned(), "KeyB".to_owned(), false, false, false);
  run_pass(&mut runtime);

  let grown = runtime.find_element(|_| true).unwrap().bounds();
  assert_eq!(value.get(), "A\nB");
  assert!(
    grown.height > initial.height,
    "multiline input should grow to contain new lines"
  );
}
