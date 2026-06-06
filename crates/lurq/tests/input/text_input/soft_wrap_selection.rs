use lurq::{app::Tree, core::Signal};

use crate::support::{pointer_click, run_pass};
use lurq::app::events::MouseButton;

/// Clicking the second *visual* row of a soft-wrapped multiline input must
/// place the caret on that wrapped row, not back on the first row. Regression
/// for caret positions being computed without wrapping (so every wrapped
/// glyph collapsed onto the first row's y).
fn caret_index_after_left_edge_click(text: &str, click_from_top: bool) -> usize {
  let value = Signal::new(text.to_owned());
  let mut runtime = Tree::new();
  runtime.set_root(lurq::components::TextInput::new(value.clone()).multiline().width(60.0));
  run_pass(&mut runtime);

  let rect = runtime.find_element(|_| true).unwrap().bounds();
  assert!(rect.height > 0.0, "input should have a measured height");

  let y = if click_from_top {
    rect.y + 1.0
  } else {
    rect.y + rect.height - 1.0
  };
  pointer_click(&mut runtime, rect.x + 1.0, y, MouseButton::Left);
  runtime.key_down("X".to_owned(), "KeyX".to_owned(), false, false, false);

  value.get().find('X').expect("marker should be inserted")
}

#[test]
fn clicking_soft_wrapped_row_places_caret_on_that_row() {
  let text = "aaaa bbbb cccc dddd eeee ffff gggg hhhh";

  let first_row_caret = caret_index_after_left_edge_click(text, true);
  let last_row_caret = caret_index_after_left_edge_click(text, false);

  assert!(
    last_row_caret > first_row_caret,
    "clicking the last wrapped row should land later in the text than the first row \
     (first={first_row_caret}, last={last_row_caret})"
  );
}
