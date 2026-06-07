use std::collections::BTreeMap;

use crate::app::theme::Breakpoint;

/// A value that varies by viewport breakpoint.
///
/// Resolution is mobile-first: for a given breakpoint, the value is the one set
/// at that breakpoint or, if unset, the nearest smaller breakpoint that is set,
/// falling back to `base`.
///
/// ```ignore
/// let columns = Responsive::new(1).md(2).lg(3).xl(4);
/// let current = ctx.responsive(&columns);
/// ```
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Responsive<T> {
  base: T,
  overrides: BTreeMap<Breakpoint, T>,
}

impl<T> Responsive<T> {
  pub fn new(base: T) -> Self {
    Self {
      base,
      overrides: BTreeMap::new(),
    }
  }

  pub fn at(mut self, breakpoint: Breakpoint, value: T) -> Self {
    self.overrides.insert(breakpoint, value);
    self
  }

  pub fn sm(self, value: T) -> Self {
    self.at(Breakpoint::Sm, value)
  }

  pub fn md(self, value: T) -> Self {
    self.at(Breakpoint::Md, value)
  }

  pub fn lg(self, value: T) -> Self {
    self.at(Breakpoint::Lg, value)
  }

  pub fn xl(self, value: T) -> Self {
    self.at(Breakpoint::Xl, value)
  }

  /// Resolve the value for the current breakpoint (`None` = base tier).
  pub fn resolve(&self, breakpoint: Option<Breakpoint>) -> &T {
    match breakpoint {
      Some(breakpoint) => self
        .overrides
        .range(..=breakpoint)
        .next_back()
        .map(|(_, value)| value)
        .unwrap_or(&self.base),
      None => &self.base,
    }
  }
}

impl<T> From<T> for Responsive<T> {
  fn from(base: T) -> Self {
    Self::new(base)
  }
}
