use lurq::layout::box_shadow::{blurred_rounded_rect_coverage, erf, spread_radius};

/// The shape convolved with a Gaussian by brute force: the definition the
/// analytic shader formula approximates.
fn reference_coverage(x: f32, y: f32, half: [f32; 2], radius: f32, sigma: f32) -> f32 {
  let inside = |px: f64, py: f64| {
    let (hx, hy, r) = (f64::from(half[0]), f64::from(half[1]), f64::from(radius));
    let qx = px.abs() - (hx - r);
    let qy = py.abs() - (hy - r);
    if px.abs() > hx || py.abs() > hy {
      return false;
    }
    if qx > 0.0 && qy > 0.0 {
      return qx * qx + qy * qy <= r * r;
    }
    true
  };
  let sigma = f64::from(sigma);
  let extent = 4.0 * sigma;
  let steps = 160;
  let step = 2.0 * extent / f64::from(steps);
  let mut sum = 0.0;
  let mut weight = 0.0;
  for j in 0..steps {
    let dy = -extent + (f64::from(j) + 0.5) * step;
    for i in 0..steps {
      let dx = -extent + (f64::from(i) + 0.5) * step;
      let w = (-(dx * dx + dy * dy) / (2.0 * sigma * sigma)).exp();
      weight += w;
      if inside(f64::from(x) - dx, f64::from(y) - dy) {
        sum += w;
      }
    }
  }
  (sum / weight) as f32
}

#[test]
fn box_shadow_erf_matches_known_values() {
  for (x, expected) in [(0.0, 0.0), (0.5, 0.5205), (1.0, 0.8427), (2.0, 0.9953), (-1.0, -0.8427)] {
    assert!((erf(x) - expected).abs() < 1e-3, "erf({x}) = {}", erf(x));
  }
}

#[test]
fn box_shadow_blur_matches_a_brute_force_gaussian_convolution() {
  let cases = [
    ([40.0, 24.0], 0.0, 4.0),
    ([40.0, 24.0], 8.0, 4.0),
    ([40.0, 24.0], 12.0, 10.0),
    ([20.0, 20.0], 20.0, 6.0),
    ([60.0, 10.0], 4.0, 12.0),
  ];
  let mut worst = 0.0_f32;
  for (half, radius, sigma) in cases {
    let mut y = -half[1] - 3.0 * sigma;
    while y <= half[1] + 3.0 * sigma {
      let mut x = -half[0] - 3.0 * sigma;
      while x <= half[0] + 3.0 * sigma {
        let analytic = blurred_rounded_rect_coverage(x, y, half, [radius; 4], sigma);
        let reference = reference_coverage(x, y, half, radius, sigma);
        worst = worst.max((analytic - reference).abs());
        x += half[0] / 5.0 + 0.37;
      }
      y += half[1] / 5.0 + 0.41;
    }
  }
  // 1.5% of full coverage is 4 of 255 levels at an opaque shadow colour
  // (measured: 1.15%); real shadows are translucent, so the visible error is
  // smaller still.
  assert!(worst < 0.015, "worst coverage error {worst}");
}

#[test]
fn box_shadow_coverage_is_half_on_a_straight_edge_and_full_inside() {
  let half = [50.0, 30.0];
  let at_edge = blurred_rounded_rect_coverage(50.0, 0.0, half, [0.0; 4], 5.0);
  assert!((at_edge - 0.5).abs() < 0.01, "{at_edge}");
  let inside = blurred_rounded_rect_coverage(0.0, 0.0, half, [0.0; 4], 5.0);
  assert!(inside > 0.99, "{inside}");
  let outside = blurred_rounded_rect_coverage(50.0 + 15.5, 0.0, half, [0.0; 4], 5.0);
  assert!(outside < 0.002, "{outside}");
}

#[test]
fn box_shadow_spread_adjusts_radii_like_css() {
  // Square corners stay square.
  assert_eq!(spread_radius(0.0, 10.0), 0.0);
  // A corner at least as large as the spread grows by the spread.
  assert_eq!(spread_radius(12.0, 4.0), 16.0);
  // A smaller corner grows by less: r + s * (1 + (r/s - 1)^3).
  let expected = 2.0 + 8.0 * (1.0 + (0.25_f32 - 1.0).powi(3));
  assert!((spread_radius(2.0, 8.0) - expected).abs() < 1e-5);
  // Shrinking stops at zero.
  assert_eq!(spread_radius(6.0, -4.0), 2.0);
  assert_eq!(spread_radius(3.0, -4.0), 0.0);
}
