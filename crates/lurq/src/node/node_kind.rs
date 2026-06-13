use std::sync::{
  Arc, Mutex,
  atomic::{AtomicBool, Ordering},
};

use crate::{
  app::{
    events::{KeyboardEvent, TextInputEvent},
    theme::{ThemePalette, ThemeTypography, TypographyStyle},
  },
  core::Signal,
  layout::text_style::{TextAlign, TextStyle},
  node::{
    CheckboxStyle, SelectStyle, SliderPartStyle, TextColor, TextTransformMode,
    text_selection::{
      CaretPosition, TextSelectionRange, caret_x_for_index, caret_y_for_index, clamp_to_char_boundary,
      closest_caret_in_range, closest_caret_to_point, line_bounds, next_char_boundary, next_word_boundary,
      previous_char_boundary, previous_word_boundary, selection_range_indices, selection_ranges_for_positions,
      word_selection_bounds,
    },
  },
};

const MAX_TEXT_INPUT_HISTORY: usize = 128;

type TextInputCallback = Arc<dyn Fn(&TextInputEvent) + Send + Sync>;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TextInputOverflow {
  Multiline,
  #[default]
  Scroll,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TextOverflow {
  #[default]
  Clip,
  Elipsis,
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
  Select {
    state: SelectState,
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
  text_align: Option<TextAlign>,
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
      text_align: None,
    }
  }

  pub(crate) fn explicit(style: TextStyle) -> Self {
    Self {
      base: TextStyleBase::Explicit(style),
      color: None,
      text_align: None,
    }
  }

  pub(crate) fn set_variant(&mut self, style: impl Into<TypographyStyle>) {
    self.base = TextStyleBase::Typography(style.into());
  }

  pub(crate) fn set_color(&mut self, color: impl Into<TextColor>) {
    self.color = Some(color.into());
  }

  pub(crate) fn set_text_align(&mut self, align: impl Into<TextAlign>) {
    self.text_align = Some(align.into());
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
    if let Some(text_align) = self.text_align {
      style.text_align = text_align;
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
  display_text: Option<String>,
  render_wrap: bool,
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
        display_text: None,
        render_wrap: false,
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

  pub(crate) fn update_selection_to_point(&self, value: &str, x: f32, y: f32) {
    if !self.selectable() {
      return;
    }
    let caret = self.caret_index_at_point(value, x, y);
    self.inner.lock().unwrap().caret = caret;
  }

  pub(crate) fn clear_selection_at_point(&self, value: &str, x: f32, y: f32) {
    let caret = self.caret_index_at_point(value, x, y);
    let mut inner = self.inner.lock().unwrap();
    inner.caret = caret;
    inner.selection_anchor = None;
  }

  pub(crate) fn clear_selection(&self) {
    self.inner.lock().unwrap().selection_anchor = None;
  }

  pub(crate) fn set_selection_indices(&self, value: &str, anchor: usize, caret: usize) {
    if !self.selectable() {
      return;
    }
    let mut inner = self.inner.lock().unwrap();
    inner.selection_anchor = Some(clamp_to_char_boundary(value, anchor));
    inner.caret = clamp_to_char_boundary(value, caret);
  }

  pub(crate) fn select_word_at_point(&self, value: &str, x: f32, y: f32) {
    if !self.selectable() {
      return;
    }
    let caret = self.caret_index_at_point(value, x, y);
    let (start, end) = word_selection_bounds(value, caret);
    let mut inner = self.inner.lock().unwrap();
    inner.selection_anchor = Some(start);
    inner.caret = end;
  }

  pub(crate) fn select_line_at_point(&self, value: &str, x: f32, y: f32) {
    if !self.selectable() {
      return;
    }
    let caret = self.caret_index_at_point(value, x, y);
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

  pub(crate) fn set_display_text(&self, display_text: Option<String>) {
    self.inner.lock().unwrap().display_text = display_text;
  }

  pub(crate) fn display_text(&self) -> Option<String> {
    self.inner.lock().unwrap().display_text.clone()
  }

  pub(crate) fn set_render_wrap(&self, wrap: bool) {
    self.inner.lock().unwrap().render_wrap = wrap;
  }

  pub(crate) fn render_wrap(&self) -> bool {
    self.inner.lock().unwrap().render_wrap
  }

  pub(crate) fn copy_runtime_state_from(&self, old: &Self, value: &str, preserve_display_text: bool) {
    if Arc::ptr_eq(&self.inner, &old.inner) {
      return;
    }
    let old_inner = old.inner.lock().unwrap();
    let selectable = self.selectable();
    let len = value.len();
    let mut inner = self.inner.lock().unwrap();
    inner.selectable = selectable;
    inner.display_text = if preserve_display_text {
      old_inner.display_text.clone()
    } else {
      None
    };
    inner.render_wrap = old_inner.render_wrap;
    if selectable {
      inner.caret = old_inner.caret.min(len);
      inner.selection_anchor = old_inner.selection_anchor.map(|anchor| anchor.min(len));
      inner.caret_positions = old_inner.caret_positions.clone();
    } else {
      inner.caret = 0;
      inner.selection_anchor = None;
    }
  }

  pub(crate) fn caret_index_at_point(&self, value: &str, x: f32, y: f32) -> usize {
    let inner = self.inner.lock().unwrap();
    clamp_to_char_boundary(value, closest_caret_to_point(&inner.caret_positions, x, y))
  }
}

#[derive(Clone)]
pub(crate) struct TextInputState {
  value: Signal<String>,
  inner: Arc<Mutex<TextInputInner>>,
  on_input: Arc<Mutex<Option<TextInputCallback>>>,
  layout_dirty: Arc<AtomicBool>,
}

struct TextInputInner {
  placeholder: Option<String>,
  last_value: String,
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
  mask: Option<char>,
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

#[derive(Clone, PartialEq)]
pub(crate) struct TextInputLayoutSignature {
  placeholder: Option<String>,
  overflow: TextInputOverflow,
  min_rows: Option<usize>,
  max_rows: Option<usize>,
  mask: Option<char>,
}

impl TextInputState {
  pub(crate) fn new(value: Signal<String>) -> Self {
    let initial_value = value.get_untracked();
    let caret = initial_value.len();
    Self {
      value,
      inner: Arc::new(Mutex::new(TextInputInner {
        placeholder: None,
        last_value: initial_value,
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
        mask: None,
        focused: false,
        undo_stack: Vec::new(),
        redo_stack: Vec::new(),
      })),
      on_input: Arc::new(Mutex::new(None)),
      layout_dirty: Arc::new(AtomicBool::new(false)),
    }
  }

  pub(crate) fn value(&self) -> String {
    self.value.get_untracked()
  }

  pub(crate) fn set_on_input(&self, f: impl Fn(&TextInputEvent) + Send + Sync + 'static) {
    *self.on_input.lock().unwrap() = Some(Arc::new(f));
  }

  pub(crate) fn sync_external_value(&self) -> bool {
    let value = self.value();
    let mut inner = self.inner.lock().unwrap();
    if inner.last_value == value {
      return false;
    }

    let previous_len = inner.last_value.len();
    if inner.selection_anchor.is_none() && inner.caret == previous_len {
      inner.caret = value.len();
    } else {
      inner.caret = clamp_to_char_boundary(&value, inner.caret.min(value.len()));
      inner.selection_anchor = inner
        .selection_anchor
        .map(|anchor| clamp_to_char_boundary(&value, anchor.min(value.len())));
    }
    inner.last_value = value;
    drop(inner);
    self.mark_layout_dirty();
    true
  }

  pub(crate) fn set_placeholder(&self, placeholder: impl Into<String>) {
    let placeholder = placeholder.into();
    let mut inner = self.inner.lock().unwrap();
    if inner.placeholder.as_deref() == Some(placeholder.as_str()) {
      return;
    }
    inner.placeholder = Some(placeholder);
    drop(inner);
    self.mark_layout_dirty();
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
    let text = match self.overflow() {
      TextInputOverflow::Multiline => text,
      TextInputOverflow::Scroll => text.replace(['\r', '\n'], " "),
    };
    match self.mask() {
      Some(mask) if !self.is_showing_placeholder() => mask_text(&text, mask),
      _ => text,
    }
  }

  /// Text used to compute caret geometry. Masked the same way the rendered text
  /// is, so caret x-coordinates line up with the displayed mask glyphs.
  pub(crate) fn caret_source_text(&self) -> String {
    let value = self.value();
    match self.mask() {
      Some(mask) => mask_text(&value, mask),
      None => value,
    }
  }

  /// Caret positions are computed over the masked string, so their `index` is a
  /// byte offset into the mask — not the real value. Remap each back to the real
  /// value's char boundaries so editing and hit-testing stay correct.
  pub(crate) fn remap_caret_positions(&self, positions: &mut [CaretPosition]) {
    let Some(mask) = self.mask() else {
      return;
    };
    let mask_len = mask.len_utf8();
    let value = self.value();
    let boundaries: Vec<usize> = value
      .char_indices()
      .map(|(index, _)| index)
      .chain(std::iter::once(value.len()))
      .collect();
    for position in positions.iter_mut() {
      let char_index = position.index / mask_len;
      position.index = boundaries.get(char_index).copied().unwrap_or(value.len());
    }
  }

  pub(crate) fn set_mask(&self, mask: Option<char>) {
    let mut inner = self.inner.lock().unwrap();
    if inner.mask == mask {
      return;
    }
    inner.mask = mask;
    drop(inner);
    self.mark_layout_dirty();
  }

  pub(crate) fn mask(&self) -> Option<char> {
    self.inner.lock().unwrap().mask
  }

  pub(crate) fn insert(&self, text: &str, keyboard: &KeyboardEvent) -> bool {
    if text.is_empty() {
      return false;
    }
    if !self.fire_input(keyboard) {
      return true;
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
    drop(inner);
    self.mark_layout_dirty();
    true
  }

  pub(crate) fn insert_newline(&self, keyboard: &KeyboardEvent) -> bool {
    if self.overflow() != TextInputOverflow::Multiline {
      return false;
    }
    self.insert("\n", keyboard)
  }

  pub(crate) fn backspace(&self, keyboard: &KeyboardEvent) -> bool {
    if self.has_selection() {
      if !self.fire_input(keyboard) {
        return true;
      }
      self.push_undo_snapshot();
      self.delete_selection_if_present();
      return true;
    }

    let mut caret = self.inner.lock().unwrap().caret;
    if caret == 0 {
      return false;
    }
    if !self.fire_input(keyboard) {
      return true;
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
    self.mark_layout_dirty();
    true
  }

  pub(crate) fn delete(&self, keyboard: &KeyboardEvent) -> bool {
    if self.has_selection() {
      if !self.fire_input(keyboard) {
        return true;
      }
      self.push_undo_snapshot();
      self.delete_selection_if_present();
      return true;
    }

    let mut caret = self.inner.lock().unwrap().caret;
    let value = self.value();
    caret = clamp_to_char_boundary(&value, caret);
    if caret >= value.len() {
      return false;
    }
    if !self.fire_input(keyboard) {
      return true;
    }

    self.push_undo_snapshot();
    self.value.update(|value| {
      let next = next_char_boundary(value, caret);
      value.replace_range(caret..next, "");
    });
    self.inner.lock().unwrap().caret = caret;
    self.mark_layout_dirty();
    true
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
    drop(inner);
    self.mark_layout_dirty();
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
    drop(inner);
    self.mark_layout_dirty();
  }

  pub(crate) fn move_up(&self, selecting: bool) {
    let value = self.value();
    let mut inner = self.inner.lock().unwrap();
    let caret = clamp_to_char_boundary(&value, inner.caret);
    let (line_start, _) = line_bounds(&value, caret);
    if line_start == 0 {
      move_inner_to(&mut inner, 0, selecting);
      drop(inner);
      self.mark_layout_dirty();
      return;
    }

    let target_x = caret_x_for_index(&inner.caret_positions, caret);
    let previous_line_end = line_start - 1;
    let (previous_line_start, previous_line_end) = line_bounds(&value, previous_line_end);
    let target = closest_caret_in_range(&inner.caret_positions, previous_line_start, previous_line_end, target_x);
    move_inner_to(&mut inner, target, selecting);
    drop(inner);
    self.mark_layout_dirty();
  }

  pub(crate) fn move_down(&self, selecting: bool) {
    let value = self.value();
    let mut inner = self.inner.lock().unwrap();
    let caret = clamp_to_char_boundary(&value, inner.caret);
    let (_, line_end) = line_bounds(&value, caret);
    if line_end >= value.len() {
      move_inner_to(&mut inner, value.len(), selecting);
      drop(inner);
      self.mark_layout_dirty();
      return;
    }

    let target_x = caret_x_for_index(&inner.caret_positions, caret);
    let next_line_start = line_end + 1;
    let (_, next_line_end) = line_bounds(&value, next_line_start);
    let target = closest_caret_in_range(&inner.caret_positions, next_line_start, next_line_end, target_x);
    move_inner_to(&mut inner, target, selecting);
    drop(inner);
    self.mark_layout_dirty();
  }

  pub(crate) fn move_word_left(&self, selecting: bool) {
    let value = self.value();
    let mut inner = self.inner.lock().unwrap();
    let target = previous_word_boundary(&value, inner.caret);
    move_inner_to(&mut inner, target, selecting);
    drop(inner);
    self.mark_layout_dirty();
  }

  pub(crate) fn move_word_right(&self, selecting: bool) {
    let value = self.value();
    let mut inner = self.inner.lock().unwrap();
    let target = next_word_boundary(&value, inner.caret);
    move_inner_to(&mut inner, target, selecting);
    drop(inner);
    self.mark_layout_dirty();
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
    drop(inner);
    self.mark_layout_dirty();
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
    drop(inner);
    self.mark_layout_dirty();
  }

  pub(crate) fn select_all(&self) {
    let len = self.value().len();
    let mut inner = self.inner.lock().unwrap();
    inner.selection_anchor = Some(0);
    inner.caret = len;
    drop(inner);
    self.mark_layout_dirty();
  }

  pub(crate) fn selected_text(&self) -> Option<String> {
    let value = self.value();
    let inner = self.inner.lock().unwrap();
    let (start, end) = selection_range_indices(&value, inner.selection_anchor, inner.caret)?;
    Some(value[start..end].to_owned())
  }

  pub(crate) fn cut_selection(&self, keyboard: &KeyboardEvent) -> Option<String> {
    let selected = self.selected_text()?;
    if !self.fire_input(keyboard) {
      return Some(selected);
    }
    self.push_undo_snapshot();
    self.delete_selection_if_present();
    Some(selected)
  }

  pub(crate) fn undo(&self, keyboard: &KeyboardEvent) -> bool {
    let current = self.snapshot();
    let Some(snapshot) = self.inner.lock().unwrap().undo_stack.pop() else {
      return false;
    };
    if !self.fire_input(keyboard) {
      self.push_undo_snapshot_value(snapshot);
      return true;
    }
    self.push_redo_snapshot(current);
    self.restore_snapshot(snapshot);
    self.mark_layout_dirty();
    true
  }

  pub(crate) fn redo(&self, keyboard: &KeyboardEvent) -> bool {
    let current = self.snapshot();
    let Some(snapshot) = self.inner.lock().unwrap().redo_stack.pop() else {
      return false;
    };
    if !self.fire_input(keyboard) {
      self.push_redo_snapshot(snapshot);
      return true;
    }
    self.push_undo_snapshot_value(current);
    self.restore_snapshot(snapshot);
    self.mark_layout_dirty();
    true
  }

  pub(crate) fn begin_selection_at_point(&self, x: f32, y: f32) {
    let caret = self.closest_caret_to_point(x, y);
    let mut inner = self.inner.lock().unwrap();
    inner.caret = caret;
    inner.selection_anchor = Some(caret);
    drop(inner);
    self.mark_layout_dirty();
  }

  pub(crate) fn update_selection_to_point(&self, x: f32, y: f32) {
    let caret = self.closest_caret_to_point(x, y);
    self.inner.lock().unwrap().caret = caret;
    self.mark_layout_dirty();
  }

  pub(crate) fn set_caret_from_point(&self, x: f32, y: f32) {
    let caret = self.closest_caret_to_point(x, y);
    let mut inner = self.inner.lock().unwrap();
    inner.caret = caret;
    inner.selection_anchor = None;
    drop(inner);
    self.mark_layout_dirty();
  }

  pub(crate) fn select_word_at_point(&self, x: f32, y: f32) {
    let caret = self.closest_caret_to_point(x, y);
    let value = self.value();
    let (start, end) = word_selection_bounds(&value, caret);
    let mut inner = self.inner.lock().unwrap();
    inner.selection_anchor = Some(start);
    inner.caret = end;
    drop(inner);
    self.mark_layout_dirty();
  }

  pub(crate) fn select_line_at_point(&self, x: f32, y: f32) {
    let caret = self.closest_caret_to_point(x, y);
    let value = self.value();
    let (start, end) = line_bounds(&value, caret);
    let mut inner = self.inner.lock().unwrap();
    inner.selection_anchor = Some(start);
    inner.caret = end;
    drop(inner);
    self.mark_layout_dirty();
  }

  pub(crate) fn copy_runtime_state_from(&self, old: &Self, layout_signature_matches: bool) {
    if Arc::ptr_eq(&self.inner, &old.inner) {
      return;
    }
    let old_inner = old.inner.lock().unwrap();
    let old_caret = old_inner.caret;
    let old_last_value = old_inner.last_value.clone();
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
    let layout_dirty = if layout_signature_matches {
      old.layout_dirty.load(Ordering::Relaxed)
    } else {
      self.layout_dirty.load(Ordering::Relaxed) || old.layout_dirty.load(Ordering::Relaxed)
    };
    let len = self.value().len();
    let mut inner = self.inner.lock().unwrap();
    inner.last_value = old_last_value;
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
    self.layout_dirty.store(layout_dirty, Ordering::Relaxed);
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
    let mut inner = self.inner.lock().unwrap();
    if inner.overflow == overflow {
      return;
    }
    inner.overflow = overflow;
    drop(inner);
    self.mark_layout_dirty();
  }

  pub(crate) fn overflow(&self) -> TextInputOverflow {
    self.inner.lock().unwrap().overflow
  }

  pub(crate) fn set_rows(&self, min_rows: usize, max_rows: usize) {
    let min_rows = min_rows.max(1);
    let max_rows = max_rows.max(min_rows);
    let mut inner = self.inner.lock().unwrap();
    if inner.overflow == TextInputOverflow::Multiline
      && inner.min_rows == Some(min_rows)
      && inner.max_rows == Some(max_rows)
    {
      return;
    }
    inner.overflow = TextInputOverflow::Multiline;
    inner.min_rows = Some(min_rows);
    inner.max_rows = Some(max_rows);
    drop(inner);
    self.mark_layout_dirty();
  }

  pub(crate) fn set_min_rows(&self, min_rows: usize) {
    let min_rows = min_rows.max(1);
    let mut inner = self.inner.lock().unwrap();
    let next_max_rows = inner.max_rows.map(|max_rows| max_rows.max(min_rows));
    if inner.overflow == TextInputOverflow::Multiline
      && inner.min_rows == Some(min_rows)
      && inner.max_rows == next_max_rows
    {
      return;
    }
    inner.overflow = TextInputOverflow::Multiline;
    inner.min_rows = Some(min_rows);
    inner.max_rows = next_max_rows;
    drop(inner);
    self.mark_layout_dirty();
  }

  pub(crate) fn set_max_rows(&self, max_rows: usize) {
    let max_rows = max_rows.max(1);
    let mut inner = self.inner.lock().unwrap();
    let next_min_rows = inner.min_rows.map(|min_rows| min_rows.min(max_rows));
    if inner.overflow == TextInputOverflow::Multiline
      && inner.max_rows == Some(max_rows)
      && inner.min_rows == next_min_rows
    {
      return;
    }
    inner.overflow = TextInputOverflow::Multiline;
    inner.max_rows = Some(max_rows);
    inner.min_rows = next_min_rows;
    drop(inner);
    self.mark_layout_dirty();
  }

  pub(crate) fn set_rows_exact(&self, rows: usize) {
    let rows = rows.max(1);
    let mut inner = self.inner.lock().unwrap();
    if inner.overflow == TextInputOverflow::Multiline && inner.min_rows == Some(rows) && inner.max_rows == Some(rows) {
      return;
    }
    inner.overflow = TextInputOverflow::Multiline;
    inner.min_rows = Some(rows);
    inner.max_rows = Some(rows);
    drop(inner);
    self.mark_layout_dirty();
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
    drop(inner);
    self.mark_layout_dirty();
    Some(start)
  }

  pub(crate) fn layout_signature(&self) -> TextInputLayoutSignature {
    let inner = self.inner.lock().unwrap();
    TextInputLayoutSignature {
      placeholder: inner.placeholder.clone(),
      overflow: inner.overflow,
      min_rows: inner.min_rows,
      max_rows: inner.max_rows,
      mask: inner.mask,
    }
  }

  pub(crate) fn take_layout_dirty(&self) -> bool {
    self.layout_dirty.swap(false, Ordering::Relaxed)
  }

  fn mark_layout_dirty(&self) {
    self.layout_dirty.store(true, Ordering::Relaxed);
  }

  fn fire_input(&self, keyboard: &KeyboardEvent) -> bool {
    let handler = self.on_input.lock().unwrap().clone();
    let Some(handler) = handler else {
      return true;
    };

    let event = TextInputEvent::new(self.value.clone(), keyboard.clone());
    handler(&event);
    self.sync_external_value();
    !event.default_prevented()
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

fn mask_text(text: &str, mask: char) -> String {
  let mut out = String::with_capacity(text.chars().count() * mask.len_utf8());
  for _ in text.chars() {
    out.push(mask);
  }
  out
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
  pub(crate) fn resolve_resource_images(
    &self,
    mut resolve: impl FnMut(&Arc<str>) -> Option<crate::images::ImageData>,
  ) -> bool {
    fn resolve_style(
      style: &mut CheckboxStyle,
      resolve: &mut impl FnMut(&Arc<str>) -> Option<crate::images::ImageData>,
    ) -> bool {
      let Some(key) = style.indicator_resource_image.clone() else {
        return false;
      };
      let Some(img) = resolve(&key) else {
        return false;
      };
      if style.indicator_image.as_ref().map(crate::images::ImageData::id) != Some(img.id()) {
        style.indicator_image = Some(img);
        return true;
      }
      false
    }

    let mut inner = self.inner.lock().unwrap();
    let mut changed = resolve_style(&mut inner.style, &mut resolve);
    if let Some(style) = &mut inner.checked_style {
      changed |= resolve_style(style, &mut resolve);
    }
    if let Some(style) = &mut inner.hovered_style {
      changed |= resolve_style(style, &mut resolve);
    }
    if let Some(style) = &mut inner.checked_hovered_style {
      changed |= resolve_style(style, &mut resolve);
    }
    changed
  }
}

#[derive(Clone)]
pub(crate) struct SliderState {
  value: SliderValue,
  inner: Arc<Mutex<SliderInner>>,
}

#[derive(Clone)]
enum SliderValue {
  Int(Signal<i32>),
  Float(Signal<f32>),
}

struct SliderInner {
  min: f32,
  max: f32,
  step: f32,
  track_style: SliderPartStyle,
  track_hovered_style: Option<SliderPartStyle>,
  thumb_style: SliderPartStyle,
  thumb_hovered_style: Option<SliderPartStyle>,
  hovered: bool,
  drag_ratio: Option<f32>,
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
      value: SliderValue::Int(value),
      inner: Arc::new(Mutex::new(SliderInner {
        min: 0.0,
        max: 1.0,
        step: 1.0,
        track_style: SliderPartStyle::new(),
        track_hovered_style: None,
        thumb_style: SliderPartStyle::new(),
        thumb_hovered_style: None,
        hovered: false,
        drag_ratio: None,
      })),
    }
  }

  pub(crate) fn new_f32(value: Signal<f32>) -> Self {
    Self {
      value: SliderValue::Float(value),
      inner: Arc::new(Mutex::new(SliderInner {
        min: 0.0,
        max: 1.0,
        step: 0.01,
        track_style: SliderPartStyle::new(),
        track_hovered_style: None,
        thumb_style: SliderPartStyle::new(),
        thumb_hovered_style: None,
        hovered: false,
        drag_ratio: None,
      })),
    }
  }

  #[cfg(feature = "form")]
  pub(crate) fn value_string(&self) -> String {
    match &self.value {
      SliderValue::Int(value) => value.get_untracked().to_string(),
      SliderValue::Float(value) => value.get_untracked().to_string(),
    }
  }

  fn value_f32(&self) -> f32 {
    match &self.value {
      SliderValue::Int(value) => value.get_untracked() as f32,
      SliderValue::Float(value) => value.get_untracked(),
    }
  }

  fn set_value(&self, value: f32) -> bool {
    match &self.value {
      SliderValue::Int(signal) => {
        let next = value.round() as i32;
        let current = signal.get_untracked();
        if current != next {
          signal.set(next);
          return true;
        }
      }
      SliderValue::Float(signal) => {
        let current = signal.get_untracked();
        if (current - value).abs() > f32::EPSILON {
          signal.set(value);
          return true;
        }
      }
    }
    false
  }

  pub(crate) fn visual_ratio(&self) -> f32 {
    let inner = self.inner.lock().unwrap();
    let value_ratio = slider_ratio_for_value(self.value_f32(), inner.min, inner.max);
    inner
      .drag_ratio
      .filter(|ratio| slider_drag_ratio_matches_value(*ratio, value_ratio, inner.min, inner.max, inner.step))
      .unwrap_or(value_ratio)
  }

  pub(crate) fn set_range(&self, min: i32, max: i32) {
    self.set_range_f32(min as f32, max as f32);
  }

  pub(crate) fn set_range_f32(&self, min: f32, max: f32) {
    let mut inner = self.inner.lock().unwrap();
    inner.min = min;
    inner.max = max.max(min);
    let current = self.value_f32();
    let clamped = snap_slider_value(current, inner.min, inner.max, inner.step);
    drop(inner);
    self.set_value(clamped);
  }

  pub(crate) fn set_step(&self, step: f32) {
    let mut inner = self.inner.lock().unwrap();
    inner.step = step.abs().max(f32::EPSILON);
    let current = self.value_f32();
    let stepped = snap_slider_value(current, inner.min, inner.max, inner.step);
    drop(inner);
    self.set_value(stepped);
  }

  pub(crate) fn set_from_ratio(&self, ratio: f32) -> bool {
    let inner = self.inner.lock().unwrap();
    let raw_value = inner.min + ratio.clamp(0.0, 1.0) * (inner.max - inner.min);
    let value = snap_slider_value(raw_value, inner.min, inner.max, inner.step);
    drop(inner);
    self.set_value(value)
  }

  pub(crate) fn set_drag_ratio(&self, ratio: f32) {
    self.inner.lock().unwrap().drag_ratio = Some(ratio.clamp(0.0, 1.0));
  }

  pub(crate) fn clear_drag_ratio(&self) {
    self.inner.lock().unwrap().drag_ratio = None;
  }

  pub(crate) fn is_dragging(&self) -> bool {
    self.inner.lock().unwrap().drag_ratio.is_some()
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
    let mut inner = self.inner.lock().unwrap();
    inner.drag_ratio = None;
    let current = self.value_f32();
    let next = snap_slider_value(current + delta as f32 * inner.step, inner.min, inner.max, inner.step);
    drop(inner);
    self.set_value(next);
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

  pub(crate) fn copy_runtime_state_from(&self, old: &Self) {
    if Arc::ptr_eq(&self.inner, &old.inner) {
      return;
    }
    let old_inner = old.inner.lock().unwrap();
    let mut inner = self.inner.lock().unwrap();
    inner.hovered = old_inner.hovered;
    inner.drag_ratio = old_inner.drag_ratio.filter(|ratio| {
      let current = slider_ratio_for_value(self.value_f32(), inner.min, inner.max);
      slider_drag_ratio_matches_value(*ratio, current, inner.min, inner.max, inner.step)
    });
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
  ) -> bool {
    fn resolve_style(
      style: &mut SliderPartStyle,
      resolve: &mut impl FnMut(&std::sync::Arc<str>) -> Option<crate::images::ImageData>,
    ) -> bool {
      let Some(key) = style.background_resource_image.clone() else {
        return false;
      };
      let Some(img) = resolve(&key) else {
        return false;
      };
      if style.background_image.as_ref().map(crate::images::ImageData::id) != Some(img.id()) {
        style.background_image = Some(img);
        return true;
      }
      false
    }

    let mut inner = self.inner.lock().unwrap();
    let mut changed = resolve_style(&mut inner.track_style, &mut resolve);
    if let Some(style) = &mut inner.track_hovered_style {
      changed |= resolve_style(style, &mut resolve);
    }
    changed |= resolve_style(&mut inner.thumb_style, &mut resolve);
    if let Some(style) = &mut inner.thumb_hovered_style {
      changed |= resolve_style(style, &mut resolve);
    }
    changed
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
      track_x + thumb_width * 0.5 + thumb_travel_width * self.visual_ratio()
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

fn slider_ratio_for_value(value: f32, min: f32, max: f32) -> f32 {
  if max <= min {
    return 0.0;
  }
  ((value - min) / (max - min)).clamp(0.0, 1.0)
}

fn slider_drag_ratio_matches_value(ratio: f32, value_ratio: f32, min: f32, max: f32, step: f32) -> bool {
  let range = (max - min).abs();
  let tolerance = if range > f32::EPSILON {
    (step.abs() / range * 0.75).max(0.001)
  } else {
    0.001
  };
  (ratio - value_ratio).abs() <= tolerance
}

fn snap_slider_value(value: f32, min: f32, max: f32, step: f32) -> f32 {
  let value = value.clamp(min, max);
  let step = step.abs();
  if step <= f32::EPSILON {
    return value;
  }
  let steps = ((value - min) / step).round();
  let snapped = min + steps * step;
  snapped.clamp(min, max)
}

pub(crate) type SelectChangeCallback = Arc<dyn Fn(usize) + Send + Sync>;

#[derive(Clone)]
pub(crate) struct SelectState {
  inner: Arc<Mutex<SelectInner>>,
}

struct SelectInner {
  // Per-render config, rebuilt by the generic `Select<T>` wrapper each render.
  labels: Vec<Arc<str>>,
  selected: Vec<usize>,
  multiple: bool,
  placeholder: Option<Arc<str>>,
  style: SelectStyle,
  on_change: Option<SelectChangeCallback>,
  // Runtime state, preserved across re-renders via `copy_runtime_state_from`.
  open: bool,
  highlighted: Option<usize>,
}

impl SelectState {
  pub(crate) fn new() -> Self {
    Self {
      inner: Arc::new(Mutex::new(SelectInner {
        labels: Vec::new(),
        selected: Vec::new(),
        multiple: false,
        placeholder: None,
        style: SelectStyle::new(),
        on_change: None,
        open: false,
        highlighted: None,
      })),
    }
  }

  pub(crate) fn set_labels(&self, labels: Vec<Arc<str>>) {
    self.inner.lock().unwrap().labels = labels;
  }

  pub(crate) fn set_selected(&self, selected: Vec<usize>) {
    self.inner.lock().unwrap().selected = selected;
  }

  pub(crate) fn set_multiple(&self, multiple: bool) {
    self.inner.lock().unwrap().multiple = multiple;
  }

  pub(crate) fn set_placeholder(&self, placeholder: Option<Arc<str>>) {
    self.inner.lock().unwrap().placeholder = placeholder;
  }

  pub(crate) fn set_style(&self, style: SelectStyle) {
    self.inner.lock().unwrap().style = style;
  }

  pub(crate) fn set_on_change(&self, on_change: SelectChangeCallback) {
    self.inner.lock().unwrap().on_change = Some(on_change);
  }

  pub(crate) fn labels(&self) -> Vec<Arc<str>> {
    self.inner.lock().unwrap().labels.clone()
  }

  pub(crate) fn multiple(&self) -> bool {
    self.inner.lock().unwrap().multiple
  }

  pub(crate) fn style(&self) -> SelectStyle {
    self.inner.lock().unwrap().style.clone()
  }

  pub(crate) fn is_open(&self) -> bool {
    self.inner.lock().unwrap().open
  }

  pub(crate) fn highlighted(&self) -> Option<usize> {
    self.inner.lock().unwrap().highlighted
  }

  pub(crate) fn is_selected(&self, index: usize) -> bool {
    self.inner.lock().unwrap().selected.contains(&index)
  }

  #[cfg(feature = "form")]
  pub(crate) fn selected_labels(&self) -> Vec<Arc<str>> {
    let inner = self.inner.lock().unwrap();
    inner
      .selected
      .iter()
      .filter_map(|index| inner.labels.get(*index).cloned())
      .collect()
  }

  /// Open the menu if closed; otherwise commit the highlighted option.
  pub(crate) fn activate(&self) {
    if self.is_open() {
      let highlighted = self.highlighted().unwrap_or(0);
      self.commit(highlighted);
    } else {
      self.open_with_highlight();
    }
  }

  pub(crate) fn set_open(&self, open: bool) {
    let mut inner = self.inner.lock().unwrap();
    inner.open = open;
    if !open {
      inner.highlighted = None;
    }
  }

  pub(crate) fn open_with_highlight(&self) {
    let mut inner = self.inner.lock().unwrap();
    inner.open = true;
    inner.highlighted = Some(inner.selected.first().copied().unwrap_or(0));
  }

  pub(crate) fn toggle_open(&self) {
    let open = self.inner.lock().unwrap().open;
    self.set_open(!open);
  }

  pub(crate) fn move_highlight(&self, delta: i32) {
    let mut inner = self.inner.lock().unwrap();
    let count = inner.labels.len();
    if count == 0 {
      return;
    }
    let current = inner
      .highlighted
      .unwrap_or_else(|| if delta < 0 { 0 } else { count - 1 });
    let next = (current as i32 + delta).rem_euclid(count as i32);
    inner.highlighted = Some(next as usize);
  }

  /// Commit a click on option `index`: fire the change callback and, for
  /// single-select, close the menu. Multi-select keeps the menu open.
  pub(crate) fn commit(&self, index: usize) {
    let (callback, multiple) = {
      let inner = self.inner.lock().unwrap();
      (inner.on_change.clone(), inner.multiple)
    };
    if let Some(callback) = callback {
      callback(index);
    }
    if !multiple {
      self.set_open(false);
    }
  }

  pub(crate) fn copy_runtime_state_from(&self, old: &SelectState) {
    let (open, highlighted) = {
      let old_inner = old.inner.lock().unwrap();
      (old_inner.open, old_inner.highlighted)
    };
    let mut inner = self.inner.lock().unwrap();
    inner.open = open;
    let count = inner.labels.len();
    inner.highlighted = highlighted.and_then(|index| if count == 0 { None } else { Some(index.min(count - 1)) });
  }
}
