//! An element that clips and is laid out with no width or no height cuts away
//! all it holds: nothing inside it is a Tab stop or keeps the focus, as a
//! splitter's closed pane. One that lets what it holds show past it does not.

use lurq::{
  app::App,
  components::{Button, Row},
  core::ElementRefMut,
  node::dimension::Dimension,
};

use super::{focused_id, headless, tab};

fn button(id: &str) -> Button {
  Button::new(id).id(id.to_owned()).tab_index(0)
}

#[test]
fn nothing_in_an_element_of_no_width_that_clips_is_a_stop() {
  let mut tree = headless(
    Row::new()
      .child(Row::new().width(0.0).child(button("hidden")))
      .child(button("shown")),
  );
  tab(&mut tree);
  assert_eq!(focused_id(&tree).as_deref(), Some("shown"));
  tab(&mut tree);
  assert_eq!(focused_id(&tree).as_deref(), Some("shown"), "the only stop");
}

#[test]
fn what_shows_past_an_element_of_no_size_is_a_stop() {
  let mut tree = headless(
    Row::new()
      .child(Row::new().width(0.0).overflow_visible().child(button("past")))
      .child(button("shown")),
  );
  tab(&mut tree);
  assert_eq!(focused_id(&tree).as_deref(), Some("past"));
}

#[test]
fn closing_to_no_width_takes_the_focus_from_what_is_in_it() {
  let pane = ElementRefMut::new();
  let mut tree = headless(
    Row::new()
      .child(
        Row::new()
          .width(120.0)
          .ref_element(pane.clone())
          .child(button("inside")),
      )
      .child(button("after")),
  );
  tab(&mut tree);
  assert_eq!(focused_id(&tree).as_deref(), Some("inside"));
  pane.set_size(Some(Dimension::Px(0.0)), None);
  tree.pass_headless(&mut App::new());
  assert_eq!(focused_id(&tree), None, "the focus left what was cut away");
  tab(&mut tree);
  assert_eq!(focused_id(&tree).as_deref(), Some("after"));
}
