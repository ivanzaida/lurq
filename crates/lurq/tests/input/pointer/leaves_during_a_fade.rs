//! The pointer leaves a row whose fill is still on its way: pressed and released, the row fades from its pressed
//! fill back to its hovered one, and the pointer goes before that ends. A row has no fill at rest, so its fill fades
//! out from where it was, and once that is over it is drawn with none, wherever the pointer went: beside the rows,
//! onto another row, or out of the window.

use std::time::{Duration, Instant};

use lurq::{
  animation::{Easing, Transition},
  app::{App, Tree, events::MouseButton},
  components::{Column, Rect},
  node::color::Color,
};

use crate::support::{RenderSnapshot, render_pass_if_needed};

const HOVER: &str = "#22c55e";
const PRESSED: &str = "#0ea5e9";
const ROW: f32 = 30.0;
const GAP: f32 = 10.0;
/// How long a row takes from one fill to another.
const FADE: u64 = 200;

struct Stage {
  app: App,
  tree: Tree,
  start: Instant,
  now: u64,
  /// The last frame drawn: what the window shows.
  screen: Option<RenderSnapshot>,
}

impl Stage {
  /// A 300 by 300 window with three rows of 200 by 30, 10 apart: no fill at rest, one under the pointer and
  /// another while pressed.
  fn new() -> Self {
    let rows = (0..3).map(|_| {
      Rect::new(200.0, ROW)
        .hovered(|style| style.background(HOVER))
        .active(|style| style.background(PRESSED))
        .transition(Transition::background_color().duration_ms(FADE).easing(Easing::Linear))
    });
    let mut tree = Tree::new();
    tree.resize(300, 300);
    tree.set_root(Column::new().spacing(GAP).with_children(rows));
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
    for _ in 0..200 {
      if !self.frame() {
        return;
      }
    }
    panic!("the tree still asks for frames after three seconds");
  }

  /// The middle of the row `index`.
  fn row(index: usize) -> (f32, f32) {
    (100.0, index as f32 * (ROW + GAP) + ROW / 2.0)
  }

  /// Presses and releases the row `index` and draws two frames: its fill is on its way back from the pressed
  /// one to the hovered one.
  fn press_and_release(&mut self, index: usize) {
    let (x, y) = Self::row(index);
    self.tree.mouse_move(x, y);
    self.settle();
    self.tree.mouse_down(x, y, MouseButton::Left);
    self.settle();
    assert_eq!(self.fills()[index], Some(Color::from_hex(PRESSED)));
    self.tree.mouse_up(x, y, MouseButton::Left);
    self.frame();
    self.frame();
    let fading = self.fills()[index].expect("filled under the pointer");
    assert!(
      fading != Color::from_hex(HOVER) && fading != Color::from_hex(PRESSED),
      "on its way back to the hovered fill: {fading:?}"
    );
  }

  /// What each row is filled with on the screen, if it is filled.
  fn fills(&self) -> [Option<Color>; 3] {
    let screen = self.screen.as_ref().expect("a frame was drawn");
    std::array::from_fn(|index| {
      let top = index as f32 * (ROW + GAP);
      screen
        .rects
        .iter()
        .find(|rect| rect.y == top && rect.height == ROW && rect.color.a() > 0)
        .map(|rect| rect.color)
    })
  }
}

#[test]
fn a_row_the_pointer_left_for_the_gap_has_no_fill() {
  let mut stage = Stage::new();
  stage.press_and_release(1);

  stage.tree.mouse_move(100.0, ROW + GAP / 2.0);
  stage.frame();

  stage.frame();
  assert!(
    stage.fills()[1].is_none_or(|fill| fill.a() < 255),
    "fading out from where it was"
  );
  stage.settle();
  assert_eq!(
    stage.fills(),
    [None, None, None],
    "and none once nothing is left to draw"
  );
}

#[test]
fn a_row_the_pointer_left_for_another_row_has_no_fill() {
  let mut stage = Stage::new();
  stage.press_and_release(1);

  let (x, y) = Stage::row(2);
  stage.tree.mouse_move(x, y);
  stage.settle();

  assert_eq!(stage.fills(), [None, None, Some(Color::from_hex(HOVER))]);
}

#[test]
fn a_row_the_pointer_left_the_window_from_has_no_fill() {
  let mut stage = Stage::new();
  stage.press_and_release(1);

  stage.tree.mouse_leave_window();
  stage.frame();

  stage.frame();
  assert!(
    stage.fills()[1].is_none_or(|fill| fill.a() < 255),
    "fading out from where it was"
  );
  stage.settle();
  assert_eq!(
    stage.fills(),
    [None, None, None],
    "and none once nothing is left to draw"
  );

  // It comes back in beside the rows: nothing is under it.
  stage.tree.mouse_move(250.0, 200.0);
  stage.settle();
  assert_eq!(stage.fills(), [None, None, None]);
}

#[test]
fn rows_the_pointer_crossed_fast_have_no_fill() {
  let mut stage = Stage::new();
  // Every row was pressed until its fill was the pressed one, and the press dragged off it and let go beside
  // the rows: the last fill each had is the pressed one.
  for index in 0..3 {
    let (x, y) = Stage::row(index);
    stage.tree.mouse_move(x, y);
    stage.tree.mouse_down(x, y, MouseButton::Left);
    stage.settle();
    stage.tree.mouse_move(250.0, 200.0);
    stage.tree.mouse_up(250.0, 200.0, MouseButton::Left);
    stage.settle();
  }
  assert_eq!(stage.fills(), [None, None, None]);

  // Down across the three rows, a frame on each, and on to the room below them.
  for index in 0..3 {
    let (x, y) = Stage::row(index);
    stage.tree.mouse_move(x, y);
    stage.frame();
    stage.frame();
    let hover = Color::from_hex(HOVER);
    assert!(
      stage.fills()[index].is_some_and(|fill| (fill.r(), fill.g(), fill.b()) == (hover.r(), hover.g(), hover.b())),
      "the hovered fill fading in, not a fade from the pressed one: {:?}",
      stage.fills()[index]
    );
  }
  stage.tree.mouse_move(100.0, 250.0);
  stage.frame();
  stage.settle();
  assert_eq!(stage.fills(), [None, None, None], "and once nothing is left to draw");
}
