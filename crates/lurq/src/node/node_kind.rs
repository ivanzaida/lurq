use std::sync::{
  Arc, Mutex,
  atomic::{AtomicBool, Ordering},
};

pub(crate) use crate::node::select_state::{SelectChangeCallback, SelectState};
use crate::{
  app::{
    events::{KeyboardEvent, TextInputEvent},
    theme::{ThemePalette, ThemeTypography, TypographyStyle},
  },
  core::Signal,
  layout::text_style::{TextAlign, TextStyle},
  node::{
    CheckboxStyle, EventHandler, IntoTextInputEventHandler, SliderPartStyle, TextColor, TextTransformMode,
    text_selection::{
      CaretPosition, CaretPositions, TextSelectionRange, caret_x_for_index, caret_y_for_index, clamp_to_char_boundary,
      closest_caret_on_adjacent_row, closest_caret_to_point, line_bounds, next_char_boundary, next_word_boundary,
      previous_char_boundary, previous_word_boundary, selection_range_indices, selection_ranges_for_positions,
      word_selection_bounds,
    },
  },
};

mod checkbox;
mod slider;
mod text_input;

pub(crate) use checkbox::CheckboxState;
pub(crate) use slider::{SliderPartRect, SliderState};
pub(crate) use text_input::{TextInputState, WeakTextInputState};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TextInputOverflow {
  Multiline,
  #[default]
  Scroll,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TextInputOverflowAnchor {
  #[default]
  Start,
  End,
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
  #[cfg(feature = "canvas")]
  Canvas {
    canvas: crate::canvas::CanvasHandle,
  },
  Text {
    state: TextState,
    style: TextStyleSource,
    transform_mode: TextTransformMode,
  },
  #[cfg(feature = "markdown")]
  RichText {
    state: TextState,
    spans: Vec<crate::layout::quad::RichTextSpan>,
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
  #[cfg(feature = "raster")]
  Image {
    data: crate::images::ImageData,
  },
  #[cfg(feature = "raster")]
  Video {
    data: crate::images::ImageData,
    fit: crate::node::BackgroundSize,
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
  letter_spacing: Option<f32>,
  font_features: Option<crate::layout::text_style::FontFeatures>,
  shadow: Option<crate::layout::text_style::TextShadow>,
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
      letter_spacing: None,
      font_features: None,
      shadow: None,
    }
  }

  pub(crate) fn explicit(style: TextStyle) -> Self {
    Self {
      base: TextStyleBase::Explicit(style),
      color: None,
      text_align: None,
      letter_spacing: None,
      font_features: None,
      shadow: None,
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

  pub(crate) fn set_letter_spacing(&mut self, letter_spacing: f32) {
    self.letter_spacing = Some(letter_spacing);
  }

  pub(crate) fn set_font_features(&mut self, font_features: crate::layout::text_style::FontFeatures) {
    self.font_features = Some(font_features);
  }

  pub(crate) fn set_shadow(&mut self, shadow: crate::layout::text_style::TextShadow) {
    self.shadow = Some(shadow);
  }

  pub(crate) fn resolve(&self, typography: &ThemeTypography, palette: &ThemePalette) -> TextStyle {
    let mut style = match &self.base {
      TextStyleBase::Default => typography.default_style().clone(),
      TextStyleBase::Typography(style) => typography.resolve_or_default(*style),
      TextStyleBase::Explicit(style) => style.clone(),
    };
    if let Some(color) = self.color.as_ref().and_then(|color| color.resolve(palette)) {
      style.color = color;
    }
    if let Some(text_align) = self.text_align {
      style.text_align = text_align;
    }
    if let Some(letter_spacing) = self.letter_spacing {
      style.letter_spacing = letter_spacing;
    }
    if let Some(font_features) = &self.font_features {
      style.font_features = font_features.clone();
    }
    if let Some(shadow) = self.shadow {
      style.shadow = Some(shadow);
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
  /// Inspectors show `REDACTED` instead of the content (`Text::sensitive`).
  sensitive: bool,
  caret: usize,
  selection_anchor: Option<usize>,
  caret_positions: CaretPositions,
  display_text: Option<Arc<str>>,
  render_wrap: bool,
}

impl TextState {
  pub(crate) fn new() -> Self {
    Self {
      inner: Arc::new(Mutex::new(TextInner {
        selectable: false,
        sensitive: false,
        caret: 0,
        selection_anchor: None,
        caret_positions: vec![CaretPosition {
          index: 0,
          x: 0.0,
          y: 0.0,
        }]
        .into(),
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

  pub(crate) fn set_sensitive(&self) {
    self.inner.lock().unwrap().sensitive = true;
  }

  pub(crate) fn is_sensitive(&self) -> bool {
    self.inner.lock().unwrap().sensitive
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

  /// Writes what laying the text out under one set of constraints produced:
  /// the string to draw instead of the content, whether the renderer wraps,
  /// and, for selectable text, the caret geometry (`None` keeps the current
  /// positions).
  pub(crate) fn set_layout_output(
    &self,
    display_text: Option<&Arc<str>>,
    render_wrap: bool,
    caret_positions: Option<&CaretPositions>,
  ) {
    let mut inner = self.inner.lock().unwrap();
    // Serving a cached layout usually writes back what the state already
    // holds; skip the reference-count traffic then.
    if inner.display_text.as_ref().map(Arc::as_ptr) != display_text.map(Arc::as_ptr) {
      inner.display_text = display_text.cloned();
    }
    inner.render_wrap = render_wrap;
    if let Some(caret_positions) = caret_positions {
      inner.caret_positions = caret_positions.clone();
    }
  }

  pub(crate) fn display_text(&self) -> Option<Arc<str>> {
    self.inner.lock().unwrap().display_text.clone()
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
