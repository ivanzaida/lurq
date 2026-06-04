use std::{fmt, sync::Arc};

use super::{
  FormValue, FormValues, ValidationResult,
  validation::{FieldValidator, ValidatorCallback},
};

#[derive(Clone, Default)]
pub struct FormOptions {
  pub(crate) defaults: FormValues,
  pub(crate) validators: Vec<FieldValidator>,
}

impl fmt::Debug for FormOptions {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_struct("FormOptions")
      .field("defaults", &self.defaults)
      .field("validators", &self.validators.len())
      .finish()
  }
}

impl PartialEq for FormOptions {
  fn eq(&self, other: &Self) -> bool {
    self.defaults == other.defaults
      && self.validators.len() == other.validators.len()
      && self
        .validators
        .iter()
        .map(|validator| validator.name.as_ref())
        .eq(other.validators.iter().map(|validator| validator.name.as_ref()))
  }
}

impl FormOptions {
  pub fn new() -> Self {
    <Self as Default>::default()
  }

  pub fn default(mut self, defaults: impl Into<FormValues>) -> Self {
    self.defaults = defaults.into();
    self
  }

  pub fn field(mut self, name: impl Into<Arc<str>>, value: impl Into<FormValue>) -> Self {
    self.defaults.set(name, value);
    self
  }

  pub fn validate(
    mut self,
    name: impl Into<Arc<str>>,
    validator: impl Fn(Option<&FormValue>, &FormValues) -> ValidationResult + Send + Sync + 'static,
  ) -> Self {
    let validate: ValidatorCallback = Arc::new(validator);
    self.validators.push(FieldValidator {
      name: name.into(),
      validate,
    });
    self
  }

  pub fn validate_string(
    self,
    name: impl Into<Arc<str>>,
    validator: impl Fn(&str, &FormValues) -> ValidationResult + Send + Sync + 'static,
  ) -> Self {
    self.validate(name, move |value, values| {
      validator(value.and_then(FormValue::as_str).unwrap_or_default(), values)
    })
  }

  pub fn validate_number(
    self,
    name: impl Into<Arc<str>>,
    validator: impl Fn(Option<f64>, &FormValues) -> ValidationResult + Send + Sync + 'static,
  ) -> Self {
    self.validate(name, move |value, values| {
      validator(value.and_then(FormValue::as_number), values)
    })
  }

  pub fn validate_bool(
    self,
    name: impl Into<Arc<str>>,
    validator: impl Fn(bool, &FormValues) -> ValidationResult + Send + Sync + 'static,
  ) -> Self {
    self.validate(name, move |value, values| {
      validator(value.and_then(FormValue::as_bool).unwrap_or(false), values)
    })
  }
}
