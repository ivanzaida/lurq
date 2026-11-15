use std::{marker::PhantomData, sync::Arc};

use super::{FormField, FormHandle};
use crate::{
  app::component::{ComponentInfo, DevtoolsInspectable},
  core::signal::{Signal, SignalValue},
};

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct FormContext {
  form: FormHandle,
}

impl FormContext {
  pub(crate) fn new(form: FormHandle) -> Self {
    Self { form }
  }

  pub(crate) fn form(&self) -> FormHandle {
    self.form.clone()
  }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, crate::DevtoolsInspectable)]
pub enum ErrorVisibility {
  #[default]
  TouchedOrSubmitted,
  Always,
  Never,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, crate::DevtoolsInspectable)]
pub struct ControlOptions {
  pub error_visibility: ErrorVisibility,
  pub disabled: bool,
  pub required: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Control<T: SignalValue> {
  field: FormField<T>,
  options: ControlOptions,
  marker: PhantomData<T>,
}

impl<T: SignalValue> Control<T> {
  pub(crate) fn new(field: FormField<T>) -> Self {
    Self {
      field,
      options: ControlOptions::default(),
      marker: PhantomData,
    }
  }

  pub fn field(&self) -> &FormField<T> {
    &self.field
  }

  pub fn name(&self) -> &str {
    self.field.name()
  }

  pub fn form(&self) -> FormHandle {
    self.field.form()
  }

  pub fn error_visibility(mut self, visibility: ErrorVisibility) -> Self {
    self.options.error_visibility = visibility;
    self
  }

  pub fn disabled(mut self, disabled: bool) -> Self {
    self.options.disabled = disabled;
    self
  }

  pub fn required(mut self, required: bool) -> Self {
    self.options.required = required;
    self
  }

  pub fn options(&self) -> ControlOptions {
    self.options
  }

  pub(crate) fn resolve(&self) -> ResolvedControl<T> {
    ResolvedControl {
      name: Arc::from(self.field.name()),
      value: self.field.value(),
      error: self.field.error(),
      touched: self.field.touched(),
      dirty: self.field.dirty(),
      submit_attempted: self.field.submit_attempted(),
      submitting: self.field.form().submitting(),
      form: self.field.form(),
      options: self.options,
    }
  }
}

impl<T: SignalValue> DevtoolsInspectable for Control<T> {
  fn write_info(&self, buffer: &mut Vec<ComponentInfo>) {
    buffer.push(ComponentInfo::with_value(
      "name",
      std::any::type_name::<Arc<str>>(),
      self.name().to_owned(),
    ));
    self.options.write_info(buffer);
  }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, crate::DevtoolsInspectable)]
pub struct ControlState {
  pub touched: bool,
  pub dirty: bool,
  pub submit_attempted: bool,
  pub submitting: bool,
  pub has_error: bool,
  pub disabled: bool,
  pub required: bool,
}

#[derive(Clone, Debug)]
pub struct ResolvedControl<T: SignalValue> {
  name: Arc<str>,
  value: Signal<T>,
  error: Signal<Option<Arc<str>>>,
  touched: Signal<bool>,
  dirty: Signal<bool>,
  submit_attempted: Signal<bool>,
  submitting: Signal<bool>,
  form: FormHandle,
  options: ControlOptions,
}

impl<T> ResolvedControl<T>
where
  T: SignalValue + Clone + PartialEq + Send + Sync + 'static,
{
  pub fn name(&self) -> &str {
    &self.name
  }

  pub fn value(&self) -> Signal<T> {
    self.value.clone()
  }

  pub fn error(&self) -> Signal<Option<Arc<str>>> {
    self.error.clone()
  }

  pub fn touched(&self) -> Signal<bool> {
    self.touched.clone()
  }

  pub fn dirty(&self) -> Signal<bool> {
    self.dirty.clone()
  }

  pub fn submit_attempted(&self) -> Signal<bool> {
    self.submit_attempted.clone()
  }

  pub fn submitting(&self) -> Signal<bool> {
    self.submitting.clone()
  }

  pub fn state(&self) -> ControlState {
    ControlState {
      touched: self.is_touched(),
      dirty: self.is_dirty(),
      submit_attempted: self.has_submit_attempted(),
      submitting: self.is_submitting(),
      has_error: self.error.get().is_some(),
      disabled: self.is_disabled(),
      required: self.is_required(),
    }
  }

  pub fn visible_error(&self) -> Option<Arc<str>> {
    let error = self.error.get();
    let should_show = match self.options.error_visibility {
      ErrorVisibility::TouchedOrSubmitted => self.is_touched() || self.has_submit_attempted(),
      ErrorVisibility::Always => true,
      ErrorVisibility::Never => false,
    };
    if should_show { error } else { None }
  }

  pub fn should_show_error(&self) -> bool {
    self.visible_error().is_some()
  }

  pub fn is_invalid(&self) -> bool {
    self.error.get().is_some()
  }

  pub fn is_touched(&self) -> bool {
    self.touched.get()
  }

  pub fn is_dirty(&self) -> bool {
    self.dirty.get()
  }

  pub fn has_submit_attempted(&self) -> bool {
    self.submit_attempted.get()
  }

  pub fn is_submitting(&self) -> bool {
    self.submitting.get()
  }

  pub fn is_disabled(&self) -> bool {
    self.options.disabled || self.is_submitting()
  }

  pub fn is_required(&self) -> bool {
    self.options.required
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

  pub fn on_blur(&self) -> impl Fn() + Send + Sync + 'static {
    let control = self.clone();
    move || control.mark_touched()
  }
}
