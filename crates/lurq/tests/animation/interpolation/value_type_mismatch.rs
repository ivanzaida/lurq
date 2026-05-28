use lurq::animation::AnimatableValue;
use lurq::node::color::Color;

#[test]
fn mismatched_types_below_half_returns_from() {
  let from = AnimatableValue::Float(10.0);
  let to = AnimatableValue::Color(Color::new(255, 0, 0, 255));
  assert_eq!(from.lerp(&to, 0.3), from);
}

#[test]
fn mismatched_types_at_half_returns_to() {
  let from = AnimatableValue::Float(10.0);
  let to = AnimatableValue::Color(Color::new(255, 0, 0, 255));
  assert_eq!(from.lerp(&to, 0.5), to);
}

#[test]
fn mismatched_types_above_half_returns_to() {
  let from = AnimatableValue::Float(10.0);
  let to = AnimatableValue::Color(Color::new(255, 0, 0, 255));
  assert_eq!(from.lerp(&to, 0.7), to);
}

#[test]
fn mismatched_color_to_float_below_half_returns_from() {
  let from = AnimatableValue::Color(Color::new(0, 0, 0, 255));
  let to = AnimatableValue::Float(100.0);
  assert_eq!(from.lerp(&to, 0.4), from);
}
