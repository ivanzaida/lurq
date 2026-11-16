//! A property the node has only in a state: a fill its hovered and pressed styles give it, and none at rest.
//! A transition runs from one value to another, not from none and not to none, so the fill is there at once when
//! the pointer comes and gone at once when it leaves. A run on its way when the value goes away ends with it:
//! it does not go on to where it was heading and leave that drawn on a node that no longer has it. And the value
//! is forgotten, so the next one the node gets is there at once, as its first was, and does not start from a
//! color the node lost long before.

use std::time::{Duration, Instant};

use lurq::{
  animation::{Easing, Transition},
  app::{App, Tree, events::MouseButton},
  components::Rect,
  node::color::Color,
};

use crate::support::{RenderSnapshot, render_pass_with_app};

const HOVER: &str = "#22c55e";
const PRESSED: &str = "#0ea5e9";
/// The middle of the row.
const ON: (f32, f32) = (50.0, 20.0);
/// Beside the row, in the window.
const OFF: (f32, f32) = (300.0, 200.0);

struct Stage {
  app: App,
  tree: Tree,
  start: Instant,
}

impl Stage {
  /// A 400 by 300 window with a row of 100 by 40 at its corner: no fill at rest, one under the pointer and
  /// another while pressed, and 400 ms between two fills.
  fn new() -> Self {
    let mut tree = Tree::new();
    tree.resize(400, 300);
    tree.set_root(
      Rect::new(100.0, 40.0)
        .hovered(|style| style.background(HOVER))
        .active(|style| style.background(PRESSED))
        .transition(Transition::background_color().duration_ms(400).easing(Easing::Linear)),
    );
    let mut stage = Self {
      app: App::new(),
      tree,
      start: Instant::now(),
    };
    stage.frame(0);
    stage
  }

  /// The frame `millis` after the start.
  fn frame(&mut self, millis: u64) -> RenderSnapshot {
    self
      .tree
      .set_animation_clock(Some(self.start + Duration::from_millis(millis)));
    render_pass_with_app(&mut self.tree, &mut self.app)
  }
}

/// What the row is filled with in the frame, if it is filled.
fn fill(frame: &RenderSnapshot) -> Option<Color> {
  frame
    .rects
    .iter()
    .find(|rect| (rect.x, rect.y, rect.width, rect.height) == (0.0, 0.0, 100.0, 40.0) && rect.color.a() > 0)
    .map(|rect| rect.color)
}

#[test]
fn a_fill_only_a_state_gives_is_there_at_once_and_gone_at_once() {
  let mut stage = Stage::new();

  stage.tree.mouse_move(ON.0, ON.1);
  assert_eq!(fill(&stage.frame(16)), Some(Color::from_hex(HOVER)));
  assert!(!stage.tree.has_active_timeline(), "nothing to fade from");

  stage.tree.mouse_move(OFF.0, OFF.1);
  assert_eq!(fill(&stage.frame(32)), None);
  assert!(!stage.tree.has_active_timeline(), "nothing to fade to");
}

#[test]
fn a_run_on_its_way_ends_when_the_node_loses_the_value() {
  let mut stage = Stage::new();
  stage.tree.mouse_move(ON.0, ON.1);
  stage.frame(16);
  // Pressed, the fill fades to the pressed one; released at once, it fades back.
  stage.tree.mouse_down(ON.0, ON.1, MouseButton::Left);
  stage.frame(32);
  stage.frame(132);
  stage.tree.mouse_up(ON.0, ON.1, MouseButton::Left);
  let fading = fill(&stage.frame(148)).expect("still filled under the pointer");
  assert!(
    fading != Color::from_hex(HOVER) && fading != Color::from_hex(PRESSED),
    "on its way back to the hovered fill: {fading:?}"
  );
  assert!(stage.tree.has_active_timeline());

  // The pointer leaves while the fill is still on its way.
  stage.tree.mouse_move(OFF.0, OFF.1);

  assert_eq!(fill(&stage.frame(164)), None, "no fill without the pointer");
  assert!(!stage.tree.has_active_timeline(), "and nothing left to run");
  for later in [264, 564, 1064] {
    assert_eq!(
      fill(&stage.frame(later)),
      None,
      "the run did not go on to the hovered fill, {later} ms on"
    );
  }
}

#[test]
fn a_value_that_went_away_is_not_where_the_next_run_starts() {
  let mut stage = Stage::new();
  stage.tree.mouse_move(ON.0, ON.1);
  stage.frame(16);
  // Pressed until the fill is the pressed one, then dragged off the row and released there: the last fill the
  // row had is the pressed one.
  stage.tree.mouse_down(ON.0, ON.1, MouseButton::Left);
  stage.frame(32);
  assert_eq!(fill(&stage.frame(1032)), Some(Color::from_hex(PRESSED)));
  stage.tree.mouse_move(OFF.0, OFF.1);
  stage.tree.mouse_up(OFF.0, OFF.1, MouseButton::Left);
  assert_eq!(fill(&stage.frame(1048)), None);

  // Much later the pointer comes back.
  stage.tree.mouse_move(ON.0, ON.1);

  assert_eq!(
    fill(&stage.frame(5000)),
    Some(Color::from_hex(HOVER)),
    "the hovered fill at once, not a fade from the pressed one"
  );
  assert!(!stage.tree.has_active_timeline());
}
