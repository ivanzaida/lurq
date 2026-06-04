use std::{collections::BTreeMap, sync::Arc};

use super::{FormValue, FormValues};
use crate::app::component::{ComponentInfo, DevtoolsInspectable};

pub(crate) type InvalidCallback = Arc<dyn Fn(FormErrors) + Send + Sync>;
pub(crate) type ValidatorCallback = Arc<dyn Fn(Option<&FormValue>, &FormValues) -> ValidationResult + Send + Sync>;

#[derive(Clone)]
pub(crate) struct FieldValidator {
  pub(crate) name: Arc<str>,
  pub(crate) validate: ValidatorCallback,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ValidationResult {
  Valid,
  Invalid(Arc<str>),
}

impl ValidationResult {
  pub fn valid() -> Self {
    Self::Valid
  }

  pub fn invalid(message: impl Into<Arc<str>>) -> Self {
    Self::Invalid(message.into())
  }

  pub fn is_valid(&self) -> bool {
    matches!(self, Self::Valid)
  }
}

impl DevtoolsInspectable for ValidationResult {
  fn write_info(&self, buffer: &mut Vec<ComponentInfo>) {
    match self {
      Self::Valid => buffer.push(ComponentInfo::with_value(
        "value",
        std::any::type_name::<Self>(),
        "Valid",
      )),
      Self::Invalid(message) => buffer.push(ComponentInfo::with_value(
        "value",
        std::any::type_name::<Self>(),
        format!("Invalid({message:?})"),
      )),
    }
  }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FormErrors {
  fields: BTreeMap<Arc<str>, Vec<Arc<str>>>,
}

impl FormErrors {
  pub fn new() -> Self {
    <Self as Default>::default()
  }

  pub fn get(&self, name: &str) -> Option<&[Arc<str>]> {
    self.fields.get(name).map(Vec::as_slice)
  }

  pub fn with(mut self, name: impl Into<Arc<str>>, message: impl Into<Arc<str>>) -> Self {
    self.push(name, message);
    self
  }

  pub fn with_messages(
    mut self,
    name: impl Into<Arc<str>>,
    messages: impl IntoIterator<Item = impl Into<Arc<str>>>,
  ) -> Self {
    self.set_messages(name, messages);
    self
  }

  pub fn first(&self, name: &str) -> Option<&str> {
    self.get(name).and_then(|errors| errors.first()).map(Arc::as_ref)
  }

  pub fn entries(&self) -> impl Iterator<Item = (&str, &[Arc<str>])> {
    self
      .fields
      .iter()
      .map(|(name, errors)| (name.as_ref(), errors.as_slice()))
  }

  pub fn len(&self) -> usize {
    self.fields.len()
  }

  pub fn message_count(&self) -> usize {
    self.fields.values().map(Vec::len).sum()
  }

  pub fn is_empty(&self) -> bool {
    self.fields.is_empty()
  }

  pub fn push(&mut self, name: impl Into<Arc<str>>, message: impl Into<Arc<str>>) {
    self.fields.entry(name.into()).or_default().push(message.into());
  }

  pub fn set_messages(&mut self, name: impl Into<Arc<str>>, messages: impl IntoIterator<Item = impl Into<Arc<str>>>) {
    self
      .fields
      .insert(name.into(), messages.into_iter().map(Into::into).collect());
  }

  pub(crate) fn set_one(&mut self, name: Arc<str>, message: Arc<str>) {
    self.fields.insert(name, vec![message]);
  }

  pub(crate) fn remove(&mut self, name: &str) {
    self.fields.remove(name);
  }

  pub(crate) fn first_cloned(&self, name: &str) -> Option<Arc<str>> {
    self.fields.get(name).and_then(|errors| errors.first()).cloned()
  }
}

impl DevtoolsInspectable for FormErrors {
  fn write_info(&self, buffer: &mut Vec<ComponentInfo>) {
    let children = self
      .entries()
      .map(|(name, errors)| {
        ComponentInfo::with_value(
          "field",
          std::any::type_name::<Vec<Arc<str>>>(),
          format!(
            "{name}: {}",
            errors.iter().map(Arc::as_ref).collect::<Vec<_>>().join(", ")
          ),
        )
      })
      .collect();
    buffer.push(ComponentInfo::with_children(
      "errors",
      std::any::type_name::<Self>(),
      children,
    ));
  }
}
