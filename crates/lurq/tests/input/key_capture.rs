//! An element that takes every key of the window (`.captures_keys(true)`): its own handlers alone are told of a key,
//! and nothing the window does with a key is done, until it lets go.

use std::sync::{Arc, Mutex};

use lurq::{
  app::{App, Tree, events::KeyboardEvent},
  components::{Button, Column, Row},
  node::Element,
};

use crate::support::TestSurface;

/// What was told of a key, in order: who, which key, and whether the focus was within.
type Log = Arc<Mutex<Vec<(&'static str, String, bool)>>>;

fn scene(captures: bool, log: &Log) -> Element {
  let (own, other) = (log.clone(), log.clone());
  Column::new()
    .child(
      Row::new()
        .size(100.0, 20.0)
        .id("recorder")
        .captures_keys(captures)
        .on_key_down(move |event: KeyboardEvent| {
          own
            .lock()
            .unwrap()
            .push(("recorder", event.key.clone(), event.focus_within))
        }),
    )
    .child(Button::new("First").id("first").tab_index(0))
    .child(Button::new("Second").id("second").tab_index(0))
    .child(
      Row::new()
        .size(10.0, 10.0)
        .on_key_down(move |event: KeyboardEvent| other.lock().unwrap().push(("app", event.key.clone(), false))),
    )
    .into()
}

fn tree(captures: bool, log: &Log, app: &mut App) -> Tree {
  let mut tree = Tree::new();
  tree.set_root(scene(captures, log));
  tree.pass(app, &TestSurface);
  tree
}

fn key(tree: &mut Tree, app: &mut App, key: &str) {
  tree.key_down(key.to_owned(), key.to_owned(), false, false, false);
  tree.key_up(key.to_owned(), key.to_owned(), false, false, false);
  tree.pass(app, &TestSurface);
}

fn focused(tree: &Tree) -> Option<String> {
  tree
    .focused_element()
    .and_then(|element| element.id().map(str::to_owned))
}

#[test]
fn while_an_element_captures_the_keys_it_alone_hears_them_and_tab_moves_nothing() {
  let log: Log = Arc::new(Mutex::new(Vec::new()));
  let mut app = App::new();
  let mut tree = tree(true, &log, &mut app);
  for name in ["Tab", "Enter", "Escape", "a"] {
    key(&mut tree, &mut app, name);
  }
  assert_eq!(
    *log.lock().unwrap(),
    [
      ("recorder", "Tab".to_owned(), true),
      ("recorder", "Enter".to_owned(), true),
      ("recorder", "Escape".to_owned(), true),
      ("recorder", "a".to_owned(), true),
    ],
    "no other handler hears a key"
  );
  assert_eq!(focused(&tree), None, "Tab moved no focus");
}

#[test]
fn an_element_that_lets_go_gives_the_keys_back_to_the_window() {
  let log: Log = Arc::new(Mutex::new(Vec::new()));
  let mut app = App::new();
  let mut tree = tree(true, &log, &mut app);
  key(&mut tree, &mut app, "Tab");
  tree.set_root(scene(false, &log));
  tree.pass(&mut app, &TestSurface);
  log.lock().unwrap().clear();
  key(&mut tree, &mut app, "Tab");
  assert_eq!(focused(&tree).as_deref(), Some("first"), "Tab moves the focus again");
  let heard: Vec<&'static str> = log.lock().unwrap().iter().map(|(who, ..)| *who).collect();
  assert!(heard.contains(&"app") && heard.contains(&"recorder"), "{heard:?}");
}

#[test]
fn a_disabled_element_captures_nothing() {
  let log: Log = Arc::new(Mutex::new(Vec::new()));
  let mut app = App::new();
  let mut tree = Tree::new();
  let own = log.clone();
  tree.set_root(
    Column::new()
      .child(
        Row::new()
          .size(100.0, 20.0)
          .captures_keys(true)
          .disabled(true)
          .on_key_down(move |event: KeyboardEvent| own.lock().unwrap().push(("recorder", event.key.clone(), true))),
      )
      .child(Button::new("First").id("first").tab_index(0)),
  );
  tree.pass(&mut app, &TestSurface);
  key(&mut tree, &mut app, "Tab");
  assert_eq!(focused(&tree).as_deref(), Some("first"));
}
