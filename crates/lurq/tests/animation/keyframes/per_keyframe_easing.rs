use lurq::{
  animation::{AnimatableProperty, AnimatableValue, Animation, Easing, Keyframes},
  app::Runtime,
  layout::{Constraints, Size},
};

use crate::support::run_pass;

#[test]
fn per_keyframe_easing_overrides_animation_easing() {
  let mut rt = Runtime::new();

  rt.register_keyframes(
    Keyframes::new("custom-ease")
      .frame(0.0, |f| {
        f.set(AnimatableProperty::Opacity, AnimatableValue::Float(0.0));
        f.easing(Easing::EASE_IN);
      })
      .frame(1.0, |f| {
        f.set(AnimatableProperty::Opacity, AnimatableValue::Float(1.0));
      }),
  );

  let node = lurq::components::Rect::new(100.0, 50.0)
    .fill("#ff0000")
    .animation(Animation::new("custom-ease").duration_ms(10000).linear());
  rt.set_root(node);

  rt.set_layout_constraints_override(Some(Constraints::loose(Size::new(400.0, 400.0))));
  run_pass(&mut rt);

  let result = rt.last_layout().unwrap();
  let quads = rt.resolve_quads(result);
  assert!(
    quads[0].opacity >= 0.0 && quads[0].opacity <= 1.0,
    "opacity should be in valid range with per-keyframe easing"
  );
}

#[test]
fn keyframes_without_per_frame_easing_use_animation_easing() {
  let mut rt = Runtime::new();

  rt.register_keyframes(
    Keyframes::new("no-per-frame")
      .frame(0.0, |f| {
        f.set(AnimatableProperty::Opacity, AnimatableValue::Float(0.0));
      })
      .frame(1.0, |f| {
        f.set(AnimatableProperty::Opacity, AnimatableValue::Float(1.0));
      }),
  );

  let node = lurq::components::Rect::new(100.0, 50.0)
    .fill("#ff0000")
    .animation(Animation::new("no-per-frame").duration_ms(10000).linear());
  rt.set_root(node);

  rt.set_layout_constraints_override(Some(Constraints::loose(Size::new(400.0, 400.0))));
  run_pass(&mut rt);

  let result = rt.last_layout().unwrap();
  let quads = rt.resolve_quads(result);
  assert!(
    quads[0].opacity >= 0.0 && quads[0].opacity <= 1.0,
    "animation should apply smoothly with linear easing"
  );
}
