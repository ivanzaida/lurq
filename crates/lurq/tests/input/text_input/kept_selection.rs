//! `TextInput::keep_selection`: a selection painted while another element has the focus, and where it is.

use lurq::{
  components::{Column, TextInput, TextInputController},
  core::Signal,
  node::color::Color,
};

use crate::support::{render_pass, run_pass};

const MARK: &str = "#ff00ff";

fn input(keep: bool) -> (lurq::app::Tree, TextInputController) {
  let controller = TextInputController::new();
  let mut tree = lurq::app::Tree::new();
  let mut input = TextInput::new(Signal::new("make this bold\nand this too".to_owned()))
    .multiline()
    .width(300.0)
    .id("editor")
    .selection_color(Color::from_hex(MARK))
    .controller(controller.clone());
  if keep {
    input = input.keep_selection(true);
  }
  tree.set_root(
    Column::new()
      .child(input)
      .child(TextInput::new(Signal::new(String::new())).id("other")),
  );
  run_pass(&mut tree);
  tree.get_element_by_id_mut("editor").unwrap().focus();
  controller.select(5, 9).unwrap();
  run_pass(&mut tree);
  (tree, controller)
}

/// The rectangles painted in the selection's color, at the opacity `alpha`.
fn marked(tree: &mut lurq::app::Tree, alpha: u8) -> usize {
  let mark = Color::from_hex(MARK);
  render_pass(tree)
    .rects
    .iter()
    .filter(|rect| {
      (rect.color.r(), rect.color.g(), rect.color.b(), rect.color.a()) == (mark.r(), mark.g(), mark.b(), alpha)
    })
    .count()
}

#[test]
fn a_kept_selection_stays_painted_at_half_its_opacity_while_another_element_has_the_focus() {
  let (mut tree, _) = input(true);
  assert_eq!(marked(&mut tree, 255), 1, "painted while it has the focus");
  tree.get_element_by_id_mut("other").unwrap().focus();
  run_pass(&mut tree);
  assert_eq!(marked(&mut tree, 255), 0);
  assert_eq!(
    marked(&mut tree, 127),
    1,
    "and at half its opacity once the focus is elsewhere"
  );
}

#[test]
fn a_selection_not_kept_is_painted_only_with_the_focus() {
  let (mut tree, controller) = input(false);
  tree.get_element_by_id_mut("other").unwrap().focus();
  run_pass(&mut tree);
  assert_eq!(marked(&mut tree, 255) + marked(&mut tree, 127), 0);
  assert_eq!(controller.selection().unwrap().range(), 5..9, "though it is kept");
}

#[test]
fn the_selected_text_has_a_rectangle_for_each_row_and_a_box_round_them() {
  let (mut tree, controller) = input(true);
  let one = controller.selection_rects().unwrap();
  assert_eq!(one.len(), 1);
  let caret = controller.caret_point(5).unwrap();
  assert_eq!(
    (one[0].x, one[0].y, one[0].height),
    (caret.x, caret.y, caret.height),
    "where the caret before it is"
  );
  assert!(one[0].width > 0.0);
  // From "bold" on the first line to "and" on the second.
  controller.select(10, 18).unwrap();
  run_pass(&mut tree);
  let two = controller.selection_rects().unwrap();
  assert_eq!(two.len(), 2, "{two:?}");
  let bounds = controller.selection_bounds().unwrap().expect("a box");
  assert_eq!(bounds.y, two[0].y);
  assert_eq!(bounds.y + bounds.height, two[1].y + two[1].height);
  assert_eq!(bounds.x, two[0].x.min(two[1].x));
  controller.select(3, 3).unwrap();
  run_pass(&mut tree);
  assert!(
    controller.selection_bounds().unwrap().is_none(),
    "none where nothing is selected"
  );
}
