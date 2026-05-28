use std::sync::{Arc, Mutex};

use crate::{
  layout::{Alignment, StackAlignment, scrollbar::ScrollBarStyle},
  node::{dimension::Dimension, padding::Padding},
};

#[derive(Clone)]
pub enum LayoutKind {
  Leaf,
  Row {
    spacing: f32,
    align: Alignment,
    justify: Justify,
    wrap: FlexWrap,
  },
  Column {
    spacing: f32,
    align: Alignment,
    justify: Justify,
    wrap: FlexWrap,
  },
  Stack {
    align: StackAlignment,
  },
  PaddingModifier(Padding),
  FrameModifier(FrameConstraints),
  OffsetModifier {
    x: f32,
    y: f32,
  },
  AbsoluteModifier {
    x: f32,
    y: f32,
    width: Option<Dimension>,
    height: Option<Dimension>,
  },
  AlignModifier(Alignment),
  FlexModifier(FlexParams),
  ScrollModifier {
    state: ScrollState,
    direction: ScrollDirection,
  },
}

#[derive(Clone)]
pub struct ScrollState {
  inner: Arc<Mutex<ScrollStateInner>>,
}

struct ScrollStateInner {
  scroll_x: f32,
  scroll_y: f32,
  max_scroll_x: f32,
  max_scroll_y: f32,
  content_width: f32,
  content_height: f32,
  viewport_width: f32,
  viewport_height: f32,
  viewport_abs_x: f32,
  viewport_abs_y: f32,
  thumb_hovered: bool,
  dragging: bool,
  drag_start_x: f32,
  drag_start_y: f32,
  drag_start_scroll_x: f32,
  drag_start_scroll_y: f32,
  scrollbar_style: ScrollBarStyle,
  scroll_dirty: bool,
}

impl ScrollState {
  pub fn new() -> Self {
    Self {
      inner: Arc::new(Mutex::new(ScrollStateInner {
        scroll_x: 0.0,
        scroll_y: 0.0,
        max_scroll_x: 0.0,
        max_scroll_y: 0.0,
        content_width: 0.0,
        content_height: 0.0,
        viewport_width: 0.0,
        viewport_height: 0.0,
        viewport_abs_x: 0.0,
        viewport_abs_y: 0.0,
        thumb_hovered: false,
        dragging: false,
        drag_start_x: 0.0,
        drag_start_y: 0.0,
        drag_start_scroll_x: 0.0,
        drag_start_scroll_y: 0.0,
        scrollbar_style: ScrollBarStyle::default(),
        scroll_dirty: false,
      })),
    }
  }

  pub fn scroll_x(&self) -> f32 {
    self.inner.lock().unwrap().scroll_x
  }

  pub fn scroll_y(&self) -> f32 {
    self.inner.lock().unwrap().scroll_y
  }

  pub fn set_scroll(&self, x: f32, y: f32) {
    let mut inner = self.inner.lock().unwrap();
    inner.scroll_x = x.clamp(0.0, inner.max_scroll_x);
    inner.scroll_y = y.clamp(0.0, inner.max_scroll_y);
    inner.scroll_dirty = true;
  }

  pub fn scroll_by(&self, dx: f32, dy: f32) {
    self.scroll_by_with_overflow(dx, dy);
  }

  pub fn scroll_by_with_overflow(&self, dx: f32, dy: f32) -> (f32, f32) {
    let mut inner = self.inner.lock().unwrap();
    let old_x = inner.scroll_x;
    let old_y = inner.scroll_y;
    inner.scroll_x = (inner.scroll_x + dx).clamp(0.0, inner.max_scroll_x);
    inner.scroll_y = (inner.scroll_y + dy).clamp(0.0, inner.max_scroll_y);

    let consumed_x = inner.scroll_x - old_x;
    let consumed_y = inner.scroll_y - old_y;
    if consumed_x != 0.0 || consumed_y != 0.0 {
      inner.scroll_dirty = true;
    }

    (dx - consumed_x, dy - consumed_y)
  }

  pub fn content_width(&self) -> f32 {
    self.inner.lock().unwrap().content_width
  }

  pub fn content_height(&self) -> f32 {
    self.inner.lock().unwrap().content_height
  }

  pub fn viewport_width(&self) -> f32 {
    self.inner.lock().unwrap().viewport_width
  }

  pub fn viewport_height(&self) -> f32 {
    self.inner.lock().unwrap().viewport_height
  }

  pub fn style(&self) -> ScrollBarStyle {
    self.inner.lock().unwrap().scrollbar_style.clone()
  }

  pub fn set_style(&self, style: ScrollBarStyle) {
    self.inner.lock().unwrap().scrollbar_style = style;
  }

  pub fn is_thumb_hovered(&self) -> bool {
    self.inner.lock().unwrap().thumb_hovered
  }

  pub fn is_dragging(&self) -> bool {
    self.inner.lock().unwrap().dragging
  }

  pub fn set_thumb_hovered(&self, hovered: bool) {
    self.inner.lock().unwrap().thumb_hovered = hovered;
  }

  pub fn begin_drag(&self, mouse_y: f32) {
    self.begin_drag_axis(ScrollAxis::Vertical, 0.0, mouse_y);
  }

  pub fn begin_drag_axis(&self, _axis: ScrollAxis, mouse_x: f32, mouse_y: f32) {
    let mut inner = self.inner.lock().unwrap();
    inner.dragging = true;
    inner.drag_start_x = mouse_x;
    inner.drag_start_y = mouse_y;
    inner.drag_start_scroll_x = inner.scroll_x;
    inner.drag_start_scroll_y = inner.scroll_y;
  }

  pub fn end_drag(&self) {
    self.inner.lock().unwrap().dragging = false;
  }

  pub fn drag_to(&self, mouse_y: f32, style: &crate::layout::scrollbar::ScrollBarStyle) {
    self.drag_to_axis(ScrollAxis::Vertical, 0.0, mouse_y, style);
  }

  pub fn drag_to_axis(
    &self,
    axis: ScrollAxis,
    mouse_x: f32,
    mouse_y: f32,
    style: &crate::layout::scrollbar::ScrollBarStyle,
  ) {
    let mut inner = self.inner.lock().unwrap();
    if !inner.dragging {
      return;
    }

    match axis {
      ScrollAxis::Horizontal => {
        let track_width = inner.viewport_width - style.padding * 2.0;
        if track_width <= 0.0 {
          return;
        }
        let ratio = inner.viewport_width / inner.content_width.max(1.0);
        let thumb_width = (track_width * ratio).max(style.min_thumb_length).min(track_width);
        let scrollable_track = track_width - thumb_width;
        if scrollable_track <= 0.0 {
          return;
        }

        let delta_px = mouse_x - inner.drag_start_x;
        let scroll_delta = delta_px / scrollable_track * inner.max_scroll_x;
        inner.scroll_x = (inner.drag_start_scroll_x + scroll_delta).clamp(0.0, inner.max_scroll_x);
      }
      ScrollAxis::Vertical => {
        let track_height = inner.viewport_height - style.padding * 2.0;
        if track_height <= 0.0 {
          return;
        }
        let ratio = inner.viewport_height / inner.content_height.max(1.0);
        let thumb_height = (track_height * ratio).max(style.min_thumb_length).min(track_height);
        let scrollable_track = track_height - thumb_height;
        if scrollable_track <= 0.0 {
          return;
        }

        let delta_px = mouse_y - inner.drag_start_y;
        let scroll_delta = delta_px / scrollable_track * inner.max_scroll_y;
        inner.scroll_y = (inner.drag_start_scroll_y + scroll_delta).clamp(0.0, inner.max_scroll_y);
      }
    }
    inner.scroll_dirty = true;
  }

  pub fn thumb_rect(&self, style: &crate::layout::scrollbar::ScrollBarStyle) -> (f32, f32, f32, f32) {
    let inner = self.inner.lock().unwrap();
    let track_x = inner.viewport_abs_x + inner.viewport_width - style.width - style.padding;
    let track_y = inner.viewport_abs_y + style.padding;
    let track_height = inner.viewport_height - style.padding * 2.0;

    let ratio = inner.viewport_height / inner.content_height.max(1.0);
    let thumb_height = (track_height * ratio).max(style.min_thumb_length).min(track_height);
    let scroll_ratio = if inner.max_scroll_y > 0.0 {
      inner.scroll_y / inner.max_scroll_y
    } else {
      0.0
    };
    let thumb_y = track_y + (track_height - thumb_height) * scroll_ratio;

    (track_x, thumb_y, style.width, thumb_height)
  }

  pub fn thumb_rect_for_axis(
    &self,
    axis: ScrollAxis,
    style: &crate::layout::scrollbar::ScrollBarStyle,
  ) -> Option<(f32, f32, f32, f32)> {
    let inner = self.inner.lock().unwrap();
    let geo = match axis {
      ScrollAxis::Horizontal => crate::layout::scrollbar::compute_horizontal_scrollbar(
        style,
        inner.viewport_abs_x,
        inner.viewport_abs_y,
        inner.viewport_width,
        inner.viewport_height,
        inner.content_width,
        inner.scroll_x,
      ),
      ScrollAxis::Vertical => crate::layout::scrollbar::compute_vertical_scrollbar(
        style,
        inner.viewport_abs_x,
        inner.viewport_abs_y,
        inner.viewport_width,
        inner.viewport_height,
        inner.content_height,
        inner.scroll_y,
      ),
    }?;

    Some((geo.thumb_x, geo.thumb_y, geo.thumb_width, geo.thumb_height))
  }

  pub(crate) fn take_scroll_dirty(&self) -> bool {
    let mut inner = self.inner.lock().unwrap();
    let dirty = inner.scroll_dirty;
    inner.scroll_dirty = false;
    dirty
  }

  pub(crate) fn update_layout(&self, content_w: f32, content_h: f32, viewport_w: f32, viewport_h: f32) {
    let mut inner = self.inner.lock().unwrap();
    inner.content_width = content_w;
    inner.content_height = content_h;
    inner.viewport_width = viewport_w;
    inner.viewport_height = viewport_h;
    inner.max_scroll_x = (content_w - viewport_w).max(0.0);
    inner.max_scroll_y = (content_h - viewport_h).max(0.0);
    inner.scroll_x = inner.scroll_x.clamp(0.0, inner.max_scroll_x);
    inner.scroll_y = inner.scroll_y.clamp(0.0, inner.max_scroll_y);
  }

  pub(crate) fn set_viewport_position(&self, x: f32, y: f32) {
    let mut inner = self.inner.lock().unwrap();
    inner.viewport_abs_x = x;
    inner.viewport_abs_y = y;
  }
}

#[derive(Clone, Copy)]
pub struct FlexParams {
  pub grow: f32,
  pub shrink: f32,
  pub basis: Option<f32>,
}

impl FlexParams {
  pub fn grow(factor: f32) -> Self {
    Self {
      grow: factor,
      shrink: 0.0,
      basis: None,
    }
  }
}

impl Default for FlexParams {
  fn default() -> Self {
    Self {
      grow: 1.0,
      shrink: 0.0,
      basis: None,
    }
  }
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum FlexWrap {
  #[default]
  NoWrap,
  Wrap,
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum Justify {
  #[default]
  Start,
  End,
  Center,
  SpaceBetween,
  SpaceAround,
  SpaceEvenly,
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum Overflow {
  Visible,
  #[default]
  Hidden,
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum ScrollDirection {
  Horizontal,
  #[default]
  Vertical,
  Both,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ScrollAxis {
  Horizontal,
  Vertical,
}

#[derive(Clone, Copy, Default)]
pub struct FrameConstraints {
  pub width: Option<Dimension>,
  pub height: Option<Dimension>,
  pub min_width: Option<Dimension>,
  pub max_width: Option<Dimension>,
  pub min_height: Option<Dimension>,
  pub max_height: Option<Dimension>,
}

impl FrameConstraints {
  pub fn with_width(mut self, value: Option<Dimension>) -> Self {
    self.width = value;
    self
  }
  pub fn with_height(mut self, value: Option<Dimension>) -> Self {
    self.height = value;
    self
  }
  pub fn with_min_width(mut self, value: Option<Dimension>) -> Self {
    self.min_width = value;
    self
  }
  pub fn with_max_width(mut self, value: Option<Dimension>) -> Self {
    self.max_width = value;
    self
  }
  pub fn with_min_height(mut self, value: Option<Dimension>) -> Self {
    self.min_height = value;
    self
  }
  pub fn with_max_height(mut self, value: Option<Dimension>) -> Self {
    self.max_height = value;
    self
  }
}
