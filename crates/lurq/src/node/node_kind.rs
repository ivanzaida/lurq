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

#[derive(Clone, Copy)]
pub(crate) struct CaretPosition {
  pub(crate) index: usize,
  pub(crate) x: f32,
  pub(crate) y: f32,
}

struct TextInputInner {
  placeholder: Option<String>,
  caret: usize,
  selection_anchor: Option<usize>,
  caret_x: f32,
  caret_y: f32,
  caret_height: f32,
  caret_positions: Vec<CaretPosition>,
  scroll_x: f32,
  scroll_y: f32,
  overflow: TextInputOverflow,
  min_rows: Option<usize>,
  max_rows: Option<usize>,
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
        caret_positions: vec![CaretPosition {
          index: 0,
          x: 0.0,
          y: 0.0,
        }],
        scroll_x: 0.0,
        scroll_y: 0.0,
        overflow: TextInputOverflow::default(),
        min_rows: None,
        max_rows: None,
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

  pub(crate) fn move_up(&self, selecting: bool) {
    let value = self.value();
    let mut inner = self.inner.lock().unwrap();
    let caret = clamp_to_char_boundary(&value, inner.caret);
    let (line_start, _) = line_bounds(&value, caret);
    if line_start == 0 {
      move_inner_to(&mut inner, 0, selecting);
      return;
    }

    let target_x = caret_x_for_index(&inner.caret_positions, caret);
    let previous_line_end = line_start - 1;
    let (previous_line_start, previous_line_end) = line_bounds(&value, previous_line_end);
    let target = closest_caret_in_range(&inner.caret_positions, previous_line_start, previous_line_end, target_x);
    move_inner_to(&mut inner, target, selecting);
  }

  pub(crate) fn move_down(&self, selecting: bool) {
    let value = self.value();
    let mut inner = self.inner.lock().unwrap();
    let caret = clamp_to_char_boundary(&value, inner.caret);
    let (_, line_end) = line_bounds(&value, caret);
    if line_end >= value.len() {
      move_inner_to(&mut inner, value.len(), selecting);
      return;
    }

    let target_x = caret_x_for_index(&inner.caret_positions, caret);
    let next_line_start = line_end + 1;
    let (_, next_line_end) = line_bounds(&value, next_line_start);
    let target = closest_caret_in_range(&inner.caret_positions, next_line_start, next_line_end, target_x);
    move_inner_to(&mut inner, target, selecting);
  }

  pub(crate) fn move_word_left(&self, selecting: bool) {
    let value = self.value();
    let mut inner = self.inner.lock().unwrap();
    let target = previous_word_boundary(&value, inner.caret);
    move_inner_to(&mut inner, target, selecting);
  }

  pub(crate) fn move_word_right(&self, selecting: bool) {
    let value = self.value();
    let mut inner = self.inner.lock().unwrap();
    let target = next_word_boundary(&value, inner.caret);
    move_inner_to(&mut inner, target, selecting);
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

  pub(crate) fn begin_selection_at_point(&self, x: f32, y: f32) {
    let caret = self.closest_caret_to_point(x, y);
    let mut inner = self.inner.lock().unwrap();
    inner.caret = caret;
    inner.selection_anchor = Some(caret);
  }

  pub(crate) fn update_selection_to_point(&self, x: f32, y: f32) {
    let caret = self.closest_caret_to_point(x, y);
    self.inner.lock().unwrap().caret = caret;
  }

  pub(crate) fn set_caret_from_point(&self, x: f32, y: f32) {
    let caret = self.closest_caret_to_point(x, y);
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
    let old_scroll_y = old_inner.scroll_y;
    let old_overflow = old_inner.overflow;
    let old_min_rows = old_inner.min_rows;
    let old_max_rows = old_inner.max_rows;
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
    inner.scroll_y = old_scroll_y;
    inner.overflow = old_overflow;
    inner.min_rows = old_min_rows;
    inner.max_rows = old_max_rows;
    inner.focused = old_focused;
  }

  pub(crate) fn sync_caret_metrics_to_position(&self, line_height: f32) {
    let value = self.value();
    let mut inner = self.inner.lock().unwrap();
    inner.caret_x = caret_x_for_index(&inner.caret_positions, inner.caret);
    inner.caret_y = caret_y_for_index(&inner.caret_positions, inner.caret).unwrap_or_else(|| {
      let caret = clamp_to_char_boundary(&value, inner.caret);
      value[..caret].chars().filter(|ch| *ch == '\n').count() as f32 * line_height
    });
  }

  pub(crate) fn caret_x(&self) -> f32 {
    let inner = self.inner.lock().unwrap();
    inner.caret_x - inner.scroll_x
  }

  pub(crate) fn caret_y(&self) -> f32 {
    let inner = self.inner.lock().unwrap();
    inner.caret_y - inner.scroll_y
  }

  pub(crate) fn set_caret_height(&self, caret_height: f32) {
    self.inner.lock().unwrap().caret_height = caret_height;
  }

  pub(crate) fn caret_height(&self) -> f32 {
    self.inner.lock().unwrap().caret_height
  }

  pub(crate) fn set_caret_positions(&self, positions: Vec<CaretPosition>) {
    self.inner.lock().unwrap().caret_positions = positions;
  }

  pub(crate) fn set_scroll_x(&self, scroll_x: f32) {
    self.inner.lock().unwrap().scroll_x = scroll_x.max(0.0);
  }

  pub(crate) fn scroll_x(&self) -> f32 {
    self.inner.lock().unwrap().scroll_x
  }

  pub(crate) fn set_scroll_y(&self, scroll_y: f32) {
    self.inner.lock().unwrap().scroll_y = scroll_y.max(0.0);
  }

  pub(crate) fn scroll_y(&self) -> f32 {
    self.inner.lock().unwrap().scroll_y
  }

  pub(crate) fn set_overflow(&self, overflow: TextInputOverflow) {
    self.inner.lock().unwrap().overflow = overflow;
  }

  pub(crate) fn overflow(&self) -> TextInputOverflow {
    self.inner.lock().unwrap().overflow
  }

  pub(crate) fn set_rows(&self, min_rows: usize, max_rows: usize) {
    let min_rows = min_rows.max(1);
    let max_rows = max_rows.max(min_rows);
    let mut inner = self.inner.lock().unwrap();
    inner.overflow = TextInputOverflow::Multiline;
    inner.min_rows = Some(min_rows);
    inner.max_rows = Some(max_rows);
  }

  pub(crate) fn set_min_rows(&self, min_rows: usize) {
    let min_rows = min_rows.max(1);
    let mut inner = self.inner.lock().unwrap();
    inner.overflow = TextInputOverflow::Multiline;
    inner.min_rows = Some(min_rows);
    if let Some(max_rows) = inner.max_rows
      && max_rows < min_rows
    {
      inner.max_rows = Some(min_rows);
    }
  }

  pub(crate) fn set_max_rows(&self, max_rows: usize) {
    let max_rows = max_rows.max(1);
    let mut inner = self.inner.lock().unwrap();
    inner.overflow = TextInputOverflow::Multiline;
    inner.max_rows = Some(max_rows);
    if let Some(min_rows) = inner.min_rows
      && min_rows > max_rows
    {
      inner.min_rows = Some(max_rows);
    }
  }

  pub(crate) fn set_rows_exact(&self, rows: usize) {
    let rows = rows.max(1);
    let mut inner = self.inner.lock().unwrap();
    inner.overflow = TextInputOverflow::Multiline;
    inner.min_rows = Some(rows);
    inner.max_rows = Some(rows);
  }

  pub(crate) fn row_limits(&self) -> (Option<usize>, Option<usize>) {
    let inner = self.inner.lock().unwrap();
    (inner.min_rows, inner.max_rows)
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

  fn closest_caret_to_point(&self, x: f32, y: f32) -> usize {
    let inner = self.inner.lock().unwrap();
    let content_x = x + inner.scroll_x;
    let content_y = y + inner.scroll_y;
    let closest_y = inner
      .caret_positions
      .iter()
      .min_by(|a, b| {
        (a.y - content_y)
          .abs()
          .partial_cmp(&(b.y - content_y).abs())
          .unwrap_or(std::cmp::Ordering::Equal)
      })
      .map(|position| position.y)
      .unwrap_or(0.0);

    inner
      .caret_positions
      .iter()
      .filter(|position| (position.y - closest_y).abs() <= f32::EPSILON)
      .min_by(|a, b| {
        (a.x - content_x)
          .abs()
          .partial_cmp(&(b.x - content_x).abs())
          .unwrap_or(std::cmp::Ordering::Equal)
      })
      .map(|position| position.index)
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

fn caret_x_for_index(positions: &[CaretPosition], index: usize) -> f32 {
  positions
    .iter()
    .find(|position| position.index == index)
    .map(|position| position.x)
    .unwrap_or_else(|| positions.last().map(|position| position.x).unwrap_or(0.0))
}

fn caret_y_for_index(positions: &[CaretPosition], index: usize) -> Option<f32> {
  positions
    .iter()
    .find(|position| position.index == index)
    .map(|position| position.y)
}

fn move_inner_to(inner: &mut TextInputInner, target: usize, selecting: bool) {
  if selecting {
    if inner.selection_anchor.is_none() {
      inner.selection_anchor = Some(inner.caret);
    }
  } else {
    inner.selection_anchor = None;
  }
  inner.caret = target;
}

fn line_bounds(value: &str, index: usize) -> (usize, usize) {
  let index = clamp_to_char_boundary(value, index);
  let line_start = value[..index].rfind('\n').map(|position| position + 1).unwrap_or(0);
  let line_end = value[index..]
    .find('\n')
    .map(|position| index + position)
    .unwrap_or(value.len());
  (line_start, line_end)
}

fn closest_caret_in_range(positions: &[CaretPosition], start: usize, end: usize, x: f32) -> usize {
  positions
    .iter()
    .filter(|position| position.index >= start && position.index <= end)
    .min_by(|a, b| {
      (a.x - x)
        .abs()
        .partial_cmp(&(b.x - x).abs())
        .unwrap_or(std::cmp::Ordering::Equal)
    })
    .map(|position| position.index)
    .unwrap_or(start)
}

fn previous_word_boundary(value: &str, index: usize) -> usize {
  let mut index = clamp_to_char_boundary(value, index);
  while let Some((previous, ch)) = char_before(value, index) {
    if !ch.is_whitespace() {
      break;
    }
    index = previous;
  }
  while let Some((previous, ch)) = char_before(value, index) {
    if ch.is_whitespace() {
      break;
    }
    index = previous;
  }
  index
}

fn next_word_boundary(value: &str, index: usize) -> usize {
  let mut index = clamp_to_char_boundary(value, index);
  while index < value.len() {
    let ch = value[index..].chars().next().unwrap();
    if ch.is_whitespace() {
      break;
    }
    index += ch.len_utf8();
  }
  while index < value.len() {
    let ch = value[index..].chars().next().unwrap();
    if !ch.is_whitespace() {
      break;
    }
    index += ch.len_utf8();
  }
  index
}

fn char_before(value: &str, index: usize) -> Option<(usize, char)> {
  let index = clamp_to_char_boundary(value, index);
  value[..index].char_indices().last()
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
