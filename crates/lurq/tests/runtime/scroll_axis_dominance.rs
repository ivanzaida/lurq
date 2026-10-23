//! A scroll container that scrolls sideways only takes no part of a mostly
//! vertical gesture: a slightly slanted trackpad swipe over it scrolls the
//! page around it and leaves it where it is, while a mostly sideways swipe
//! and a wheel with Shift (a delta on x alone) scroll it.

use lurq::{
  app::{Tree, events::ScrollPhase},
  components::{Column, Rect, ScrollHorizontal, ScrollVertical},
  layout::layout_kind::ScrollState,
};

use crate::support::run_pass;

const POINTER: (f32, f32) = (200.0, 25.0);

/// A 400x200 page scrolling down, with a 400x50 strip scrolling sideways at
/// its top and more below; the pointer rests on the strip.
fn page() -> (Tree, ScrollState, ScrollState) {
  let (page, strip) = (ScrollState::new(), ScrollState::new());
  let mut tree = Tree::new();
  tree.set_root(
    ScrollVertical::new(
      Column::new()
        .spacing(0.0)
        .child(
          ScrollHorizontal::new(Rect::new(2000.0, 50.0).background("#1f2937"))
            .with_scroll_state(strip.clone())
            .size(400.0, 50.0),
        )
        .child(Rect::new(400.0, 1000.0).background("#111827")),
    )
    .with_scroll_state(page.clone())
    .size(400.0, 200.0),
  );
  run_pass(&mut tree);
  (tree, page, strip)
}

/// A trackpad swipe of `times` moves, each `(dx, dy)`.
fn swipe(tree: &mut Tree, dx: f32, dy: f32, times: usize) {
  tree.scroll(POINTER.0, POINTER.1, 0.0, 0.0, ScrollPhase::Start);
  for _ in 0..times {
    tree.scroll(POINTER.0, POINTER.1, dx, dy, ScrollPhase::Scroll);
  }
  tree.scroll(POINTER.0, POINTER.1, 0.0, 0.0, ScrollPhase::End);
}

#[test]
fn a_mostly_vertical_swipe_scrolls_the_page_and_leaves_the_sideways_strip() {
  let (mut tree, page, strip) = page();
  swipe(&mut tree, -3.0, -6.0, 5);
  assert_eq!(page.scroll_y(), 30.0, "the page takes the whole vertical delta");
  assert_eq!(strip.scroll_x(), 0.0, "the strip is not nudged by the slant");
}

#[test]
fn a_mostly_sideways_swipe_scrolls_the_strip_and_its_slant_the_page() {
  let (mut tree, page, strip) = page();
  swipe(&mut tree, -6.0, -3.0, 5);
  assert_eq!(strip.scroll_x(), 30.0, "the strip takes the sideways delta");
  assert_eq!(page.scroll_y(), 15.0, "the page takes the vertical rest");
}

#[test]
fn a_delta_on_x_alone_scrolls_the_strip() {
  let (mut tree, page, strip) = page();
  tree.scroll(POINTER.0, POINTER.1, -40.0, 0.0, ScrollPhase::Scroll);
  assert_eq!(strip.scroll_x(), 40.0, "Shift+wheel, as the shell delivers it");
  assert_eq!(page.scroll_y(), 0.0);
  swipe(&mut tree, -5.0, -5.0, 2);
  assert_eq!(strip.scroll_x(), 50.0, "an even slant still counts as sideways");
  assert_eq!(page.scroll_y(), 10.0);
}
