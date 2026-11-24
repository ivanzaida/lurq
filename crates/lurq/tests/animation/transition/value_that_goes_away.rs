//! A property the node has only in a state: a fill its hovered and pressed styles give it, and none at rest. No
//! fill is a value a transition runs from and to, as no shadow and no outline are: the fill fades in from its own
//! color with nothing of it when the pointer comes, and out to it when the pointer goes. A run on its way when the
//! fill goes turns back to nothing from where it was, and once the fill has gone nothing is left to run: the next
//! fill fades in from nothing of its own color, not from a color the node lost long before.

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

/// The hovered fill with `alpha` of it.
fn hovered(alpha: u8) -> Color {
  let hover = Color::from_hex(HOVER);
  Color::new(hover.r(), hover.g(), hover.b(), alpha)
}

fn near(color: Option<Color>, to: Color) -> bool {
  color.is_some_and(|color| {
    (color.r(), color.g(), color.b()) == (to.r(), to.g(), to.b())
      && (i16::from(color.a()) - i16::from(to.a())).abs() <= 2
  })
}

#[test]
fn a_fill_only_a_state_gives_fades_in_from_nothing_and_out_to_nothing() {
  let mut stage = Stage::new();

  stage.tree.mouse_move(ON.0, ON.1);
  stage.frame(16);
  let half = fill(&stage.frame(216));
  assert!(
    near(half, hovered(128)),
    "half of the way in, in its own color: {half:?}"
  );
  assert_eq!(fill(&stage.frame(416)), Some(Color::from_hex(HOVER)));
  assert!(!stage.tree.has_active_timeline());

  stage.tree.mouse_move(OFF.0, OFF.1);
  stage.frame(432);
  let half = fill(&stage.frame(632));
  assert!(near(half, hovered(128)), "half of the way out: {half:?}");
  assert_eq!(fill(&stage.frame(832)), None);
  assert!(!stage.tree.has_active_timeline(), "nothing left to run");
}

#[test]
fn a_run_on_its_way_turns_back_to_nothing_when_the_node_loses_the_value() {
  let mut stage = Stage::new();
  stage.tree.mouse_move(ON.0, ON.1);
  stage.frame(16);
  // The pointer leaves while the fill is still fading in.
  let fading = fill(&stage.frame(116)).expect("a quarter of the way in");
  stage.tree.mouse_move(OFF.0, OFF.1);
  let turned = fill(&stage.frame(132)).expect("where it was when the pointer left");
  assert!(
    turned.a() < 96,
    "not on to the hovered fill: {turned:?} after {fading:?}"
  );
  let back = fill(&stage.frame(232));
  assert!(
    back.is_none_or(|back| back.a() < turned.a()),
    "on its way back to nothing: {back:?} after {turned:?}"
  );
  for later in [532, 832, 1132] {
    assert_eq!(
      fill(&stage.frame(later)),
      None,
      "and does not go on to the hovered fill, {later} ms on"
    );
  }
  assert!(!stage.tree.has_active_timeline(), "nothing left to run");
}

#[test]
fn a_value_that_went_away_is_not_where_the_next_run_starts() {
  let mut stage = Stage::new();
  stage.tree.mouse_move(ON.0, ON.1);
  stage.frame(16);
  // Pressed until the fill is the pressed one, then dragged off the row and released there: the last fill the
  // row had is the pressed one, which fades out.
  stage.tree.mouse_down(ON.0, ON.1, MouseButton::Left);
  stage.frame(32);
  assert_eq!(fill(&stage.frame(1032)), Some(Color::from_hex(PRESSED)));
  stage.tree.mouse_move(OFF.0, OFF.1);
  stage.tree.mouse_up(OFF.0, OFF.1, MouseButton::Left);
  stage.frame(1048);
  assert_eq!(fill(&stage.frame(1500)), None);

  // Much later the pointer comes back: the hovered fill fades in from nothing of its own color.
  stage.tree.mouse_move(ON.0, ON.1);
  stage.frame(5000);
  let half = fill(&stage.frame(5200));
  assert!(near(half, hovered(128)), "not a fade from the pressed fill: {half:?}");
}

#[test]
fn a_line_only_a_state_gives_fades_in_and_out_in_its_shape() {
  use lurq::node::border::Border;
  const LINE: &str = "#1d4ed8";
  let mut tree = Tree::new();
  tree.resize(400, 300);
  tree.set_root(
    Rect::new(100.0, 40.0)
      .hovered(|style| style.border_bottom(Border::inside(2.0, LINE)))
      .transition(Transition::border_color().duration_ms(400).easing(Easing::Linear)),
  );
  let mut stage = Stage {
    app: App::new(),
    tree,
    start: Instant::now(),
  };
  stage.frame(0);
  // A line under the row is drawn as a fill of its own, as high as the line.
  let blue = Color::from_hex(LINE);
  let line = |frame: &RenderSnapshot| {
    frame
      .rects
      .iter()
      .find(|rect| {
        (rect.color.r(), rect.color.g(), rect.color.b()) == (blue.r(), blue.g(), blue.b()) && rect.color.a() > 0
      })
      .map(|rect| ((rect.y, rect.height), rect.color))
  };
  stage.tree.mouse_move(ON.0, ON.1);
  stage.frame(16);
  let (place, color) = line(&stage.frame(216)).expect("half of the way in");
  assert_eq!(place, (38.0, 2.0), "under it, 2 high");
  assert!((i16::from(color.a()) - 128).abs() <= 2, "{color:?}");
  stage.frame(416);
  stage.tree.mouse_move(OFF.0, OFF.1);
  stage.frame(432);
  let (place, color) = line(&stage.frame(632)).expect("half of the way out, in the shape it had");
  assert_eq!(place, (38.0, 2.0));
  assert!((i16::from(color.a()) - 128).abs() <= 2, "{color:?}");
  assert_eq!(line(&stage.frame(832)), None);
  assert!(!stage.tree.has_active_timeline());
}
