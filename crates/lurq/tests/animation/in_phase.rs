//! Keyframes registered in phase play on the tree's animation clock: a node that mounts late shows the same
//! pose as one that mounted earlier, and a `Steps` animation shown many times draws one frame per step.

use std::time::{Duration, Instant};

use lurq::{
  animation::{AnimatableProperty, AnimatableValue, Animation, Easing, Keyframes, KeyframesId, StepPosition},
  app::{App, Tree, component::Component, ctx::Ctx, events::MouseButton},
  core::Signal,
  layout::quad::QuadContent,
  node::Element,
};

use crate::support::pointer_click;

const FADE: u16 = 61;
const STEP: Duration = Duration::from_millis(250);

fn fade_keyframes() -> Keyframes {
  Keyframes::new(KeyframesId::new(FADE))
    .frame(0.0, |f| {
      f.set(AnimatableProperty::Opacity, AnimatableValue::Float(0.2));
    })
    .frame(1.0, |f| {
      f.set(AnimatableProperty::Opacity, AnimatableValue::Float(1.0));
    })
}

/// A fade in four steps a second; its poses (0.2, 0.4, 0.6, 0.8) are all below full opacity.
fn stepped_fade() -> Animation {
  Animation::new(KeyframesId::new(FADE))
    .duration_ms(1000)
    .easing(Easing::Steps {
      count: 4,
      position: StepPosition::JumpEnd,
    })
    .infinite()
}

#[derive(Clone, Copy, Debug, PartialEq, lurq::DevtoolsInspectable)]
struct HostProps;

/// A button that shows a second fading rect beside the first: two spinners started at different moments.
struct TwoFades {
  second: Signal<bool>,
}

impl Component for TwoFades {
  type Props = HostProps;

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      second: ctx.signal(false),
    }
  }

  fn render(&self, _ctx: &mut Ctx) -> impl Into<Element> {
    let second = self.second.clone();
    let fade = || {
      lurq::components::Rect::new(40.0, 40.0)
        .background("#2563eb")
        .animation(stepped_fade())
    };
    let mut row = lurq::components::Row::new()
      .spacing(8.0)
      .child(
        lurq::components::Rect::new(40.0, 40.0)
          .background("#22c55e")
          .on_click(move |_| second.set(true)),
      )
      .child(fade());
    if self.second.get() {
      row = row.child(fade());
    }
    row
  }
}

fn two_fades(register: impl FnOnce(&mut Tree, Keyframes)) -> (Tree, App) {
  let (mut tree, mut app) = (Tree::new(), App::new());
  register(&mut tree, fade_keyframes());
  tree.mount_root::<TwoFades>(&mut app, HostProps);
  tree.resize(300, 100);
  (tree, app)
}

/// The faded rects' opacities, in tree order; the button is drawn at full opacity.
fn fades(tree: &Tree) -> Vec<f32> {
  let quads = tree.resolve_quads(tree.last_layout().expect("a pass laid the tree out"));
  quads
    .iter()
    .filter(|quad| matches!(quad.content, QuadContent::Rect { .. }) && quad.opacity < 1.0)
    .map(|quad| quad.opacity)
    .collect()
}

/// Mounts the first fade, mounts the second 130 ms later, then draws the frame the timeline asks for next.
/// Returns the opacities of that frame and how long after it the timeline asks for the one after.
fn second_mounted_late((mut tree, mut app): (Tree, App)) -> (Vec<f32>, Duration) {
  tree.pass_headless(&mut app);
  std::thread::sleep(Duration::from_millis(130));
  pointer_click(&mut tree, 20.0, 20.0, MouseButton::Left);
  tree.pass_headless(&mut app);
  assert_eq!(fades(&tree).len(), 2, "both fades show");

  let next = tree.next_timeline_change().expect("the fades run");
  std::thread::sleep(next.saturating_duration_since(Instant::now()));
  tree.pass_headless(&mut app);
  let drawn = Instant::now();
  let after = tree.next_timeline_change().expect("the fades run");
  (fades(&tree), after.saturating_duration_since(drawn))
}

#[test]
fn fades_in_phase_mounted_at_different_moments_step_together() {
  let (opacities, next) = second_mounted_late(two_fades(Tree::register_keyframes_in_phase));

  assert!(
    (opacities[0] - opacities[1]).abs() < 1e-4,
    "the late fade shows the same pose: {opacities:?}"
  );
  assert!(
    next > STEP - Duration::from_millis(20),
    "both fades step at the same moments, so the next frame is a whole step away: {next:?}"
  );
}

#[test]
fn fades_registered_plainly_start_when_their_node_mounts() {
  let (opacities, next) = second_mounted_late(two_fades(Tree::register_keyframes));

  assert!(
    (opacities[0] - opacities[1]).abs() > 0.1,
    "the late fade is a step behind: {opacities:?}"
  );
  assert!(
    next < STEP - Duration::from_millis(50),
    "the late fade steps between the first one's steps: {next:?}"
  );
}
