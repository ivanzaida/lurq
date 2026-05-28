use lurq::animation::AnimatableValue;

fn float(v: f32) -> AnimatableValue {
  AnimatableValue::Float(v)
}

#[test]
fn lerp_at_zero_returns_from() {
  assert_eq!(float(10.0).lerp(&float(20.0), 0.0), float(10.0));
}

#[test]
fn lerp_at_one_returns_to() {
  assert_eq!(float(10.0).lerp(&float(20.0), 1.0), float(20.0));
}

#[test]
fn lerp_at_half() {
  assert_eq!(float(0.0).lerp(&float(100.0), 0.5), float(50.0));
}

#[test]
fn lerp_negative_values() {
  assert_eq!(float(-50.0).lerp(&float(50.0), 0.5), float(0.0));
}

#[test]
fn lerp_same_value_returns_same() {
  assert_eq!(float(42.0).lerp(&float(42.0), 0.5), float(42.0));
}

#[test]
fn lerp_beyond_one_extrapolates() {
  assert_eq!(float(0.0).lerp(&float(100.0), 1.5), float(150.0));
}

#[test]
fn lerp_below_zero_extrapolates() {
  assert_eq!(float(0.0).lerp(&float(100.0), -0.5), float(-50.0));
}
