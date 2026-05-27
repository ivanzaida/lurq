use std::sync::{Arc, Mutex};

use crate::{core::Signal, layout::text_style::TextStyle};

pub(crate) enum NodeKind {
  Empty,
  Text { style: TextStyle },
  TextInput { state: TextInputState, style: TextStyle },
  Checkbox { state: CheckboxState },
  Slider { state: SliderState },
}

#[derive(Clone)]
pub(crate) struct TextInputState {
  value: Signal<String>,
  inner: Arc<Mutex<TextInputInner>>,
}

struct TextInputInner {
  placeholder: Option<String>,
  caret: usize,
  caret_x: f32,
  focused: bool,
}

impl TextInputState {
  pub(crate) fn new(value: Signal<String>) -> Self {
    let caret = value.get_untracked().len();
    Self {
      value,
      inner: Arc::new(Mutex::new(TextInputInner {
        placeholder: None,
        caret,
        caret_x: 0.0,
        focused: false,
      })),
    }
  }

  pub(crate) fn value(&self) -> String {
    self.value.get_untracked()
  }

  pub(crate) fn set_placeholder(&self, placeholder: impl Into<String>) {
    self.inner.lock().unwrap().placeholder = Some(placeholder.into());
  }

  pub(crate) fn placeholder(&self) -> Option<String> {
    self.inner.lock().unwrap().placeholder.clone()
  }

  pub(crate) fn rendered_text(&self) -> Option<String> {
    let value = self.value();
    if value.is_empty() {
      self.placeholder()
    } else {
      Some(value)
    }
  }

  pub(crate) fn insert(&self, text: &str) {
    if text.is_empty() {
      return;
    }

    let mut caret = self.inner.lock().unwrap().caret;
    self.value.update(|value| {
      caret = clamp_to_char_boundary(value, caret);
      value.insert_str(caret, text);
      caret += text.len();
    });
    self.inner.lock().unwrap().caret = caret;
  }

  pub(crate) fn backspace(&self) {
    let mut caret = self.inner.lock().unwrap().caret;
    if caret == 0 {
      return;
    }

    self.value.update(|value| {
      caret = clamp_to_char_boundary(value, caret);
      if caret > 0 {
        let previous = previous_char_boundary(value, caret);
        value.replace_range(previous..caret, "");
        caret = previous;
      }
    });
    self.inner.lock().unwrap().caret = caret;
  }

  pub(crate) fn delete(&self) {
    let mut caret = self.inner.lock().unwrap().caret;
    self.value.update(|value| {
      caret = clamp_to_char_boundary(value, caret);
      if caret < value.len() {
        let next = next_char_boundary(value, caret);
        value.replace_range(caret..next, "");
      }
    });
    self.inner.lock().unwrap().caret = caret;
  }

  pub(crate) fn move_left(&self) {
    let value = self.value();
    let mut inner = self.inner.lock().unwrap();
    inner.caret = previous_char_boundary(&value, clamp_to_char_boundary(&value, inner.caret));
  }

  pub(crate) fn move_right(&self) {
    let value = self.value();
    let mut inner = self.inner.lock().unwrap();
    inner.caret = next_char_boundary(&value, clamp_to_char_boundary(&value, inner.caret));
  }

  pub(crate) fn move_home(&self) {
    self.inner.lock().unwrap().caret = 0;
  }

  pub(crate) fn move_end(&self) {
    self.inner.lock().unwrap().caret = self.value().len();
  }

  pub(crate) fn copy_runtime_state_from(&self, old: &Self) {
    let old_inner = old.inner.lock().unwrap();
    let old_caret = old_inner.caret;
    let old_caret_x = old_inner.caret_x;
    let old_focused = old_inner.focused;
    let len = self.value().len();
    let mut inner = self.inner.lock().unwrap();
    inner.caret = old_caret.min(len);
    inner.caret_x = old_caret_x;
    inner.focused = old_focused;
  }

  pub(crate) fn caret_prefix(&self) -> String {
    let value = self.value();
    let caret = clamp_to_char_boundary(&value, self.inner.lock().unwrap().caret);
    value[..caret].to_owned()
  }

  pub(crate) fn set_caret_x(&self, caret_x: f32) {
    self.inner.lock().unwrap().caret_x = caret_x;
  }

  pub(crate) fn caret_x(&self) -> f32 {
    self.inner.lock().unwrap().caret_x
  }

  pub(crate) fn set_focused(&self, focused: bool) {
    self.inner.lock().unwrap().focused = focused;
  }

  pub(crate) fn is_focused(&self) -> bool {
    self.inner.lock().unwrap().focused
  }
}

fn clamp_to_char_boundary(value: &str, index: usize) -> usize {
  let mut index = index.min(value.len());
  while index > 0 && !value.is_char_boundary(index) {
    index -= 1;
  }
  index
}

fn previous_char_boundary(value: &str, index: usize) -> usize {
  let index = clamp_to_char_boundary(value, index);
  value[..index].char_indices().last().map(|(idx, _)| idx).unwrap_or(0)
}

fn next_char_boundary(value: &str, index: usize) -> usize {
  let index = clamp_to_char_boundary(value, index);
  value[index..]
    .char_indices()
    .nth(1)
    .map(|(offset, _)| index + offset)
    .unwrap_or(value.len())
}

#[derive(Clone)]
pub(crate) struct CheckboxState {
  value: Signal<bool>,
}

impl CheckboxState {
  pub(crate) fn new(value: Signal<bool>) -> Self {
    Self { value }
  }

  pub(crate) fn is_checked(&self) -> bool {
    self.value.get_untracked()
  }

  pub(crate) fn toggle(&self) {
    self.value.update(|checked| *checked = !*checked);
  }
}

#[derive(Clone)]
pub(crate) struct SliderState {
  value: Signal<f32>,
  inner: Arc<Mutex<SliderInner>>,
}

struct SliderInner {
  min: f32,
  max: f32,
}

impl SliderState {
  pub(crate) fn new(value: Signal<f32>) -> Self {
    Self {
      value,
      inner: Arc::new(Mutex::new(SliderInner { min: 0.0, max: 1.0 })),
    }
  }

  pub(crate) fn value(&self) -> f32 {
    self.value.get_untracked()
  }

  pub(crate) fn ratio(&self) -> f32 {
    let inner = self.inner.lock().unwrap();
    if inner.max <= inner.min {
      return 0.0;
    }
    ((self.value() - inner.min) / (inner.max - inner.min)).clamp(0.0, 1.0)
  }

  pub(crate) fn set_range(&self, min: f32, max: f32) {
    let mut inner = self.inner.lock().unwrap();
    inner.min = min;
    inner.max = max.max(min);
    let current = self.value();
    self.value.set(current.clamp(inner.min, inner.max));
  }

  pub(crate) fn set_from_ratio(&self, ratio: f32) {
    let inner = self.inner.lock().unwrap();
    let value = inner.min + ratio.clamp(0.0, 1.0) * (inner.max - inner.min);
    self.value.set(value);
  }

  pub(crate) fn nudge(&self, delta: f32) {
    let inner = self.inner.lock().unwrap();
    let current = self.value();
    self.value.set((current + delta).clamp(inner.min, inner.max));
  }
}
