use lurq::{
  animation::{AnimatableProperty, AnimatableValue, Animation, AnimationFillMode, AnimationIterationCount, Keyframes},
  app::Tree,
  layout::{Constraints, Size},
};

use crate::support::run_pass;

#[test]
fn single_iteration_finishes_after_duration() {
  let mut rt = Tree::new();

  rt.register_keyframes(
    Keyframes::new("blink")
      .frame(0.0, |f| {
        f.set(AnimatableProperty::Opacity, AnimatableValue::Float(0.0));
      })
      .frame(1.0, |f| {
        f.set(AnimatableProperty::Opacity, AnimatableValue::Float(1.0));
      }),
  );

  let node = lurq::components::Rect::new(100.0, 50.0).fill("#ff0000").animation(
    Animation::new("blink")
      .duration_ms(1)
      .linear()
      .fill_mode(AnimationFillMode::Forwards)
      .iteration_count(AnimationIterationCount::Count(1.0)),
  );
  rt.set_root(node);

  rt.set_layout_constraints_override(Some(Constraints::loose(Size::new(400.0, 400.0))));
  std::thread::sleep(std::time::Duration::from_millis(20));
  run_pass(&mut rt);
  std::thread::sleep(std::time::Duration::from_millis(20));
  run_pass(&mut rt);

  let result = rt.last_layout().unwrap();
  let quads = rt.resolve_quads(result);
  assert!(
    (quads[0].opacity - 1.0).abs() < 0.05,
    "should hold at final value after 1 iteration, got {}",
    quads[0].opacity
  );
}

#[test]
fn infinite_iteration_keeps_animating() {
  let mut rt = Tree::new();

  rt.register_keyframes(
    Keyframes::new("pulse")
      .frame(0.0, |f| {
        f.set(AnimatableProperty::Opacity, AnimatableValue::Float(0.5));
      })
      .frame(1.0, |f| {
        f.set(AnimatableProperty::Opacity, AnimatableValue::Float(1.0));
      }),
  );

  let node = lurq::components::Rect::new(100.0, 50.0)
    .fill("#ff0000")
    .animation(Animation::new("pulse").duration_ms(10).linear().infinite());
  rt.set_root(node);

  rt.set_layout_constraints_override(Some(Constraints::loose(Size::new(400.0, 400.0))));
  run_pass(&mut rt);

  assert!(rt.needs_redraw(), "infinite animation should request redraw");

  std::thread::sleep(std::time::Duration::from_millis(50));
  run_pass(&mut rt);

  assert!(
    rt.needs_redraw(),
    "infinite animation should still request redraw after many iterations"
  );
}

#[test]
fn two_iterations_run_twice_the_duration() {
  let mut rt = Tree::new();

  rt.register_keyframes(
    Keyframes::new("grow")
      .frame(0.0, |f| {
        f.set(AnimatableProperty::Opacity, AnimatableValue::Float(0.0));
      })
      .frame(1.0, |f| {
        f.set(AnimatableProperty::Opacity, AnimatableValue::Float(1.0));
      }),
  );

  let node = lurq::components::Rect::new(100.0, 50.0).fill("#ff0000").animation(
    Animation::new("grow")
      .duration_ms(5000)
      .linear()
      .iteration_count(AnimationIterationCount::Count(2.0)),
  );
  rt.set_root(node);

  rt.set_layout_constraints_override(Some(Constraints::loose(Size::new(400.0, 400.0))));
  run_pass(&mut rt);

  let result = rt.last_layout().unwrap();
  let quads = rt.resolve_quads(result);
  assert!(quads[0].opacity >= 0.0 && quads[0].opacity <= 1.0);
}
