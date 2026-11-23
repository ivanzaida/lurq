//! `TextInput::on_caret`: where the caret is, told as it moves and as the selection changes.

use std::sync::{Arc, Mutex};

use lurq::{
  app::{Tree, events::MouseButton},
  components::{CaretInfo, TextInput, TextInputController},
  core::Signal,
};

use crate::support::run_pass;

const SCRIPT: &str = "echo one\necho two\nexit";

fn editor() -> (Tree, TextInputController, Arc<Mutex<Vec<CaretInfo>>>) {
  let told = Arc::new(Mutex::new(Vec::new()));
  let heard = told.clone();
  let controller = TextInputController::new();
  let mut tree = Tree::new();
  tree.set_root(
    TextInput::new(Signal::new(SCRIPT.to_owned()))
      .multiline()
      .width(300.0)
      .id("editor")
      .controller(controller.clone())
      .on_caret(move |caret: &CaretInfo| heard.lock().unwrap().push(*caret)),
  );
  run_pass(&mut tree);
  (tree, controller, told)
}

fn last(told: &Arc<Mutex<Vec<CaretInfo>>>) -> Option<CaretInfo> {
  told.lock().unwrap().last().copied()
}

#[test]
fn the_caret_is_told_by_its_line_and_column_as_keys_move_it() {
  let (mut tree, _, told) = editor();
  assert!(told.lock().unwrap().is_empty(), "nothing until it has the focus");
  tree.get_element_by_id_mut("editor").unwrap().focus();
  run_pass(&mut tree);
  let at = last(&told).expect("told as it takes the focus");
  assert_eq!((at.line, at.column), (3, 5), "at the end of `exit`");
  tree.key_down("ArrowUp".to_owned(), "ArrowUp".to_owned(), false, false, false);
  run_pass(&mut tree);
  let up = last(&told).unwrap();
  // Up keeps the caret's x: the column under the end of `exit` in the line above.
  assert_eq!((up.line, up.selected()), (2, None));
  assert!(up.column > 1 && up.column < 9, "{up:?}");
  let count = told.lock().unwrap().len();
  run_pass(&mut tree);
  assert_eq!(told.lock().unwrap().len(), count, "a frame with no move tells nothing");
  tree.key_down("ArrowLeft".to_owned(), "ArrowLeft".to_owned(), true, false, false);
  run_pass(&mut tree);
  let at = last(&told).unwrap();
  assert_eq!((at.line, at.column), (2, up.column - 1));
  let caret = up.selection.caret;
  assert_eq!(at.selected(), Some(caret - 1..caret), "Shift selects");
}

#[test]
fn a_click_a_drag_and_the_apps_own_selection_are_told() {
  let (mut tree, controller, told) = editor();
  let rect = tree.get_element_by_id_mut("editor").unwrap().bounds().unwrap();
  let (x, y) = (rect.x + 2.0, rect.y + 3.0);
  tree.mouse_move(x, y);
  tree.mouse_down(x, y, MouseButton::Left);
  tree.mouse_up(x, y, MouseButton::Left);
  run_pass(&mut tree);
  let at = last(&told).expect("a click focuses it and places the caret");
  assert_eq!((at.line, at.column, at.selected()), (1, 1, None));
  tree.mouse_down(x, y, MouseButton::Left);
  tree.mouse_move(rect.x + rect.width - 2.0, y);
  run_pass(&mut tree);
  let at = last(&told).unwrap();
  assert_eq!(
    at.selected(),
    Some(0..8),
    "a drag along the first line selects it, told before it is let go"
  );
  tree.mouse_up(rect.x + rect.width - 2.0, y, MouseButton::Left);
  controller.select(9, 13).unwrap();
  run_pass(&mut tree);
  let at = last(&told).unwrap();
  assert_eq!((at.line, at.column, at.selected()), (2, 5, Some(9..13)));
}
