use lurq::animation::{Easing, StepPosition};

fn steps(count: u32, position: StepPosition) -> Easing {
  Easing::Steps { count, position }
}

#[test]
fn jump_end_holds_zero_at_start() {
  let e = steps(4, StepPosition::JumpEnd);
  assert_eq!(e.evaluate(0.0), 0.0);
  assert_eq!(e.evaluate(0.1), 0.0);
}

#[test]
fn jump_end_reaches_one_at_end() {
  let e = steps(4, StepPosition::JumpEnd);
  assert_eq!(e.evaluate(1.0), 1.0);
}

#[test]
fn jump_end_steps_at_quarter_boundaries() {
  let e = steps(4, StepPosition::JumpEnd);
  assert!((e.evaluate(0.26) - 0.25).abs() < 1e-9);
  assert!((e.evaluate(0.51) - 0.5).abs() < 1e-9);
  assert!((e.evaluate(0.76) - 0.75).abs() < 1e-9);
}

#[test]
fn jump_start_jumps_immediately() {
  let e = steps(4, StepPosition::JumpStart);
  assert_eq!(e.evaluate(0.0), 0.0);
  let v = e.evaluate(0.01);
  assert!(v > 0.0, "jump-start should jump above 0 immediately, got {v}");
}

#[test]
fn jump_start_reaches_one_before_end() {
  let e = steps(4, StepPosition::JumpStart);
  let v = e.evaluate(0.99);
  assert_eq!(v, 1.0, "jump-start at t=0.99 should be 1.0, got {v}");
}

#[test]
fn single_step_jump_end_flips_at_end() {
  let e = steps(1, StepPosition::JumpEnd);
  assert_eq!(e.evaluate(0.0), 0.0);
  assert_eq!(e.evaluate(0.5), 0.0);
  assert_eq!(e.evaluate(1.0), 1.0);
}

#[test]
fn single_step_jump_start_flips_immediately() {
  let e = steps(1, StepPosition::JumpStart);
  assert_eq!(e.evaluate(0.0), 0.0);
  assert_eq!(e.evaluate(0.01), 1.0);
}

#[test]
fn jump_both_has_extra_step() {
  let e = steps(3, StepPosition::JumpBoth);
  assert_eq!(e.evaluate(0.0), 0.0);
  let v = e.evaluate(0.01);
  assert!(v > 0.0, "jump-both should jump above 0 at start, got {v}");
  assert_eq!(e.evaluate(1.0), 1.0);
}

#[test]
fn jump_none_with_one_step_stays_zero() {
  let e = steps(1, StepPosition::JumpNone);
  assert_eq!(e.evaluate(0.0), 0.0);
  assert_eq!(e.evaluate(0.5), 0.0);
  assert_eq!(e.evaluate(1.0), 1.0);
}

#[test]
fn negative_t_returns_zero() {
  let e = steps(4, StepPosition::JumpEnd);
  assert_eq!(e.evaluate(-1.0), 0.0);
}

#[test]
fn t_above_one_returns_one() {
  let e = steps(4, StepPosition::JumpEnd);
  assert_eq!(e.evaluate(2.0), 1.0);
}
