use std::sync::Arc;

use super::{FormValues, ValidationResult};

pub fn required(message: impl Into<Arc<str>>) -> impl Fn(&str, &FormValues) -> ValidationResult {
  let message = message.into();
  move |value, _| {
    if value.trim().is_empty() {
      ValidationResult::invalid(message.clone())
    } else {
      ValidationResult::valid()
    }
  }
}

pub fn min_len(min: usize, message: impl Into<Arc<str>>) -> impl Fn(&str, &FormValues) -> ValidationResult {
  let message = message.into();
  move |value, _| {
    if value.chars().count() < min {
      ValidationResult::invalid(message.clone())
    } else {
      ValidationResult::valid()
    }
  }
}

pub fn max_len(max: usize, message: impl Into<Arc<str>>) -> impl Fn(&str, &FormValues) -> ValidationResult {
  let message = message.into();
  move |value, _| {
    if value.chars().count() > max {
      ValidationResult::invalid(message.clone())
    } else {
      ValidationResult::valid()
    }
  }
}

pub fn email(message: impl Into<Arc<str>>) -> impl Fn(&str, &FormValues) -> ValidationResult {
  let message = message.into();
  move |value, _| {
    let value = value.trim();
    let at = value.find('@');
    if value.len() >= 3 && at.is_some_and(|index| index > 0 && value[index + 1..].contains('.')) {
      ValidationResult::valid()
    } else {
      ValidationResult::invalid(message.clone())
    }
  }
}

pub fn range(
  min: f64,
  max: f64,
  message: impl Into<Arc<str>>,
) -> impl Fn(Option<f64>, &FormValues) -> ValidationResult {
  let message = message.into();
  move |value, _| match value {
    Some(value) if value >= min && value <= max => ValidationResult::valid(),
    _ => ValidationResult::invalid(message.clone()),
  }
}
