use std::sync::{
  Arc,
  atomic::{AtomicUsize, Ordering},
};

#[allow(deprecated)]
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
#[allow(deprecated)]
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

/// Two buttons the pointer reaches: Tab passes them by, a press leaves the focus in the field, and the app can still
/// focus one, which Enter presses then and Tab goes on from. In a focus group the arrows still move to them.
fn pointer_only(clicks: &Arc<AtomicUsize>) -> Column {
  let counted = clicks.clone();
  Column::new()
    .child(
      TextInput::new(Signal::new(String::new()))
        .id("field")
        .width(120.0)
        .tab_index(0),
    )
    .child(
      lurq::components::Row::new()
        .focus_group(lurq::node::FocusGroup::horizontal())
        .child(
          Button::new("Bold")
            .id("bold")
            .tab_index(0)
            .tab_stop(false)
            .focus_on_press(false)
            .on_click(move |_| {
              counted.fetch_add(1, Ordering::SeqCst);
            }),
        )
        .child(Button::new("Italic").id("italic").tab_index(0).tab_stop(false)),
    )
    .child(Button::new("Next").id("next").tab_index(0))
}

#[test]
fn a_button_off_tab_is_passed_by_and_a_press_leaves_the_focus_where_it_was() {
  let clicks = Arc::new(AtomicUsize::new(0));
  let mut tree = headless(pointer_only(&clicks));

  tab(&mut tree);
  assert_eq!(focused_id(&tree).as_deref(), Some("field"));
  tab(&mut tree);
  assert_eq!(focused_id(&tree).as_deref(), Some("next"), "Tab passes both by");

  click_id(&mut tree, "field");
  click_id(&mut tree, "bold");
  assert_eq!(clicks.load(Ordering::SeqCst), 1, "the click runs");
  assert_eq!(
    focused_id(&tree).as_deref(),
    Some("field"),
    "and the field keeps the focus"
  );

  // One off Tab that a press focuses still: only `focus_on_press(false)` keeps the focus where it was.
  click_id(&mut tree, "italic");
  assert_eq!(focused_id(&tree).as_deref(), Some("italic"));
}

#[test]
fn a_button_off_tab_the_app_focuses_is_pressed_by_enter_and_its_group_arrows_move() {
  let clicks = Arc::new(AtomicUsize::new(0));
  let mut tree = headless(pointer_only(&clicks));

  tree.get_element_by_id_mut("bold").unwrap().focus();
  assert_eq!(focused_id(&tree).as_deref(), Some("bold"), "the app can focus it");
  tree.key_down("Enter".to_owned(), "Enter".to_owned(), false, false, false);
  assert_eq!(clicks.load(Ordering::SeqCst), 1, "Enter presses it");

  tree.key_down("ArrowRight".to_owned(), "ArrowRight".to_owned(), false, false, false);
  assert_eq!(
    focused_id(&tree).as_deref(),
    Some("italic"),
    "the group's arrows move to the next"
  );

  tab(&mut tree);
  assert_eq!(
    focused_id(&tree).as_deref(),
    Some("next"),
    "Tab goes on from where it is"
  );
}
