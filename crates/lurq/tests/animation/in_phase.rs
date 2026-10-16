//! Keyframes registered in phase play on the process's animation clock: a node that mounts late shows the same
//! pose as one that mounted earlier, in every direction and in every tree, and a `Steps` animation shown many
//! times draws one frame per step.
//!
//! The tests drive the animation clock explicitly (`Tree::set_animation_clock`) at moments counted from
//! `clock_origin()`, so nothing depends on how long a pass or a sleep takes. They play a finite count, long enough
//! for every test: a run started at the clock's origin itself, rather than on its grid, has ended by then.

use std::time::{Duration, Instant};

use lurq::{
  animation::{
    AnimatableProperty, AnimatableValue, Animation, AnimationDirection, AnimationIterationCount, Easing, Keyframes,
    KeyframesId, StepPosition, clock_origin,
  },
  app::{App, Tree, component::Component, ctx::Ctx, events::MouseButton},
  components::{Rect, Row},
  core::Signal,
  layout::quad::QuadContent,
  node::{Element, color::Color},
};

use crate::support::pointer_click;

const PHASED: u16 = 61;
const FREE: u16 = 62;
const PHASED_COLOR: &str = "#2563eb";
const FREE_COLOR: &str = "#ef4444";
/// A step's frame is drawn this long after the step, at most (`STEP_MARGIN` in the engine, with room to spare).
const MARGIN: Duration = Duration::from_millis(1);

fn fade_keyframes(id: u16) -> Keyframes {
  Keyframes::new(KeyframesId::new(id))
    .frame(0.0, |f| {
      f.set(AnimatableProperty::Opacity, AnimatableValue::Float(0.2));
    })
    .frame(1.0, |f| {
      f.set(AnimatableProperty::Opacity, AnimatableValue::Float(1.0));
    })
}

/// A fade over a second in four steps, whose poses (0.2, 0.4, 0.6, 0.8) are all below full opacity; a node that
/// does not play it draws at full opacity.
fn stepped_fade(id: u16) -> Animation {
  Animation::new(KeyframesId::new(id))
    .duration_ms(1000)
    .easing(Easing::Steps {
      count: 4,
      position: StepPosition::JumpEnd,
    })
    .iteration_count(AnimationIterationCount::Count(20.0))
}

/// What a [`Fades`] host plays.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Plays {
  /// The stepped fade in phase.
  Phased,
  /// The stepped fade in phase, in this direction.
  Directed(AnimationDirection),
  /// The stepped fade in phase, waiting this many milliseconds first.
  Delayed(u64),
  /// The stepped fade in phase, this many times.
  Count(f64),
  /// The stepped fade in phase, each beside a free one.
  Mixed,
}

#[cfg(feature = "devtools")]
impl lurq::app::component::DevtoolsInspectable for Plays {}

impl Plays {
  fn phased(self) -> Animation {
    match self {
      Self::Phased | Self::Mixed => stepped_fade(PHASED),
      Self::Directed(direction) => stepped_fade(PHASED).direction(direction),
      Self::Delayed(ms) => stepped_fade(PHASED).delay_ms(ms),
      Self::Count(count) => stepped_fade(PHASED).iteration_count(AnimationIterationCount::Count(count)),
    }
  }
}

/// Two buttons, one that shows one more fade and one that hides the last, then the fades: one when the host
/// mounts.
struct Fades {
  shown: Signal<usize>,
}

impl Component for Fades {
  type Props = Plays;

  fn create(ctx: &mut Ctx) -> Self {
    Self { shown: ctx.signal(1) }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let plays = *ctx.props::<Plays>();
    let (more, fewer) = (self.shown.clone(), self.shown.clone());
    let mut row = Row::new()
      .spacing(8.0)
      .child(
        Rect::new(40.0, 40.0)
          .background("#22c55e")
          .on_click(move |_| more.update(|shown| *shown += 1)),
      )
      .child(
        Rect::new(40.0, 40.0)
          .background("#f59e0b")
          .on_click(move |_| fewer.update(|shown| *shown = shown.saturating_sub(1))),
      );
    for _ in 0..self.shown.get() {
      row = row.child(Rect::new(40.0, 40.0).background(PHASED_COLOR).animation(plays.phased()));
      if plays == Plays::Mixed {
        row = row.child(
          Rect::new(40.0, 40.0)
            .background(FREE_COLOR)
            .animation(stepped_fade(FREE)),
        );
      }
    }
    row
  }
}

struct Host {
  tree: Tree,
  app: App,
}

impl Host {
  fn new(plays: Plays) -> Self {
    let (mut tree, mut app) = (Tree::new(), App::new());
    tree.register_keyframes_in_phase(fade_keyframes(PHASED));
    tree.register_keyframes(fade_keyframes(FREE));
    tree.mount_root::<Fades>(&mut app, plays);
    tree.resize(600, 100);
    Self { tree, app }
  }

  /// Draws a frame `ms` after the grid start that [`at`] counts from.
  fn pass_at(&mut self, ms: u64) -> &mut Self {
    self.tree.set_animation_clock(Some(at(ms)));
    self.tree.pass_headless(&mut self.app);
    self
  }

  fn show_one_more(&mut self) -> &mut Self {
    pointer_click(&mut self.tree, 20.0, 20.0, MouseButton::Left);
    self
  }

  fn hide_the_last(&mut self) -> &mut Self {
    pointer_click(&mut self.tree, 68.0, 20.0, MouseButton::Left);
    self
  }

  /// The opacities of the rects of `color`, in tree order.
  fn opacities(&self, color: &str) -> Vec<f32> {
    let color = Color::from_hex(color);
    let quads = self
      .tree
      .resolve_quads(self.tree.last_layout().expect("a pass laid the tree out"));
    quads
      .iter()
      .filter(|quad| matches!(quad.content, QuadContent::Rect { color: drawn, .. } if drawn == color))
      .map(|quad| quad.opacity)
      .collect()
  }

  fn phased(&self) -> Vec<f32> {
    self.opacities(PHASED_COLOR)
  }

  fn next_change(&self) -> Instant {
    self.tree.next_timeline_change().expect("the fades run")
  }
}

/// `ms` after a moment 100 s past the clock's origin, a whole number of periods past it in every direction.
fn at(ms: u64) -> Instant {
  clock_origin() + Duration::from_secs(100) + Duration::from_millis(ms)
}

#[track_caller]
fn assert_poses(drawn: &[f32], expected: &[f32]) {
  assert!(
    drawn.len() == expected.len()
      && drawn
        .iter()
        .zip(expected)
        .all(|(drawn, expected)| (drawn - expected).abs() < 1e-4),
    "drawn {drawn:?}, expected {expected:?}"
  );
}

/// The next frame is due at the step `ms` after the grid start, with the engine's margin.
#[track_caller]
fn assert_next_step_at(host: &Host, ms: u64) {
  let (next, step) = (host.next_change(), at(ms));
  assert!(
    next >= step && next <= step + MARGIN,
    "the next frame is due {:?} after the grid start, not at the step {ms} ms after it",
    next.saturating_duration_since(at(0))
  );
}

#[test]
fn a_fade_mounted_more_than_a_duration_late_shows_the_same_pose_and_steps_together() {
  let mut host = Host::new(Plays::Phased);
  host.pass_at(100);
  assert_poses(&host.phased(), &[0.2]);

  host.show_one_more().pass_at(2650);

  assert_poses(&host.phased(), &[0.6, 0.6]);
  assert_next_step_at(&host, 2750);
}

#[test]
fn alternating_fades_mounted_late_play_the_same_iteration_backward() {
  let mut host = Host::new(Plays::Directed(AnimationDirection::Alternate));
  host.pass_at(100);
  host.show_one_more().pass_at(3300);

  // The fourth iteration plays backward: 0.3 into it is 0.7 of the fade.
  assert_poses(&host.phased(), &[0.6, 0.6]);
  assert_next_step_at(&host, 3500);
}

#[test]
fn alternate_reverse_fades_mounted_late_play_the_same_iteration_forward() {
  let mut host = Host::new(Plays::Directed(AnimationDirection::AlternateReverse));
  host.pass_at(100);
  host.show_one_more().pass_at(3300);

  // The fourth iteration of an alternate-reverse fade plays forward.
  assert_poses(&host.phased(), &[0.4, 0.4]);
  assert_next_step_at(&host, 3500);
}

#[test]
fn a_finite_count_ends_on_the_clock_not_a_count_after_the_mount() {
  let mut host = Host::new(Plays::Count(2.0));
  host.pass_at(100);
  host.show_one_more().pass_at(1500);
  assert_poses(&host.phased(), &[0.6, 0.6]);

  host.pass_at(1950);
  assert_poses(&host.phased(), &[0.8, 0.8]);
  assert_next_step_at(&host, 2000);

  // The first fade ended on the clock, 1.9 s after its mount; the second plays its last iteration.
  host.pass_at(2050);
  assert_poses(&host.phased()[1..], &[0.2]);
  assert_next_step_at(&host, 2250);
  host.pass_at(2950);
  assert_poses(&host.phased()[1..], &[0.8]);
  assert_next_step_at(&host, 3000);
  host.pass_at(3050);
  assert!(
    !host.tree.has_active_timeline(),
    "the second fade ended on the clock too, 1.5 s after its mount"
  );
}

#[test]
fn a_delay_counts_from_the_run_s_start_on_the_clock() {
  let mut host = Host::new(Plays::Delayed(400));
  host.pass_at(100);
  assert_poses(&host.phased(), &[1.0]);
  assert_eq!(
    host.next_change(),
    at(400),
    "the delay ends 400 ms after the grid start"
  );

  host.pass_at(450);
  assert_poses(&host.phased(), &[0.2]);

  // Mounted past the delay of its period: it plays at once, in the same pose.
  host.show_one_more().pass_at(1700);
  assert_poses(&host.phased(), &[0.4, 0.4]);

  // Mounted within the delay of its period: it waits for the rest of it.
  host.show_one_more().pass_at(2200);
  assert_poses(&host.phased(), &[0.8, 0.8, 1.0]);
  assert_eq!(
    host.next_change(),
    at(2400),
    "the last fade's delay ends with the others' iteration"
  );
  host.pass_at(2450);
  assert_poses(&host.phased(), &[0.2, 0.2, 0.2]);
}

#[test]
fn a_remounted_fade_shows_the_pose_of_the_fades_that_stayed() {
  let mut host = Host::new(Plays::Phased);
  host.pass_at(100);
  host.show_one_more().pass_at(300);
  assert_poses(&host.phased(), &[0.4, 0.4]);
  host.hide_the_last().pass_at(500);
  assert_poses(&host.phased(), &[0.6]);

  host.show_one_more().pass_at(3700);

  assert_poses(&host.phased(), &[0.6, 0.6]);
  assert_next_step_at(&host, 3750);
}

#[test]
fn free_fades_keep_their_own_phase_beside_fades_in_phase() {
  let mut host = Host::new(Plays::Mixed);
  host.pass_at(100);
  host.show_one_more().pass_at(1400);

  assert_poses(&host.phased(), &[0.4, 0.4]);
  assert_poses(&host.opacities(FREE_COLOR), &[0.4, 0.2]);
  assert_next_step_at(&host, 1500);

  host.pass_at(1501);
  assert_poses(&host.phased(), &[0.6, 0.6]);
  assert_poses(&host.opacities(FREE_COLOR), &[0.4, 0.2]);
  // The first free fade, mounted 100 ms after the grid start, steps 100 ms after the fades in phase.
  assert_next_step_at(&host, 1600);
}

#[test]
fn fades_in_phase_in_two_trees_show_the_same_pose() {
  let mut first = Host::new(Plays::Phased);
  first.pass_at(100);
  // With a clock per tree, the second tree's would start this much later.
  std::thread::sleep(Duration::from_millis(300));
  let mut second = Host::new(Plays::Phased);

  second.pass_at(2650);
  first.pass_at(2650);

  assert_poses(&first.phased(), &[0.6]);
  assert_poses(&second.phased(), &[0.6]);
  assert_next_step_at(&second, 2750);
}
