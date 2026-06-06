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
