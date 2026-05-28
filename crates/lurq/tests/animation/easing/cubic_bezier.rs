use lurq::animation::Easing;

#[test]
fn ease_starts_at_zero_and_ends_at_one() {
  assert_eq!(Easing::EASE.evaluate(0.0), 0.0);
  assert_eq!(Easing::EASE.evaluate(1.0), 1.0);
}

#[test]
fn ease_in_starts_at_zero_and_ends_at_one() {
  assert_eq!(Easing::EASE_IN.evaluate(0.0), 0.0);
  assert_eq!(Easing::EASE_IN.evaluate(1.0), 1.0);
}

#[test]
fn ease_out_starts_at_zero_and_ends_at_one() {
  assert_eq!(Easing::EASE_OUT.evaluate(0.0), 0.0);
  assert_eq!(Easing::EASE_OUT.evaluate(1.0), 1.0);
}

#[test]
fn ease_in_out_starts_at_zero_and_ends_at_one() {
  assert_eq!(Easing::EASE_IN_OUT.evaluate(0.0), 0.0);
  assert_eq!(Easing::EASE_IN_OUT.evaluate(1.0), 1.0);
}

#[test]
fn ease_in_is_slower_than_linear_at_start() {
  let v = Easing::EASE_IN.evaluate(0.25);
  assert!(v < 0.25, "ease-in at 0.25 should be < 0.25, got {v}");
}

#[test]
fn ease_out_is_faster_than_linear_at_start() {
  let v = Easing::EASE_OUT.evaluate(0.25);
  assert!(v > 0.25, "ease-out at 0.25 should be > 0.25, got {v}");
}

#[test]
fn ease_in_out_is_symmetric_around_midpoint() {
  let a = Easing::EASE_IN_OUT.evaluate(0.25);
  let b = Easing::EASE_IN_OUT.evaluate(0.75);
  assert!(
    (a + b - 1.0).abs() < 0.01,
    "ease-in-out should be symmetric: f(0.25)={a} + f(0.75)={b}"
  );
}

#[test]
fn ease_is_monotonically_increasing() {
  let mut prev = 0.0;
  for i in 1..=100 {
    let t = i as f64 / 100.0;
    let v = Easing::EASE.evaluate(t);
    assert!(v >= prev - 1e-9, "ease should be monotonic: f({t})={v} < prev={prev}");
    prev = v;
  }
}

#[test]
fn custom_cubic_bezier_endpoints() {
  let e = Easing::CubicBezier {
    x1: 0.1,
    y1: 0.9,
    x2: 0.9,
    y2: 0.1,
  };
  assert_eq!(e.evaluate(0.0), 0.0);
  assert_eq!(e.evaluate(1.0), 1.0);
}

#[test]
fn linear_bezier_matches_linear() {
  let e = Easing::CubicBezier {
    x1: 0.0,
    y1: 0.0,
    x2: 1.0,
    y2: 1.0,
  };
  for i in 0..=20 {
    let t = i as f64 / 20.0;
    let v = e.evaluate(t);
    assert!((v - t).abs() < 0.01, "linear bezier at {t}: got {v}");
  }
}
