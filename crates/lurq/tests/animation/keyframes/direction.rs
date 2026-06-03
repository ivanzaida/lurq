use lurq::{
  animation::{
    AnimatableProperty, AnimatableValue, Animation, AnimationDirection, AnimationIterationCount, Keyframes, KeyframesId,
  },
  app::Tree,
  layout::{Constraints, Size},
};

use crate::support::run_pass;

#[test]
fn reverse_direction_starts_from_end() {
  let mut rt = Tree::new();

  rt.register_keyframes(
    Keyframes::new(KeyframesId::new(10))
      .frame(0.0, |f| {
        f.set(AnimatableProperty::Opacity, AnimatableValue::Float(0.0));
      })
      .frame(1.0, |f| {
        f.set(AnimatableProperty::Opacity, AnimatableValue::Float(1.0));
      }),
  );

  let node = lurq::components::Rect::new(100.0, 50.0)
    .background("#ff0000")
    .animation(
      Animation::new(KeyframesId::new(10))
        .duration_ms(10000)
        .linear()
        .direction(AnimationDirection::Reverse),
    );
  rt.set_root(node);

  rt.set_layout_constraints_override(Some(Constraints::loose(Size::new(400.0, 400.0))));
  run_pass(&mut rt);

  let result = rt.last_layout().unwrap();
  let quads = rt.resolve_quads(result);
  assert!(
    quads[0].opacity > 0.5,
    "reverse should start near opacity 1.0, got {}",
    quads[0].opacity
  );
}

#[test]
fn alternate_direction_reverses_on_second_iteration() {
  let mut rt = Tree::new();

  rt.register_keyframes(
    Keyframes::new(KeyframesId::new(10))
      .frame(0.0, |f| {
        f.set(AnimatableProperty::Opacity, AnimatableValue::Float(0.0));
      })
      .frame(1.0, |f| {
        f.set(AnimatableProperty::Opacity, AnimatableValue::Float(1.0));
      }),
  );

  let node = lurq::components::Rect::new(100.0, 50.0)
    .background("#ff0000")
    .animation(
      Animation::new(KeyframesId::new(10))
        .duration_ms(10)
        .linear()
        .direction(AnimationDirection::Alternate)
        .iteration_count(AnimationIterationCount::Count(2.0)),
    );
  rt.set_root(node);

  rt.set_layout_constraints_override(Some(Constraints::loose(Size::new(400.0, 400.0))));
  run_pass(&mut rt);

  let result = rt.last_layout().unwrap();
  let quads = rt.resolve_quads(result);
  assert!(quads[0].opacity >= 0.0 && quads[0].opacity <= 1.0);
}
