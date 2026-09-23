use std::{
  collections::HashSet,
  sync::{LazyLock, Mutex},
};

static NAMES: LazyLock<Mutex<HashSet<&'static str>>> = LazyLock::new(Default::default);

/// Interns an extra role name for the life of the process. Size and typography
/// roles are `Copy` and nest in `Copy` values such as `Padding`, so they carry a
/// `&'static str` rather than an `Arc<str>`. Each distinct name is stored once;
/// role names are expected to be a small, fixed vocabulary.
pub(super) fn intern(name: &str) -> &'static str {
  let mut names = NAMES.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
  if let Some(interned) = names.get(name) {
    return interned;
  }
  let interned: &'static str = Box::leak(name.into());
  names.insert(interned);
  interned
}
