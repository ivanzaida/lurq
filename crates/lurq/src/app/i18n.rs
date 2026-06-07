use std::{
  collections::HashMap,
  sync::{Arc, RwLock},
};

use crate::core::Signal;

const DEFAULT_LOCALE: &str = "en";
const DEFAULT_NAMESPACE: &str = "translation";

#[derive(Clone)]
pub struct I18n {
  inner: Arc<RwLock<I18nInner>>,
  version_signal: Signal<u64>,
}

struct I18nInner {
  locale: Arc<str>,
  fallback_locale: Arc<str>,
  resources: HashMap<ResourceKey, HashMap<Arc<str>, Arc<str>>>,
  version: u64,
}

#[derive(Clone, PartialEq, Eq, Hash)]
struct ResourceKey {
  locale: Arc<str>,
  namespace: Arc<str>,
}

impl Default for I18n {
  fn default() -> Self {
    Self::new()
  }
}

impl I18n {
  pub fn new() -> Self {
    Self {
      inner: Arc::new(RwLock::new(I18nInner {
        locale: Arc::from(DEFAULT_LOCALE),
        fallback_locale: Arc::from(DEFAULT_LOCALE),
        resources: HashMap::new(),
        version: 0,
      })),
      version_signal: Signal::new(0),
    }
  }

  pub fn locale(&self) -> String {
    self.track_access();
    self.inner.read().unwrap().locale.to_string()
  }

  pub fn set_locale(&self, locale: impl Into<Arc<str>>) {
    let locale = locale.into();
    let mut inner = self.inner.write().unwrap();
    if inner.locale == locale {
      return;
    }
    inner.locale = locale;
    self.bump_version(&mut inner);
  }

  pub fn fallback_locale(&self) -> String {
    self.track_access();
    self.inner.read().unwrap().fallback_locale.to_string()
  }

  pub fn set_fallback_locale(&self, locale: impl Into<Arc<str>>) {
    let locale = locale.into();
    let mut inner = self.inner.write().unwrap();
    if inner.fallback_locale == locale {
      return;
    }
    inner.fallback_locale = locale;
    self.bump_version(&mut inner);
  }

  pub fn add_resource(
    &self,
    locale: impl Into<Arc<str>>,
    namespace: impl Into<Arc<str>>,
    key: impl Into<Arc<str>>,
    value: impl Into<Arc<str>>,
  ) {
    let mut inner = self.inner.write().unwrap();
    let namespace = ResourceKey {
      locale: locale.into(),
      namespace: namespace.into(),
    };
    inner
      .resources
      .entry(namespace)
      .or_default()
      .insert(key.into(), value.into());
    self.bump_version(&mut inner);
  }

  pub fn add_resources<I, K, V>(&self, locale: impl Into<Arc<str>>, namespace: impl Into<Arc<str>>, values: I)
  where
    I: IntoIterator<Item = (K, V)>,
    K: Into<Arc<str>>,
    V: Into<Arc<str>>,
  {
    let mut inner = self.inner.write().unwrap();
    let namespace = ResourceKey {
      locale: locale.into(),
      namespace: namespace.into(),
    };
    let entries = inner.resources.entry(namespace).or_default();
    for (key, value) in values {
      entries.insert(key.into(), value.into());
    }
    self.bump_version(&mut inner);
  }

  #[cfg(feature = "serde")]
  pub fn add_resources_json(
    &self,
    locale: impl Into<Arc<str>>,
    namespace: impl Into<Arc<str>>,
    path: impl AsRef<std::path::Path>,
  ) -> Result<(), LoadJsonError> {
    let contents = std::fs::read_to_string(path)?;
    let value: serde_json::Value = serde_json::from_str(&contents)?;
    let mut entries = Vec::new();
    flatten_json(&value, &mut String::new(), &mut entries);
    self.add_resources(locale, namespace, entries);
    Ok(())
  }

  pub fn t(&self, key: &str) -> Arc<str> {
    self.t_ns(DEFAULT_NAMESPACE, key)
  }

  pub fn t_ns(&self, namespace: &str, key: &str) -> Arc<str> {
    self.t_ns_args(namespace, key, std::iter::empty::<(&str, &str)>())
  }

  pub fn t_args<I, K, V>(&self, key: &str, args: I) -> Arc<str>
  where
    I: IntoIterator<Item = (K, V)>,
    K: AsRef<str>,
    V: ToString,
  {
    self.t_ns_args(DEFAULT_NAMESPACE, key, args)
  }

  pub fn t_ns_args<I, K, V>(&self, namespace: &str, key: &str, args: I) -> Arc<str>
  where
    I: IntoIterator<Item = (K, V)>,
    K: AsRef<str>,
    V: ToString,
  {
    self.track_access();
    let args = args
      .into_iter()
      .map(|(key, value)| (key.as_ref().to_owned(), value.to_string()))
      .collect::<Vec<_>>();
    let inner = self.inner.read().unwrap();
    let value = inner
      .lookup(&inner.locale, namespace, key)
      .or_else(|| inner.lookup(&inner.fallback_locale, namespace, key))
      .unwrap_or_else(|| Arc::from(key));
    if args.is_empty() {
      return value;
    }
    interpolate(&value, &args)
  }

  pub(crate) fn track_access(&self) {
    let _ = self.version_signal.get();
  }

  fn bump_version(&self, inner: &mut I18nInner) {
    inner.version = inner.version.wrapping_add(1);
    self.version_signal.set(inner.version);
  }
}

impl I18nInner {
  fn lookup(&self, locale: &Arc<str>, namespace: &str, key: &str) -> Option<Arc<str>> {
    let namespace = ResourceKey {
      locale: locale.clone(),
      namespace: Arc::from(namespace),
    };
    self.resources.get(&namespace)?.get(key).cloned()
  }
}

#[cfg(feature = "serde")]
fn flatten_json(value: &serde_json::Value, prefix: &mut String, out: &mut Vec<(String, String)>) {
  match value {
    serde_json::Value::Object(map) => {
      for (key, child) in map {
        let len = prefix.len();
        if !prefix.is_empty() {
          prefix.push('.');
        }
        prefix.push_str(key);
        flatten_json(child, prefix, out);
        prefix.truncate(len);
      }
    }
    serde_json::Value::String(s) => out.push((prefix.clone(), s.clone())),
    other => out.push((prefix.clone(), other.to_string())),
  }
}

#[cfg(feature = "serde")]
#[derive(Debug)]
pub enum LoadJsonError {
  Io(std::io::Error),
  Parse(serde_json::Error),
}

#[cfg(feature = "serde")]
impl std::fmt::Display for LoadJsonError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      LoadJsonError::Io(err) => write!(f, "failed to read i18n resource file: {err}"),
      LoadJsonError::Parse(err) => write!(f, "failed to parse i18n resource JSON: {err}"),
    }
  }
}

#[cfg(feature = "serde")]
impl std::error::Error for LoadJsonError {
  fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
    match self {
      LoadJsonError::Io(err) => Some(err),
      LoadJsonError::Parse(err) => Some(err),
    }
  }
}

#[cfg(feature = "serde")]
impl From<std::io::Error> for LoadJsonError {
  fn from(err: std::io::Error) -> Self {
    LoadJsonError::Io(err)
  }
}

#[cfg(feature = "serde")]
impl From<serde_json::Error> for LoadJsonError {
  fn from(err: serde_json::Error) -> Self {
    LoadJsonError::Parse(err)
  }
}

fn interpolate(value: &str, args: &[(String, String)]) -> Arc<str> {
  let mut value = value.to_owned();
  for (key, replacement) in args {
    value = value.replace(&format!("{{{{{key}}}}}"), replacement);
  }
  Arc::from(value)
}

#[cfg(test)]
mod tests {
  use super::I18n;

  #[test]
  fn resolves_default_namespace_key() {
    let i18n = I18n::new();
    i18n.add_resource("en", "translation", "hello", "Hello");

    assert_eq!(&*i18n.t("hello"), "Hello");
  }

  #[test]
  fn falls_back_to_fallback_locale() {
    let i18n = I18n::new();
    i18n.set_locale("uk");
    i18n.add_resource("en", "translation", "hello", "Hello");

    assert_eq!(&*i18n.t("hello"), "Hello");
  }

  #[test]
  fn interpolates_named_args() {
    let i18n = I18n::new();
    i18n.add_resource("en", "translation", "hello", "Hello, {{name}}");

    assert_eq!(&*i18n.t_args("hello", [("name", "Ada")]), "Hello, Ada");
  }
}
