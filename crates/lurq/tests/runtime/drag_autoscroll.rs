//! A drag that scrolls its container (`drag_autoscroll`): while the pointer of a drag is at the edge of the scroll
//! container the dragged element is in, or past it, the content scrolls toward the pointer a step each frame, and
//! the drag's move handlers hear of each step. Off by default.

use std::{
  sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
  },
  time::{Duration, Instant},
};

use lurq::{
  app::{
    App, Tree,
    events::{DragEvent, MouseButton, MouseButtonMask},
  },
  components::{Column, Rect, ScrollVertical},
  layout::layout_kind::ScrollState,
  node::Element,
};

use crate::support::render_pass_with_app;

const FRAME: Duration = Duration::from_millis(16);

/// Fifty rows of 20 in a column that is dragged: 1000 of content in a viewport of 200.
fn list(state: &ScrollState, autoscroll: bool, moves: &Arc<AtomicUsize>) -> ScrollVertical {
  let mut column = Column::new().spacing(0.0);
  for _ in 0..50 {
    column = column.child(Rect::new(150.0, 20.0));
  }
  let moves = moves.clone();
  let column = column
    .start_drag_buttons(MouseButtonMask::LEFT)
    .drag_autoscroll(autoscroll)
    .on_drag_move(move |_: DragEvent| {
      moves.fetch_add(1, Ordering::Relaxed);
    });
  ScrollVertical::new(column)
    .with_scroll_state(state.clone())
    .size(200.0, 200.0)
}

struct Page {
  tree: Tree,
  app: App,
  now: Instant,
}

impl Page {
  fn new(root: impl Into<Element>) -> Self {
    let mut page = Self {
      tree: Tree::new(),
      app: App::new(),
      now: Instant::now(),
    };
    page.tree.set_animation_clock(Some(page.now));
    page.tree.set_root(root);
    render_pass_with_app(&mut page.tree, &mut page.app);
    page
  }

  /// `frames` passes, a frame apart.
  fn frames(&mut self, frames: u32) {
    for _ in 0..frames {
      self.now += FRAME;
      self.tree.set_animation_clock(Some(self.now));
      render_pass_with_app(&mut self.tree, &mut self.app);
    }
  }
}

#[test]
fn a_drag_held_at_the_edge_scrolls_the_container_and_tells_its_moves() {
  let (state, moves) = (ScrollState::new(), Arc::new(AtomicUsize::new(0)));
  let mut page = Page::new(list(&state, true, &moves));
  page.tree.mouse_down(80.0, 100.0, MouseButton::Left);
  // In the middle of the viewport nothing scrolls.
  page.tree.mouse_move(80.0, 120.0);
  page.frames(5);
  assert_eq!(state.scroll_y(), 0.0);
  let told = moves.load(Ordering::Relaxed);
  assert_eq!(told, 1, "the one move of the pointer");

  // At the bottom edge it scrolls down, a step each frame, with the pointer held still.
  page.tree.mouse_move(80.0, 198.0);
  page.frames(5);
  let after_five = state.scroll_y();
  assert!(after_five > 0.0, "the content scrolls down");
  page.frames(5);
  let after_ten = state.scroll_y();
  assert!(after_ten > after_five);
  // Each step told the drag that what is under the pointer moved.
  assert!(
    moves.load(Ordering::Relaxed) >= told + 10,
    "{}",
    moves.load(Ordering::Relaxed)
  );

  // Further past the edge it is faster.
  page.tree.mouse_move(80.0, 260.0);
  page.frames(5);
  assert!(state.scroll_y() - after_ten > after_ten - after_five);

  // Back in the middle it stops, and the top edge scrolls up.
  page.tree.mouse_move(80.0, 100.0);
  let held = state.scroll_y();
  page.frames(5);
  assert_eq!(state.scroll_y(), held);
  page.tree.mouse_move(80.0, 2.0);
  page.frames(5);
  assert!(state.scroll_y() < held, "the content scrolls up");

  // The release ends it.
  page.tree.mouse_up(80.0, 2.0, MouseButton::Left);
  let held = state.scroll_y();
  page.frames(5);
  assert_eq!(state.scroll_y(), held);
}

#[test]
fn it_stops_at_the_end_of_the_content_and_asks_for_no_more_frames() {
  let (state, moves) = (ScrollState::new(), Arc::new(AtomicUsize::new(0)));
  let mut page = Page::new(list(&state, true, &moves));
  page.tree.mouse_down(80.0, 100.0, MouseButton::Left);
  page.tree.mouse_move(80.0, 600.0);
  page.frames(400);
  assert_eq!(state.scroll_y(), 800.0, "the end of 1000 of content in 200");
  let told = moves.load(Ordering::Relaxed);
  page.frames(10);
  assert_eq!(moves.load(Ordering::Relaxed), told, "nothing moves at the end");
}

#[test]
fn a_drag_does_not_scroll_its_container_unless_it_says_so() {
  let (state, moves) = (ScrollState::new(), Arc::new(AtomicUsize::new(0)));
  let mut page = Page::new(list(&state, false, &moves));
  page.tree.mouse_down(80.0, 100.0, MouseButton::Left);
  page.tree.mouse_move(80.0, 198.0);
  page.frames(10);
  assert_eq!(state.scroll_y(), 0.0);
  assert_eq!(moves.load(Ordering::Relaxed), 1);
}
