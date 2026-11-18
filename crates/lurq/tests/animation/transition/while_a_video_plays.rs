//! A transition runs while a video plays. The shell then draws the window at the video's pace: it calls a pass for
//! each frame of the display and asks for no frame of the timeline meanwhile, so each pass comes with no redraw
//! asked for. Each such pass steps the transition all the same, as a pass the timeline asked for does: a sidebar
//! that collapses beside a playing video moves at the display's pace. Such passes once drew the list the window
//! kept, where the transition was when something else last laid the window out, and the sidebar moved only when
//! the player's own clock rendered it again, four times a second. Once the transition is over, the video's frames
//! are drawn from the kept list again, with no layout.

use std::{
  sync::{Arc, Mutex},
  time::{Duration, Instant},
};

use lurq::{
  animation::{Easing, Transition},
  app::{App, PassReport, Tree, render_engine::RenderEngine},
  components::{Rect, Row, Video},
  images::ImageData,
  layout::render_list::RenderList,
  node::color::Color,
};
use raw_window_handle::{DisplayHandle, WindowHandle};

use crate::support::TestSurface;

const SIDEBAR: &str = "#336699";
const WIDE: f32 = 240.0;
const NARROW: f32 = 52.0;
/// How long the sidebar takes to collapse.
const TAKES: u64 = 300;
/// A display that refreshes at 60 Hz.
const FRAME: u64 = 16;

/// What the engine drew last: the sidebar's width, by its fill.
struct Drawn(Arc<Mutex<Option<f32>>>);

impl RenderEngine for Drawn {
  fn resize(&mut self, _width: u32, _height: u32) {}

  fn render(&mut self, list: &RenderList, _window: WindowHandle<'_>, _display: DisplayHandle<'_>) -> bool {
    let sidebar = Color::from_hex(SIDEBAR);
    *self.0.lock().unwrap() = list
      .rects
      .iter()
      .find(|rect| rect.color == sidebar)
      .map(|rect| rect.width);
    true
  }
}

struct Stage {
  app: App,
  tree: Tree,
  start: Instant,
  drawn: Arc<Mutex<Option<f32>>>,
}

impl Stage {
  /// A window with a sidebar that collapses under the pointer and a video beside it whose frames keep coming.
  fn new() -> Self {
    let mut tree = Tree::new();
    tree.resize(800, 400);
    let drawn = Arc::new(Mutex::new(None));
    let shown = drawn.clone();
    tree.set_render_engine_factory(move || Box::new(Drawn(shown.clone())) as Box<dyn RenderEngine>);
    let frames = ImageData::streaming_rgba(vec![0; 16 * 9 * 4], 16, 9);
    assert!(frames.requires_continuous_redraw(), "a video's frames keep coming");
    tree.set_root(
      Row::new()
        .child(
          Rect::new(WIDE, 400.0)
            .background(SIDEBAR)
            .hovered(|style| style.width(NARROW))
            .transition(Transition::width().duration_ms(TAKES).easing(Easing::Linear)),
        )
        .child(Video::new(frames).size(320.0, 180.0)),
    );
    let mut stage = Self {
      app: App::new(),
      tree,
      start: Instant::now(),
      drawn,
    };
    stage.tree.set_animation_clock(Some(stage.start));
    stage.tree.request_redraw();
    assert!(stage.tree.pass(&mut stage.app, &TestSurface).rendered);
    stage
  }

  /// The pass the shell runs for the video's frame `millis` after the start: no redraw asked for.
  fn video_frame(&mut self, millis: u64) -> PassReport {
    self
      .tree
      .set_animation_clock(Some(self.start + Duration::from_millis(millis)));
    self.tree.pass(&mut self.app, &TestSurface)
  }

  fn width(&self) -> f32 {
    self.drawn.lock().unwrap().expect("the sidebar is drawn")
  }
}

#[test]
fn a_sidebar_collapses_at_the_displays_pace_beside_a_playing_video() {
  let mut stage = Stage::new();
  // The video plays a while: its frames are drawn from the kept list.
  for at in (FRAME..=4 * FRAME).step_by(FRAME as usize) {
    let report = stage.video_frame(at);
    assert!(
      report.rendered && report.used_cached_render_list,
      "a video's frame at {at} ms"
    );
  }
  assert_eq!(stage.width(), WIDE);

  // The pointer collapses the sidebar: the pass after it starts the transition.
  stage.tree.mouse_move(20.0, 200.0);
  let began = 5 * FRAME;
  assert!(stage.video_frame(began).rendered);
  assert!(stage.tree.has_active_timeline(), "the sidebar collapses");

  let mut widths = vec![stage.width()];
  let mut at = began;
  while at < began + TAKES {
    at += FRAME;
    let report = stage.video_frame(at);
    assert!(report.rendered, "the frame at {at} ms is drawn");
    assert!(
      !report.used_cached_render_list,
      "the frame at {at} ms is laid out, not the kept list drawn again"
    );
    widths.push(stage.width());
  }
  // Each frame drew the sidebar narrower than the one before, where the clock had it, down to its rail.
  for (frame, pair) in widths.windows(2).enumerate() {
    assert!(
      pair[1] < pair[0],
      "frame {frame} after the start moved the sidebar: {widths:?}"
    );
  }
  for (frame, width) in widths.iter().enumerate() {
    let share = ((frame as u64 * FRAME) as f32 / TAKES as f32).min(1.0);
    let expected = WIDE + (NARROW - WIDE) * share;
    assert!(
      (width - expected).abs() < 1.0,
      "frame {frame}: {width}, where the clock has {expected}: {widths:?}"
    );
  }
  assert_eq!(*widths.last().unwrap(), NARROW);
  assert!(!stage.tree.has_active_timeline(), "the collapse is over");

  // The video plays on: its frames are drawn from the kept list again, with no layout.
  for frame in 1..=4 {
    let report = stage.video_frame(at + frame * FRAME);
    assert!(
      report.rendered && report.used_cached_render_list,
      "a video's frame {frame} after the collapse"
    );
  }
  assert_eq!(stage.width(), NARROW);
}
