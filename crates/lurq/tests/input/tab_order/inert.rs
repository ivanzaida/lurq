//! An inert element (`.inert(true)`, or `ElementRefMut::set_inert`): Tab and
//! a focus group's keys pass by everything in it, a focus request for it is
//! refused, no pointer reaches it, and the focus in it leaves when it becomes
//! inert, to the next stop of its group or to nothing.

use std::sync::{
  Arc,
  atomic::{AtomicUsize, Ordering},
};

use lurq::{
  app::{App, Tree, events::MouseEvent},
  components::{Button, Column, Row},
  core::ElementRefMut,
  node::FocusGroup,
};

use super::{click_id, focused_id, headless, tab};

fn stop(id: &str) -> Button {
  Button::new(id).id(id.to_owned()).tab_index(0)
}

fn key(tree: &mut Tree, key: &str) {
  tree.key_down(key.to_owned(), key.to_owned(), false, false, false);
  tree.key_up(key.to_owned(), key.to_owned(), false, false, false);
}

#[test]
fn tab_passes_by_everything_in_an_inert_element() {
  let mut tree = headless(
    Column::new()
      .child(stop("before"))
      .child(Row::new().inert(true).child(stop("inside")).child(stop("also")))
      .child(stop("after")),
  );
  tab(&mut tree);
  assert_eq!(focused_id(&tree).as_deref(), Some("before"));
  tab(&mut tree);
  assert_eq!(focused_id(&tree).as_deref(), Some("after"));
}

#[test]
fn a_focus_groups_arrows_pass_an_inert_element_by() {
  let mut tree = headless(
    Row::new()
      .focus_group(FocusGroup::horizontal())
      .child(stop("one"))
      .child(stop("two").inert(true))
      .child(stop("three")),
  );
  tab(&mut tree);
  assert_eq!(focused_id(&tree).as_deref(), Some("one"));
  key(&mut tree, "ArrowRight");
  assert_eq!(focused_id(&tree).as_deref(), Some("three"));
}

#[test]
fn a_focus_request_for_what_is_inert_is_refused() {
  let mut tree = headless(Column::new().child(Row::new().inert(true).child(stop("inside"))));
  tree.get_element_by_id_mut("inside").unwrap().focus();
  assert_eq!(focused_id(&tree), None);
}

#[test]
fn no_pointer_reaches_it_and_what_is_under_it_gets_the_press() {
  let (inside, under) = (Arc::new(AtomicUsize::new(0)), Arc::new(AtomicUsize::new(0)));
  let mut tree = headless({
    let (inside, under) = (inside.clone(), under.clone());
    Row::new()
      .id("under")
      .on_click(move |_: MouseEvent| {
        under.fetch_add(1, Ordering::SeqCst);
      })
      .child(stop("inside").inert(true).on_click(move |_: MouseEvent| {
        inside.fetch_add(1, Ordering::SeqCst);
      }))
  });
  click_id(&mut tree, "inside");
  assert_eq!(inside.load(Ordering::SeqCst), 0);
  assert_eq!(under.load(Ordering::SeqCst), 1);
  assert_eq!(focused_id(&tree), None, "nor does a press focus it");
}

#[test]
fn a_ref_makes_it_inert_and_the_focus_in_it_leaves() {
  let pane = ElementRefMut::new();
  let mut tree = headless(
    Column::new()
      .child(Row::new().ref_element(pane.clone()).child(stop("inside")))
      .child(stop("after")),
  );
  tab(&mut tree);
  assert_eq!(focused_id(&tree).as_deref(), Some("inside"));
  pane.set_inert(true);
  assert!(tree.needs_redraw(), "it asks for a pass");
  tree.pass_headless(&mut App::new());
  assert_eq!(focused_id(&tree), None, "the focus left it");
  tab(&mut tree);
  assert_eq!(focused_id(&tree).as_deref(), Some("after"));
  pane.set_inert(false);
  tree.pass_headless(&mut App::new());
  tab(&mut tree);
  assert_eq!(focused_id(&tree).as_deref(), Some("inside"), "a stop again");
}

#[test]
fn in_a_focus_group_the_focus_goes_to_the_next_stop() {
  let two = ElementRefMut::new();
  let mut tree = headless(
    Row::new()
      .focus_group(FocusGroup::horizontal())
      .child(stop("one"))
      .child(stop("two").ref_element(two.clone()))
      .child(stop("three")),
  );
  tab(&mut tree);
  key(&mut tree, "ArrowRight");
  assert_eq!(focused_id(&tree).as_deref(), Some("two"));
  two.set_inert(true);
  tree.pass_headless(&mut App::new());
  assert_eq!(
    focused_id(&tree).as_deref(),
    Some("three"),
    "the stop that takes its place, as for one taken out of the tree"
  );
}
