use std::{
  collections::HashMap,
  fmt,
  ops::Deref,
  sync::{Arc, LazyLock},
};

use parking_lot::RwLock;

#[derive(Default)]
struct Names {
  ids: HashMap<&'static str, u32>,
  names: Vec<&'static str>,
}

static NAMES: LazyLock<RwLock<Names>> = LazyLock::new(Default::default);

/// The interned name of an application-defined theme role, carried by the
/// `Extra` variants of [`SpacingSize`](super::SpacingSize),
/// [`RadiusSize`](super::RadiusSize), [`BorderSize`](super::BorderSize),
/// [`TypographyStyle`](super::TypographyStyle) and [`ShadowStyle`](super::ShadowStyle).
///
/// Those roles are `Copy`, nest in `Copy` values such as `Padding`, and are
/// stored many times in every element, so the name is a 4-byte handle into a
/// process-wide table rather than a string: each distinct name is stored once
/// for the life of the process. Role names are expected to be a small, fixed
/// vocabulary, not per-item data.
///
/// A `RoleName` dereferences to `str` and compares equal to a `&str`.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct RoleName(u32);

impl RoleName {
  /// Interns `name`; the same name always gives the same handle.
  pub fn new(name: impl AsRef<str>) -> Self {
    let name = name.as_ref();
    if let Some(&id) = NAMES.read().ids.get(name) {
      return Self(id);
    }
    let mut names = NAMES.write();
    if let Some(&id) = names.ids.get(name) {
      return Self(id);
    }
    let id = u32::try_from(names.names.len()).expect("too many interned theme role names");
    let interned: &'static str = Box::leak(name.into());
    names.names.push(interned);
    names.ids.insert(interned, id);
    Self(id)
  }

  pub fn as_str(self) -> &'static str {
    NAMES.read().names[self.0 as usize]
  }
}

impl Deref for RoleName {
  type Target = str;

  fn deref(&self) -> &str {
    self.as_str()
  }
}

impl AsRef<str> for RoleName {
  fn as_ref(&self) -> &str {
    self.as_str()
  }
}

impl fmt::Debug for RoleName {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    fmt::Debug::fmt(self.as_str(), f)
  }
}

impl fmt::Display for RoleName {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str(self.as_str())
  }
}

impl PartialEq<str> for RoleName {
  fn eq(&self, other: &str) -> bool {
    self.as_str() == other
  }
}

impl PartialEq<&str> for RoleName {
  fn eq(&self, other: &&str) -> bool {
    self.as_str() == *other
  }
}

impl PartialEq<RoleName> for str {
  fn eq(&self, other: &RoleName) -> bool {
    self == other.as_str()
  }
}

impl PartialEq<RoleName> for &str {
  fn eq(&self, other: &RoleName) -> bool {
    *self == other.as_str()
  }
}

impl From<&str> for RoleName {
  fn from(name: &str) -> Self {
    Self::new(name)
  }
}

impl From<String> for RoleName {
  fn from(name: String) -> Self {
    Self::new(name)
  }
}

impl From<Arc<str>> for RoleName {
  fn from(name: Arc<str>) -> Self {
    Self::new(name)
  }
}

impl From<RoleName> for Arc<str> {
  fn from(name: RoleName) -> Self {
    Arc::from(name.as_str())
  }
}

#[cfg(test)]
mod tests {
  use super::RoleName;

  #[test]
  fn interning_gives_one_handle_per_name() {
    let card = RoleName::new("role-name-test-card");
    assert_eq!(card, RoleName::new(String::from("role-name-test-card")));
    assert_ne!(card, RoleName::new("role-name-test-panel"));
    assert_eq!(card.as_str(), "role-name-test-card");
    assert_eq!(card, "role-name-test-card");
    assert_eq!(
      format!("{card:?} {card}"),
      "\"role-name-test-card\" role-name-test-card"
    );
  }
}
