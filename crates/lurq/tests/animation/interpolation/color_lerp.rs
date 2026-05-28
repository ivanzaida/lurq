use lurq::animation::AnimatableValue;
use lurq::node::color::Color;

fn color(r: u8, g: u8, b: u8, a: u8) -> AnimatableValue {
  AnimatableValue::Color(Color::new(r, g, b, a))
}

#[test]
fn lerp_at_zero_returns_from() {
  let from = color(0, 0, 0, 255);
  let to = color(255, 255, 255, 255);
  assert_eq!(from.lerp(&to, 0.0), from);
}

#[test]
fn lerp_at_one_returns_to() {
  let from = color(0, 0, 0, 255);
  let to = color(255, 255, 255, 255);
  assert_eq!(from.lerp(&to, 1.0), to);
}

#[test]
fn lerp_at_half_returns_midpoint() {
  let from = color(0, 0, 0, 255);
  let to = color(254, 100, 200, 255);
  let mid = from.lerp(&to, 0.5);
  if let AnimatableValue::Color(c) = mid {
    assert_eq!(c.r(), 127);
    assert_eq!(c.g(), 50);
    assert_eq!(c.b(), 100);
    assert_eq!(c.a(), 255);
  } else {
    panic!("expected color");
  }
}

#[test]
fn lerp_alpha_channel() {
  let from = color(100, 100, 100, 0);
  let to = color(100, 100, 100, 200);
  let mid = from.lerp(&to, 0.5);
  if let AnimatableValue::Color(c) = mid {
    assert_eq!(c.a(), 100);
  } else {
    panic!("expected color");
  }
}

#[test]
fn lerp_same_color_returns_same() {
  let c = color(42, 128, 200, 255);
  assert_eq!(c.lerp(&c, 0.5), c);
}

#[test]
fn lerp_black_to_white_quarter() {
  let from = color(0, 0, 0, 255);
  let to = color(255, 255, 255, 255);
  let result = from.lerp(&to, 0.25);
  if let AnimatableValue::Color(c) = result {
    assert_eq!(c.r(), 64);
    assert_eq!(c.g(), 64);
    assert_eq!(c.b(), 64);
  } else {
    panic!("expected color");
  }
}
