use std::sync::{Arc, Mutex};

use crate::{
  app::theme::{ThemePalette, ThemeTypography, TypographyStyle},
  core::Signal,
  layout::text_style::TextStyle,
  node::{
    CheckboxStyle, SliderPartStyle, TextColor, TextTransformMode,
    text_selection::{
      CaretPosition, TextSelectionRange, caret_x_for_index, caret_y_for_index, clamp_to_char_boundary,
      closest_caret_in_range, closest_caret_to_point, line_bounds, next_char_boundary, next_word_boundary,
      previous_char_boundary, previous_word_boundary, selection_range_indices, selection_ranges_for_positions,
      word_selection_bounds,
    },
  },
};

const MAX_TEXT_INPUT_HISTORY: usize = 128;

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
    state: TextState,
    style: TextStyleSource,
    transform_mode: TextTransformMode,
  },
  TextInput {
    state: TextInputState,
    style: TextStyle,
    placeholder_style: Option<TextStyle>,
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

#[derive(Clone, PartialEq)]
pub(crate) struct TextStyleSource {
  base: TextStyleBase,
  color: Option<TextColor>,
}

#[derive(Clone, PartialEq)]
enum TextStyleBase {
  Default,
  Typography(TypographyStyle),
  Explicit(TextStyle),
}

impl TextStyleSource {
  pub(crate) fn default_style() -> Self {
    Self {
      base: TextStyleBase::Default,
      color: None,
    }
  }

  pub(crate) fn explicit(style: TextStyle) -> Self {
    Self {
      base: TextStyleBase::Explicit(style),
      color: None,
    }
  }

  pub(crate) fn set_variant(&mut self, style: impl Into<TypographyStyle>) {
    self.base = TextStyleBase::Typography(style.into());
  }

  pub(crate) fn set_color(&mut self, color: impl Into<TextColor>) {
    self.color = Some(color.into());
  }

  pub(crate) fn resolve(&self, typography: &ThemeTypography, palette: &ThemePalette) -> TextStyle {
    let mut style = match &self.base {
      TextStyleBase::Default => typography.default_style().clone(),
      TextStyleBase::Typography(style) => typography.resolve(*style),
      TextStyleBase::Explicit(style) => style.clone(),
    };
    if let Some(color) = self.color.as_ref().and_then(|color| color.resolve(palette)) {
      style.color = color;
    }
    style
  }
}

#[derive(Clone)]
pub(crate) struct TextState {
  inner: Arc<Mutex<TextInner>>,
}

struct TextInner {
  selectable: bool,
  caret: usize,
  selection_anchor: Option<usize>,
  caret_positions: Vec<CaretPosition>,
}

impl TextState {
  pub(crate) fn new() -> Self {
    Self {
      inner: Arc::new(Mutex::new(TextInner {
        selectable: false,
        caret: 0,
        selection_anchor: None,
        caret_positions: vec![CaretPosition {
          index: 0,
          x: 0.0,
          y: 0.0,
        }],
      })),
    }
  }

  pub(crate) fn set_selectable(&self, selectable: bool) {
    let mut inner = self.inner.lock().unwrap();
    inner.selectable = selectable;
    if !selectable {
      inner.selection_anchor = None;
      inner.caret = 0;
    }
  }

  pub(crate) fn selectable(&self) -> bool {
    self.inner.lock().unwrap().selectable
  }

  pub(crate) fn begin_selection_at_point(&self, value: &str, x: f32, y: f32) {
    if !self.selectable() {
      return;
    }
    let caret = self.closest_caret_to_point(value, x, y);
    let mut inner = self.inner.lock().unwrap();
    inner.caret = caret;
    inner.selection_anchor = Some(caret);
  }

  pub(crate) fn update_selection_to_point(&self, value: &str, x: f32, y: f32) {
    if !self.selectable() {
      return;
    }
    let caret = self.closest_caret_to_point(value, x, y);
    self.inner.lock().unwrap().caret = caret;
  }

  pub(crate) fn clear_selection_at_point(&self, value: &str, x: f32, y: f32) {
    let caret = self.closest_caret_to_point(value, x, y);
    let mut inner = self.inner.lock().unwrap();
    inner.caret = caret;
    inner.selection_anchor = None;
  }

  pub(crate) fn select_word_at_point(&self, value: &str, x: f32, y: f32) {
    if !self.selectable() {
      return;
    }
    let caret = self.closest_caret_to_point(value, x, y);
    let (start, end) = word_selection_bounds(value, caret);
    let mut inner = self.inner.lock().unwrap();
    inner.selection_anchor = Some(start);
    inner.caret = end;
  }

  pub(crate) fn select_line_at_point(&self, value: &str, x: f32, y: f32) {
    if !self.selectable() {
      return;
    }
    let caret = self.closest_caret_to_point(value, x, y);
    let (start, end) = line_bounds(value, caret);
    let mut inner = self.inner.lock().unwrap();
    inner.selection_anchor = Some(start);
    inner.caret = end;
  }

  pub(crate) fn has_selection(&self, value: &str) -> bool {
    let inner = self.inner.lock().unwrap();
    selection_range_indices(value, inner.selection_anchor, inner.caret).is_some()
  }

  pub(crate) fn selected_text(&self, value: &str) -> Option<String> {
    let inner = self.inner.lock().unwrap();
    let (start, end) = selection_range_indices(value, inner.selection_anchor, inner.caret)?;
    Some(value[start..end].to_owned())
  }

  pub(crate) fn selection_ranges(&self, value: &str) -> Vec<TextSelectionRange> {
    if !self.selectable() {
      return Vec::new();
    }
    let inner = self.inner.lock().unwrap();
    let Some((start, end)) = selection_range_indices(value, inner.selection_anchor, inner.caret) else {
      return Vec::new();
    };
    selection_ranges_for_positions(&inner.caret_positions, start, end, 0.0, 0.0)
  }

  pub(crate) fn set_caret_positions(&self, positions: Vec<CaretPosition>) {
    self.inner.lock().unwrap().caret_positions = positions;
  }

  pub(crate) fn copy_runtime_state_from(&self, old: &Self, value: &str) {
    if Arc::ptr_eq(&self.inner, &old.inner) {
      return;
    }
    let old_inner = old.inner.lock().unwrap();
    let selectable = self.selectable();
    let len = value.len();
    let mut inner = self.inner.lock().unwrap();
    inner.selectable = selectable;
    if selectable {
      inner.caret = old_inner.caret.min(len);
      inner.selection_anchor = old_inner.selection_anchor.map(|anchor| anchor.min(len));
      inner.caret_positions = old_inner.caret_positions.clone();
    } else {
      inner.caret = 0;
      inner.selection_anchor = None;
    }
  }

  fn closest_caret_to_point(&self, value: &str, x: f32, y: f32) -> usize {
    let inner = self.inner.lock().unwrap();
    clamp_to_char_boundary(value, closest_caret_to_point(&inner.caret_positions, x, y))
  }
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
  caret_positions: Vec<CaretPosition>,
  scroll_x: f32,
  scroll_y: f32,
  overflow: TextInputOverflow,
  min_rows: Option<usize>,
  max_rows: Option<usize>,
  focused: bool,
  undo_stack: Vec<TextInputSnapshot>,
  redo_stack: Vec<TextInputSnapshot>,
}

#[derive(Clone)]
struct TextInputSnapshot {
  value: String,
  caret: usize,
  selection_anchor: Option<usize>,
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
        undo_stack: Vec::new(),
        redo_stack: Vec::new(),
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

  pub(crate) fn is_showing_placeholder(&self) -> bool {
    self.value().is_empty() && self.placeholder().is_some()
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

    self.push_undo_snapshot();
    let mut caret = self
      .delete_selection_if_present()
      .unwrap_or_else(|| self.inner.lock().unwrap().caret);
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
    if self.has_selection() {
      self.push_undo_snapshot();
      self.delete_selection_if_present();
      return;
    }

    let mut caret = self.inner.lock().unwrap().caret;
    if caret == 0 {
      return;
    }

    self.push_undo_snapshot();
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
    if self.has_selection() {
      self.push_undo_snapshot();
      self.delete_selection_if_present();
      return;
    }

    let mut caret = self.inner.lock().unwrap().caret;
    let value = self.value();
    caret = clamp_to_char_boundary(&value, caret);
    if caret >= value.len() {
      return;
    }

    self.push_undo_snapshot();
    self.value.update(|value| {
      let next = next_char_boundary(value, caret);
      value.replace_range(caret..next, "");
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

  pub(crate) fn selected_text(&self) -> Option<String> {
    let value = self.value();
    let inner = self.inner.lock().unwrap();
    let (start, end) = selection_range_indices(&value, inner.selection_anchor, inner.caret)?;
    Some(value[start..end].to_owned())
  }

  pub(crate) fn cut_selection(&self) -> Option<String> {
    let selected = self.selected_text()?;
    self.push_undo_snapshot();
    self.delete_selection_if_present();
    Some(selected)
  }

  pub(crate) fn undo(&self) -> bool {
    let current = self.snapshot();
    let Some(snapshot) = self.inner.lock().unwrap().undo_stack.pop() else {
      return false;
    };
    self.push_redo_snapshot(current);
    self.restore_snapshot(snapshot);
    true
  }

  pub(crate) fn redo(&self) -> bool {
    let current = self.snapshot();
    let Some(snapshot) = self.inner.lock().unwrap().redo_stack.pop() else {
      return false;
    };
    self.push_undo_snapshot_value(current);
    self.restore_snapshot(snapshot);
    true
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

  pub(crate) fn select_word_at_point(&self, x: f32, y: f32) {
    let caret = self.closest_caret_to_point(x, y);
    let value = self.value();
    let (start, end) = word_selection_bounds(&value, caret);
    let mut inner = self.inner.lock().unwrap();
    inner.selection_anchor = Some(start);
    inner.caret = end;
  }

  pub(crate) fn select_line_at_point(&self, x: f32, y: f32) {
    let caret = self.closest_caret_to_point(x, y);
    let value = self.value();
    let (start, end) = line_bounds(&value, caret);
    let mut inner = self.inner.lock().unwrap();
    inner.selection_anchor = Some(start);
    inner.caret = end;
  }

  pub(crate) fn copy_runtime_state_from(&self, old: &Self) {
    if Arc::ptr_eq(&self.inner, &old.inner) {
      return;
    }
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
    let old_undo_stack = old_inner.undo_stack.clone();
    let old_redo_stack = old_inner.redo_stack.clone();
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
    inner.undo_stack = old_undo_stack;
    inner.redo_stack = old_redo_stack;
  }

  pub(crate) fn sync_caret_metrics_to_position(&self, line_height: f32) {
    let value = self.value();
    let mut inner = self.inner.lock().unwrap();
    let caret = clamp_to_char_boundary(&value, inner.caret);
    if caret == value.len() && value[..caret].ends_with('\n') {
      inner.caret_x = 0.0;
      inner.caret_y = value[..caret].chars().filter(|ch| *ch == '\n').count() as f32 * line_height;
      return;
    }
    inner.caret_x = caret_x_for_index(&inner.caret_positions, inner.caret);
    inner.caret_y = caret_y_for_index(&inner.caret_positions, inner.caret)
      .unwrap_or_else(|| value[..caret].chars().filter(|ch| *ch == '\n').count() as f32 * line_height);
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

  pub(crate) fn selection_ranges(&self) -> Vec<TextSelectionRange> {
    let value = self.value();
    let inner = self.inner.lock().unwrap();
    let Some((start, end)) = selection_range_indices(&value, inner.selection_anchor, inner.caret) else {
      return Vec::new();
    };
    selection_ranges_for_positions(&inner.caret_positions, start, end, inner.scroll_x, inner.scroll_y)
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
    closest_caret_to_point(&inner.caret_positions, content_x, content_y)
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

  pub(crate) fn has_selection(&self) -> bool {
    let value = self.value();
    let inner = self.inner.lock().unwrap();
    selection_range_indices(&value, inner.selection_anchor, inner.caret).is_some()
  }

  fn snapshot(&self) -> TextInputSnapshot {
    let value = self.value();
    let inner = self.inner.lock().unwrap();
    TextInputSnapshot {
      value,
      caret: inner.caret,
      selection_anchor: inner.selection_anchor,
    }
  }

  fn push_undo_snapshot(&self) {
    let snapshot = self.snapshot();
    let mut inner = self.inner.lock().unwrap();
    push_limited(&mut inner.undo_stack, snapshot);
    inner.redo_stack.clear();
  }

  fn push_undo_snapshot_value(&self, snapshot: TextInputSnapshot) {
    let mut inner = self.inner.lock().unwrap();
    push_limited(&mut inner.undo_stack, snapshot);
  }

  fn push_redo_snapshot(&self, snapshot: TextInputSnapshot) {
    let mut inner = self.inner.lock().unwrap();
    push_limited(&mut inner.redo_stack, snapshot);
  }

  fn restore_snapshot(&self, snapshot: TextInputSnapshot) {
    let value = snapshot.value;
    let len = value.len();
    let caret = clamp_to_char_boundary(&value, snapshot.caret.min(len));
    let selection_anchor = snapshot
      .selection_anchor
      .map(|anchor| clamp_to_char_boundary(&value, anchor.min(len)));
    self.value.set(value);
    let mut inner = self.inner.lock().unwrap();
    inner.caret = caret;
    inner.selection_anchor = selection_anchor;
  }
}

fn push_limited(stack: &mut Vec<TextInputSnapshot>, snapshot: TextInputSnapshot) {
  if stack.last().is_some_and(|last| {
    last.value == snapshot.value && last.caret == snapshot.caret && last.selection_anchor == snapshot.selection_anchor
  }) {
    return;
  }
  stack.push(snapshot);
  if stack.len() > MAX_TEXT_INPUT_HISTORY {
    stack.remove(0);
  }
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

#[derive(Clone)]
pub(crate) struct CheckboxState {
  value: Signal<bool>,
  inner: Arc<Mutex<CheckboxInner>>,
}

struct CheckboxInner {
  style: CheckboxStyle,
  checked_style: Option<CheckboxStyle>,
  hovered_style: Option<CheckboxStyle>,
  checked_hovered_style: Option<CheckboxStyle>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct CheckboxLayoutSignature {
  width: Option<f32>,
  height: Option<f32>,
  checked_width: Option<f32>,
  checked_height: Option<f32>,
  hovered_width: Option<f32>,
  hovered_height: Option<f32>,
  checked_hovered_width: Option<f32>,
  checked_hovered_height: Option<f32>,
}

impl CheckboxState {
  pub(crate) fn new(value: Signal<bool>) -> Self {
    Self {
      value,
      inner: Arc::new(Mutex::new(CheckboxInner {
        style: CheckboxStyle::new(),
        checked_style: None,
        hovered_style: None,
        checked_hovered_style: None,
      })),
    }
  }

  pub(crate) fn is_checked(&self) -> bool {
    self.value.get_untracked()
  }

  pub(crate) fn toggle(&self) {
    self.value.update(|checked| *checked = !*checked);
  }

  pub(crate) fn set_style(&self, style: CheckboxStyle) {
    self.inner.lock().unwrap().style = style;
  }

  pub(crate) fn set_checked_style(&self, style: CheckboxStyle) {
    self.inner.lock().unwrap().checked_style = Some(style);
  }

  pub(crate) fn set_hovered_style(&self, style: CheckboxStyle) {
    self.inner.lock().unwrap().hovered_style = Some(style);
  }

  pub(crate) fn set_checked_hovered_style(&self, style: CheckboxStyle) {
    self.inner.lock().unwrap().checked_hovered_style = Some(style);
  }

  pub(crate) fn style(&self, checked: bool, hovered: bool) -> CheckboxStyle {
    let inner = self.inner.lock().unwrap();
    let mut style = inner.style.clone();
    if checked && let Some(checked_style) = &inner.checked_style {
      style.merge_from(checked_style);
    }
    if hovered && let Some(hovered_style) = &inner.hovered_style {
      style.merge_from(hovered_style);
    }
    if checked
      && hovered
      && let Some(checked_hovered_style) = &inner.checked_hovered_style
    {
      style.merge_from(checked_hovered_style);
    }
    style
  }

  pub(crate) fn preferred_size(&self, default_width: f32, default_height: f32) -> (f32, f32) {
    let inner = self.inner.lock().unwrap();
    let width = std::iter::once(inner.style.width)
      .chain(inner.checked_style.as_ref().map(|style| style.width))
      .chain(inner.hovered_style.as_ref().map(|style| style.width))
      .chain(inner.checked_hovered_style.as_ref().map(|style| style.width))
      .flatten()
      .fold(default_width, f32::max);
    let height = std::iter::once(inner.style.height)
      .chain(inner.checked_style.as_ref().map(|style| style.height))
      .chain(inner.hovered_style.as_ref().map(|style| style.height))
      .chain(inner.checked_hovered_style.as_ref().map(|style| style.height))
      .flatten()
      .fold(default_height, f32::max);
    (width, height)
  }

  pub(crate) fn layout_signature(&self) -> CheckboxLayoutSignature {
    let inner = self.inner.lock().unwrap();
    CheckboxLayoutSignature {
      width: inner.style.width,
      height: inner.style.height,
      checked_width: inner.checked_style.as_ref().and_then(|style| style.width),
      checked_height: inner.checked_style.as_ref().and_then(|style| style.height),
      hovered_width: inner.hovered_style.as_ref().and_then(|style| style.width),
      hovered_height: inner.hovered_style.as_ref().and_then(|style| style.height),
      checked_hovered_width: inner.checked_hovered_style.as_ref().and_then(|style| style.width),
      checked_hovered_height: inner.checked_hovered_style.as_ref().and_then(|style| style.height),
    }
  }

  #[cfg(all(feature = "image", feature = "resources"))]
  pub(crate) fn resolve_resource_images(&self, mut resolve: impl FnMut(&Arc<str>) -> Option<crate::images::ImageData>) {
    fn resolve_style(
      style: &mut CheckboxStyle,
      resolve: &mut impl FnMut(&Arc<str>) -> Option<crate::images::ImageData>,
    ) {
      let Some(key) = style.indicator_resource_image.clone() else {
        return;
      };
      let Some(img) = resolve(&key) else {
        return;
      };
      if style.indicator_image.as_ref().map(crate::images::ImageData::id) != Some(img.id()) {
        style.indicator_image = Some(img);
      }
    }

    let mut inner = self.inner.lock().unwrap();
    resolve_style(&mut inner.style, &mut resolve);
    if let Some(style) = &mut inner.checked_style {
      resolve_style(style, &mut resolve);
    }
    if let Some(style) = &mut inner.hovered_style {
      resolve_style(style, &mut resolve);
    }
    if let Some(style) = &mut inner.checked_hovered_style {
      resolve_style(style, &mut resolve);
    }
  }
}

#[derive(Clone)]
pub(crate) struct SliderState {
  value: Signal<i32>,
  inner: Arc<Mutex<SliderInner>>,
}

struct SliderInner {
  min: i32,
  max: i32,
  track_style: SliderPartStyle,
  track_hovered_style: Option<SliderPartStyle>,
  thumb_style: SliderPartStyle,
  thumb_hovered_style: Option<SliderPartStyle>,
  hovered: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SliderLayoutSignature {
  track_width: Option<f32>,
  track_height: Option<f32>,
  track_hovered_width: Option<f32>,
  track_hovered_height: Option<f32>,
  thumb_width: Option<f32>,
  thumb_height: Option<f32>,
  thumb_hovered_width: Option<f32>,
  thumb_hovered_height: Option<f32>,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct SliderPartRect {
  pub(crate) x: f32,
  pub(crate) y: f32,
  pub(crate) width: f32,
  pub(crate) height: f32,
}

impl SliderState {
  pub(crate) fn new(value: Signal<i32>) -> Self {
    Self {
      value,
      inner: Arc::new(Mutex::new(SliderInner {
        min: 0,
        max: 1,
        track_style: SliderPartStyle::new(),
        track_hovered_style: None,
        thumb_style: SliderPartStyle::new(),
        thumb_hovered_style: None,
        hovered: false,
      })),
    }
  }

  pub(crate) fn value(&self) -> i32 {
    self.value.get_untracked()
  }

  pub(crate) fn ratio(&self) -> f32 {
    let inner = self.inner.lock().unwrap();
    if inner.max <= inner.min {
      return 0.0;
    }
    ((self.value() - inner.min) as f32 / (inner.max - inner.min) as f32).clamp(0.0, 1.0)
  }

  pub(crate) fn set_range(&self, min: i32, max: i32) {
    let mut inner = self.inner.lock().unwrap();
    inner.min = min;
    inner.max = max.max(min);
    let current = self.value();
    let clamped = current.clamp(inner.min, inner.max);
    if current != clamped {
      self.value.set(clamped);
    }
  }

  pub(crate) fn set_from_ratio(&self, ratio: f32) -> bool {
    let inner = self.inner.lock().unwrap();
    let value = inner.min + (ratio.clamp(0.0, 1.0) * (inner.max - inner.min) as f32).round() as i32;
    let current = self.value();
    let changed = current != value;
    if changed {
      self.value.set(value);
    }
    changed
  }

  pub(crate) fn pointer_ratio(&self, x: f32, track_rect: SliderPartRect, thumb_rect: SliderPartRect) -> f32 {
    let travel_width = track_rect.width - thumb_rect.width;
    if travel_width > 0.0 {
      (x - (track_rect.x + thumb_rect.width * 0.5)) / travel_width
    } else if track_rect.width > 0.0 {
      (x - track_rect.x) / track_rect.width
    } else {
      0.0
    }
  }

  pub(crate) fn nudge(&self, delta: i32) {
    let inner = self.inner.lock().unwrap();
    let current = self.value();
    let next = (current + delta).clamp(inner.min, inner.max);
    if current != next {
      self.value.set(next);
    }
  }

  pub(crate) fn set_track_style(&self, style: SliderPartStyle) {
    self.inner.lock().unwrap().track_style = style;
  }

  pub(crate) fn set_track_hovered_style(&self, style: SliderPartStyle) {
    self.inner.lock().unwrap().track_hovered_style = Some(style);
  }

  pub(crate) fn set_thumb_style(&self, style: SliderPartStyle) {
    self.inner.lock().unwrap().thumb_style = style;
  }

  pub(crate) fn set_thumb_hovered_style(&self, style: SliderPartStyle) {
    self.inner.lock().unwrap().thumb_hovered_style = Some(style);
  }

  pub(crate) fn track_style(&self, hovered: bool) -> SliderPartStyle {
    let inner = self.inner.lock().unwrap();
    let mut style = inner.track_style.clone();
    if hovered && let Some(hovered_style) = &inner.track_hovered_style {
      style.merge_from(hovered_style);
    }
    style
  }

  pub(crate) fn thumb_style(&self, hovered: bool) -> SliderPartStyle {
    let inner = self.inner.lock().unwrap();
    let mut style = inner.thumb_style.clone();
    if hovered && let Some(hovered_style) = &inner.thumb_hovered_style {
      style.merge_from(hovered_style);
    }
    style
  }

  pub(crate) fn set_hovered(&self, hovered: bool) {
    self.inner.lock().unwrap().hovered = hovered;
  }

  pub(crate) fn is_hovered(&self) -> bool {
    self.inner.lock().unwrap().hovered
  }

  pub(crate) fn layout_signature(&self) -> SliderLayoutSignature {
    let inner = self.inner.lock().unwrap();
    SliderLayoutSignature {
      track_width: inner.track_style.width,
      track_height: inner.track_style.height,
      track_hovered_width: inner.track_hovered_style.as_ref().and_then(|style| style.width),
      track_hovered_height: inner.track_hovered_style.as_ref().and_then(|style| style.height),
      thumb_width: inner.thumb_style.width,
      thumb_height: inner.thumb_style.height,
      thumb_hovered_width: inner.thumb_hovered_style.as_ref().and_then(|style| style.width),
      thumb_hovered_height: inner.thumb_hovered_style.as_ref().and_then(|style| style.height),
    }
  }

  pub(crate) fn preferred_size(&self, default_width: f32, default_height: f32, default_thumb_size: f32) -> (f32, f32) {
    let inner = self.inner.lock().unwrap();
    let track_width = inner
      .track_style
      .width
      .into_iter()
      .chain(inner.track_hovered_style.as_ref().and_then(|style| style.width))
      .max_by(f32::total_cmp)
      .unwrap_or(default_width);
    let track_height = inner
      .track_style
      .height
      .into_iter()
      .chain(inner.track_hovered_style.as_ref().and_then(|style| style.height))
      .max_by(f32::total_cmp)
      .unwrap_or(default_height);
    let thumb_width = inner
      .thumb_style
      .width
      .into_iter()
      .chain(inner.thumb_hovered_style.as_ref().and_then(|style| style.width))
      .max_by(f32::total_cmp)
      .unwrap_or(track_height.max(default_thumb_size));
    let thumb_height = inner
      .thumb_style
      .height
      .into_iter()
      .chain(inner.thumb_hovered_style.as_ref().and_then(|style| style.height))
      .max_by(f32::total_cmp)
      .unwrap_or(track_height.max(default_thumb_size));
    (track_width.max(thumb_width), track_height.max(thumb_height))
  }

  #[cfg(all(feature = "image", feature = "resources"))]
  pub(crate) fn resolve_resource_images(
    &self,
    mut resolve: impl FnMut(&std::sync::Arc<str>) -> Option<crate::images::ImageData>,
  ) {
    fn resolve_style(
      style: &mut SliderPartStyle,
      resolve: &mut impl FnMut(&std::sync::Arc<str>) -> Option<crate::images::ImageData>,
    ) {
      let Some(key) = style.background_resource_image.clone() else {
        return;
      };
      let Some(img) = resolve(&key) else {
        return;
      };
      if style.background_image.as_ref().map(crate::images::ImageData::id) != Some(img.id()) {
        style.background_image = Some(img);
      }
    }

    let mut inner = self.inner.lock().unwrap();
    resolve_style(&mut inner.track_style, &mut resolve);
    if let Some(style) = &mut inner.track_hovered_style {
      resolve_style(style, &mut resolve);
    }
    resolve_style(&mut inner.thumb_style, &mut resolve);
    if let Some(style) = &mut inner.thumb_hovered_style {
      resolve_style(style, &mut resolve);
    }
  }

  pub(crate) fn part_rects(
    &self,
    bounds_x: f32,
    bounds_y: f32,
    bounds_width: f32,
    bounds_height: f32,
    hovered: bool,
    default_thumb_size: f32,
  ) -> (SliderPartRect, SliderPartRect) {
    let track_style = self.track_style(hovered);
    let thumb_style = self.thumb_style(hovered);
    let track_width = track_style.width.unwrap_or(bounds_width).max(0.0);
    let track_height = track_style.height.unwrap_or(bounds_height).max(0.0);
    let thumb_width = thumb_style
      .width
      .unwrap_or(track_height.max(default_thumb_size))
      .max(0.0);
    let thumb_height = thumb_style
      .height
      .unwrap_or(track_height.max(default_thumb_size))
      .max(0.0);
    let track_x = bounds_x + (bounds_width - track_width) * 0.5;
    let track_y = bounds_y + (bounds_height - track_height) * 0.5;
    let thumb_travel_width = (track_width - thumb_width).max(0.0);
    let thumb_center_x = if thumb_travel_width > 0.0 {
      track_x + thumb_width * 0.5 + thumb_travel_width * self.ratio()
    } else {
      track_x + track_width * 0.5
    };
    let thumb_center_y = track_y + track_height * 0.5;

    (
      SliderPartRect {
        x: track_x,
        y: track_y,
        width: track_width,
        height: track_height,
      },
      SliderPartRect {
        x: thumb_center_x - thumb_width * 0.5,
        y: thumb_center_y - thumb_height * 0.5,
        width: thumb_width,
        height: thumb_height,
      },
    )
  }
}
