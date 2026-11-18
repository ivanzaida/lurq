//! What a step of undo takes back of what was typed: a run of typing on Apple's systems, a word on Windows
//! (`design/decisions.md`, "Undo takes back what was typed as the platform's own fields do").

use super::*;
use crate::core::NodeId;

fn key(text: &str) -> KeyboardEvent {
  KeyboardEvent::new(text, "", false, false, false, false, NodeId::UNASSIGNED)
}

/// An input of the platform's grouping, with no pause ending a step: the steps of these tests do not hang on the time
/// they take.
fn input(grouping: UndoGrouping) -> TextInputState {
  let state = TextInputState::new(Signal::new(String::new()));
  state.set_undo_grouping(grouping);
  state.set_typing_pause(None);
  state
}

fn typed(state: &TextInputState, text: &str) {
  for character in text.chars() {
    state.type_text(&character.to_string(), &key(&character.to_string()));
  }
}

/// What each undo leaves, until there is nothing more to take back.
fn undone(state: &TextInputState) -> Vec<String> {
  let mut values = Vec::new();
  while state.undo(&key("z")) {
    values.push(state.value());
  }
  values
}

#[test]
fn on_a_mac_a_run_of_typing_is_one_step() {
  let state = input(UndoGrouping::Runs);
  typed(&state, "hello world");
  assert_eq!(undone(&state), vec![""]);
}

#[test]
fn on_windows_each_word_with_its_spaces_is_a_step() {
  let state = input(UndoGrouping::Words);
  typed(&state, "hello big world");
  assert_eq!(undone(&state), vec!["hello big ", "hello ", ""]);
}

#[test]
fn moving_the_caret_ends_the_step() {
  for grouping in [UndoGrouping::Runs, UndoGrouping::Words] {
    let state = input(grouping);
    typed(&state, "ab");
    state.move_left(false);
    state.move_right(false);
    typed(&state, "cd");
    assert_eq!(undone(&state), vec!["ab", ""], "{grouping:?}");
  }
}

#[test]
fn deleting_after_typing_is_a_step_of_its_own_and_held_backspace_is_one() {
  for grouping in [UndoGrouping::Runs, UndoGrouping::Words] {
    let state = input(grouping);
    typed(&state, "abcd");
    for _ in 0..3 {
      state.backspace(&key("Backspace"));
    }
    typed(&state, "xy");
    assert_eq!(state.value(), "axy");
    assert_eq!(undone(&state), vec!["a", "abcd", ""], "{grouping:?}");
  }
}

#[test]
fn a_paste_is_a_step_of_its_own() {
  let state = input(UndoGrouping::Runs);
  typed(&state, "ab");
  state.insert("PASTED", &key("v"));
  typed(&state, "cd");
  assert_eq!(undone(&state), vec!["abPASTED", "ab", ""]);
}

#[test]
fn redo_brings_the_whole_step_back_and_typing_after_an_undo_is_a_new_step() {
  let state = input(UndoGrouping::Runs);
  typed(&state, "abc");
  assert!(state.undo(&key("z")));
  assert!(state.redo(&key("Z")));
  assert_eq!(state.value(), "abc");
  typed(&state, "de");
  assert_eq!(undone(&state), vec!["abc", ""]);
}

#[test]
fn a_pause_in_typing_ends_the_step() {
  for grouping in [UndoGrouping::Runs, UndoGrouping::Words] {
    let state = input(grouping);
    state.set_typing_pause(Some(std::time::Duration::from_millis(40)));
    typed(&state, "part");
    std::thread::sleep(std::time::Duration::from_millis(80));
    typed(&state, "more");
    assert_eq!(undone(&state), vec!["part", ""], "{grouping:?}");
  }
}

#[test]
fn the_pause_is_a_second_until_the_app_says_another() {
  assert_eq!(TYPING_UNDO_PAUSE, Some(std::time::Duration::from_secs(1)));
  let state = TextInputState::new(Signal::new(String::new()));
  state.set_undo_grouping(UndoGrouping::Runs);
  typed(&state, "quick");
  assert_eq!(undone(&state), vec![""], "keys a moment apart are one step");
}
