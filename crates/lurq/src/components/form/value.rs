use std::{collections::BTreeMap, sync::Arc};

use crate::app::component::{ComponentInfo, DevtoolsInspectable};

#[derive(Clone, Debug, PartialEq)]
pub enum FormValue {
  String(Arc<str>),
  Number(f64),
  Bool(bool),
}

impl FormValue {
  pub fn as_str(&self) -> Option<&str> {
    match self {
      Self::String(value) => Some(value),
      _ => None,
    }
  }

  pub fn as_number(&self) -> Option<f64> {
    match self {
      Self::Number(value) => Some(*value),
      _ => None,
    }
  }

  pub fn as_bool(&self) -> Option<bool> {
    match self {
      Self::Bool(value) => Some(*value),
      _ => None,
    }
  }

  pub(crate) fn to_string_value(&self) -> String {
    match self {
      Self::String(value) => value.to_string(),
      Self::Number(value) => value.to_string(),
      Self::Bool(value) => value.to_string(),
    }
  }
}

impl DevtoolsInspectable for FormValue {
  fn write_info(&self, buffer: &mut Vec<ComponentInfo>) {
    buffer.push(ComponentInfo::with_value(
      "value",
      std::any::type_name::<Self>(),
      match self {
        Self::String(value) => format!("{value:?}"),
        Self::Number(value) => value.to_string(),
        Self::Bool(value) => value.to_string(),
      },
    ));
  }
}

impl From<&str> for FormValue {
  fn from(value: &str) -> Self {
    Self::String(Arc::from(value))
  }
}

impl From<String> for FormValue {
  fn from(value: String) -> Self {
    Self::String(Arc::from(value))
  }
}

impl From<Arc<str>> for FormValue {
  fn from(value: Arc<str>) -> Self {
    Self::String(value)
  }
}

impl From<bool> for FormValue {
  fn from(value: bool) -> Self {
    Self::Bool(value)
  }
}

impl From<i32> for FormValue {
  fn from(value: i32) -> Self {
    Self::Number(value as f64)
  }
}

impl From<f64> for FormValue {
  fn from(value: f64) -> Self {
    Self::Number(value)
  }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct FormValues {
  fields: BTreeMap<Arc<str>, FormValue>,
}

impl FormValues {
  pub fn new() -> Self {
    <Self as Default>::default()
  }

  pub fn with(mut self, name: impl Into<Arc<str>>, value: impl Into<FormValue>) -> Self {
    self.set(name, value);
    self
  }

  pub fn set(&mut self, name: impl Into<Arc<str>>, value: impl Into<FormValue>) {
    self.fields.insert(name.into(), value.into());
  }

  pub fn get(&self, name: &str) -> Option<&FormValue> {
    self.fields.get(name)
  }

  pub(crate) fn contains(&self, name: &str) -> bool {
    self.fields.contains_key(name)
  }

  pub(crate) fn remove(&mut self, name: &str) {
    self.fields.remove(name);
  }

  pub fn get_string(&self, name: &str) -> Option<&str> {
    self.get(name).and_then(FormValue::as_str)
  }

  pub fn get_number(&self, name: &str) -> Option<f64> {
    self.get(name).and_then(FormValue::as_number)
  }

  pub fn get_bool(&self, name: &str) -> Option<bool> {
    self.get(name).and_then(FormValue::as_bool)
  }

  pub fn entries(&self) -> impl Iterator<Item = (&str, &FormValue)> {
    self.fields.iter().map(|(name, value)| (name.as_ref(), value))
  }

  pub fn len(&self) -> usize {
    self.fields.len()
  }

  pub fn is_empty(&self) -> bool {
    self.fields.is_empty()
  }
}

impl DevtoolsInspectable for FormValues {
  fn write_info(&self, buffer: &mut Vec<ComponentInfo>) {
    let children = self
      .entries()
      .map(|(name, value)| {
        ComponentInfo::with_value(
          "field",
          std::any::type_name::<FormValue>(),
          format!("{name}: {value:?}"),
        )
      })
      .collect();
    buffer.push(ComponentInfo::with_children(
      "values",
      std::any::type_name::<Self>(),
      children,
    ));
  }
}

impl<const N: usize> From<[(&str, FormValue); N]> for FormValues {
  fn from(values: [(&str, FormValue); N]) -> Self {
    values
      .into_iter()
      .fold(Self::new(), |values, (name, value)| values.with(name, value))
  }
}
