//! `on_wheel`: a wheel turned over an element is that element's first, whether or not anything under the
//! pointer can scroll, and an element that takes it stops what it would have scrolled.

use std::sync::{
  Arc,
  atomic::{AtomicUsize, Ordering},
};

use lurq::{
  app::{
    Tree,
    events::{ScrollEvent, ScrollPhase},
  },
  components::{Rect, ScrollVertical},
  layout::layout_kind::ScrollState,
};

use crate::support::run_pass;

/// A list of 300 by 150 whose content is `content` high, which counts the wheels and the scrolls it hears of,
/// and takes the wheel where `takes`.
fn list(content: f32, takes: bool) -> (Tree, ScrollState, Arc<AtomicUsize>, Arc<AtomicUsize>) {
  let state = ScrollState::new();
  let (wheels, scrolls) = (Arc::new(AtomicUsize::new(0)), Arc::new(AtomicUsize::new(0)));
  let (wheel, scroll) = (wheels.clone(), scrolls.clone());
  let mut tree = Tree::new();
  tree.set_root(
    ScrollVertical::new(Rect::new(300.0, content).background("#1f2937"))
      .with_scroll_state(state.clone())
      .size(300.0, 150.0)
      .on_wheel(move |event: ScrollEvent| {
        wheel.fetch_add(1, Ordering::SeqCst);
        if takes {
          event.prevent_default();
        }
      })
      .on_scroll(move |_: ScrollEvent| {
        scroll.fetch_add(1, Ordering::SeqCst);
      }),
  );
  run_pass(&mut tree);
  (tree, state, wheels, scrolls)
}

fn wheel(tree: &mut Tree, delta_y: f32) {
  tree.scroll(150.0, 75.0, 0.0, delta_y, ScrollPhase::Scroll);
  run_pass(tree);
}

#[test]
fn it_is_heard_over_a_list_that_has_nowhere_to_scroll() {
  let (mut tree, _, wheels, scrolls) = list(150.0, false);
  wheel(&mut tree, -60.0);
  assert_eq!(wheels.load(Ordering::SeqCst), 1);
  assert_eq!(
    scrolls.load(Ordering::SeqCst),
    0,
    "a scroll is what a list does, and this one did none"
  );
}

#[test]
fn an_element_that_takes_it_stops_the_scroll() {
  let (mut tree, state, wheels, scrolls) = list(400.0, true);
  wheel(&mut tree, -60.0);
  assert_eq!(wheels.load(Ordering::SeqCst), 1);
  assert_eq!(state.scroll_y(), 0.0);
  assert_eq!(scrolls.load(Ordering::SeqCst), 0);
}

#[test]
fn one_that_only_listens_leaves_the_list_to_scroll() {
  let (mut tree, state, wheels, scrolls) = list(400.0, false);
  wheel(&mut tree, -60.0);
  assert_eq!(wheels.load(Ordering::SeqCst), 1);
  assert!(state.scroll_y() > 0.0);
  assert_eq!(scrolls.load(Ordering::SeqCst), 1);
}
