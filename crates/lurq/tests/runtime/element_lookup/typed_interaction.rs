use std::sync::{
  Arc,
  atomic::{AtomicUsize, Ordering},
};

use lurq::{
  app::{Tree, events::TextInputEvent},
  components::{Checkbox, Column, Slider, Text, TextInput},
  core::Signal,
};

use crate::support::run_pass;

#[test]
fn set_value_writes_signal_without_firing_on_input() {
  let value = Signal::new("hello".to_owned());
  let inputs = Arc::new(AtomicUsize::new(0));
  let observed = inputs.clone();
  let mut tree = Tree::new();
  tree.set_root(
    Column::new().child(
      TextInput::new(value.clone())
        .id("field")
        .on_input(move |_event: TextInputEvent| {
          observed.fetch_add(1, Ordering::Relaxed);
        }),
    ),
  );
  run_pass(&mut tree);

  let mut input = tree
    .get_element_by_id_mut("field")
    .unwrap()
    .as_text_input()
    .expect("node should downcast to a text input");
  input.set_value("replaced");

  assert_eq!(value.get(), "replaced");
  assert_eq!(input.value(), "replaced");
  assert_eq!(inputs.load(Ordering::Relaxed), 0, "set_value must not fire on_input");
  assert!(tree.needs_redraw());

  // The rendered text content syncs from the signal on the next pass.
  run_pass(&mut tree);
  assert_eq!(
    tree.get_element_by_id("field").unwrap().text_content(),
    Some("replaced")
  );
}

#[test]
fn set_value_shrinking_text_keeps_editing_consistent() {
  let value = Signal::new("a longer initial value".to_owned());
  let mut tree = Tree::new();
  tree.set_root(Column::new().child(TextInput::new(value.clone()).id("field")));
  run_pass(&mut tree);

  // Shrink the value; the caret (at the old end) must be clamped so that
  // subsequent typing appends instead of panicking on an out-of-range index.
  let mut handle = tree.get_element_by_id_mut("field").unwrap();
  handle.focus();
  handle.as_text_input().unwrap().set_value("ab");
  run_pass(&mut tree);

  tree.key_down("C".to_owned(), "KeyC".to_owned(), false, false, false);
  assert_eq!(value.get(), "abC");
}

#[test]
fn checkbox_handle_toggles_signal() {
  let checked = Signal::new(false);
  let mut tree = Tree::new();
  tree.set_root(Column::new().child(Checkbox::new(checked.clone()).id("agree")));
  run_pass(&mut tree);

  let mut checkbox = tree.get_element_by_id_mut("agree").unwrap().as_checkbox().unwrap();
  assert!(!checkbox.is_checked());
  checkbox.toggle();
  assert!(checkbox.is_checked());
  assert!(checked.get());

  checkbox.set_checked(false);
  assert!(!checked.get());
}

#[test]
fn slider_handle_drives_signal() {
  let volume = Signal::new(0);
  let mut tree = Tree::new();
  tree.set_root(Column::new().child(Slider::new(volume.clone()).id("volume").range(0, 10)));
  run_pass(&mut tree);

  let mut slider = tree.get_element_by_id_mut("volume").unwrap().as_slider().unwrap();
  slider.set_from_ratio(1.0);
  assert_eq!(volume.get(), 10);
  slider.nudge(-2);
  assert_eq!(volume.get(), 8);
  assert_eq!(slider.value(), 8.0);
}

#[test]
fn downcast_to_wrong_kind_returns_none() {
  let mut tree = Tree::new();
  tree.set_root(Column::new().child(Text::new("plain").id("label")));

  assert!(tree.get_element_by_id_mut("label").unwrap().as_text_input().is_none());
  assert!(tree.get_element_by_id_mut("label").unwrap().as_checkbox().is_none());
  assert!(tree.get_element_by_id_mut("label").unwrap().as_slider().is_none());
  assert!(tree.get_element_by_id_mut("label").unwrap().as_select().is_none());
}
