use super::*;

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
  /// Whether the app set the track (or thumb) style; without one the part
  /// takes the theme's fill.
  track_styled: bool,
  track_hovered_style: Option<SliderPartStyle>,
  fill_style: Option<SliderPartStyle>,
  fill_hovered_style: Option<SliderPartStyle>,
  thumb_style: SliderPartStyle,
  thumb_styled: bool,
  thumb_hovered_style: Option<SliderPartStyle>,
  thumb_focused_style: Option<SliderPartStyle>,
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
        track_styled: false,
        track_hovered_style: None,
        fill_style: None,
        fill_hovered_style: None,
        thumb_style: SliderPartStyle::new(),
        thumb_styled: false,
        thumb_hovered_style: None,
        thumb_focused_style: None,
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
        track_styled: false,
        track_hovered_style: None,
        fill_style: None,
        fill_hovered_style: None,
        thumb_style: SliderPartStyle::new(),
        thumb_styled: false,
        thumb_hovered_style: None,
        thumb_focused_style: None,
        hovered: false,
        drag_ratio: None,
      })),
    }
  }

  #[cfg(any(feature = "form", feature = "mcp"))]
  pub(crate) fn value_string(&self) -> String {
    match &self.value {
      SliderValue::Int(value) => value.get_untracked().to_string(),
      SliderValue::Float(value) => value.get_untracked().to_string(),
    }
  }

  pub(crate) fn value_f32(&self) -> f32 {
    match &self.value {
      SliderValue::Int(value) => value.get_untracked() as f32,
      SliderValue::Float(value) => value.get_untracked(),
    }
  }

  pub(crate) fn value_id(&self) -> usize {
    match &self.value {
      SliderValue::Int(value) => value.id(),
      SliderValue::Float(value) => value.id(),
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

  /// Set the slider from outside the pointer/keyboard path (automation
  /// tooling); the value is snapped to the slider's range and step.
  #[cfg(feature = "mcp")]
  pub(crate) fn set_value_external(&self, value: f32) -> bool {
    let inner = self.inner.lock().unwrap();
    let value = snap_slider_value(value, inner.min, inner.max, inner.step);
    drop(inner);
    self.set_value(value)
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
    let mut inner = self.inner.lock().unwrap();
    inner.track_style = style;
    inner.track_styled = true;
  }

  pub(crate) fn set_track_hovered_style(&self, style: SliderPartStyle) {
    self.inner.lock().unwrap().track_hovered_style = Some(style);
  }

  pub(crate) fn set_fill_style(&self, style: SliderPartStyle) {
    self.inner.lock().unwrap().fill_style = Some(style);
  }

  pub(crate) fn set_fill_hovered_style(&self, style: SliderPartStyle) {
    self.inner.lock().unwrap().fill_hovered_style = Some(style);
  }

  /// Whether the app set the track style; without one an unset track fill
  /// falls back to the theme.
  pub(crate) fn track_styled(&self) -> bool {
    self.inner.lock().unwrap().track_styled
  }

  /// Whether the app set the thumb style; see [`SliderState::track_styled`].
  pub(crate) fn thumb_styled(&self) -> bool {
    self.inner.lock().unwrap().thumb_styled
  }

  pub(crate) fn set_thumb_style(&self, style: SliderPartStyle) {
    let mut inner = self.inner.lock().unwrap();
    inner.thumb_style = style;
    inner.thumb_styled = true;
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

  pub(crate) fn fill_style(&self, hovered: bool) -> Option<SliderPartStyle> {
    let inner = self.inner.lock().unwrap();
    let mut style = inner.fill_style.clone()?;
    if hovered && let Some(hovered_style) = &inner.fill_hovered_style {
      style.merge_from(hovered_style);
    }
    Some(style)
  }

  pub(crate) fn set_thumb_focused_style(&self, style: SliderPartStyle) {
    self.inner.lock().unwrap().thumb_focused_style = Some(style);
  }

  /// Layers: base, focused (paint only; its size is ignored so focus never
  /// moves the thumb), hovered.
  pub(crate) fn thumb_style(&self, hovered: bool, focused: bool) -> SliderPartStyle {
    let inner = self.inner.lock().unwrap();
    let mut style = inner.thumb_style.clone();
    if focused && let Some(focused_style) = &inner.thumb_focused_style {
      let (width, height) = (style.width, style.height);
      style.merge_from(focused_style);
      style.width = width;
      style.height = height;
    }
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
    if let Some(style) = &mut inner.fill_style {
      changed |= resolve_style(style, &mut resolve);
    }
    if let Some(style) = &mut inner.fill_hovered_style {
      changed |= resolve_style(style, &mut resolve);
    }
    changed |= resolve_style(&mut inner.thumb_style, &mut resolve);
    if let Some(style) = &mut inner.thumb_hovered_style {
      changed |= resolve_style(style, &mut resolve);
    }
    if let Some(style) = &mut inner.thumb_focused_style {
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
    // Only the size is read, and the focused layer never changes it.
    let thumb_style = self.thumb_style(hovered, false);
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
