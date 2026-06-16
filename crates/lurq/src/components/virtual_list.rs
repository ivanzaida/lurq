use std::{
  collections::{HashMap, HashSet},
  sync::Arc,
  time::Duration,
};

use parking_lot::Mutex;

use crate::{
  app::ctx::{Ctx, Timeout},
  core::{ElementRef, Signal},
  layout::layout_kind::ScrollState,
};

const DEFAULT_OVERSCAN: usize = 4;
const DEFAULT_WINDOW_STRIDE: usize = 4;
const DEFAULT_INITIAL_VISIBLE_COUNT: usize = 20;
const DEFAULT_MEASUREMENT_BATCH: usize = 64;
const MEASURE_EPSILON: f32 = 0.5;
const VIEWPORT_WIDTH_EPSILON: f32 = 0.5;

/// State for `Ctx::virtual_list`.
///
/// The list renders rows in normal flow until every row height has been
/// measured, then virtualizes from the exact measured heights. Keep this state
/// on your component struct so scroll position and measured row heights survive
/// rerenders.
#[derive(Clone)]
pub struct VirtualListState {
  inner: Arc<Mutex<VirtualListStateInner>>,
  scroll_state: ScrollState,
  revision: Signal<u64>,
  measurement_refresh: Timeout,
}

#[derive(Default)]
struct VirtualListStateInner {
  overscan: usize,
  window_stride: usize,
  initial_visible_count: usize,
  measurement_batch: usize,
  active_window: Option<VirtualListActiveWindow>,
  measured_viewport_width: Option<f32>,
  heights: HashMap<String, f32>,
  refs: HashMap<String, ElementRef>,
}

#[derive(Clone, Copy)]
struct VirtualListActiveWindow {
  top: f32,
  bottom: f32,
}

pub(crate) struct VirtualListWindow {
  pub start: usize,
  pub end: usize,
  pub top_spacer: f32,
  pub bottom_spacer: f32,
  pub total_height: f32,
}

impl VirtualListState {
  pub fn new(ctx: &mut Ctx) -> Self {
    let revision = ctx.signal(0_u64);
    let refresh_revision = revision.clone();
    let measurement_refresh = ctx.create_timeout(Duration::ZERO, move || {
      refresh_revision.update(|revision| *revision = revision.wrapping_add(1));
    });
    Self {
      inner: Arc::new(Mutex::new(VirtualListStateInner {
        overscan: DEFAULT_OVERSCAN,
        window_stride: DEFAULT_WINDOW_STRIDE,
        initial_visible_count: DEFAULT_INITIAL_VISIBLE_COUNT,
        measurement_batch: DEFAULT_MEASUREMENT_BATCH,
        active_window: None,
        measured_viewport_width: None,
        heights: HashMap::new(),
        refs: HashMap::new(),
      })),
      scroll_state: ScrollState::new(),
      revision,
      measurement_refresh,
    }
  }

  pub fn with_overscan(self, rows: usize) -> Self {
    self.set_overscan(rows);
    self
  }

  pub fn with_window_stride(self, rows: usize) -> Self {
    self.set_window_stride(rows);
    self
  }

  pub fn with_initial_visible_count(self, count: usize) -> Self {
    self.set_initial_visible_count(count);
    self
  }

  pub fn set_overscan(&self, rows: usize) {
    self.inner.lock().overscan = rows;
    self.request_refresh();
  }

  pub fn set_window_stride(&self, rows: usize) {
    self.inner.lock().window_stride = rows.max(1);
    self.request_refresh();
  }

  pub fn set_initial_visible_count(&self, count: usize) {
    self.inner.lock().initial_visible_count = count.max(1);
    self.request_refresh();
  }

  pub fn scroll_state(&self) -> ScrollState {
    self.scroll_state.clone()
  }

  pub fn request_refresh(&self) {
    self.revision.update(|revision| *revision = revision.wrapping_add(1));
  }

  pub(crate) fn request_measurement_refresh(&self) {
    if !self.measurement_refresh.is_active() {
      self.measurement_refresh.start();
    }
  }

  pub(crate) fn request_scroll_refresh_if_needed(&self, delta_y: f32) {
    let should_refresh = {
      let inner = self.inner.lock();
      let Some(active_window) = inner.active_window else {
        return self.request_refresh();
      };

      let viewport_height = self.scroll_state.viewport_height();
      if viewport_height <= 0.0 {
        return self.request_refresh();
      }

      let next_scroll_y = (self.scroll_state.scroll_y() - delta_y).max(0.0);
      let next_viewport_end = next_scroll_y + viewport_height;
      next_scroll_y < active_window.top || next_viewport_end > active_window.bottom
    };

    if should_refresh {
      self.request_refresh();
    }
  }

  pub(crate) fn track(&self) {
    let _ = self.revision.get();
  }

  pub(crate) fn sync_measurements(&self) {
    let measurements = {
      let inner = self.inner.lock();
      inner
        .refs
        .iter()
        .filter_map(|(key, row_ref)| {
          let height = row_ref.height();
          (height > 0.0).then(|| (key.clone(), height))
        })
        .collect::<Vec<_>>()
    };

    if measurements.is_empty() {
      return;
    }

    let mut changed = false;
    {
      let mut inner = self.inner.lock();
      for (key, height) in measurements {
        let previous = inner.heights.get(&key).copied();
        if previous.is_none_or(|previous| (previous - height).abs() > MEASURE_EPSILON) {
          inner.heights.insert(key, height);
          changed = true;
        }
      }
    }

    if changed {
      self.request_refresh();
    }
  }

  pub(crate) fn row_ref(&self, key: &str) -> ElementRef {
    self
      .inner
      .lock()
      .refs
      .entry(key.to_owned())
      .or_insert_with(ElementRef::new)
      .clone()
  }

  pub(crate) fn height_for_key(&self, key: &str) -> Option<f32> {
    self.inner.lock().height_for(key)
  }

  pub(crate) fn all_heights_measured(&self, keys: &[String]) -> bool {
    self.inner.lock().all_heights_measured(keys)
  }

  pub(crate) fn measured_count(&self, keys: &[String]) -> usize {
    self.inner.lock().measured_count(keys)
  }

  pub(crate) fn measurement_indices(&self, keys: &[String], visible_start: usize, visible_end: usize) -> Vec<usize> {
    self.inner.lock().measurement_indices(keys, visible_start, visible_end)
  }

  pub(crate) fn prune_removed_keys(&self, keys: &HashSet<String>) {
    let mut inner = self.inner.lock();
    inner.heights.retain(|key, _| keys.contains(key));
    inner.refs.retain(|key, _| keys.contains(key));
  }

  pub(crate) fn window(&self, keys: &[String]) -> VirtualListWindow {
    let mut inner = self.inner.lock();
    let item_count = keys.len();
    if item_count == 0 {
      inner.active_window = None;
      return VirtualListWindow {
        start: 0,
        end: 0,
        top_spacer: 0.0,
        bottom_spacer: 0.0,
        total_height: 0.0,
      };
    }

    inner.sync_viewport_width(self.scroll_state.viewport_width());

    if inner.measured_count(keys) == 0 {
      inner.active_window = None;
      return VirtualListWindow {
        start: 0,
        end: item_count.min(inner.initial_visible_count),
        top_spacer: 0.0,
        bottom_spacer: 0.0,
        total_height: 0.0,
      };
    }

    let viewport_height = self.scroll_state.viewport_height();
    if viewport_height <= 0.0 {
      let total_height = inner.total_height(keys);
      if self.scroll_state.is_scroll_to_bottom_pending() {
        let start = item_count.saturating_sub(inner.initial_visible_count);
        let rendered_height = keys[start..]
          .iter()
          .map(|key| inner.height_for(key).unwrap_or(0.0))
          .sum::<f32>();
        return inner.make_window(
          start,
          item_count,
          (total_height - rendered_height).max(0.0),
          rendered_height,
          total_height,
        );
      }

      let end = item_count.min(inner.initial_visible_count);
      let rendered_height = keys[..end]
        .iter()
        .map(|key| inner.height_for(key).unwrap_or(0.0))
        .sum::<f32>();
      return inner.make_window(0, end, 0.0, rendered_height, total_height);
    }

    let scroll_y = self.scroll_state.scroll_y();
    let viewport_end = scroll_y + viewport_height;
    let mut cursor_y = 0.0;
    let mut first_visible = 0;
    for (index, key) in keys.iter().enumerate() {
      let height = inner.height_for(key).unwrap_or(0.0);
      if cursor_y + height > scroll_y {
        first_visible = index;
        break;
      }
      cursor_y += height;
      first_visible = (index + 1).min(item_count - 1);
    }

    let mut end = first_visible;
    let mut end_y = cursor_y;
    while end < item_count && end_y < viewport_end {
      end_y += inner.height_for(&keys[end]).unwrap_or(0.0);
      end += 1;
    }

    let stride = inner.window_stride.max(1);
    let start = first_visible.saturating_sub(inner.overscan);
    let start = (start / stride) * stride;
    end = (end + inner.overscan).min(item_count);
    end = end.div_ceil(stride).saturating_mul(stride).min(item_count);

    let top_spacer = keys[..start]
      .iter()
      .map(|key| inner.height_for(key).unwrap_or(0.0))
      .sum::<f32>();
    let rendered_height = keys[start..end]
      .iter()
      .map(|key| inner.height_for(key).unwrap_or(0.0))
      .sum::<f32>();
    let total_height = inner.total_height(keys);

    inner.make_window(start, end, top_spacer, rendered_height, total_height)
  }
}

impl VirtualListStateInner {
  fn sync_viewport_width(&mut self, viewport_width: f32) {
    if viewport_width <= 0.0 {
      return;
    }

    if self
      .measured_viewport_width
      .is_some_and(|previous| (previous - viewport_width).abs() > VIEWPORT_WIDTH_EPSILON)
    {
      self.heights.clear();
      self.active_window = None;
    }
    self.measured_viewport_width = Some(viewport_width);
  }

  fn make_window(
    &mut self,
    start: usize,
    end: usize,
    top_spacer: f32,
    rendered_height: f32,
    total_height: f32,
  ) -> VirtualListWindow {
    self.active_window = Some(VirtualListActiveWindow {
      top: top_spacer,
      bottom: top_spacer + rendered_height,
    });
    VirtualListWindow {
      start,
      end,
      top_spacer,
      bottom_spacer: (total_height - top_spacer - rendered_height).max(0.0),
      total_height,
    }
  }

  fn height_for(&self, key: &str) -> Option<f32> {
    self.heights.get(key).copied().map(|height| height.max(1.0))
  }

  fn total_height(&self, keys: &[String]) -> f32 {
    keys.iter().map(|key| self.height_for(key).unwrap_or(0.0)).sum()
  }

  fn all_heights_measured(&self, keys: &[String]) -> bool {
    keys.iter().all(|key| self.heights.contains_key(key))
  }

  fn measured_count(&self, keys: &[String]) -> usize {
    keys.iter().filter(|key| self.heights.contains_key(*key)).count()
  }

  fn measurement_indices(&self, keys: &[String], visible_start: usize, visible_end: usize) -> Vec<usize> {
    let mut indices = Vec::new();
    let mut seen = HashSet::new();
    let limit = self.measurement_batch.max(1);

    for index in visible_start..visible_end.min(keys.len()) {
      if !self.heights.contains_key(&keys[index]) && seen.insert(index) {
        indices.push(index);
        if indices.len() >= limit {
          return indices;
        }
      }
    }

    for (index, key) in keys.iter().enumerate() {
      if !self.heights.contains_key(key) && seen.insert(index) {
        indices.push(index);
        if indices.len() >= limit {
          break;
        }
      }
    }

    indices
  }
}
