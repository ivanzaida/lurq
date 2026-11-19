//! What a wheel's animated scroll leaves drawn under a pointer that holds still. A row has no fill at rest, one
//! under the pointer and another while pressed, and fades between two fills. Pressed and released, it is on its
//! way back to its hovered fill when the wheel turns: the scroll takes it from under the pointer, which did not
//! move, and brings other rows under it. When the content has stopped and nothing is left to animate, the row
//! under the pointer is drawn hovered and no other row has a fill.

use std::time::{Duration, Instant};

use lurq::{
  animation::{Easing, Transition},
  app::{
    App, Tree,
    events::{MouseButton, ScrollPhase},
  },
  components::{Column, Rect, ScrollVertical},
  node::color::Color,
};

use crate::support::{RenderSnapshot, render_pass_if_needed};

const HOVER: &str = "#22c55e";
const PRESSED: &str = "#0ea5e9";
const ROW: f32 = 30.0;
const GAP: f32 = 2.0;
const ROWS: usize = 80;
const HEIGHT: f32 = 400.0;
/// Longer than a wheel's scroll takes, so that a fade is still on its way when the scroll ends.
const FADE: u64 = 400;

/// A 300 by 400 list of rows 30 high, 2 apart.
fn list() -> ScrollVertical {
  let rows = (0..ROWS).map(|_| {
    Rect::new(280.0, ROW)
      .hovered(|style| style.background(HOVER))
      .active(|style| style.background(PRESSED))
      .transition(Transition::background_color().duration_ms(FADE).easing(Easing::Linear))
  });
  ScrollVertical::new(Column::new().spacing(GAP).with_children(rows)).size(300.0, HEIGHT)
}

struct Stage {
  app: App,
  tree: Tree,
  start: Instant,
  now: u64,
  /// The last frame drawn: what the window shows.
  screen: Option<RenderSnapshot>,
}

impl Stage {
  fn new() -> Self {
    let mut tree = Tree::new();
    tree.resize(300, HEIGHT as u32);
    tree.set_root(list());
    let mut stage = Self {
      app: App::new(),
      tree,
      start: Instant::now(),
      now: 0,
      screen: None,
    };
    stage.tree.request_redraw();
    stage.settle();
    stage
  }

  /// The frame 16 ms after the last, as a shell paces it: asked for by a redraw or by something on its way.
  /// Tells whether the tree asked for one.
  fn frame(&mut self) -> bool {
    self.now += 16;
    self
      .tree
      .set_animation_clock(Some(self.start + Duration::from_millis(self.now)));
    if self.tree.has_active_timeline() {
      self.tree.request_redraw();
    }
    if !self.tree.needs_redraw() {
      return false;
    }
    if let Some(frame) = render_pass_if_needed(&mut self.tree, &mut self.app) {
      self.screen = Some(frame);
    }
    true
  }

  /// Every frame the tree asks for.
  fn settle(&mut self) {
    for _ in 0..400 {
      if !self.frame() {
        return;
      }
    }
    panic!("the tree still asks for frames after six seconds");
  }

  /// The rows the window shows with a fill: where the top of each is, and its fill.
  fn filled_rows(&self) -> Vec<(f32, Color)> {
    let screen = self.screen.as_ref().expect("a frame was drawn");
    screen
      .rects
      .iter()
      .filter(|rect| rect.height == ROW && rect.color.a() > 0 && rect.y + ROW > 0.0 && rect.y < HEIGHT)
      .map(|rect| (rect.y, rect.color))
      .collect()
  }
}

/// Presses and releases the row under `pointer`, then turns the wheel by `delta` while the row is still on its
/// way back to its hovered fill, and draws every frame that follows.
fn wheel_during_a_fade(pointer: f32, delta: f32) -> Stage {
  let mut stage = Stage::new();
  stage.tree.mouse_move(150.0, pointer);
  stage.settle();
  stage.tree.mouse_down(150.0, pointer, MouseButton::Left);
  stage.settle();
  stage.tree.mouse_up(150.0, pointer, MouseButton::Left);
  stage.frame();
  stage.frame();
  let fading = stage.filled_rows();
  assert!(
    matches!(fading[..], [(_, fill)] if fill != Color::from_hex(HOVER) && fill != Color::from_hex(PRESSED)),
    "the row is on its way back to the hovered fill: {fading:?}"
  );

  stage.tree.wheel(150.0, pointer, 0.0, delta, ScrollPhase::Scroll);
  stage.settle();
  stage
}

#[test]
fn a_row_scrolled_from_under_the_pointer_during_its_fade_is_left_with_no_fill() {
  // The pointer is in the middle of the ninth row; five rows on, another row's middle is under it, and the
  // ninth is still in view, five rows up.
  let pointer = 8.0 * (ROW + GAP) + ROW / 2.0;
  let stage = wheel_during_a_fade(pointer, -5.0 * (ROW + GAP));

  let filled = stage.filled_rows();
  assert_eq!(
    filled,
    [(8.0 * (ROW + GAP), Color::from_hex(HOVER))],
    "the row now under the pointer is the one row with a fill, the hovered one"
  );
}

#[test]
fn a_row_scrolled_away_for_a_gap_leaves_no_row_with_a_fill() {
  // Half a row and half a gap on, the room between two rows is under the pointer; the row is still in view.
  let pointer = 8.0 * (ROW + GAP) + ROW / 2.0;
  let stage = wheel_during_a_fade(pointer, -(4.0 * (ROW + GAP) + ROW / 2.0 + GAP / 2.0));

  assert_eq!(stage.filled_rows(), [], "nothing is under the pointer");
}
