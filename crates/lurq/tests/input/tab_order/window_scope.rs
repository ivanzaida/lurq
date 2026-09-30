use lurq::{
  components::{Button, Column, Row, TextInput},
  core::Signal,
};

use super::{click_id, focused_id, headless, shift_tab, tab};

fn toolbar() -> Column {
  Column::new()
    .child(
      Row::new()
        .child(Button::new("New").id("new").tab_index(0))
        .child(Button::new("Open").id("open").tab_index(0))
        .child(Button::new("Help").id("help")),
    )
    .child(Button::new("Save").id("save").tab_index(0))
}

#[test]
fn tab_outside_forms_visits_only_elements_with_a_tab_index() {
  let mut tree = headless(toolbar());

  tab(&mut tree);
  assert_eq!(focused_id(&tree).as_deref(), Some("new"));
  tab(&mut tree);
  assert_eq!(focused_id(&tree).as_deref(), Some("open"));
  tab(&mut tree);
  assert_eq!(focused_id(&tree).as_deref(), Some("save"), "unset tab_index is skipped");
  tab(&mut tree);
  assert_eq!(focused_id(&tree).as_deref(), Some("new"), "Tab wraps around the window");
}

#[test]
fn shift_tab_outside_forms_moves_backward_and_wraps() {
  let mut tree = headless(toolbar());

  shift_tab(&mut tree);
  assert_eq!(focused_id(&tree).as_deref(), Some("save"));
  shift_tab(&mut tree);
  assert_eq!(focused_id(&tree).as_deref(), Some("open"));
}

#[test]
fn positive_tab_indices_come_first_in_ascending_order() {
  let mut tree = headless(
    Column::new()
      .child(Button::new("A").id("a").tab_index(0))
      .child(Button::new("B").id("b").tab_index(2))
      .child(Button::new("C").id("c").tab_index(1))
      .child(Button::new("D").id("d").tab_index(0)),
  );

  let mut order = Vec::new();
  for _ in 0..4 {
    tab(&mut tree);
    order.push(focused_id(&tree).unwrap());
  }
  assert_eq!(order, ["c", "b", "a", "d"]);
}

#[test]
fn negative_tab_index_is_skipped_by_tab_but_focused_by_click() {
  let mut tree = headless(
    Column::new()
      .child(Button::new("Skipped").id("skipped").tab_index(-1))
      .child(Button::new("Next").id("next").tab_index(0)),
  );

  tab(&mut tree);
  assert_eq!(focused_id(&tree).as_deref(), Some("next"));

  click_id(&mut tree, "skipped");
  assert_eq!(focused_id(&tree).as_deref(), Some("skipped"));
}

#[test]
fn tab_after_clicking_a_button_without_tab_index_continues_from_it() {
  let mut tree = headless(toolbar());

  click_id(&mut tree, "help");
  assert_eq!(focused_id(&tree).as_deref(), Some("help"), "a click focuses the button");

  tab(&mut tree);
  assert_eq!(
    focused_id(&tree).as_deref(),
    Some("save"),
    "Tab is not dead after a click"
  );

  click_id(&mut tree, "help");
  shift_tab(&mut tree);
  assert_eq!(focused_id(&tree).as_deref(), Some("open"));
}

#[test]
fn text_input_outside_form_without_tab_index_is_not_a_stop() {
  let mut tree = headless(
    Column::new()
      .child(TextInput::new(Signal::new(String::new())).id("search"))
      .child(Button::new("Go").id("go").tab_index(0)),
  );

  tab(&mut tree);
  assert_eq!(focused_id(&tree).as_deref(), Some("go"));
  tab(&mut tree);
  assert_eq!(focused_id(&tree).as_deref(), Some("go"));
}

#[test]
fn tab_without_stops_is_not_handled_and_keeps_focus_empty() {
  let mut tree = headless(Column::new().child(Button::new("Help").id("help")));

  tab(&mut tree);
  assert_eq!(focused_id(&tree), None);
}
