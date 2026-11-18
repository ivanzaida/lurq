//! An app's own frames: `ctx.on_frame` runs a callback once in each frame the window draws, told the frame's time
//! and the time since the one before, until it returns `false`, is cancelled, or its component unmounts. While one
//! runs, each pass is laid out and painted (never the kept list drawn again), with what the callback set in it. On
//! the tree's clock (`Tree::set_animation_clock`): no test here reads the machine's.

use std::{
  sync::{Arc, Mutex},
  time::{Duration, Instant},
};

use lurq::{
  app::{
    App, PassReport, Tree,
    component::Component,
    ctx::{Ctx, Frame, FrameCallback},
    render_engine::RenderEngine,
    window::WindowHandle as AppWindow,
  },
  components::{Rect, Row},
  core::Signal,
  layout::render_list::RenderList,
  node::{Element, color::Color},
};
use raw_window_handle::{DisplayHandle, WindowHandle};

use crate::support::TestSurface;

const BAR: &str = "#22aa66";
/// A display that refreshes at 60 Hz.
const FRAME: u64 = 16;

/// How wide the bar is drawn once the callback ran `frames` times: each frame sets it.
fn bar_width(frames: usize) -> f32 {
  10.0 + 10.0 * frames as f32
}

/// What a test tells its component and reads back from it.
#[derive(Default)]
struct Shared {
  /// Every frame the callback ran for.
  frames: Mutex<Vec<Frame>>,
  /// The callback returns `false` in the frame that makes this many.
  stop_after: Option<usize>,
  /// The callback's handle, once the component is created.
  handle: Mutex<Option<FrameCallback>>,
  /// Whether the parent shows the component, where there is a parent.
  shown: Option<Signal<bool>>,
}

#[derive(Clone, lurq::DevtoolsInspectable)]
struct Script(#[devtools_ignore] Arc<Shared>);

impl PartialEq for Script {
  fn eq(&self, other: &Self) -> bool {
    Arc::ptr_eq(&self.0, &other.0)
  }
}

impl Script {
  fn new(shared: Shared) -> Self {
    Self(Arc::new(shared))
  }

  fn frames(&self) -> Vec<Frame> {
    self.0.frames.lock().unwrap().clone()
  }

  fn handle(&self) -> FrameCallback {
    self.0.handle.lock().unwrap().clone().expect("the component is created")
  }
}

/// A bar that grows by ten with every frame its callback runs for.
struct Bar {
  width: Signal<f32>,
}

impl Component for Bar {
  type Props = Script;

  fn create(ctx: &mut Ctx) -> Self {
    let script = ctx.props::<Script>().0.clone();
    let width = ctx.signal(bar_width(0));
    let frames = ctx.on_frame({
      let (script, width) = (script.clone(), width.clone());
      move |frame| {
        let mut frames = script.frames.lock().unwrap();
        frames.push(frame);
        width.set(bar_width(frames.len()));
        script.stop_after.is_none_or(|stop| frames.len() < stop)
      }
    });
    *script.handle.lock().unwrap() = Some(frames);
    Self { width }
  }

  fn render(&self, _ctx: &mut Ctx) -> impl Into<Element> {
    Row::new().child(Rect::new(self.width.get(), 20.0).background(BAR))
  }
}

/// A parent that shows a [`Bar`] while its signal says so.
struct Holder;

impl Component for Holder {
  type Props = Script;

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let script = ctx.props::<Script>().clone();
    let shown = script.0.shown.as_ref().expect("a holder has a signal").get();
    let mut row = Row::new().child(Rect::new(4.0, 4.0).background("#000000"));
    if shown {
      row = row.child(ctx.mount::<Bar>(script));
    }
    row
  }
}

/// What the engine drew last: the bar's width, by its fill.
struct Drawn(Arc<Mutex<Option<f32>>>);

impl RenderEngine for Drawn {
  fn resize(&mut self, _width: u32, _height: u32) {}

  fn render(&mut self, list: &RenderList, _window: WindowHandle<'_>, _display: DisplayHandle<'_>) -> bool {
    let bar = Color::from_hex(BAR);
    *self.0.lock().unwrap() = list.rects.iter().find(|rect| rect.color == bar).map(|rect| rect.width);
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
  /// A window on a clock of its own, with nothing mounted yet.
  fn new() -> Self {
    let mut tree = Tree::new();
    tree.resize(800, 400);
    let drawn = Arc::new(Mutex::new(None));
    let shown = drawn.clone();
    tree.set_render_engine_factory(move || Box::new(Drawn(shown.clone())) as Box<dyn RenderEngine>);
    let start = Instant::now();
    tree.set_animation_clock(Some(start));
    Self {
      app: App::new(),
      tree,
      start,
      drawn,
    }
  }

  fn mounting<C: Component>(props: C::Props) -> Self {
    let mut stage = Self::new();
    stage.tree.mount_root::<C>(&mut stage.app, props);
    stage
  }

  fn at(&self, millis: u64) -> Instant {
    self.start + Duration::from_millis(millis)
  }

  /// The pass of the frame `millis` after the start, as the shell runs one the timeline asked for or a video's:
  /// nothing asks the tree to draw.
  fn frame(&mut self, millis: u64) -> PassReport {
    self.tree.set_animation_clock(Some(self.at(millis)));
    self.tree.pass(&mut self.app, &TestSurface)
  }

  fn bar(&self) -> Option<f32> {
    *self.drawn.lock().unwrap()
  }
}

fn frame(stage: &Stage, at: u64, delta: u64) -> Frame {
  Frame {
    time: stage.at(at),
    delta: Duration::from_millis(delta),
  }
}

#[test]
fn a_callback_runs_once_in_each_frame_with_its_time_and_the_time_since_the_one_before() {
  let script = Script::new(Shared::default());
  let mut stage = Stage::mounting::<Bar>(script.clone());
  assert!(script.frames().is_empty(), "no frame was drawn yet");

  for at in [0, 16, 32, 50, 150] {
    let report = stage.frame(at);
    assert!(report.rendered, "the frame at {at} ms is drawn");
    assert!(
      !report.used_cached_render_list,
      "the frame at {at} ms is laid out and painted, not the kept list drawn again"
    );
  }

  assert_eq!(
    script.frames(),
    [
      frame(&stage, 0, 0),
      frame(&stage, 16, 16),
      frame(&stage, 32, 16),
      frame(&stage, 50, 18),
      // A frame the machine was late for has its real delta.
      frame(&stage, 150, 100),
    ]
  );
  assert_eq!(Frame::delta_secs(&frame(&stage, 50, 18)), 0.018);
}

#[test]
fn what_a_callback_sets_is_shown_in_the_frame_it_ran_for() {
  let script = Script::new(Shared::default());
  let mut stage = Stage::mounting::<Bar>(script.clone());

  for (index, at) in [0, 16, 32, 48].into_iter().enumerate() {
    stage.frame(at);
    assert_eq!(
      stage.bar(),
      Some(bar_width(index + 1)),
      "the frame at {at} ms shows what its callback set"
    );
  }
}

#[test]
fn while_a_callback_runs_the_window_wants_its_next_frame_at_once_and_asks_for_no_redraw() {
  let script = Script::new(Shared::default());
  let mut stage = Stage::mounting::<Bar>(script);
  stage.frame(0);

  assert!(stage.tree.has_frame_callbacks());
  assert!(
    stage.tree.has_active_timeline(),
    "the callback is a timeline that changes each frame"
  );
  // The shell paces the next frame from this time, one a refresh, as it does a transition's.
  assert_eq!(stage.tree.next_timeline_change(), Some(stage.at(0)));
  assert!(
    !stage.tree.needs_redraw(),
    "the frame is paced by the shell: a redraw asked for at once would draw faster than the display"
  );
}

#[test]
fn a_callback_that_returns_false_runs_no_more_and_no_frame_is_drawn_for_it() {
  let script = Script::new(Shared {
    stop_after: Some(3),
    ..Shared::default()
  });
  let mut stage = Stage::mounting::<Bar>(script.clone());

  for at in [0, 16, 32] {
    assert!(stage.frame(at).rendered);
  }
  assert_eq!(script.frames().len(), 3);
  assert_eq!(stage.bar(), Some(bar_width(3)), "its last frame shows what it set");
  assert!(!script.handle().is_active());
  assert!(!stage.tree.has_frame_callbacks());
  assert!(!stage.tree.has_active_timeline());
  assert_eq!(
    stage.tree.next_timeline_change(),
    None,
    "the shell is asked for no frame"
  );

  for at in [48, 64] {
    assert!(!stage.frame(at).required, "nothing is left to draw at {at} ms");
  }
  assert_eq!(script.frames().len(), 3);
}

#[test]
fn a_cancelled_callback_runs_no_more_and_starts_again_as_on_a_first_frame() {
  let script = Script::new(Shared::default());
  let mut stage = Stage::mounting::<Bar>(script.clone());
  stage.frame(0);
  stage.frame(16);

  script.handle().cancel();
  assert!(!stage.tree.has_frame_callbacks());
  assert_eq!(stage.tree.next_timeline_change(), None);
  assert!(!stage.frame(32).required);
  assert_eq!(script.frames().len(), 2);

  // Started again much later: its first frame does not jump by the time it stood still.
  script.handle().start();
  assert!(stage.tree.has_frame_callbacks());
  assert!(
    stage.tree.next_timeline_change().is_some(),
    "the shell is asked for its frame"
  );
  assert!(stage.frame(5000).rendered);
  stage.frame(5016);
  assert_eq!(script.frames()[2..], [frame(&stage, 5000, 0), frame(&stage, 5016, 16)]);
  assert_eq!(stage.bar(), Some(bar_width(4)));
}

#[test]
fn a_callback_goes_with_its_component() {
  let shown = Signal::new(true);
  let script = Script::new(Shared {
    shown: Some(shown.clone()),
    ..Shared::default()
  });
  let mut stage = Stage::mounting::<Holder>(script.clone());
  stage.frame(0);
  stage.frame(16);
  assert_eq!(script.frames().len(), 2);

  shown.set(false);
  assert!(stage.frame(32).rendered, "the frame without the bar");
  assert_eq!(stage.bar(), None);
  let ran = script.frames().len();
  assert!(!stage.tree.has_frame_callbacks(), "the callback went with the bar");
  assert_eq!(stage.tree.next_timeline_change(), None);

  for at in [48, 64] {
    assert!(!stage.frame(at).required);
  }
  assert_eq!(
    script.frames().len(),
    ran,
    "an unmounted component's callback runs no more"
  );
  // Its handle starts nothing: the component that had it is gone.
  script.handle().start();
  assert!(!stage.tree.has_frame_callbacks());
}

/// The frames callbacks ran for, each with the callback's name, and the window's handle as a component is given
/// it (`ctx.window()`), for a callback from outside any component.
#[derive(Clone, Default, lurq::DevtoolsInspectable)]
struct Names(
  #[devtools_ignore] Arc<Mutex<Vec<(&'static str, Frame)>>>,
  #[devtools_ignore] Arc<Mutex<Option<AppWindow>>>,
);

impl PartialEq for Names {
  fn eq(&self, other: &Self) -> bool {
    Arc::ptr_eq(&self.0, &other.0)
  }
}

impl Names {
  fn window(&self) -> AppWindow {
    self.1.lock().unwrap().clone().expect("the component is created")
  }
}

/// A square, with no callback of its own.
struct Plain;

impl Component for Plain {
  type Props = Names;

  fn create(ctx: &mut Ctx) -> Self {
    *ctx.props::<Names>().1.lock().unwrap() = Some(ctx.window());
    Self
  }

  fn render(&self, _ctx: &mut Ctx) -> impl Into<Element> {
    Row::new().child(Rect::new(10.0, 10.0).background(BAR))
  }
}

/// Two callbacks of its own, each logging its name with the frame.
struct Pair;

impl Component for Pair {
  type Props = Names;

  fn create(ctx: &mut Ctx) -> Self {
    *ctx.props::<Names>().1.lock().unwrap() = Some(ctx.window());
    let names = ctx.props::<Names>().0.clone();
    for (name, frames) in [("first", 2), ("second", 4)] {
      let names = names.clone();
      let mut left = frames;
      ctx.on_frame(move |frame| {
        names.lock().unwrap().push((name, frame));
        left -= 1;
        left > 0
      });
    }
    Self
  }

  fn render(&self, _ctx: &mut Ctx) -> impl Into<Element> {
    Row::new().child(Rect::new(10.0, 10.0).background(BAR))
  }
}

#[test]
fn several_callbacks_each_run_in_every_frame_until_their_own_end() {
  let names = Names::default();
  let mut stage = Stage::mounting::<Pair>(names.clone());
  // The window's own callback, from outside any component: it runs before the components'.
  let window = {
    let log = names.0.clone();
    names.window().on_frame(move |frame| {
      log.lock().unwrap().push(("window", frame));
      true
    })
  };

  for at in [0, 16, 32, 48] {
    stage.frame(at);
  }
  window.cancel();
  assert!(!stage.frame(64).required, "every callback has stopped");

  let ran: Vec<(&str, u64, u64)> = names
    .0
    .lock()
    .unwrap()
    .iter()
    .map(|(name, frame)| {
      (
        *name,
        frame.time.duration_since(stage.start).as_millis() as u64,
        frame.delta.as_millis() as u64,
      )
    })
    .collect();
  assert_eq!(
    ran,
    [
      ("window", 0, 0),
      ("first", 0, 0),
      ("second", 0, 0),
      ("window", 16, 16),
      ("first", 16, 16),
      ("second", 16, 16),
      ("window", 32, 16),
      ("second", 32, 16),
      ("window", 48, 16),
      ("second", 48, 16),
    ]
  );
}

#[test]
fn a_windows_callback_is_dropped_when_it_stops_and_goes_back_when_started() {
  let names = Names::default();
  let mut stage = Stage::mounting::<Plain>(names.clone());
  assert!(stage.frame(0).rendered);
  assert!(!stage.tree.has_frame_callbacks());

  let ran = Arc::new(Mutex::new(Vec::new()));
  let callback = {
    let ran = ran.clone();
    names.window().on_frame(move |frame: Frame| {
      let mut ran = ran.lock().unwrap();
      ran.push(frame);
      ran.len() % 2 != 0
    })
  };
  assert!(stage.tree.has_frame_callbacks());
  assert!(
    stage.frame(16).rendered,
    "a window's callback has the window draw its frames"
  );
  stage.frame(32);
  assert!(!callback.is_active(), "it returned false in its second frame");
  assert!(!stage.frame(48).required);

  callback.start();
  assert!(stage.frame(64).rendered);
  assert_eq!(
    *ran.lock().unwrap(),
    [frame(&stage, 16, 0), frame(&stage, 32, 16), frame(&stage, 64, 0)]
  );
}

#[test]
fn with_no_callback_no_frame_is_asked_for() {
  let mut stage = Stage::new();
  stage
    .tree
    .set_root(Row::new().child(Rect::new(10.0, 10.0).background(BAR)));
  stage.tree.request_redraw();
  assert!(stage.frame(0).rendered);

  assert!(!stage.tree.has_frame_callbacks());
  assert!(!stage.tree.has_active_timeline());
  assert_eq!(stage.tree.next_timeline_change(), None);
  assert!(!stage.tree.needs_redraw());
  for at in [16, 32] {
    let report = stage.frame(at);
    assert!(!report.required && !report.rendered, "nothing is drawn at {at} ms");
  }
}

/// A component that asks for its frames in its render, at one place, while its props say so.
struct InRender {
  count: Signal<usize>,
}

#[derive(Clone, lurq::DevtoolsInspectable)]
struct RenderScript {
  #[devtools_ignore]
  frames: Names,
  #[devtools_ignore]
  wanted: Signal<bool>,
}

impl PartialEq for RenderScript {
  fn eq(&self, other: &Self) -> bool {
    self.frames == other.frames
  }
}

impl Component for InRender {
  type Props = RenderScript;

  fn create(ctx: &mut Ctx) -> Self {
    Self { count: ctx.signal(0) }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let script = ctx.props::<RenderScript>().clone();
    let count = self.count.get();
    if script.wanted.get() {
      let (frames, counted) = (script.frames.0.clone(), self.count.clone());
      // Each frame renders the component again, which makes this call again: it is the same callback.
      ctx.on_frame(move |frame| {
        frames.lock().unwrap().push(("render", frame));
        counted.set(count + 1);
        true
      });
    }
    Row::new().child(Rect::new(bar_width(count), 20.0).background(BAR))
  }
}

#[test]
fn a_callback_asked_for_in_a_render_is_one_callback_however_often_it_renders() {
  let script = RenderScript {
    frames: Names::default(),
    wanted: Signal::new(true),
  };
  let mut stage = Stage::mounting::<InRender>(script.clone());

  for (index, at) in [0, 16, 32].into_iter().enumerate() {
    stage.frame(at);
    assert_eq!(
      stage.bar(),
      Some(bar_width(index + 1)),
      "the closure of the last render ran"
    );
  }
  let deltas: Vec<u64> = script
    .frames
    .0
    .lock()
    .unwrap()
    .iter()
    .map(|(_, frame)| frame.delta.as_millis() as u64)
    .collect();
  assert_eq!(
    deltas,
    [0, 16, 16],
    "once a frame, keeping its time through the renders"
  );

  // A render that no longer asks drops it.
  script.wanted.set(false);
  assert!(stage.frame(48).rendered);
  assert!(!stage.tree.has_frame_callbacks());
  assert!(!stage.frame(64).required);
  assert_eq!(script.frames.0.lock().unwrap().len(), 3);
}

/// A video beside a bar whose callback runs: the shell draws such a window at the video's pace, each pass with no
/// redraw asked for, and those passes once drew the list the window kept (`while_a_video_plays`).
#[cfg(feature = "raster")]
mod beside_a_video {
  use lurq::{components::Video, images::ImageData};

  use super::*;

  struct BarAndVideo;

  impl Component for BarAndVideo {
    type Props = Script;

    fn create(_ctx: &mut Ctx) -> Self {
      Self
    }

    fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
      let script = ctx.props::<Script>().clone();
      let frames = ImageData::streaming_rgba(vec![0; 16 * 9 * 4], 16, 9);
      assert!(frames.requires_continuous_redraw(), "a video's frames keep coming");
      Row::new()
        .child(ctx.mount::<Bar>(script))
        .child(Video::new(frames).size(320.0, 180.0))
    }
  }

  #[test]
  fn a_callback_runs_in_each_frame_of_a_playing_video_and_then_the_kept_list_is_drawn_again() {
    let script = Script::new(Shared {
      stop_after: Some(6),
      ..Shared::default()
    });
    let mut stage = Stage::mounting::<BarAndVideo>(script.clone());

    for index in 0..6 {
      let at = index as u64 * FRAME;
      let report = stage.frame(at);
      assert!(report.rendered, "the frame at {at} ms is drawn");
      assert!(
        !report.used_cached_render_list,
        "the frame at {at} ms is laid out, with what the callback set, not the kept list drawn again"
      );
      assert_eq!(stage.bar(), Some(bar_width(index + 1)), "the frame at {at} ms");
    }
    let deltas: Vec<u64> = script
      .frames()
      .iter()
      .map(|frame| frame.delta.as_millis() as u64)
      .collect();
    assert_eq!(deltas, [0, FRAME, FRAME, FRAME, FRAME, FRAME]);

    // The callback stopped: the video's frames come from the kept list again, with no layout.
    stage.frame(6 * FRAME);
    for index in 7..10 {
      let report = stage.frame(index * FRAME);
      assert!(
        report.rendered && report.used_cached_render_list,
        "a video's frame at {} ms, once the callback stopped",
        index * FRAME
      );
    }
    assert_eq!(script.frames().len(), 6);
  }
}
