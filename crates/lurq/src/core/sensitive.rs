//! Values kept out of inspection: DevTools, the MCP tree and logs see a fixed marker instead.

use std::fmt;

use crate::app::component::{DevtoolsFormatter, DevtoolsInspectable};

/// What inspection surfaces show in place of a sensitive value: always the same three dots, so not even its length
/// is revealed.
pub const REDACTED: &str = "•••";

/// A value kept out of every inspection surface. Its `Debug` output and its DevTools rendering are [`REDACTED`], so
/// it can live in a signal, a store or component props without reaching DevTools' signal values and history, its
/// props inspector, or a log line that formats it with `{:?}`. It does not implement `Display`; read the value with
/// [`Sensitive::expose`] where the app really uses it.
///
/// Showing such a value on screen without it reaching the element tree's inspectors takes
/// [`Text::sensitive`](crate::components::Text::sensitive):
///
/// ```
/// use lurq::{components::Text, core::Sensitive};
///
/// let code = Sensitive::new("493 117".to_owned());
/// let shown = Text::new(code.expose()).sensitive();
/// assert_eq!(format!("{code:?}"), "Sensitive(•••)");
/// # let _ = shown;
/// ```
#[derive(Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Sensitive<T>(T);

impl<T> Sensitive<T> {
  pub fn new(value: T) -> Self {
    Self(value)
  }

  /// The value itself, for the code that uses it. Whatever is done with it from here is the app's to keep private.
  pub fn expose(&self) -> &T {
    &self.0
  }

  pub fn into_inner(self) -> T {
    self.0
  }
}

impl<T> From<T> for Sensitive<T> {
  fn from(value: T) -> Self {
    Self(value)
  }
}

impl<T> fmt::Debug for Sensitive<T> {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(formatter, "Sensitive({REDACTED})")
  }
}

impl<T> DevtoolsInspectable for Sensitive<T> {
  fn inspect(&self, formatter: &mut DevtoolsFormatter<'_>) {
    formatter.value(std::any::type_name::<Self>(), REDACTED);
  }
}
