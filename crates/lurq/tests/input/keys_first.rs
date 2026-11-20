//! An element that hears keys first (`.keys_first(true)`): while the focus is in it, it and what is in it are told of
//! a key before the rest of the tree, and what they stop the rest does not hear; the window's default still acts.

use std::sync::{
  Arc, Mutex,
  atomic::{AtomicUsize, Ordering},
};

use lurq::{
  app::{App, Tree, events::KeyboardEvent},
  components::{Button, Column, Row},
  node::Element,
};

use crate::support::TestSurface;

/// Who was told of a key, in order: who, and which key, down or up.
type Log = Arc<Mutex<Vec<(&'static str, String)>>>;

/// A page that hears every key, a button before the panel, and the panel, which hears keys first, stops Enter and
/// holds a button that counts its presses.
fn scene(log: &Log, presses: &Arc<AtomicUsize>) -> Element {
  let (page, page_up, panel, panel_up) = (log.clone(), log.clone(), log.clone(), log.clone());
  let presses = presses.clone();
  Column::new()
    .child(
      Row::new()
        .size(10.0, 10.0)
        .on_key_down(move |event: KeyboardEvent| page.lock().unwrap().push(("page", event.key.clone())))
        .on_key_up(move |event: KeyboardEvent| page_up.lock().unwrap().push(("page up", event.key.clone()))),
    )
    .child(Button::new("Outside").id("outside").tab_index(0))
    .child(
      Column::new()
        .id("panel")
        .keys_first(true)
        .on_key_down(move |event: KeyboardEvent| {
          panel.lock().unwrap().push(("panel", event.key.clone()));
          if event.focus_within && event.key == "Enter" {
            event.stop_propagation();
          }
        })
        .on_key_up(move |event: KeyboardEvent| panel_up.lock().unwrap().push(("panel up", event.key.clone())))
        .child(
          Button::new("Inside")
            .id("inside")
            .tab_index(0)
            .on_click(move |_| {
              presses.fetch_add(1, Ordering::SeqCst);
            }),
        ),
    )
    .into()
}

fn key(tree: &mut Tree, app: &mut App, key: &str) {
  tree.key_down(key.to_owned(), key.to_owned(), false, false, false);
  tree.key_up(key.to_owned(), key.to_owned(), false, false, false);
  tree.pass(app, &TestSurface);
}

fn told(log: &Log) -> Vec<(&'static str, String)> {
  std::mem::take(&mut *log.lock().unwrap())
}

fn owned(log: &[(&'static str, &str)]) -> Vec<(&'static str, String)> {
  log.iter().map(|(who, key)| (*who, (*key).to_owned())).collect()
}

#[test]
fn the_panel_with_the_focus_hears_a_key_before_the_page_and_keeps_what_it_stops() {
  let (log, presses): (Log, _) = (Arc::new(Mutex::new(Vec::new())), Arc::new(AtomicUsize::new(0)));
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.set_root(scene(&log, &presses));
  tree.pass(&mut app, &TestSurface);

  key(&mut tree, &mut app, "Tab");
  told(&log);
  key(&mut tree, &mut app, "a");
  assert_eq!(
    told(&log),
    owned(&[("page", "a"), ("panel", "a"), ("page up", "a"), ("panel up", "a")]),
    "with the focus outside the panel, the order of the tree, each told once"
  );

  key(&mut tree, &mut app, "Tab");
  told(&log);
  key(&mut tree, &mut app, "a");
  assert_eq!(
    told(&log),
    owned(&[("panel", "a"), ("page", "a"), ("panel up", "a"), ("page up", "a")]),
    "with the focus in the panel, the panel first, then the rest, the panel not told twice"
  );

  key(&mut tree, &mut app, "Enter");
  assert_eq!(
    told(&log),
    owned(&[("panel", "Enter"), ("panel up", "Enter"), ("page up", "Enter")]),
    "the Enter the panel stops is not the page's"
  );
  assert_eq!(presses.load(Ordering::SeqCst), 1, "and still presses the focused button");
}
