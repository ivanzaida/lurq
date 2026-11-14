//! `on_change` and `on_change_end` of the engine's slider: told of what a person does (a drag as it moves, a press,
//! a key) and of the end of it, given the low value and the high one, and never of what the app sets nor of a range
//! or a step that snaps the value.

use std::sync::{Arc, Mutex};

use lurq::{
  app::{Tree, events::MouseButton},
  components::Slider,
  core::Signal,
};

use crate::support::run_pass;

type Heard = Arc<Mutex<Vec<String>>>;

fn slider(value: &Signal<i32>, heard: &Heard) -> Slider {
  let (told, ended) = (heard.clone(), heard.clone());
  Slider::new(value.clone())
    .range(0, 10)
    .width(100.0)
    .tab_index(0)
    .on_change(move |low, high| told.lock().unwrap().push(format!("change {low} {high}")))
    .on_change_end(move |low, _| ended.lock().unwrap().push(format!("end {low}")))
}

fn take(heard: &Heard) -> Vec<String> {
  std::mem::take(&mut *heard.lock().unwrap())
}

#[test]
fn a_drag_is_told_as_it_moves_and_when_it_is_let_go() {
  let (value, heard) = (Signal::new(0), Heard::default());
  let mut tree = Tree::new();
  tree.set_root(slider(&value, &heard));
  run_pass(&mut tree);
  let rect = tree.find_element(|_| true).unwrap().bounds();
  let y = rect.y + rect.height / 2.0;

  tree.mouse_down(rect.x, y, MouseButton::Left);
  assert!(take(&heard).is_empty(), "a press where the value is changes nothing");
  tree.mouse_move(rect.x + 75.0, y);
  tree.mouse_move(rect.x + 76.0, y);
  assert_eq!(take(&heard), ["change 8 8"], "once for each value it goes through");
  tree.mouse_up(rect.x + 76.0, y, MouseButton::Left);
  assert_eq!(take(&heard), ["end 8"]);
}

#[test]
fn a_key_is_told_and_over_at_once_and_what_the_app_sets_is_not_told() {
  let (value, heard) = (Signal::new(4), Heard::default());
  let mut tree = Tree::new();
  tree.set_root(slider(&value, &heard));
  run_pass(&mut tree);
  tree.key_down("Tab".to_owned(), "Tab".to_owned(), false, false, false);
  tree.key_down("ArrowRight".to_owned(), "ArrowRight".to_owned(), false, false, false);
  assert_eq!(take(&heard), ["change 5 5", "end 5"]);

  value.set(9);
  run_pass(&mut tree);
  // A narrower range snaps the value, and is not told either.
  tree.set_root(slider(&value, &heard).range(0, 6));
  run_pass(&mut tree);
  assert_eq!(value.get(), 6);
  assert!(take(&heard).is_empty());
}

#[test]
fn a_drag_taken_back_to_where_it_began_ends_nothing() {
  let (value, heard) = (Signal::new(0), Heard::default());
  let mut tree = Tree::new();
  tree.set_root(slider(&value, &heard));
  run_pass(&mut tree);
  let rect = tree.find_element(|_| true).unwrap().bounds();
  let y = rect.y + rect.height / 2.0;

  tree.mouse_down(rect.x, y, MouseButton::Left);
  tree.mouse_move(rect.x + 75.0, y);
  tree.mouse_move(rect.x, y);
  assert_eq!(take(&heard), ["change 8 8", "change 0 0"]);
  tree.mouse_up(rect.x, y, MouseButton::Left);
  assert!(take(&heard).is_empty(), "the value is what it was as the drag began");
}
