use lurq::{
  app::{Tree, events::MouseButton},
  components::TextInputOverflow,
  core::Signal,
  layout::{Constraints, Size},
};

use crate::support::{pointer_click, render_pass, run_pass};

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
fn placeholder_after_sizing_is_rendered() {
  let value = Signal::new(String::new());
  let mut runtime = Tree::new();

  runtime.set_root(lurq::components::TextInput::new(value).width(200.0).placeholder("Name"));
  run_pass(&mut runtime);

  let found = runtime.find_element(|el| el.text_content() == Some("Name"));
  assert!(found.is_some(), "text input placeholder should apply after sizing");
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
  assert!(
    (bounds.height - 19.2).abs() < 0.01,
    "default input height should follow text line height; got {}",
    bounds.height
  );
}

#[test]
fn scroll_overflow_keeps_glyphs_visible_when_text_is_scrolled() {
  let value = Signal::new("Mira Cqwewqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq".to_owned());
  let mut runtime = Tree::new();

  runtime.set_layout_constraints_override(Some(Constraints::loose(Size::new(400.0, 400.0))));
  runtime.set_root(lurq::components::TextInput::new(value).width(80.0));
  run_pass(&mut runtime);

  let bounds = runtime.find_element(|_| true).unwrap().bounds();
  pointer_click(
    &mut runtime,
    bounds.x + 1.0,
    bounds.y + bounds.height / 2.0,
    MouseButton::Left,
  );
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

  pointer_click(&mut runtime, x, y, MouseButton::Left);
  runtime.key_down("Enter".to_owned(), "Enter".to_owned(), false, false, false);
  run_pass(&mut runtime);

  let grown = runtime.find_element(|_| true).unwrap().bounds();
  assert_eq!(value.get(), "A\n");
  assert!(
    grown.height > initial.height,
    "multiline input should grow as soon as the caret moves to a new row"
  );

  runtime.key_down("B".to_owned(), "KeyB".to_owned(), false, false, false);
  run_pass(&mut runtime);
  let with_text = runtime.find_element(|_| true).unwrap().bounds();
  assert_eq!(value.get(), "A\nB");
  assert_eq!(
    with_text.height, grown.height,
    "typing the first character on the row should not be the moment that changes height"
  );
}

#[test]
fn rows_sets_multiline_minimum_height() {
  let value = Signal::new("A".to_owned());
  let mut runtime = Tree::new();

  runtime.set_layout_constraints_override(Some(Constraints::loose(Size::new(400.0, 400.0))));
  runtime.set_root(lurq::components::TextInput::new(value).rows(3, 6));
  run_pass(&mut runtime);

  let bounds = runtime.find_element(|_| true).unwrap().bounds();
  assert!(
    bounds.height > 50.0,
    "three visible rows should be taller than the default input height; got {}",
    bounds.height
  );
}

#[test]
fn max_rows_caps_multiline_growth() {
  let value = Signal::new("A\nB\nC\nD\nE\nF".to_owned());
  let mut capped = Tree::new();
  let mut uncapped = Tree::new();

  capped.set_layout_constraints_override(Some(Constraints::loose(Size::new(400.0, 400.0))));
  capped.set_root(lurq::components::TextInput::new(value.clone()).rows(2, 3));
  run_pass(&mut capped);

  uncapped.set_layout_constraints_override(Some(Constraints::loose(Size::new(400.0, 400.0))));
  uncapped.set_root(lurq::components::TextInput::new(value).multiline());
  run_pass(&mut uncapped);

  let capped_bounds = capped.find_element(|_| true).unwrap().bounds();
  let uncapped_bounds = uncapped.find_element(|_| true).unwrap().bounds();
  assert!(
    capped_bounds.height < uncapped_bounds.height,
    "max rows should cap multiline growth; capped={}, uncapped={}",
    capped_bounds.height,
    uncapped_bounds.height
  );
}
