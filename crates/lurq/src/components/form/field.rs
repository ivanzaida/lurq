use std::{fmt, sync::Arc};

use super::FormHandle;
use crate::core::signal::{Signal, SignalValue};

#[derive(Clone)]
pub struct FormField<T: SignalValue> {
  name: Arc<str>,
  value: Signal<T>,
  form: FormHandle,
}

impl<T: SignalValue> FormField<T> {
  pub(crate) fn new(name: Arc<str>, value: Signal<T>, form: FormHandle) -> Self {
    Self { name, value, form }
  }

  pub fn name(&self) -> &str {
    &self.name
  }

  pub fn value(&self) -> Signal<T> {
    self.value.clone()
  }

  pub fn form(&self) -> FormHandle {
    self.form.clone()
  }

  pub fn error(&self) -> Signal<Option<Arc<str>>> {
    self.form.error(self.name.clone())
  }

  pub fn touched(&self) -> Signal<bool> {
    self.form.touched(self.name.clone())
  }

  pub fn submit_attempted(&self) -> Signal<bool> {
    self.form.submit_attempted()
  }

  pub fn dirty(&self) -> Signal<bool> {
    self.form.dirty(self.name.clone())
  }

  pub fn is_touched(&self) -> bool {
    self.form.is_field_touched(&self.name)
  }

  pub fn is_dirty(&self) -> bool {
    self.form.is_field_dirty(&self.name)
  }

  pub fn mark_touched(&self) {
    self.form.mark_touched(self.name.clone());
  }

  pub fn clear_touched(&self) {
    self.form.clear_touched(&self.name);
  }

  pub fn validate(&self) -> bool {
    self.form.validate_field(&self.name)
  }

  pub fn reset(&self) {
    self.form.reset_field(&self.name);
  }
}

impl<T: SignalValue> fmt::Debug for FormField<T> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_struct("FormField").field("name", &self.name).finish()
  }
}

impl<T: SignalValue> PartialEq for FormField<T> {
  fn eq(&self, other: &Self) -> bool {
    self.name == other.name && self.form == other.form
  }
}
