use lurq::animation::Easing;

#[test]
fn linear_returns_input_unchanged() {
  let e = Easing::Linear;
  for i in 0..=10 {
    let t = i as f64 / 10.0;
    assert!((e.evaluate(t) - t).abs() < 1e-12, "linear({t}) should equal {t}");
  }
}

#[test]
fn linear_clamps_below_zero() {
  assert_eq!(Easing::Linear.evaluate(-0.5), -0.5);
}

#[test]
fn linear_clamps_above_one() {
  assert_eq!(Easing::Linear.evaluate(1.5), 1.5);
}
