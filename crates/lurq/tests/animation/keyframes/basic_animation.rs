use lurq::{
  animation::{AnimatableProperty, AnimatableValue, Animation, Keyframes, KeyframesId},
  app::Tree,
  layout::{Constraints, Size, quad::QuadContent},
  node::color::Color,
};

use crate::support::run_pass;

#[test]
fn keyframe_animation_applies_values_to_node() {
  let mut rt = Tree::new();

  rt.register_keyframes(
    Keyframes::new(KeyframesId::new(4))
      .frame(0.0, |f| {
        f.set(AnimatableProperty::Opacity, AnimatableValue::Float(0.0));
      })
      .frame(1.0, |f| {
        f.set(AnimatableProperty::Opacity, AnimatableValue::Float(1.0));
      }),
  );

  let node = lurq::components::Rect::new(100.0, 50.0)
    .background("#ff0000")
    .animation(Animation::new(KeyframesId::new(4)).duration_ms(100).linear());
  rt.set_root(node);

  rt.set_layout_constraints_override(Some(Constraints::loose(Size::new(400.0, 400.0))));
  run_pass(&mut rt);

  let result = rt.last_layout().unwrap();
  let quads = rt.resolve_quads(result);
  assert!(!quads.is_empty());
  assert!(quads[0].opacity < 1.0, "opacity should be animated below 1.0 at start");
}

#[test]
fn animation_with_color_keyframes() {
  let mut rt = Tree::new();

  rt.register_keyframes(
    Keyframes::new(KeyframesId::new(2))
      .frame(0.0, |f| {
        f.set(
          AnimatableProperty::BackgroundColor,
          AnimatableValue::Color(Color::new(255, 0, 0, 255)),
        );
      })
      .frame(1.0, |f| {
        f.set(
          AnimatableProperty::BackgroundColor,
          AnimatableValue::Color(Color::new(0, 0, 255, 255)),
        );
      }),
  );

  let node = lurq::components::Rect::new(100.0, 50.0)
    .background("#ff0000")
    .animation(Animation::new(KeyframesId::new(2)).duration_ms(1000).linear());
  rt.set_root(node);

  rt.set_layout_constraints_override(Some(Constraints::loose(Size::new(400.0, 400.0))));
  run_pass(&mut rt);

  let result = rt.last_layout().unwrap();
  let quads = rt.resolve_quads(result);
  let color = match &quads[0].content {
    QuadContent::Rect { color } => *color,
    _ => panic!("expected rect"),
  };
  assert_ne!(
    color,
    Color::new(0, 0, 255, 255),
    "should not have reached final color yet"
  );
}

#[test]
fn animation_with_unregistered_keyframes_does_nothing() {
  let mut rt = Tree::new();

  let node = lurq::components::Rect::new(100.0, 50.0)
    .background("#ff0000")
    .animation(Animation::new(KeyframesId::new(9)).duration_ms(100).linear());
  rt.set_root(node);

  rt.set_layout_constraints_override(Some(Constraints::loose(Size::new(400.0, 400.0))));
  run_pass(&mut rt);

  let result = rt.last_layout().unwrap();
  let quads = rt.resolve_quads(result);
  let color = match &quads[0].content {
    QuadContent::Rect { color } => *color,
    _ => panic!("expected rect"),
  };
  assert_eq!(color, Color::from_hex("#ff0000"), "color should be unchanged");
}

#[test]
fn three_keyframe_animation_interpolates_middle() {
  let mut rt = Tree::new();

  rt.register_keyframes(
    Keyframes::new(KeyframesId::new(17))
      .frame(0.0, |f| {
        f.set(AnimatableProperty::Opacity, AnimatableValue::Float(0.0));
      })
      .frame(0.5, |f| {
        f.set(AnimatableProperty::Opacity, AnimatableValue::Float(1.0));
      })
      .frame(1.0, |f| {
        f.set(AnimatableProperty::Opacity, AnimatableValue::Float(0.0));
      }),
  );

  let node = lurq::components::Rect::new(100.0, 50.0)
    .background("#ff0000")
    .animation(Animation::new(KeyframesId::new(17)).duration_ms(10000).linear());
  rt.set_root(node);

  rt.set_layout_constraints_override(Some(Constraints::loose(Size::new(400.0, 400.0))));
  run_pass(&mut rt);

  let result = rt.last_layout().unwrap();
  let quads = rt.resolve_quads(result);
  assert!(quads[0].opacity >= 0.0 && quads[0].opacity <= 1.0);
}
