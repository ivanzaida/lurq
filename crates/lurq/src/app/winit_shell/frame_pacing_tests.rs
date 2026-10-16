//! Paced animation frames, run through a model of the winit loop: each turn ticks timers, futures and scheduled
//! redraws, asks [`FramePacing`] for the window's next frame, renders when the tree needs a redraw and otherwise
//! sleeps as [`LoopWait`] says. The render engine models a 60 Hz FIFO swapchain with a frame latency of one: a
//! frame waits for the vblank after the previous present, as `get_current_texture` does.

use std::{
  collections::VecDeque,
  f32::consts::TAU,
  num::NonZeroIsize,
  sync::{Arc, Mutex},
  time::{Duration, Instant},
};

use raw_window_handle::{
  DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, Win32WindowHandle, WindowHandle, WindowsDisplayHandle,
};

use super::{
  frame_pacing::FramePacing,
  loop_wait::{LoopNeeds, LoopWait},
};
use crate::{
  animation::{AnimatableProperty, Animation, Easing, Keyframes, KeyframesId, StepPosition},
  app::{
    App, Tree,
    component::Component,
    ctx::{Ctx, Interval},
    events::MouseButton,
    render_engine::RenderEngine,
  },
  components::{Rect, Row},
  layout::render_list::RenderList,
  node::{Element, transform::Decomposed},
};

const REFRESH: Duration = Duration::from_nanos(16_666_667);
const TURN: KeyframesId = KeyframesId::new(41);
const SPINNER: (f32, f32) = (20.0, 20.0);

struct TestSurface;

impl HasWindowHandle for TestSurface {
  fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
    let handle = Win32WindowHandle::new(NonZeroIsize::new(1).expect("1 is not zero"));
    // SAFETY: the handle is never dereferenced: the test render engine ignores it.
    Ok(unsafe { WindowHandle::borrow_raw(handle.into()) })
  }
}

impl HasDisplayHandle for TestSurface {
  fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
    // SAFETY: as above, the handle is never used.
    Ok(unsafe { DisplayHandle::borrow_raw(WindowsDisplayHandle::new().into()) })
  }
}

#[derive(Clone, Copy, Debug)]
struct Frame {
  /// When the pass that drew the frame started, which is the time the animation was sampled at.
  pass_at: Instant,
  /// The spinner's rotation, in radians.
  angle: f32,
}

struct Display {
  epoch: Instant,
  last_present: Option<Instant>,
  pass_at: Instant,
  frames: Vec<Frame>,
}

impl Display {
  fn vblank_after(&self, at: Instant) -> Instant {
    let refreshes = (at - self.epoch).as_nanos().div_ceil(REFRESH.as_nanos());
    self.epoch + REFRESH * u32::try_from(refreshes).expect("a test runs for less than two years")
  }
}

struct VsyncEngine(Arc<Mutex<Display>>);

impl RenderEngine for VsyncEngine {
  fn resize(&mut self, _width: u32, _height: u32) {}

  fn render(&mut self, list: &RenderList, _window: WindowHandle<'_>, _display: DisplayHandle<'_>) -> bool {
    let mut display = self.0.lock().expect("the display is used on one thread");
    if let Some(previous) = display.last_present {
      sleep_until(display.vblank_after(previous));
    }
    display.last_present = Some(Instant::now());
    let [a, b, _, _] = list.rects.first().map_or([1.0, 0.0, 0.0, 1.0], |rect| rect.transform);
    let frame = Frame {
      pass_at: display.pass_at,
      angle: b.atan2(a).rem_euclid(TAU),
    };
    display.frames.push(frame);
    true
  }
}

fn sleep_until(at: Instant) {
  let now = Instant::now();
  if at > now {
    std::thread::sleep(at - now);
  }
}

/// The winit loop of one window, without winit.
struct Shell {
  app: App,
  tree: Tree,
  pacing: FramePacing,
  last_paint: Instant,
  display: Arc<Mutex<Display>>,
  /// Clicks on the spinner the "OS" delivers at these times.
  clicks: VecDeque<Instant>,
  /// How late each due timer fired.
  timer_lateness: Vec<Duration>,
  /// Turns that ended with the loop asleep.
  sleeps: usize,
}

impl Shell {
  fn new(tree: Tree) -> Self {
    let display = Arc::new(Mutex::new(Display {
      epoch: Instant::now(),
      last_present: None,
      pass_at: Instant::now(),
      frames: Vec::new(),
    }));
    let mut shell = Self {
      app: App::new(),
      tree,
      pacing: FramePacing::default(),
      last_paint: Instant::now(),
      display: display.clone(),
      clicks: VecDeque::new(),
      timer_lateness: Vec::new(),
      sleeps: 0,
    };
    shell.tree.resize(200, 100);
    shell
      .tree
      .set_render_engine_factory(move || Box::new(VsyncEngine(display.clone())));
    shell.tree.request_redraw();
    shell
  }

  fn run_for(&mut self, duration: Duration) {
    let end = Instant::now() + duration;
    loop {
      let now = Instant::now();
      if now >= end {
        return;
      }
      self.turn(now, end);
    }
  }

  fn turn(&mut self, now: Instant, end: Instant) {
    while self.clicks.front().is_some_and(|at| *at <= now) {
      self.clicks.pop_front();
      self.tree.mouse_down(SPINNER.0, SPINNER.1, MouseButton::Left);
      self.tree.mouse_up(SPINNER.0, SPINNER.1, MouseButton::Left);
    }
    let timer_due = self.tree.next_timer_at().filter(|due| *due <= now);
    self.tree.tick_timers();
    if let Some(due) = timer_due {
      self.timer_lateness.push(Instant::now() - due);
    }
    self.tree.tick_futures();
    self
      .pacing
      .request_frame(&mut self.tree, self.last_paint, || REFRESH, now);
    self.tree.tick_scheduled_redraw(now);
    if self.tree.needs_redraw() {
      self.display.lock().expect("one thread").pass_at = Instant::now();
      if self.tree.pass(&mut self.app, &TestSurface).rendered {
        self.last_paint = Instant::now();
      }
      return;
    }
    let needs = LoopNeeds {
      polling: self.tree.needs_polling(),
      next_redraw: self.tree.next_scheduled_redraw(),
      next_timer: self.tree.next_timer_at(),
      ..LoopNeeds::default()
    };
    let wake = match LoopWait::of(needs, now) {
      LoopWait::Poll => return,
      LoopWait::WaitUntil(at) => at.min(end),
      LoopWait::Wait => end,
    };
    let wake = self.clicks.front().map_or(wake, |click| wake.min(*click));
    self.sleeps += 1;
    sleep_until(wake);
  }

  fn frames(&self) -> Vec<Frame> {
    self.display.lock().expect("one thread").frames.clone()
  }
}

fn turn_keyframes() -> Keyframes {
  Keyframes::new(TURN)
    .frame(0.0, |f| {
      f.set(AnimatableProperty::Transform, Decomposed::IDENTITY.with_rotate(0.0));
    })
    .frame(1.0, |f| {
      f.set(AnimatableProperty::Transform, Decomposed::IDENTITY.with_rotate(TAU));
    })
}

fn steps(count: u32) -> Easing {
  Easing::Steps {
    count,
    position: StepPosition::JumpEnd,
  }
}

fn spinner(easing: Easing) -> Tree {
  let mut tree = Tree::new();
  tree.register_keyframes(turn_keyframes());
  tree.set_root(
    Row::new().child(
      Rect::new(40.0, 40.0)
        .background("#2563eb")
        .animation(Animation::new(TURN).duration_ms(1000).easing(easing).infinite()),
    ),
  );
  tree
}

/// Which of the `count` poses of a turn the angle is.
fn pose(angle: f32, count: u32) -> u32 {
  ((angle / (TAU / count as f32)).round() as u32) % count
}

#[test]
fn a_spinner_of_twelve_steps_a_second_presents_about_twelve_frames_a_second() {
  let mut shell = Shell::new(spinner(steps(12)));
  shell.run_for(Duration::from_secs(1));

  let frames = shell.frames();
  assert!(
    (11..=14).contains(&frames.len()),
    "a 12-step turn per second draws a frame per step, not one per refresh: {} frames in 1 s",
    frames.len()
  );
  for pair in frames.windows(2) {
    assert_ne!(
      pose(pair[0].angle, 12),
      pose(pair[1].angle, 12),
      "every frame shows the next step: {frames:?}"
    );
  }
}

#[test]
fn a_continuous_spinner_advances_every_refresh_through_paced_frames() {
  let mut shell = Shell::new(spinner(Easing::Linear));
  shell.run_for(Duration::from_millis(500));

  let frames = shell.frames();
  assert!(
    (24..=32).contains(&frames.len()),
    "a continuous animation runs at the display's rate: {} frames in 0.5 s",
    frames.len()
  );
  let advanced = frames.windows(2).filter(|pair| pair[0].angle != pair[1].angle).count();
  assert_eq!(advanced, frames.len() - 1, "every frame turns the spinner: {frames:?}");
  assert!(
    shell.sleeps >= frames.len() - 1,
    "the loop sleeps until each frame is due instead of waiting for the swapchain: {} sleeps for {} frames",
    shell.sleeps,
    frames.len()
  );
}

#[test]
fn the_loop_waits_for_an_event_once_an_animation_ends() {
  let mut tree = Tree::new();
  tree.register_keyframes(turn_keyframes());
  tree.set_root(
    Row::new().child(Rect::new(40.0, 40.0).animation(Animation::new(TURN).duration_ms(300).easing(steps(3)))),
  );
  let mut shell = Shell::new(tree);
  shell.run_for(Duration::from_millis(500));

  let frames = shell.frames().len();
  assert!((3..=5).contains(&frames), "three steps and the end: {frames} frames");
  assert!(!shell.tree.has_active_timeline());
  assert!(!shell.tree.needs_redraw());
  let needs = LoopNeeds {
    polling: shell.tree.needs_polling(),
    next_redraw: shell.tree.next_scheduled_redraw(),
    next_timer: shell.tree.next_timer_at(),
    ..LoopNeeds::default()
  };
  assert_eq!(
    LoopWait::of(needs, Instant::now()),
    LoopWait::Wait,
    "nothing is left to wake for"
  );
}

#[test]
fn a_minimized_window_draws_no_animation_frames_and_resumes_on_time_when_restored() {
  let mut shell = Shell::new(spinner(steps(12)));
  shell.run_for(Duration::from_millis(300));
  let started_at = shell.frames()[0].pass_at;

  shell.tree.window().set_minimized(true);
  let frames_before = shell.frames().len();
  shell.run_for(Duration::from_millis(500));
  assert_eq!(
    shell.frames().len(),
    frames_before,
    "a minimized window draws no animation frames"
  );

  shell.tree.window().set_minimized(false);
  let restored_at = Instant::now();
  shell.run_for(Duration::from_millis(400));

  let resumed = &shell.frames()[frames_before..];
  assert!(
    (4..=7).contains(&resumed.len()),
    "the spinner turns again at its pace after the restore: {} frames in 0.4 s",
    resumed.len()
  );
  assert!(
    resumed[0].pass_at - restored_at < REFRESH,
    "the first frame comes at once"
  );
  for frame in resumed {
    let elapsed_steps = ((frame.pass_at - started_at).as_secs_f64() * 12.0).floor() as u32 % 12;
    let drawn = pose(frame.angle, 12);
    let off = (drawn + 12 - elapsed_steps) % 12;
    assert!(
      off == 0 || off == 11,
      "the animation kept its time while minimized: step {drawn} drawn at step {elapsed_steps} of the clock"
    );
  }
}

/// When each click reached the spinner's handler.
#[derive(Clone, Default)]
struct ClickLog(Arc<Mutex<Vec<Instant>>>);

impl PartialEq for ClickLog {
  fn eq(&self, other: &Self) -> bool {
    Arc::ptr_eq(&self.0, &other.0)
  }
}

#[cfg(feature = "devtools")]
impl crate::app::component::DevtoolsInspectable for ClickLog {
  fn write_info(&self, _buffer: &mut Vec<crate::app::component::ComponentInfo>) {}
}

/// A turning spinner that logs its clicks, beside a 25 ms interval.
struct ClickableSpinner {
  clicks: ClickLog,
  _interval: Interval,
}

impl Component for ClickableSpinner {
  type Props = ClickLog;

  fn create(ctx: &mut Ctx) -> Self {
    let interval = ctx.create_interval(Duration::from_millis(25), || {});
    interval.start();
    Self {
      clicks: ctx.props::<ClickLog>().clone(),
      _interval: interval,
    }
  }

  fn render(&self, _ctx: &mut Ctx) -> impl Into<Element> {
    let clicks = self.clicks.clone();
    Row::new().child(
      Rect::new(40.0, 40.0)
        .background("#2563eb")
        .animation(Animation::new(TURN).duration_ms(1000).easing(steps(12)).infinite())
        .on_click(move |_| clicks.0.lock().expect("one thread").push(Instant::now())),
    )
  }
}

#[test]
fn input_and_timers_are_handled_at_once_while_a_spinner_turns() {
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.register_keyframes(turn_keyframes());
  let log = ClickLog::default();
  tree.mount_root::<ClickableSpinner>(&mut app, log.clone());
  let mut shell = Shell::new(tree);
  shell.run_for(Duration::from_millis(100));

  let first_click = Instant::now() + Duration::from_millis(7);
  let clicks: Vec<Instant> = (0..12).map(|i| first_click + Duration::from_millis(37) * i).collect();
  shell.clicks = clicks.iter().copied().collect();
  shell.timer_lateness.clear();
  shell.run_for(Duration::from_millis(600));

  let clicked_at = log.0.lock().expect("one thread").clone();
  assert_eq!(clicked_at.len(), clicks.len(), "every click is handled");
  let mut click_latency: Vec<Duration> = clicks
    .iter()
    .zip(&clicked_at)
    .map(|(sent, handled)| *handled - *sent)
    .collect();
  click_latency.sort();
  let mut timer_lateness = shell.timer_lateness.clone();
  timer_lateness.sort();
  assert!(
    timer_lateness.len() >= 15,
    "the 25 ms interval keeps firing: {timer_lateness:?}"
  );
  // A present within a refresh of the previous one still waits for the vblank, as on a real FIFO swapchain, so an
  // event that comes during it waits too: the medians tell a loop that sleeps between frames from one that waits
  // in the swapchain.
  let median = |sorted: &[Duration]| sorted[sorted.len() / 2];
  let on_time = Duration::from_millis(3);
  assert!(
    median(&click_latency) < on_time && median(&timer_lateness) < on_time,
    "clicks are handled and timers fire when they are due, not after the next frame:
  click latency {click_latency:?}
  timer lateness {timer_lateness:?}"
  );
}
