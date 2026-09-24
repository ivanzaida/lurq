use std::sync::{
  Arc,
  atomic::{AtomicUsize, Ordering},
};

use lurq::{
  components::{Button, Checkbox, Column, Rect, TextInput},
  core::Signal,
};

use super::{click_id, focused_id, headless, tab};

#[test]
fn focusable_false_button_never_takes_focus() {
  let clicks = Arc::new(AtomicUsize::new(0));
  let mut tree = headless(
    Column::new()
      .child(Button::new("Bold").id("bold").tab_index(0).focusable(false).on_click({
        let clicks = clicks.clone();
        move |_| {
          clicks.fetch_add(1, Ordering::SeqCst);
        }
      }))
      .child(Button::new("Next").id("next").tab_index(0)),
  );

  click_id(&mut tree, "bold");
  assert_eq!(clicks.load(Ordering::SeqCst), 1, "the click still runs");
  assert_eq!(focused_id(&tree), None, "but does not focus");

  tab(&mut tree);
  assert_eq!(focused_id(&tree).as_deref(), Some("next"), "Tab skips it");

  tree.get_element_by_id_mut("bold").unwrap().focus();
  assert_eq!(focused_id(&tree).as_deref(), Some("next"), "focus() is refused");
}

#[test]
fn focusable_false_text_input_and_checkbox_do_not_take_focus_on_click() {
  let checked = Signal::new(false);
  let mut tree = headless(
    Column::new()
      .child(
        TextInput::new(Signal::new(String::new()))
          .id("input")
          .width(120.0)
          .focusable(false),
      )
      .child(Checkbox::new(checked.clone()).id("check").focusable(false)),
  );

  click_id(&mut tree, "input");
  assert_eq!(focused_id(&tree), None);
  click_id(&mut tree, "check");
  assert!(checked.get_untracked(), "the checkbox still toggles");
  assert_eq!(focused_id(&tree), None);
}

#[test]
fn element_with_tab_index_is_focusable_by_click_and_tab() {
  let mut tree = headless(
    Column::new()
      .child(Rect::new(80.0, 30.0).id("card").tab_index(0))
      .child(Rect::new(80.0, 30.0).id("plain")),
  );

  tab(&mut tree);
  assert_eq!(focused_id(&tree).as_deref(), Some("card"));

  click_id(&mut tree, "plain");
  assert_eq!(
    focused_id(&tree),
    None,
    "a plain rect does not take focus; the press blurs"
  );

  click_id(&mut tree, "card");
  assert_eq!(focused_id(&tree).as_deref(), Some("card"));
}

#[test]
fn focusable_true_element_is_focused_by_click_but_not_a_tab_stop_outside_forms() {
  let mut tree = headless(Column::new().child(Rect::new(80.0, 30.0).id("tile").focusable(true)));

  tab(&mut tree);
  assert_eq!(focused_id(&tree), None);

  click_id(&mut tree, "tile");
  assert_eq!(focused_id(&tree).as_deref(), Some("tile"));
}
