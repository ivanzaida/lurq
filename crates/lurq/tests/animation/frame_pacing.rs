//! An animated pass asks the shell for its next frame through `Tree::next_timeline_change`, not by leaving
//! `needs_redraw` set: a redraw requested at once would draw every refresh whatever the animation's pace.

use std::time::{Duration, Instant};

use lurq::{
  animation::{
    AnimatableProperty, AnimatableValue, Animation, AnimationDirection, AnimationIterationCount, Easing, Keyframes,
    KeyframesId, StepPosition, Transition,
  },
  app::{App, Tree},
  components::{Rect, Row},
  layout::quad::QuadContent,
  node::color::Color,
};

const FADE: u16 = 52;
/// A step's frame is drawn this long after the step, at most (`STEP_MARGIN` in the engine, with room to spare).
const MARGIN: Duration = Duration::from_millis(1);

fn fade_keyframes() -> Keyframes {
  Keyframes::new(KeyframesId::new(FADE))
    .frame(0.0, |f| {
      f.set(AnimatableProperty::Opacity, AnimatableValue::Float(0.2));
    })
    .frame(1.0, |f| {
      f.set(AnimatableProperty::Opacity, AnimatableValue::Float(1.0));
    })
}

fn fading(animation: Animation) -> Tree {
  let mut tree = Tree::new();
  tree.register_keyframes(fade_keyframes());
  tree.set_root(Row::new().child(Rect::new(40.0, 40.0).background("#2563eb").animation(animation)));
  tree.resize(200, 100);
  tree
}

fn fade() -> Animation {
  Animation::new(KeyframesId::new(FADE)).duration_ms(1000)
}

fn steps(count: u32) -> Easing {
  Easing::Steps {
    count,
    position: StepPosition::JumpEnd,
  }
}

#[test]
fn an_animated_pass_with_nothing_else_changing_leaves_no_redraw_requested() {
  let mut app = App::new();
  let mut tree = fading(fade().linear().infinite());
  tree.pass_headless(&mut app);
  tree.pass_headless(&mut app);

  assert!(tree.has_active_timeline(), "the fade runs");
  assert!(
    !tree.needs_redraw(),
    "the pass drew the fade where it is now; nothing else changed"
  );
}

#[test]
fn a_continuous_animation_changes_at_once() {
  let mut app = App::new();
  let mut tree = fading(fade().linear().infinite());
  tree.pass_headless(&mut app);
  let after_pass = Instant::now();

  let next = tree.next_timeline_change().expect("the fade runs");
  assert!(next <= after_pass, "a linear fade draws something else at every frame");
}

#[test]
fn a_step_animation_changes_at_its_next_step() {
  let mut app = App::new();
  let mut tree = fading(fade().easing(steps(4)).infinite());
  let before_pass = Instant::now();
  tree.pass_headless(&mut app);

  let next = tree.next_timeline_change().expect("the fade runs");
  let step = Duration::from_millis(250);
  assert!(
    next >= before_pass + step && next <= Instant::now() + step + MARGIN,
    "the first of four steps a second ends 250 ms after the run starts: {:?} after the pass",
    next - before_pass
  );

  std::thread::sleep(next - Instant::now());
  let second_pass = Instant::now();
  tree.pass_headless(&mut app);
  let next = tree.next_timeline_change().expect("the fade runs");
  assert!(
    next > second_pass + step - Duration::from_millis(5) && next <= second_pass + step + MARGIN,
    "a frame at a step asks for the step after it, not for itself again: {:?} after it",
    next - second_pass
  );
}

#[test]
fn a_step_animation_played_backward_changes_at_its_next_step_down() {
  let mut app = App::new();
  let mut tree = fading(
    fade()
      .easing(steps(4))
      .direction(AnimationDirection::Reverse)
      .infinite(),
  );
  tree.pass_headless(&mut app);
  // Played backward, the run starts at the end of the span, whose value a step easing holds for an instant only.
  let next = tree.next_timeline_change().expect("the fade runs");
  assert!(next <= Instant::now() + MARGIN, "the end value lasts an instant");

  std::thread::sleep(next.saturating_duration_since(Instant::now()));
  let second_pass = Instant::now();
  tree.pass_headless(&mut app);
  let next = tree.next_timeline_change().expect("the fade runs");
  let step = Duration::from_millis(250);
  assert!(
    next > second_pass + step - Duration::from_millis(5) && next <= second_pass + step + MARGIN,
    "then each of the four steps lasts 250 ms: {:?} after the pass",
    next - second_pass
  );
}

#[test]
fn a_delayed_animation_changes_when_its_delay_ends() {
  let mut app = App::new();
  let mut tree = fading(fade().linear().delay_ms(400));
  let before_pass = Instant::now();
  tree.pass_headless(&mut app);

  let next = tree.next_timeline_change().expect("the fade waits for its delay");
  assert!(
    next >= before_pass + Duration::from_millis(400) && next <= Instant::now() + Duration::from_millis(400),
    "nothing changes before the delay ends: {:?} after the pass",
    next - before_pass
  );
}

#[test]
fn a_finished_animation_asks_for_no_frame() {
  let mut app = App::new();
  let mut tree = fading(fade().duration_ms(10).easing(steps(2)));
  tree.pass_headless(&mut app);
  std::thread::sleep(Duration::from_millis(20));
  tree.pass_headless(&mut app);

  assert!(!tree.has_active_timeline());
  assert_eq!(tree.next_timeline_change(), None);
  assert!(!tree.needs_redraw());
}

#[test]
fn a_step_transition_changes_at_its_next_step() {
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.set_root(
    Row::new().child(
      Rect::new(40.0, 40.0)
        .background("#ff0000")
        .transition(Transition::background_color().duration_ms(400).easing(steps(2)))
        .hovered(|style| style.background("#0000ff")),
    ),
  );
  tree.resize(200, 100);
  tree.pass_headless(&mut app);
  tree.mouse_move(20.0, 20.0);
  let before_pass = Instant::now();
  tree.pass_headless(&mut app);

  assert!(tree.has_active_timeline(), "the hover starts the transition");
  let next = tree.next_timeline_change().expect("the transition runs");
  let step = Duration::from_millis(200);
  assert!(
    next >= before_pass + step && next <= Instant::now() + step + MARGIN,
    "the first of two steps ends halfway: {:?} after the pass",
    next - before_pass
  );
}

fn hover_transition(transition: Transition) -> Tree {
  let mut tree = Tree::new();
  tree.set_root(
    Row::new().child(
      Rect::new(40.0, 40.0)
        .background("#ff0000")
        .transition(transition)
        .hovered(|style| style.background("#0000ff")),
    ),
  );
  tree.resize(200, 100);
  tree
}

fn first_rect_color(tree: &Tree) -> Color {
  let quads = tree.resolve_quads(tree.last_layout().expect("a pass laid the tree out"));
  quads
    .iter()
    .find_map(|quad| match &quad.content {
      QuadContent::Rect { color, .. } => Some(*color),
      _ => None,
    })
    .expect("the rect is drawn")
}

#[test]
fn a_zero_duration_step_transition_completes_at_once() {
  let mut app = App::new();
  let mut tree = hover_transition(Transition::background_color().duration_ms(0).easing(steps(2)));
  tree.pass_headless(&mut app);
  tree.mouse_move(20.0, 20.0);
  tree.pass_headless(&mut app);

  assert_eq!(
    first_rect_color(&tree),
    Color::from_hex("#0000ff"),
    "the hover color shows at once"
  );
  assert!(
    !tree.has_active_timeline(),
    "a transition of no duration is over when it starts"
  );
  assert_eq!(tree.next_timeline_change(), None);
}

#[test]
fn a_fractional_iteration_count_changes_when_the_run_ends() {
  let mut app = App::new();
  // 200 ms iterations of one step each: the value jumps at each iteration's end, and 1.5 iterations end at 300 ms,
  // where the run drops its value.
  let mut tree = fading(
    fade()
      .duration_ms(200)
      .easing(steps(1))
      .iteration_count(AnimationIterationCount::Count(1.5)),
  );
  let before_pass = Instant::now();
  tree.pass_headless(&mut app);
  let next = tree.next_timeline_change().expect("the fade runs");
  std::thread::sleep(next.saturating_duration_since(Instant::now()));
  tree.pass_headless(&mut app);

  let next = tree.next_timeline_change().expect("the second iteration runs");
  let end = Duration::from_millis(300);
  assert!(
    next >= before_pass + end && next <= Instant::now() + Duration::from_millis(100) + MARGIN,
    "the half iteration ends the run 300 ms after it starts, before its step at 400 ms: {:?} after the start",
    next - before_pass
  );
}

#[test]
fn a_jump_start_animation_draws_its_first_step_at_once() {
  let mut app = App::new();
  let jump_start = Easing::Steps {
    count: 4,
    position: StepPosition::JumpStart,
  };
  let mut tree = fading(fade().easing(jump_start).infinite());
  tree.pass_headless(&mut app);

  // At its start the run draws its start value; right after it, the first of its four steps.
  let next = tree.next_timeline_change().expect("the fade runs");
  assert!(next <= Instant::now() + MARGIN, "the first step comes at once");

  std::thread::sleep(next.saturating_duration_since(Instant::now()));
  let second_pass = Instant::now();
  tree.pass_headless(&mut app);
  let next = tree.next_timeline_change().expect("the fade runs");
  let step = Duration::from_millis(250);
  assert!(
    next > second_pass + step - Duration::from_millis(5) && next <= second_pass + step + MARGIN,
    "then each step lasts 250 ms: {:?} after the pass",
    next - second_pass
  );
}

#[test]
fn a_jump_both_transition_draws_its_first_step_at_once() {
  let mut app = App::new();
  let jump_both = Easing::Steps {
    count: 2,
    position: StepPosition::JumpBoth,
  };
  let mut tree = hover_transition(Transition::background_color().duration_ms(400).easing(jump_both));
  tree.pass_headless(&mut app);
  tree.mouse_move(20.0, 20.0);
  tree.pass_headless(&mut app);

  let next = tree.next_timeline_change().expect("the transition runs");
  assert!(next <= Instant::now() + MARGIN, "the first of the steps comes at once");
}
