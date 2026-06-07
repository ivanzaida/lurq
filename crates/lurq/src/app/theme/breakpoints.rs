/// Named viewport-width breakpoints. Ordered smallest to largest so the derived
/// `Ord` matches the threshold cascade. "Base" (below `Sm`) is represented as
/// `None` when resolving a width.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, lurq::DevtoolsInspectable)]
pub enum Breakpoint {
  Sm,
  Md,
  Lg,
  Xl,
}

impl Breakpoint {
  pub const fn as_str(self) -> &'static str {
    match self {
      Self::Sm => "sm",
      Self::Md => "md",
      Self::Lg => "lg",
      Self::Xl => "xl",
    }
  }
}

/// Minimum logical-width thresholds (in logical pixels) for each named
/// breakpoint. Thresholds are expected to be non-decreasing.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ThemeBreakpoints {
  pub sm: f32,
  pub md: f32,
  pub lg: f32,
  pub xl: f32,
}

impl ThemeBreakpoints {
  pub fn new() -> Self {
    Self::default()
  }

  pub fn get(&self, breakpoint: Breakpoint) -> f32 {
    match breakpoint {
      Breakpoint::Sm => self.sm,
      Breakpoint::Md => self.md,
      Breakpoint::Lg => self.lg,
      Breakpoint::Xl => self.xl,
    }
  }

  pub fn set(&mut self, breakpoint: Breakpoint, value: f32) {
    match breakpoint {
      Breakpoint::Sm => self.sm = value,
      Breakpoint::Md => self.md = value,
      Breakpoint::Lg => self.lg = value,
      Breakpoint::Xl => self.xl = value,
    }
  }

  /// Largest breakpoint whose threshold is `<= width`. `None` means the width is
  /// below the smallest breakpoint (the implicit base tier).
  pub(crate) fn resolve(&self, width: f32) -> Option<Breakpoint> {
    if width >= self.xl {
      Some(Breakpoint::Xl)
    } else if width >= self.lg {
      Some(Breakpoint::Lg)
    } else if width >= self.md {
      Some(Breakpoint::Md)
    } else if width >= self.sm {
      Some(Breakpoint::Sm)
    } else {
      None
    }
  }
}

impl Default for ThemeBreakpoints {
  fn default() -> Self {
    Self {
      sm: 640.0,
      md: 768.0,
      lg: 1024.0,
      xl: 1280.0,
    }
  }
}
