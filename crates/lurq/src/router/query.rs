use std::{collections::BTreeMap, str::FromStr, sync::Arc};

/// Parsed query string: the `?key=value&...` portion of a navigated path.
///
/// Values are stored exactly as they appear in the path (no percent-decoding).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Query {
  inner: BTreeMap<Arc<str>, Arc<str>>,
}

impl Query {
  pub fn get(&self, name: &str) -> Option<&str> {
    self.inner.get(name).map(|v| v.as_ref())
  }

  pub fn get_parsed<T: FromStr>(&self, name: &str) -> Option<T> {
    self.get(name)?.parse().ok()
  }

  pub fn contains(&self, name: &str) -> bool {
    self.inner.contains_key(name)
  }

  pub fn entries(&self) -> impl Iterator<Item = (&str, &str)> {
    self.inner.iter().map(|(k, v)| (k.as_ref(), v.as_ref()))
  }

  pub fn is_empty(&self) -> bool {
    self.inner.is_empty()
  }

  pub fn len(&self) -> usize {
    self.inner.len()
  }

  pub(crate) fn from_path(path: &str) -> Self {
    let Some((_, after)) = path.split_once('?') else {
      return Self::default();
    };
    let after = after.split('#').next().unwrap_or(after);
    let mut inner = BTreeMap::new();
    for pair in after.split('&') {
      if pair.is_empty() {
        continue;
      }
      let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
      if key.is_empty() {
        continue;
      }
      inner.insert(Arc::from(key), Arc::from(value));
    }
    Self { inner }
  }
}
