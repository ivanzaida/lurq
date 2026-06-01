use std::sync::{Arc, Mutex};

use crate::{core::Signal, layout::text_style::TextStyle};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TextInputOverflow {
  Multiline,
  #[default]
  Scroll,
}

#[derive(Clone)]
pub(crate) enum NodeKind {
  Empty,
  Text {
    style: TextStyle,
  },
  TextInput {
    state: TextInputState,
    style: TextStyle,
  },
  Checkbox {
    state: CheckboxState,
  },
  Slider {
    state: SliderState,
  },
  #[cfg(feature = "image")]
  Image {
    data: crate::images::ImageData,
  },
  #[cfg(feature = "image")]
  ResourceImage {
    path: Arc<str>,
  },
  #[cfg(feature = "svg")]
  Svg {
    data: crate::svg::SvgData,
  },
  #[cfg(all(feature = "svg", feature = "resources"))]
  ResourceSvg {
    path: Arc<str>,
  },
}

#[derive(Clone)]
pub(crate) struct TextInputState {
  value: Signal<String>,
  inner: Arc<Mutex<TextInputInner>>,
}

struct TextInputInner {
  placeholder: Option<String>,
  caret: usize,
  selection_anchor: Option<usize>,
  caret_x: f32,
  caret_y: f32,
  caret_height: f32,
  caret_positions: Vec<(usize, f32)>,
  scroll_x: f32,
  overflow: TextInputOverflow,
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
        selection_anchor: None,
        caret_x: 0.0,
        caret_y: 0.0,
        caret_height: 0.0,
        caret_positions: vec![(0, 0.0)],
        scroll_x: 0.0,
        overflow: TextInputOverflow::default(),
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

  pub(crate) fn rendered_text_for_layout(&self) -> String {
    let text = self.rendered_text().unwrap_or_default();
    match self.overflow() {
      TextInputOverflow::Multiline => text,
      TextInputOverflow::Scroll => text.replace(['\r', '\n'], " "),
    }
  }

  pub(crate) fn insert(&self, text: &str) {
    if text.is_empty() {
      return;
    }

    let mut caret = self.delete_selection_if_present();
    if caret.is_none() {
      caret = Some(self.inner.lock().unwrap().caret);
    }
    let mut caret = caret.unwrap();
    self.value.update(|value| {
      caret = clamp_to_char_boundary(value, caret);
      value.insert_str(caret, text);
      caret += text.len();
    });
    let mut inner = self.inner.lock().unwrap();
    inner.caret = caret;
    inner.selection_anchor = None;
  }

  pub(crate) fn insert_newline(&self) -> bool {
    if self.overflow() != TextInputOverflow::Multiline {
      return false;
    }
    self.insert("\n");
    true
  }

  pub(crate) fn backspace(&self) {
    if self.delete_selection_if_present().is_some() {
      return;
    }

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
    if self.delete_selection_if_present().is_some() {
      return;
    }

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

  pub(crate) fn move_left(&self, selecting: bool) {
    let value = self.value();
    let mut inner = self.inner.lock().unwrap();
    if selecting {
      if inner.selection_anchor.is_none() {
        inner.selection_anchor = Some(inner.caret);
      }
      inner.caret = previous_char_boundary(&value, clamp_to_char_boundary(&value, inner.caret));
    } else if let Some((start, _)) = selection_range_indices(&value, inner.selection_anchor, inner.caret) {
      inner.caret = start;
      inner.selection_anchor = None;
    } else {
      inner.caret = previous_char_boundary(&value, clamp_to_char_boundary(&value, inner.caret));
      inner.selection_anchor = None;
    }
  }

  pub(crate) fn move_right(&self, selecting: bool) {
    let value = self.value();
    let mut inner = self.inner.lock().unwrap();
    if selecting {
      if inner.selection_anchor.is_none() {
        inner.selection_anchor = Some(inner.caret);
      }
      inner.caret = next_char_boundary(&value, clamp_to_char_boundary(&value, inner.caret));
    } else if let Some((_, end)) = selection_range_indices(&value, inner.selection_anchor, inner.caret) {
      inner.caret = end;
      inner.selection_anchor = None;
    } else {
      inner.caret = next_char_boundary(&value, clamp_to_char_boundary(&value, inner.caret));
      inner.selection_anchor = None;
    }
  }

  pub(crate) fn move_home(&self, selecting: bool) {
    let mut inner = self.inner.lock().unwrap();
    if selecting && inner.selection_anchor.is_none() {
      inner.selection_anchor = Some(inner.caret);
    }
    inner.caret = 0;
    if !selecting {
      inner.selection_anchor = None;
    }
  }

  pub(crate) fn move_end(&self, selecting: bool) {
    let len = self.value().len();
    let mut inner = self.inner.lock().unwrap();
    if selecting && inner.selection_anchor.is_none() {
      inner.selection_anchor = Some(inner.caret);
    }
    inner.caret = len;
    if !selecting {
      inner.selection_anchor = None;
    }
  }

  pub(crate) fn select_all(&self) {
    let len = self.value().len();
    let mut inner = self.inner.lock().unwrap();
    inner.selection_anchor = Some(0);
    inner.caret = len;
  }

  pub(crate) fn begin_selection_at_x(&self, x: f32) {
    let caret = self.closest_caret_to_x(x);
    let mut inner = self.inner.lock().unwrap();
    inner.caret = caret;
    inner.selection_anchor = Some(caret);
  }

  pub(crate) fn update_selection_to_x(&self, x: f32) {
    let caret = self.closest_caret_to_x(x);
    self.inner.lock().unwrap().caret = caret;
  }

  pub(crate) fn set_caret_from_x(&self, x: f32) {
    let caret = self.closest_caret_to_x(x);
    let mut inner = self.inner.lock().unwrap();
    inner.caret = caret;
    inner.selection_anchor = None;
  }

  pub(crate) fn copy_runtime_state_from(&self, old: &Self) {
    let old_inner = old.inner.lock().unwrap();
    let old_caret = old_inner.caret;
    let old_selection_anchor = old_inner.selection_anchor;
    let old_caret_x = old_inner.caret_x;
    let old_caret_y = old_inner.caret_y;
    let old_caret_height = old_inner.caret_height;
    let old_caret_positions = old_inner.caret_positions.clone();
    let old_scroll_x = old_inner.scroll_x;
    let old_overflow = old_inner.overflow;
    let old_focused = old_inner.focused;
    let len = self.value().len();
    let mut inner = self.inner.lock().unwrap();
    inner.caret = old_caret.min(len);
    inner.selection_anchor = old_selection_anchor.map(|anchor| anchor.min(len));
    inner.caret_x = old_caret_x;
    inner.caret_y = old_caret_y;
    inner.caret_height = old_caret_height;
    inner.caret_positions = old_caret_positions;
    inner.scroll_x = old_scroll_x;
    inner.overflow = old_overflow;
    inner.focused = old_focused;
  }

  pub(crate) fn sync_caret_metrics_to_position(&self, line_height: f32) {
    let value = self.value();
    let mut inner = self.inner.lock().unwrap();
    inner.caret_x = caret_x_for_index(&inner.caret_positions, inner.caret);
    let caret = clamp_to_char_boundary(&value, inner.caret);
    inner.caret_y = value[..caret].chars().filter(|ch| *ch == '\n').count() as f32 * line_height;
  }

  pub(crate) fn caret_x(&self) -> f32 {
    let inner = self.inner.lock().unwrap();
    inner.caret_x - inner.scroll_x
  }

  pub(crate) fn caret_y(&self) -> f32 {
    self.inner.lock().unwrap().caret_y
  }

  pub(crate) fn set_caret_height(&self, caret_height: f32) {
    self.inner.lock().unwrap().caret_height = caret_height;
  }

  pub(crate) fn caret_height(&self) -> f32 {
    self.inner.lock().unwrap().caret_height
  }

  pub(crate) fn set_caret_positions(&self, positions: Vec<(usize, f32)>) {
    self.inner.lock().unwrap().caret_positions = positions;
  }

  pub(crate) fn set_scroll_x(&self, scroll_x: f32) {
    self.inner.lock().unwrap().scroll_x = scroll_x.max(0.0);
  }

  pub(crate) fn scroll_x(&self) -> f32 {
    self.inner.lock().unwrap().scroll_x
  }

  pub(crate) fn set_overflow(&self, overflow: TextInputOverflow) {
    self.inner.lock().unwrap().overflow = overflow;
  }

  pub(crate) fn overflow(&self) -> TextInputOverflow {
    self.inner.lock().unwrap().overflow
  }

  pub(crate) fn selection_x_range(&self) -> Option<(f32, f32)> {
    let value = self.value();
    let inner = self.inner.lock().unwrap();
    let (start, end) = selection_range_indices(&value, inner.selection_anchor, inner.caret)?;
    let start_x = caret_x_for_index(&inner.caret_positions, start);
    let end_x = caret_x_for_index(&inner.caret_positions, end);
    Some((start_x.min(end_x) - inner.scroll_x, start_x.max(end_x) - inner.scroll_x))
  }

  pub(crate) fn set_focused(&self, focused: bool) {
    self.inner.lock().unwrap().focused = focused;
  }

  pub(crate) fn is_focused(&self) -> bool {
    self.inner.lock().unwrap().focused
  }

  fn closest_caret_to_x(&self, x: f32) -> usize {
    let inner = self.inner.lock().unwrap();
    inner
      .caret_positions
      .iter()
      .min_by(|(_, ax), (_, bx)| {
        (ax - x)
          .abs()
          .partial_cmp(&(bx - x).abs())
          .unwrap_or(std::cmp::Ordering::Equal)
      })
      .map(|(index, _)| *index)
      .unwrap_or(0)
  }

  fn delete_selection_if_present(&self) -> Option<usize> {
    let mut selected = None;
    {
      let value = self.value();
      let inner = self.inner.lock().unwrap();
      if let Some(range) = selection_range_indices(&value, inner.selection_anchor, inner.caret) {
        selected = Some(range);
      }
    }

    let (start, end) = selected?;
    self.value.update(|value| {
      value.replace_range(start..end, "");
    });
    let mut inner = self.inner.lock().unwrap();
    inner.caret = start;
    inner.selection_anchor = None;
    Some(start)
  }
}

fn selection_range_indices(value: &str, anchor: Option<usize>, caret: usize) -> Option<(usize, usize)> {
  let anchor = clamp_to_char_boundary(value, anchor?);
  let caret = clamp_to_char_boundary(value, caret);
  if anchor == caret {
    return None;
  }
  Some((anchor.min(caret), anchor.max(caret)))
}

fn caret_x_for_index(positions: &[(usize, f32)], index: usize) -> f32 {
  positions
    .iter()
    .find(|(position_index, _)| *position_index == index)
    .map(|(_, x)| *x)
    .unwrap_or_else(|| positions.last().map(|(_, x)| *x).unwrap_or(0.0))
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
