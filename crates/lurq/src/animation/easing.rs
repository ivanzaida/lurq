#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Easing {
  Linear,
  CubicBezier { x1: f64, y1: f64, x2: f64, y2: f64 },
  Steps { count: u32, position: StepPosition },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum StepPosition {
  JumpStart,
  JumpEnd,
  JumpNone,
  JumpBoth,
}

impl Easing {
  pub const EASE: Self = Self::CubicBezier {
    x1: 0.25,
    y1: 0.1,
    x2: 0.25,
    y2: 1.0,
  };
  pub const EASE_IN: Self = Self::CubicBezier {
    x1: 0.42,
    y1: 0.0,
    x2: 1.0,
    y2: 1.0,
  };
  pub const EASE_OUT: Self = Self::CubicBezier {
    x1: 0.0,
    y1: 0.0,
    x2: 0.58,
    y2: 1.0,
  };
  pub const EASE_IN_OUT: Self = Self::CubicBezier {
    x1: 0.42,
    y1: 0.0,
    x2: 0.58,
    y2: 1.0,
  };

  pub fn evaluate(&self, t: f64) -> f64 {
    match self {
      Self::Linear => t,
      Self::CubicBezier { x1, y1, x2, y2 } => {
        if t <= 0.0 {
          return 0.0;
        }
        if t >= 1.0 {
          return 1.0;
        }
        let u = cubic_bezier_solve(t, *x1, *x2);
        bezier_y(u, *y1, *y2)
      }
      Self::Steps { count, position } => evaluate_steps(t, *count, *position),
    }
  }
}

impl Default for Easing {
  fn default() -> Self {
    Self::EASE
  }
}

fn bezier_x(u: f64, x1: f64, x2: f64) -> f64 {
  let u2 = u * u;
  let u3 = u2 * u;
  let inv = 1.0 - u;
  let inv2 = inv * inv;
  3.0 * inv2 * u * x1 + 3.0 * inv * u2 * x2 + u3
}

fn bezier_x_deriv(u: f64, x1: f64, x2: f64) -> f64 {
  let u2 = u * u;
  let inv = 1.0 - u;
  3.0 * inv * inv * x1 + 6.0 * inv * u * (x2 - x1) + 3.0 * u2 * (1.0 - x2)
}

fn bezier_y(u: f64, y1: f64, y2: f64) -> f64 {
  let u2 = u * u;
  let u3 = u2 * u;
  let inv = 1.0 - u;
  let inv2 = inv * inv;
  3.0 * inv2 * u * y1 + 3.0 * inv * u2 * y2 + u3
}

fn cubic_bezier_solve(t: f64, x1: f64, x2: f64) -> f64 {
  let mut u = t;

  for _ in 0..8 {
    let dx = bezier_x(u, x1, x2) - t;
    if dx.abs() < 1e-7 {
      return u;
    }
    let ddx = bezier_x_deriv(u, x1, x2);
    if ddx.abs() < 1e-12 {
      break;
    }
    u -= dx / ddx;
    u = u.clamp(0.0, 1.0);
  }

  let mut lo = 0.0_f64;
  let mut hi = 1.0_f64;
  u = (lo + hi) * 0.5;
  for _ in 0..20 {
    let x = bezier_x(u, x1, x2);
    if (x - t).abs() < 1e-7 {
      return u;
    }
    if x < t {
      lo = u;
    } else {
      hi = u;
    }
    u = (lo + hi) * 0.5;
  }
  u
}

fn evaluate_steps(t: f64, count: u32, position: StepPosition) -> f64 {
  if t <= 0.0 {
    return 0.0;
  }
  if t >= 1.0 {
    return 1.0;
  }

  let steps = count as f64;
  let (intervals, offset) = match position {
    StepPosition::JumpStart => (steps, 1.0),
    StepPosition::JumpEnd => (steps, 0.0),
    StepPosition::JumpBoth => (steps + 1.0, 1.0),
    StepPosition::JumpNone => ((steps - 1.0).max(1.0), 0.0),
  };

  let current_step = (t * steps).floor() + offset;
  (current_step / intervals).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn linear_identity() {
    let e = Easing::Linear;
    assert_eq!(e.evaluate(0.0), 0.0);
    assert_eq!(e.evaluate(0.5), 0.5);
    assert_eq!(e.evaluate(1.0), 1.0);
  }

  #[test]
  fn ease_endpoints() {
    assert_eq!(Easing::EASE.evaluate(0.0), 0.0);
    assert_eq!(Easing::EASE.evaluate(1.0), 1.0);
  }

  #[test]
  fn ease_in_slow_start() {
    let v = Easing::EASE_IN.evaluate(0.25);
    assert!(v < 0.25, "ease-in at t=0.25 should be < 0.25, got {v}");
  }

  #[test]
  fn ease_out_fast_start() {
    let v = Easing::EASE_OUT.evaluate(0.25);
    assert!(v > 0.25, "ease-out at t=0.25 should be > 0.25, got {v}");
  }

  #[test]
  fn steps_jump_end() {
    let e = Easing::Steps {
      count: 4,
      position: StepPosition::JumpEnd,
    };
    assert_eq!(e.evaluate(0.0), 0.0);
    assert_eq!(e.evaluate(0.1), 0.0);
    assert!((e.evaluate(0.3) - 0.25).abs() < 1e-9);
    assert_eq!(e.evaluate(1.0), 1.0);
  }
}
