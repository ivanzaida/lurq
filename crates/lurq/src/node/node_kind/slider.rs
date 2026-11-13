use super::*;
use crate::node::slider_style::SliderOrientation;

#[derive(Clone)]
pub(crate) struct SliderState {
  value: SliderValue,
  inner: Arc<Mutex<SliderInner>>,
}

#[derive(Clone)]
enum SliderValue {
  Int(Signal<i32>),
  Float(Signal<f32>),
  /// A range: its low value and its high one, the low never past the high.
  IntRange(Signal<(i32, i32)>),
  FloatRange(Signal<(f32, f32)>),
}

/// One of a slider's thumbs. A slider of one value has only the low one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SliderThumb {
  Low,
  High,
}

struct SliderInner {
  min: f32,
  max: f32,
  step: f32,
  orientation: SliderOrientation,
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
  /// The thumb the keys move and the drag holds: of a range, the one last pressed or reached by Tab.
  active: SliderThumb,
  /// Where the active thumb is drawn while it is dragged, between the values it snaps to.
  drag_ratio: Option<f32>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SliderLayoutSignature {
  vertical: bool,
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

impl SliderPartRect {
  fn center(&self) -> (f32, f32) {
    (self.x + self.width * 0.5, self.y + self.height * 0.5)
  }
}

/// Where a slider's parts are: its track and each of its thumbs.
#[derive(Clone, Copy, Debug)]
pub(crate) struct SliderParts {
  pub(crate) track: SliderPartRect,
  /// The low thumb and the high one; a slider of one value has the low one only, and the high is the same.
  thumbs: [SliderPartRect; 2],
  range: bool,
  vertical: bool,
}

impl SliderParts {
  pub(crate) fn thumb(&self, thumb: SliderThumb) -> SliderPartRect {
    match thumb {
      SliderThumb::Low => self.thumbs[0],
      SliderThumb::High => self.thumbs[1],
    }
  }

  /// The thumbs there are, low first.
  pub(crate) fn thumbs(&self) -> impl Iterator<Item = (SliderThumb, SliderPartRect)> + '_ {
    let count = if self.range { 2 } else { 1 };
    [SliderThumb::Low, SliderThumb::High]
      .into_iter()
      .zip(self.thumbs)
      .take(count)
  }

  /// The filled part of the track, `thickness` across it (the track's when `None`): from the start of the
  /// track to the thumb, or between a range's thumbs.
  pub(crate) fn fill(&self, thickness: Option<f32>) -> SliderPartRect {
    let track = self.track;
    let (low_x, low_y) = self.thumbs[0].center();
    let (high_x, high_y) = self.thumbs[1].center();
    if self.vertical {
      let width = thickness.unwrap_or(track.width).max(0.0);
      let bottom = track.y + track.height;
      let (from, to) = if self.range { (high_y, low_y) } else { (low_y, bottom) };
      let from = from.clamp(track.y, bottom);
      let to = to.clamp(track.y, bottom);
      SliderPartRect {
        x: track.x + (track.width - width) * 0.5,
        y: from,
        width,
        height: (to - from).max(0.0),
      }
    } else {
      let height = thickness.unwrap_or(track.height).max(0.0);
      let right = track.x + track.width;
      let (from, to) = if self.range { (low_x, high_x) } else { (track.x, low_x) };
      let from = from.clamp(track.x, right);
      let to = to.clamp(track.x, right);
      SliderPartRect {
        x: from,
        y: track.y + (track.height - height) * 0.5,
        width: (to - from).max(0.0),
        height,
      }
    }
  }

  /// The line a thumb's center travels along, from the least value to the greatest.
  pub(crate) fn axis(&self) -> SliderAxis {
    let track = self.track;
    let thumb = self.thumbs[0];
    if self.vertical {
      let travel = track.height - thumb.height;
      let (start, length) = if travel > 0.0 {
        (track.y + track.height - thumb.height * 0.5, travel)
      } else {
        (track.y + track.height, track.height)
      };
      SliderAxis {
        start,
        length,
        vertical: true,
      }
    } else {
      let travel = track.width - thumb.width;
      let (start, length) = if travel > 0.0 {
        (track.x + thumb.width * 0.5, travel)
      } else {
        (track.x, track.width)
      };
      SliderAxis {
        start,
        length,
        vertical: false,
      }
    }
  }
}

/// The line a slider's thumb travels along: where the least value is, how long it is, and which way it runs.
#[derive(Clone, Copy, Debug)]
pub(crate) struct SliderAxis {
  start: f32,
  length: f32,
  vertical: bool,
}

impl SliderAxis {
  /// How far along the line a point is, from 0 at the least value to 1 at the greatest; past either end
  /// beyond them.
  pub(crate) fn ratio(&self, x: f32, y: f32) -> f32 {
    if self.length <= 0.0 {
      return 0.0;
    }
    if self.vertical {
      (self.start - y) / self.length
    } else {
      (x - self.start) / self.length
    }
  }

  /// Moves the line by how far `(x, y)` is from the center of `thumb` along it: a drag that does not bring
  /// the thumb under the pointer, and takes it from where it is.
  pub(crate) fn hold_from(&mut self, x: f32, y: f32, thumb: SliderPartRect) {
    let (center_x, center_y) = thumb.center();
    self.start += if self.vertical { y - center_y } else { x - center_x };
  }
}

impl SliderValue {
  fn id(&self) -> usize {
    match self {
      Self::Int(value) => value.id(),
      Self::Float(value) => value.id(),
      Self::IntRange(value) => value.id(),
      Self::FloatRange(value) => value.id(),
    }
  }

  /// The low value and the high one; of a slider of one value, that value twice.
  fn get(&self) -> (f32, f32) {
    match self {
      Self::Int(value) => {
        let value = value.get_untracked() as f32;
        (value, value)
      }
      Self::Float(value) => {
        let value = value.get_untracked();
        (value, value)
      }
      Self::IntRange(value) => {
        let (low, high) = value.get_untracked();
        (low as f32, high as f32)
      }
      Self::FloatRange(value) => value.get_untracked(),
    }
  }

  /// Sets the values, the low one put below the high; a slider of one value takes the low. Returns whether
  /// the signal changed.
  fn set(&self, low: f32, high: f32) -> bool {
    let (low, high) = (low.min(high), low.max(high));
    match self {
      Self::Int(signal) => {
        let next = low.round() as i32;
        if signal.get_untracked() != next {
          signal.set(next);
          return true;
        }
      }
      Self::Float(signal) => {
        if (signal.get_untracked() - low).abs() > f32::EPSILON {
          signal.set(low);
          return true;
        }
      }
      Self::IntRange(signal) => {
        let next = (low.round() as i32, high.round() as i32);
        if signal.get_untracked() != next {
          signal.set(next);
          return true;
        }
      }
      Self::FloatRange(signal) => {
        let (current_low, current_high) = signal.get_untracked();
        if (current_low - low).abs() > f32::EPSILON || (current_high - high).abs() > f32::EPSILON {
          signal.set((low, high));
          return true;
        }
      }
    }
    false
  }

  fn is_range(&self) -> bool {
    matches!(self, Self::IntRange(_) | Self::FloatRange(_))
  }
}

impl SliderInner {
  fn new(step: f32) -> Self {
    Self {
      min: 0.0,
      max: 1.0,
      step,
      orientation: SliderOrientation::Horizontal,
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
      active: SliderThumb::Low,
      drag_ratio: None,
    }
  }
}

impl SliderState {
  fn with(value: SliderValue, step: f32) -> Self {
    Self {
      value,
      inner: Arc::new(Mutex::new(SliderInner::new(step))),
    }
  }

  pub(crate) fn new(value: Signal<i32>) -> Self {
    Self::with(SliderValue::Int(value), 1.0)
  }

  pub(crate) fn new_f32(value: Signal<f32>) -> Self {
    Self::with(SliderValue::Float(value), 0.01)
  }

  pub(crate) fn new_range(values: Signal<(i32, i32)>) -> Self {
    Self::with(SliderValue::IntRange(values), 1.0)
  }

  pub(crate) fn new_range_f32(values: Signal<(f32, f32)>) -> Self {
    Self::with(SliderValue::FloatRange(values), 0.01)
  }

  /// Whether the slider has two thumbs, bound to a low value and a high one.
  pub(crate) fn is_range(&self) -> bool {
    self.value.is_range()
  }

  pub(crate) fn orientation(&self) -> SliderOrientation {
    self.inner.lock().unwrap().orientation
  }

  pub(crate) fn set_orientation(&self, orientation: SliderOrientation) {
    self.inner.lock().unwrap().orientation = orientation;
  }

  /// The value as text, as tooling reads it; a range's two joined by a comma.
  #[cfg(any(feature = "form", feature = "mcp"))]
  pub(crate) fn value_string(&self) -> String {
    self.value_strings().join(",")
  }

  /// The values as a form submits them under the slider's name: one, or a range's low and high.
  #[cfg(any(feature = "form", feature = "mcp"))]
  pub(crate) fn value_strings(&self) -> Vec<String> {
    match &self.value {
      SliderValue::Int(value) => vec![value.get_untracked().to_string()],
      SliderValue::Float(value) => vec![value.get_untracked().to_string()],
      SliderValue::IntRange(value) => {
        let (low, high) = value.get_untracked();
        vec![low.to_string(), high.to_string()]
      }
      SliderValue::FloatRange(value) => {
        let (low, high) = value.get_untracked();
        vec![low.to_string(), high.to_string()]
      }
    }
  }

  /// The value; of a range, its low value.
  pub(crate) fn value_f32(&self) -> f32 {
    self.value.get().0
  }

  /// The low value and the high one; of a slider of one value, that value twice.
  pub(crate) fn values_f32(&self) -> (f32, f32) {
    self.value.get()
  }

  fn value_of(&self, thumb: SliderThumb) -> f32 {
    let (low, high) = self.value.get();
    match thumb {
      SliderThumb::Low => low,
      SliderThumb::High => high,
    }
  }

  pub(crate) fn value_id(&self) -> usize {
    self.value.id()
  }

  /// Sets the value of one thumb; a range's thumb stops at the other's value rather than pass it.
  fn set_value_of(&self, thumb: SliderThumb, value: f32) -> bool {
    if !self.is_range() {
      return self.value.set(value, value);
    }
    let (low, high) = self.value.get();
    match thumb {
      SliderThumb::Low => self.value.set(value.min(high), high),
      SliderThumb::High => self.value.set(low, value.max(low)),
    }
  }

  /// Where a thumb is drawn along the track, from 0 at the least value to 1 at the greatest.
  pub(crate) fn visual_ratio(&self, thumb: SliderThumb) -> f32 {
    let inner = self.inner.lock().unwrap();
    let value_ratio = slider_ratio_for_value(self.value_of(thumb), inner.min, inner.max);
    inner
      .drag_ratio
      .filter(|_| thumb == inner.active || !self.is_range())
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
    let (low, high) = self.value.get();
    let low = snap_slider_value(low, inner.min, inner.max, inner.step);
    let high = snap_slider_value(high, inner.min, inner.max, inner.step);
    drop(inner);
    self.value.set(low, high);
  }

  pub(crate) fn set_step(&self, step: f32) {
    let mut inner = self.inner.lock().unwrap();
    inner.step = step.abs().max(f32::EPSILON);
    let (low, high) = self.value.get();
    let low = snap_slider_value(low, inner.min, inner.max, inner.step);
    let high = snap_slider_value(high, inner.min, inner.max, inner.step);
    drop(inner);
    self.value.set(low, high);
  }

  /// Set the slider from outside the pointer/keyboard path (automation
  /// tooling); the value is snapped to the slider's range and step. Of a
  /// range, the thumb nearer the value moves to it.
  #[cfg(feature = "mcp")]
  pub(crate) fn set_value_external(&self, value: f32) -> bool {
    let inner = self.inner.lock().unwrap();
    let value = snap_slider_value(value, inner.min, inner.max, inner.step);
    let ratio = slider_ratio_for_value(value, inner.min, inner.max);
    drop(inner);
    let thumb = self.nearer_thumb(ratio);
    self.set_value_of(thumb, value)
  }

  /// Sets both values of a range from outside the pointer/keyboard path, each snapped to the range and step and
  /// the two put in order. Returns `false` for a slider of one value, which has no second value to take.
  #[cfg(feature = "mcp")]
  pub(crate) fn set_values_external(&self, low: f32, high: f32) -> bool {
    if !self.is_range() {
      return false;
    }
    let mut inner = self.inner.lock().unwrap();
    inner.drag_ratio = None;
    let low = snap_slider_value(low, inner.min, inner.max, inner.step);
    let high = snap_slider_value(high, inner.min, inner.max, inner.step);
    drop(inner);
    self.value.set(low, high);
    true
  }

  pub(crate) fn set_from_ratio_of(&self, thumb: SliderThumb, ratio: f32) -> bool {
    let inner = self.inner.lock().unwrap();
    let raw_value = inner.min + ratio.clamp(0.0, 1.0) * (inner.max - inner.min);
    let value = snap_slider_value(raw_value, inner.min, inner.max, inner.step);
    drop(inner);
    self.set_value_of(thumb, value)
  }

  /// A press at `ratio` along the track: of a range, the thumb nearer it becomes the active one and goes
  /// there; a slider of one value goes there.
  pub(crate) fn press_at_ratio(&self, ratio: f32) -> bool {
    let thumb = self.nearer_thumb(ratio);
    self.set_active_thumb(thumb);
    self.clear_drag_ratio();
    self.set_from_ratio_of(thumb, ratio)
  }

  /// The thumb a press at `ratio` along the track takes: the nearer. Of two at one place, the one on the
  /// side of the press, so that they part; a press on both takes the active one.
  pub(crate) fn nearer_thumb(&self, ratio: f32) -> SliderThumb {
    if !self.is_range() {
      return SliderThumb::Low;
    }
    let low = self.visual_ratio(SliderThumb::Low);
    let high = self.visual_ratio(SliderThumb::High);
    let (to_low, to_high) = ((ratio - low).abs(), (ratio - high).abs());
    if to_low < to_high {
      SliderThumb::Low
    } else if to_high < to_low {
      SliderThumb::High
    } else if ratio > high {
      SliderThumb::High
    } else if ratio < low {
      SliderThumb::Low
    } else {
      self.active_thumb()
    }
  }

  pub(crate) fn active_thumb(&self) -> SliderThumb {
    self.inner.lock().unwrap().active
  }

  pub(crate) fn set_active_thumb(&self, thumb: SliderThumb) {
    let thumb = if self.is_range() { thumb } else { SliderThumb::Low };
    self.inner.lock().unwrap().active = thumb;
  }

  /// Tab on a focused range: from its low thumb forward to its high one, or back from the high one to the
  /// low. Returns whether the focus stays in the slider for it.
  pub(crate) fn tab_within(&self, reverse: bool) -> bool {
    if !self.is_range() {
      return false;
    }
    let mut inner = self.inner.lock().unwrap();
    let next = match (inner.active, reverse) {
      (SliderThumb::Low, false) => SliderThumb::High,
      (SliderThumb::High, true) => SliderThumb::Low,
      _ => return false,
    };
    inner.active = next;
    inner.drag_ratio = None;
    true
  }

  /// The slider took the focus from Tab: a range's low thumb is the stop going forward, its high one going
  /// back.
  pub(crate) fn enter_by_tab(&self, reverse: bool) {
    self.set_active_thumb(if reverse { SliderThumb::High } else { SliderThumb::Low });
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

  /// Moves the active thumb by `delta` steps.
  pub(crate) fn nudge(&self, delta: i32) {
    let mut inner = self.inner.lock().unwrap();
    inner.drag_ratio = None;
    let thumb = inner.active;
    let current = self.value_of(thumb);
    let next = snap_slider_value(current + delta as f32 * inner.step, inner.min, inner.max, inner.step);
    drop(inner);
    self.set_value_of(thumb, next);
  }

  /// Page Up and Page Down: the active thumb moves by `delta` pages, a page being a tenth of the range in whole
  /// steps and at least one step. A browser's range input pages by a tenth of its range too; ten steps would cross
  /// a short range in one press and hardly move a long one.
  pub(crate) fn page(&self, delta: i32) {
    let inner = self.inner.lock().unwrap();
    let steps = ((inner.max - inner.min) / 10.0 / inner.step).round().max(1.0) as i32;
    drop(inner);
    self.nudge(delta.saturating_mul(steps));
  }

  /// Home and End: the active thumb goes to the least value or to the greatest; a range's thumb stops at the
  /// other's value.
  pub(crate) fn to_end(&self, greatest: bool) {
    let mut inner = self.inner.lock().unwrap();
    inner.drag_ratio = None;
    let thumb = inner.active;
    let value = if greatest { inner.max } else { inner.min };
    drop(inner);
    self.set_value_of(thumb, value);
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
    if self.is_range() {
      inner.active = old_inner.active;
    }
    let current = slider_ratio_for_value(self.value_of(inner.active), inner.min, inner.max);
    inner.drag_ratio = old_inner
      .drag_ratio
      .filter(|ratio| slider_drag_ratio_matches_value(*ratio, current, inner.min, inner.max, inner.step));
  }

  pub(crate) fn layout_signature(&self) -> SliderLayoutSignature {
    let inner = self.inner.lock().unwrap();
    SliderLayoutSignature {
      vertical: inner.orientation == SliderOrientation::Vertical,
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

  /// The size the slider asks for: its track's, `default_length` along it and `default_thickness` across
  /// it where the track has none, and room for the thumb.
  pub(crate) fn preferred_size(
    &self,
    default_length: f32,
    default_thickness: f32,
    default_thumb_size: f32,
  ) -> (f32, f32) {
    let inner = self.inner.lock().unwrap();
    let (default_width, default_height) = match inner.orientation {
      SliderOrientation::Horizontal => (default_length, default_thickness),
      SliderOrientation::Vertical => (default_thickness, default_length),
    };
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
    let thickness = match inner.orientation {
      SliderOrientation::Horizontal => track_height,
      SliderOrientation::Vertical => track_width,
    };
    let thumb_width = inner
      .thumb_style
      .width
      .into_iter()
      .chain(inner.thumb_hovered_style.as_ref().and_then(|style| style.width))
      .max_by(f32::total_cmp)
      .unwrap_or(thickness.max(default_thumb_size));
    let thumb_height = inner
      .thumb_style
      .height
      .into_iter()
      .chain(inner.thumb_hovered_style.as_ref().and_then(|style| style.height))
      .max_by(f32::total_cmp)
      .unwrap_or(thickness.max(default_thumb_size));
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

  /// Where the track and the thumbs are in the slider's bounds. The track is centered in them, its size the
  /// style's or the bounds'; a thumb is as thick as the track where its style gives it no size, and travels
  /// along the track's length without leaving it.
  pub(crate) fn part_rects(
    &self,
    bounds_x: f32,
    bounds_y: f32,
    bounds_width: f32,
    bounds_height: f32,
    hovered: bool,
    default_thumb_size: f32,
  ) -> SliderParts {
    let vertical = self.orientation() == SliderOrientation::Vertical;
    let track_style = self.track_style(hovered);
    // Only the size is read, and the focused layer never changes it.
    let thumb_style = self.thumb_style(hovered, false);
    let track_width = track_style.width.unwrap_or(bounds_width).max(0.0);
    let track_height = track_style.height.unwrap_or(bounds_height).max(0.0);
    let thickness = if vertical { track_width } else { track_height };
    let thumb_width = thumb_style.width.unwrap_or(thickness.max(default_thumb_size)).max(0.0);
    let thumb_height = thumb_style.height.unwrap_or(thickness.max(default_thumb_size)).max(0.0);
    let track_x = bounds_x + (bounds_width - track_width) * 0.5;
    let track_y = bounds_y + (bounds_height - track_height) * 0.5;
    let thumb_at = |ratio: f32| {
      let (center_x, center_y) = if vertical {
        let travel = (track_height - thumb_height).max(0.0);
        let center_y = if travel > 0.0 {
          track_y + track_height - thumb_height * 0.5 - travel * ratio
        } else {
          track_y + track_height * 0.5
        };
        (track_x + track_width * 0.5, center_y)
      } else {
        let travel = (track_width - thumb_width).max(0.0);
        let center_x = if travel > 0.0 {
          track_x + thumb_width * 0.5 + travel * ratio
        } else {
          track_x + track_width * 0.5
        };
        (center_x, track_y + track_height * 0.5)
      };
      SliderPartRect {
        x: center_x - thumb_width * 0.5,
        y: center_y - thumb_height * 0.5,
        width: thumb_width,
        height: thumb_height,
      }
    };
    let low = thumb_at(self.visual_ratio(SliderThumb::Low));
    let high = if self.is_range() {
      thumb_at(self.visual_ratio(SliderThumb::High))
    } else {
      low
    };

    SliderParts {
      track: SliderPartRect {
        x: track_x,
        y: track_y,
        width: track_width,
        height: track_height,
      },
      thumbs: [low, high],
      range: self.is_range(),
      vertical,
    }
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
