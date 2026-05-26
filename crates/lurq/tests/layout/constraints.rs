use lurq::layout::{Constraints, Size};

#[test]
fn tight_constraints() {
  let c = Constraints::tight(Size::new(100.0, 50.0));
  assert_eq!(c.min_width, 100.0);
  assert_eq!(c.max_width, 100.0);
  assert_eq!(c.min_height, 50.0);
  assert_eq!(c.max_height, 50.0);
}

#[test]
fn loose_constraints() {
  let c = Constraints::loose(Size::new(200.0, 100.0));
  assert_eq!(c.min_width, 0.0);
  assert_eq!(c.max_width, 200.0);
  assert_eq!(c.min_height, 0.0);
  assert_eq!(c.max_height, 100.0);
}

#[test]
fn unbounded_constraints() {
  let c = Constraints::unbounded();
  assert_eq!(c.min_width, 0.0);
  assert!(c.max_width.is_infinite());
  assert_eq!(c.min_height, 0.0);
  assert!(c.max_height.is_infinite());
}

#[test]
fn constrain_clamps_below_min() {
  let c = Constraints {
    min_width: 50.0,
    max_width: 200.0,
    min_height: 30.0,
    max_height: 100.0,
  };
  let s = c.constrain(Size::new(10.0, 5.0));
  assert_eq!(s.width, 50.0);
  assert_eq!(s.height, 30.0);
}

#[test]
fn constrain_clamps_above_max() {
  let c = Constraints {
    min_width: 0.0,
    max_width: 200.0,
    min_height: 0.0,
    max_height: 100.0,
  };
  let s = c.constrain(Size::new(999.0, 999.0));
  assert_eq!(s.width, 200.0);
  assert_eq!(s.height, 100.0);
}

#[test]
fn constrain_passes_through_in_range() {
  let c = Constraints::loose(Size::new(200.0, 200.0));
  let s = c.constrain(Size::new(80.0, 120.0));
  assert_eq!(s.width, 80.0);
  assert_eq!(s.height, 120.0);
}

#[test]
fn loosen_height() {
  let c = Constraints::tight(Size::new(100.0, 50.0)).loosen_height();
  assert_eq!(c.min_width, 100.0);
  assert_eq!(c.min_height, 0.0);
  assert_eq!(c.max_height, 50.0);
}

#[test]
fn loosen_width() {
  let c = Constraints::tight(Size::new(100.0, 50.0)).loosen_width();
  assert_eq!(c.min_width, 0.0);
  assert_eq!(c.max_width, 100.0);
  assert_eq!(c.min_height, 50.0);
}
