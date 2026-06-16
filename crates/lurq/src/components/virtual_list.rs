use std::{
  collections::{HashMap, HashSet},
  sync::Arc,
};

use parking_lot::Mutex;

use crate::{
  app::ctx::Ctx,
  core::{ElementRef, Signal},
  layout::layout_kind::ScrollState,
};

const DEFAULT_ESTIMATED_HEIGHT: f32 = 48.0;
const DEFAULT_OVERSCAN: usize = 4;
const DEFAULT_INITIAL_VISIBLE_COUNT: usize = 20;
const MEASURE_EPSILON: f32 = 0.5;

/// State for `Ctx::virtual_list`.
///
/// The list uses `estimated_height` for rows it has not measured yet, then
/// replaces those estimates with measured heights for rows that have been
/// rendered. Keep this state on your component struct so scroll position and
/// measured row heights survive rerenders.
#[derive(Clone)]
pub struct VirtualListState {
  inner: Arc<Mutex<VirtualListStateInner>>,
  scroll_state: ScrollState,
  revision: Signal<u64>,
}

#[derive(Default)]
struct VirtualListStateInner {
  estimated_height: f32,
  overscan: usize,
  initial_visible_count: usize,
  heights: HashMap<String, f32>,
  refs: HashMap<String, ElementRef>,
}

pub(crate) struct VirtualListWindow {
  pub start: usize,
  pub end: usize,
  pub top_spacer: f32,
  pub bottom_spacer: f32,
}

impl VirtualListState {
  pub fn new(ctx: &mut Ctx) -> Self {
    let state = Self {
      inner: Arc::new(Mutex::new(VirtualListStateInner {
        estimated_height: DEFAULT_ESTIMATED_HEIGHT,
        overscan: DEFAULT_OVERSCAN,
        initial_visible_count: DEFAULT_INITIAL_VISIBLE_COUNT,
        heights: HashMap::new(),
        refs: HashMap::new(),
      })),
      scroll_state: ScrollState::new(),
      revision: ctx.signal(0),
    };
    state
  }

  pub fn with_estimated_height(self, height: f32) -> Self {
    self.set_estimated_height(height);
    self
  }

  pub fn with_overscan(self, rows: usize) -> Self {
    self.set_overscan(rows);
    self
  }

  pub fn with_initial_visible_count(self, count: usize) -> Self {
    self.set_initial_visible_count(count);
    self
  }

  pub fn set_estimated_height(&self, height: f32) {
    self.inner.lock().estimated_height = height.max(1.0);
    self.request_refresh();
  }

  pub fn set_overscan(&self, rows: usize) {
    self.inner.lock().overscan = rows;
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

  pub(crate) fn prune_removed_keys(&self, keys: &HashSet<String>) {
    let mut inner = self.inner.lock();
    inner.heights.retain(|key, _| keys.contains(key));
    inner.refs.retain(|key, _| keys.contains(key));
  }

  pub(crate) fn window(&self, keys: &[String]) -> VirtualListWindow {
    let inner = self.inner.lock();
    let item_count = keys.len();
    if item_count == 0 {
      return VirtualListWindow {
        start: 0,
        end: 0,
        top_spacer: 0.0,
        bottom_spacer: 0.0,
      };
    }

    let viewport_height = self.scroll_state.viewport_height();
    if viewport_height <= 0.0 {
      let end = item_count.min(inner.initial_visible_count);
      let rendered_height = keys[..end].iter().map(|key| inner.height_for(key)).sum::<f32>();
      let total_height = inner.total_height(keys);
      return VirtualListWindow {
        start: 0,
        end,
        top_spacer: 0.0,
        bottom_spacer: (total_height - rendered_height).max(0.0),
      };
    }

    let scroll_y = self.scroll_state.scroll_y();
    let viewport_end = scroll_y + viewport_height;
    let mut cursor_y = 0.0;
    let mut first_visible = 0;
    for (index, key) in keys.iter().enumerate() {
      let height = inner.height_for(key);
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
      end_y += inner.height_for(&keys[end]);
      end += 1;
    }

    let start = first_visible.saturating_sub(inner.overscan);
    end = (end + inner.overscan).min(item_count);

    let top_spacer = keys[..start].iter().map(|key| inner.height_for(key)).sum::<f32>();
    let rendered_height = keys[start..end].iter().map(|key| inner.height_for(key)).sum::<f32>();
    let total_height = inner.total_height(keys);

    VirtualListWindow {
      start,
      end,
      top_spacer,
      bottom_spacer: (total_height - top_spacer - rendered_height).max(0.0),
    }
  }
}

impl VirtualListStateInner {
  fn height_for(&self, key: &str) -> f32 {
    self.heights.get(key).copied().unwrap_or(self.estimated_height).max(1.0)
  }

  fn total_height(&self, keys: &[String]) -> f32 {
    keys.iter().map(|key| self.height_for(key)).sum()
  }
}
