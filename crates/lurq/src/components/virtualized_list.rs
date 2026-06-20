use std::{
  collections::{HashMap, HashSet},
  fmt::Display,
  marker::PhantomData,
  sync::Arc,
};

use parking_lot::Mutex;

use super::{Column, ScrollVertical, Spacer};
use crate::{
  app::{component::Component, ctx::Ctx, events::ScrollEvent},
  core::{ElementRef, Signal},
  layout::{
    Alignment,
    layout_kind::{FrameConstraints, ScrollState},
    scrollbar::ScrollBarStyle,
  },
  node::{Element, EventHandler, IntoScrollEventHandler, dimension::Dimension},
};

const DEFAULT_OVERSCAN_PX: f32 = 256.0;
const HEIGHT_EPSILON: f32 = 0.01;

#[derive(Clone, Default)]
struct VirtualizedListOptions {
  frame: FrameConstraints,
  flex: Option<f32>,
  overscan_px: f32,
  scroll_state: Option<ScrollState>,
  scrollbar: Option<ScrollBarStyle>,
  scrollbar_hovered: Option<Arc<dyn Fn(ScrollBarStyle) -> ScrollBarStyle + Send + Sync>>,
  scroll_handlers: Vec<EventHandler<ScrollEvent>>,
  top_handlers: Vec<EventHandler<ScrollEvent>>,
  bottom_handlers: Vec<EventHandler<ScrollEvent>>,
  list_key: Option<String>,
}

pub struct VirtualizedList<'a, T> {
  ctx: &'a mut Ctx,
  items: Vec<T>,
  options: VirtualizedListOptions,
}

impl<'a, T> VirtualizedList<'a, T> {
  pub fn new(ctx: &'a mut Ctx, items: impl IntoIterator<Item = T>) -> Self {
    Self {
      ctx,
      items: items.into_iter().collect(),
      options: VirtualizedListOptions {
        overscan_px: DEFAULT_OVERSCAN_PX,
        ..VirtualizedListOptions::default()
      },
    }
  }

  pub fn list_key(mut self, key: impl Into<String>) -> Self {
    self.options.list_key = Some(key.into());
    self
  }

  pub fn overscan_px(mut self, overscan_px: f32) -> Self {
    self.options.overscan_px = overscan_px.max(0.0);
    self
  }

  pub fn width(mut self, width: impl Into<Dimension>) -> Self {
    self.options.frame.width = Some(width.into());
    self
  }

  pub fn height(mut self, height: impl Into<Dimension>) -> Self {
    self.options.frame.height = Some(height.into());
    self
  }

  pub fn size(mut self, width: impl Into<Dimension>, height: impl Into<Dimension>) -> Self {
    self.options.frame.width = Some(width.into());
    self.options.frame.height = Some(height.into());
    self
  }

  pub fn flex(mut self, flex: f32) -> Self {
    self.options.flex = Some(flex);
    self
  }

  pub fn scrollbar(mut self, style: ScrollBarStyle) -> Self {
    self.options.scrollbar = Some(style);
    self
  }

  pub fn scrollbar_hovered(mut self, f: impl Fn(ScrollBarStyle) -> ScrollBarStyle + Send + Sync + 'static) -> Self {
    self.options.scrollbar_hovered = Some(Arc::new(f));
    self
  }

  pub fn with_scroll_state(mut self, scroll_state: ScrollState) -> Self {
    self.options.scroll_state = Some(scroll_state);
    self
  }

  pub fn on_scroll(mut self, f: impl IntoScrollEventHandler) -> Self {
    self.options.scroll_handlers.push(f.into_event_handler());
    self
  }

  pub fn on_top_reached(mut self, f: impl Fn() + Send + Sync + 'static) -> Self {
    self.options.top_handlers.push(EventHandler::new(move |_| f()));
    self
  }

  pub fn on_scroll_reach_top(mut self, f: impl IntoScrollEventHandler) -> Self {
    self.options.top_handlers.push(f.into_event_handler());
    self
  }

  pub fn on_bottom_reached(mut self, f: impl Fn() + Send + Sync + 'static) -> Self {
    self.options.bottom_handlers.push(EventHandler::new(move |_| f()));
    self
  }

  pub fn on_scroll_reach_bottom(mut self, f: impl IntoScrollEventHandler) -> Self {
    self.options.bottom_handlers.push(f.into_event_handler());
    self
  }

  pub fn mount_keyed<C, K, KF, PF>(self, key_fn: KF, props_fn: PF) -> Element
  where
    C: Component,
    T: Clone + PartialEq + Send + Sync + 'static,
    K: Display + Send + Sync + 'static,
    KF: Fn(&T) -> K + Send + Sync + 'static,
    PF: Fn(&T) -> C::Props + Send + Sync + 'static,
  {
    let props = VirtualizedMountListProps::<T, KF, PF> {
      items: self.items,
      key_fn: Arc::new(key_fn),
      props_fn: Arc::new(props_fn),
      options: self.options.clone(),
    };

    match self.options.list_key.as_deref() {
      Some(key) => self
        .ctx
        .mount_keyed::<VirtualizedMountList<T, C, K, KF, PF>>(key, props),
      None => self.ctx.mount::<VirtualizedMountList<T, C, K, KF, PF>>(props),
    }
  }
}

struct VirtualizedMountListProps<T, KF, PF> {
  items: Vec<T>,
  key_fn: Arc<KF>,
  props_fn: Arc<PF>,
  options: VirtualizedListOptions,
}

impl<T, KF, PF> PartialEq for VirtualizedMountListProps<T, KF, PF> {
  fn eq(&self, _other: &Self) -> bool {
    false
  }
}

#[cfg(feature = "devtools")]
impl<T, KF, PF> crate::app::component::DevtoolsInspectable for VirtualizedMountListProps<T, KF, PF> {
  fn inspect(&self, formatter: &mut crate::app::component::DevtoolsFormatter<'_>) {
    formatter.value("VirtualizedList", format!("len={}", self.items.len()));
  }
}

struct VirtualizedMountList<T, C, K, KF, PF>
where
  C: Component,
{
  scroll_state: ScrollState,
  active_scroll_state: Arc<Mutex<ScrollState>>,
  revision: Signal<u64>,
  runtime: Arc<Mutex<VirtualizedRuntime<T>>>,
  _marker: PhantomData<(C, K, KF, PF)>,
}

struct VirtualizedRuntime<T> {
  heights: HashMap<String, f32>,
  items: HashMap<String, T>,
  order: Vec<String>,
  rendered_refs: HashMap<String, ElementRef>,
  pending_anchor: Option<ScrollAnchor>,
  last_scroll_y: f32,
  last_viewport_height: f32,
}

struct ScrollAnchor {
  key: String,
  offset: f32,
}

impl<T> Default for VirtualizedRuntime<T> {
  fn default() -> Self {
    Self {
      heights: HashMap::new(),
      items: HashMap::new(),
      order: Vec::new(),
      rendered_refs: HashMap::new(),
      pending_anchor: None,
      last_scroll_y: 0.0,
      last_viewport_height: 0.0,
    }
  }
}

impl<T, C, K, KF, PF> Component for VirtualizedMountList<T, C, K, KF, PF>
where
  C: Component,
  T: Clone + PartialEq + Send + Sync + 'static,
  K: Display + Send + Sync + 'static,
  KF: Fn(&T) -> K + Send + Sync + 'static,
  PF: Fn(&T) -> C::Props + Send + Sync + 'static,
{
  type Props = VirtualizedMountListProps<T, KF, PF>;

  fn create(ctx: &mut Ctx) -> Self {
    let scroll_state = ScrollState::new();
    Self {
      scroll_state: scroll_state.clone(),
      active_scroll_state: Arc::new(Mutex::new(scroll_state)),
      revision: ctx.signal(0),
      runtime: Arc::new(Mutex::new(VirtualizedRuntime::default())),
      _marker: PhantomData,
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let (items, key_fn, props_fn, options) = {
      let props = ctx.props::<Self::Props>();
      (
        props.items.clone(),
        props.key_fn.clone(),
        props.props_fn.clone(),
        props.options.clone(),
      )
    };
    let _ = self.revision.get();
    let scroll_state = options
      .scroll_state
      .clone()
      .unwrap_or_else(|| self.scroll_state.clone());
    *self.active_scroll_state.lock() = scroll_state.clone();
    let rows = self.rows(ctx, &items, &*key_fn, &*props_fn, &options, &scroll_state);
    let mut scroll = ScrollVertical::new(rows).with_scroll_state(scroll_state);

    if let Some(style) = options.scrollbar.clone() {
      scroll = scroll.scrollbar(style);
    }
    if let Some(hovered) = options.scrollbar_hovered.clone() {
      scroll = scroll.scrollbar_hovered(move |style| hovered(style));
    }

    for handler in options.top_handlers.iter().cloned() {
      scroll = scroll.on_scroll_reach_top(handler);
    }
    for handler in options.bottom_handlers.iter().cloned() {
      scroll = scroll.on_scroll_reach_bottom(handler);
    }
    for handler in options.scroll_handlers.iter().cloned() {
      scroll = scroll.on_scroll(handler);
    }

    let revision = self.revision.clone();
    scroll = scroll.on_scroll(move |_| {
      revision.update(|value| *value = value.wrapping_add(1));
    });

    if let Some(width) = options.frame.width {
      scroll = scroll.width(width);
    }
    if let Some(height) = options.frame.height {
      scroll = scroll.height(height);
    }
    if let Some(min_width) = options.frame.min_width {
      scroll = scroll.min_width(min_width);
    }
    if let Some(max_width) = options.frame.max_width {
      scroll = scroll.max_width(max_width);
    }
    if let Some(min_height) = options.frame.min_height {
      scroll = scroll.min_height(min_height);
    }
    if let Some(max_height) = options.frame.max_height {
      scroll = scroll.max_height(max_height);
    }
    if let Some(flex) = options.flex {
      scroll = scroll.flex(flex);
    }

    scroll
  }

  fn after_layout(&self) {
    let mut changed = false;
    {
      let mut runtime = self.runtime.lock();
      let rendered_refs: Vec<(String, ElementRef)> = runtime
        .rendered_refs
        .iter()
        .map(|(key, element_ref)| (key.clone(), element_ref.clone()))
        .collect();

      for (key, element_ref) in rendered_refs {
        if !element_ref.is_attached() {
          continue;
        }
        let height = element_ref.height().max(0.0);
        let previous = runtime.heights.insert(key, height);
        if previous.is_none_or(|old| (old - height).abs() > HEIGHT_EPSILON) {
          changed = true;
        }
      }

      let scroll_state = self.active_scroll_state.lock().clone();
      let scroll_y = scroll_state.scroll_y();
      let viewport_height = scroll_state.viewport_height();
      if (runtime.last_scroll_y - scroll_y).abs() > HEIGHT_EPSILON
        || (runtime.last_viewport_height - viewport_height).abs() > HEIGHT_EPSILON
      {
        runtime.last_scroll_y = scroll_y;
        runtime.last_viewport_height = viewport_height;
        changed = true;
      }

      if let Some(anchor) = runtime.pending_anchor.take() {
        if runtime.order.iter().all(|key| runtime.heights.contains_key(key)) {
          if let Some(anchor_index) = runtime.order.iter().position(|key| key == &anchor.key) {
            let new_top = prefix_height(&runtime.order[..anchor_index], &runtime.heights);
            scroll_state.set_scroll(0.0, new_top + anchor.offset);
            changed = true;
          }
        } else {
          runtime.pending_anchor = Some(anchor);
        }
      }
    }

    if changed {
      self.revision.update(|value| *value = value.wrapping_add(1));
    }
  }
}

impl<T, C, K, KF, PF> VirtualizedMountList<T, C, K, KF, PF>
where
  C: Component,
  T: Clone + PartialEq + Send + Sync + 'static,
  K: Display + Send + Sync + 'static,
  KF: Fn(&T) -> K + Send + Sync + 'static,
  PF: Fn(&T) -> C::Props + Send + Sync + 'static,
{
  fn rows(
    &self,
    ctx: &mut Ctx,
    items: &[T],
    key_fn: &KF,
    props_fn: &PF,
    options: &VirtualizedListOptions,
    scroll_state: &ScrollState,
  ) -> Column {
    let keys: Vec<String> = items.iter().map(|item| format!("{}", key_fn(item))).collect();
    let mut runtime = self.runtime.lock();
    if runtime.order != keys
      && runtime.pending_anchor.is_none()
      && runtime.order.iter().all(|key| runtime.heights.contains_key(key))
    {
      runtime.pending_anchor = scroll_anchor_for(&runtime.order, &runtime.heights, scroll_state.scroll_y());
    }
    let current_keys: HashSet<&str> = keys.iter().map(String::as_str).collect();
    runtime.heights.retain(|key, _| current_keys.contains(key.as_str()));
    runtime.items.retain(|key, _| current_keys.contains(key.as_str()));

    for (key, item) in keys.iter().zip(items.iter()) {
      if runtime.items.get(key) != Some(item) {
        runtime.heights.remove(key);
      }
      runtime.items.insert(key.clone(), item.clone());
    }
    runtime.order = keys.clone();

    let all_measured = !keys.is_empty() && keys.iter().all(|key| runtime.heights.contains_key(key));
    let has_measurements = keys.iter().any(|key| runtime.heights.contains_key(key));
    let visible_range = if all_measured || has_measurements {
      visible_range(
        &keys,
        &runtime.heights,
        scroll_state.scroll_y(),
        scroll_state.viewport_height(),
        options.overscan_px,
      )
    } else {
      (0, items.len())
    };
    let mut rendered_indices: Vec<usize> = (visible_range.0..visible_range.1).collect();
    if has_measurements && !all_measured {
      rendered_indices.extend(
        keys
          .iter()
          .enumerate()
          .filter_map(|(index, key)| (!runtime.heights.contains_key(key)).then_some(index)),
      );
      rendered_indices.sort_unstable();
      rendered_indices.dedup();
    }
    let total_height = has_measurements.then(|| prefix_height(&keys, &runtime.heights));
    let heights = runtime.heights.clone();
    runtime.rendered_refs.clear();
    drop(runtime);

    let mut column = Column::new().spacing(0.0).align_items(Alignment::Start);
    let mut cursor_y = 0.0;

    for index in rendered_indices {
      let row_y = prefix_height(&keys[..index], &heights);
      if row_y > cursor_y {
        column = column.child(Spacer::new().height(row_y - cursor_y));
      }
      let key = keys[index].clone();
      let row_ref = ctx.element_ref();
      self.runtime.lock().rendered_refs.insert(key.clone(), row_ref.clone());
      let row = ctx.mount_keyed::<C>(&key, props_fn(&items[index]));
      let wrapper = Column::new().spacing(0.0).ref_element(row_ref).child(row);
      column = column.child(wrapper);
      cursor_y = row_y + heights.get(&key).copied().unwrap_or(0.0);
    }

    if let Some(total_height) = total_height
      && total_height > cursor_y
    {
      column = column.child(Spacer::new().height(total_height - cursor_y));
    }

    column
  }
}

fn prefix_height(keys: &[String], heights: &HashMap<String, f32>) -> f32 {
  keys.iter().map(|key| heights.get(key).copied().unwrap_or(0.0)).sum()
}

fn scroll_anchor_for(keys: &[String], heights: &HashMap<String, f32>, scroll_y: f32) -> Option<ScrollAnchor> {
  let mut offset = 0.0;
  for key in keys {
    let height = heights.get(key).copied().unwrap_or(0.0);
    let next = offset + height;
    if next > scroll_y {
      return Some(ScrollAnchor {
        key: key.clone(),
        offset: scroll_y - offset,
      });
    }
    offset = next;
  }

  keys.last().map(|key| ScrollAnchor {
    key: key.clone(),
    offset: 0.0,
  })
}

fn visible_range(
  keys: &[String],
  heights: &HashMap<String, f32>,
  scroll_y: f32,
  viewport_height: f32,
  overscan_px: f32,
) -> (usize, usize) {
  if keys.is_empty() {
    return (0, 0);
  }

  let start_y = (scroll_y - overscan_px).max(0.0);
  let end_y = scroll_y + viewport_height + overscan_px;
  let mut offset = 0.0;
  let mut start = 0;

  for (index, key) in keys.iter().enumerate() {
    let next = offset + heights.get(key).copied().unwrap_or(0.0);
    if next > start_y {
      start = index;
      break;
    }
    offset = next;
    start = index + 1;
  }

  let mut end = start;
  for (index, key) in keys.iter().enumerate().skip(start) {
    if offset >= end_y {
      break;
    }
    offset += heights.get(key).copied().unwrap_or(0.0);
    end = index + 1;
  }

  (start.min(keys.len()), end.min(keys.len()).max(start.min(keys.len())))
}
