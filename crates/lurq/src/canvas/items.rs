//! Semantic items: what a canvas drew, described for hit tests and tooling.
//!
//! Canvas pixels carry no structure. An app that draws a chart can register
//! one [`CanvasItem`] per thing that matters (a bar, a point, a series, an axis
//! label) so pointer handlers can hit-test by item and tooling such as the MCP
//! server can list and target them. The set is owned by the canvas and replaced
//! as a whole, typically once per redraw.

use std::sync::Arc;

use super::CanvasHandle;
use crate::node::transform::Transform2D;

/// Geometry of a [`CanvasItem`] in canvas content coordinates: logical pixels
/// from the content box's top-left corner, before the drawing transform (the
/// space [`CanvasHandle::point_from_window`] returns).
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub enum CanvasItemShape {
  /// An axis-aligned rectangle. Negative extents are normalized.
  Rect { x: f32, y: f32, width: f32, height: f32 },
  /// A point hit within `radius` of its center.
  Point { x: f32, y: f32, radius: f32 },
}

/// One drawn thing that matters, with a stable id, a role such as `"bar"`,
/// `"point"`, `"series"` or `"label"`, an optional accessible label and value
/// text, and its shape. Build it with [`Self::rect`], [`Self::point`] or
/// [`Self::new`]; fields may be added in later versions.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct CanvasItem {
  pub id: String,
  pub role: String,
  pub label: Option<String>,
  pub value: Option<String>,
  pub shape: CanvasItemShape,
}

impl CanvasItem {
  pub fn rect(id: impl Into<String>, role: impl Into<String>, x: f32, y: f32, width: f32, height: f32) -> Self {
    Self::new(id, role, CanvasItemShape::Rect { x, y, width, height })
  }

  pub fn point(id: impl Into<String>, role: impl Into<String>, x: f32, y: f32, radius: f32) -> Self {
    Self::new(id, role, CanvasItemShape::Point { x, y, radius })
  }

  pub fn new(id: impl Into<String>, role: impl Into<String>, shape: CanvasItemShape) -> Self {
    Self {
      id: id.into(),
      role: role.into(),
      label: None,
      value: None,
      shape,
    }
  }

  /// The accessible name, such as `"Mon"` for a bar.
  pub fn label(mut self, label: impl Into<String>) -> Self {
    self.label = Some(label.into());
    self
  }

  /// The displayed value, such as `"12 runs"`.
  pub fn value(mut self, value: impl Into<String>) -> Self {
    self.value = Some(value.into());
    self
  }

  /// Whether the content-space point lies on the item. Non-finite geometry never hits.
  pub fn contains(&self, x: f32, y: f32) -> bool {
    match self.shape {
      CanvasItemShape::Rect { .. } => self
        .content_bounds()
        .is_some_and(|[left, top, width, height]| x >= left && x <= left + width && y >= top && y <= top + height),
      CanvasItemShape::Point {
        x: center_x,
        y: center_y,
        radius,
      } => {
        let (dx, dy) = (x - center_x, y - center_y);
        radius.is_finite() && dx * dx + dy * dy <= radius * radius
      }
    }
  }

  /// `[x, y, width, height]` in content coordinates, or `None` for non-finite geometry.
  pub(crate) fn content_bounds(&self) -> Option<[f32; 4]> {
    let bounds = match self.shape {
      CanvasItemShape::Rect { x, y, width, height } => [x.min(x + width), y.min(y + height), width.abs(), height.abs()],
      CanvasItemShape::Point { x, y, radius } => {
        let radius = radius.abs();
        [x - radius, y - radius, radius * 2.0, radius * 2.0]
      }
    };
    bounds.iter().all(|value| value.is_finite()).then_some(bounds)
  }

  /// Axis-aligned `[x, y, width, height]` of the item after `to_window`.
  pub(crate) fn window_bounds(&self, to_window: Transform2D) -> Option<[f32; 4]> {
    let [x, y, width, height] = self.content_bounds()?;
    let corners = [(x, y), (x + width, y), (x, y + height), (x + width, y + height)]
      .map(|(corner_x, corner_y)| to_window.transform_point(corner_x, corner_y));
    let (mut left, mut top) = (f32::INFINITY, f32::INFINITY);
    let (mut right, mut bottom) = (f32::NEG_INFINITY, f32::NEG_INFINITY);
    for (corner_x, corner_y) in corners {
      left = left.min(corner_x);
      top = top.min(corner_y);
      right = right.max(corner_x);
      bottom = bottom.max(corner_y);
    }
    let bounds = [left, top, right - left, bottom - top];
    bounds.iter().all(|value| value.is_finite()).then_some(bounds)
  }
}

impl CanvasHandle {
  /// Replace the canvas's semantic items. Call it with the full set whenever
  /// the drawing changes; it is cheap and does not repaint. A logical resize
  /// discards the items together with the pixels, so the redraw that follows
  /// a resize registers them again.
  ///
  /// Ids are unique within a canvas: when several items share an id, the last
  /// one is kept, in its own place, as the one drawn on top (the one
  /// [`Self::item_at`] would find), and a warning is logged.
  pub fn set_items(&self, items: impl IntoIterator<Item = CanvasItem>) {
    let (items, duplicates) = last_per_id(items.into_iter().collect());
    let mut s = self.inner.lock();
    let fingerprint = (!duplicates.is_empty()).then(|| {
      use std::hash::{Hash, Hasher};
      let mut hasher = std::collections::hash_map::DefaultHasher::new();
      duplicates.hash(&mut hasher);
      hasher.finish()
    });
    if fingerprint.is_some() && fingerprint != s.warned_duplicates {
      tracing::warn!(
        ?duplicates,
        "canvas items share ids; the last item with each id is kept"
      );
    }
    s.warned_duplicates = fingerprint;
    s.items = items.into();
  }

  /// The current items, in registration order. The returned slice is shared, not copied.
  pub fn items(&self) -> Arc<[CanvasItem]> {
    self.inner.lock().items.clone()
  }

  /// The last registered item containing the content-space point: items
  /// registered later are treated as drawn on top.
  pub fn item_at(&self, x: f32, y: f32) -> Option<CanvasItem> {
    let items = self.items();
    items.iter().rev().find(|item| item.contains(x, y)).cloned()
  }

  /// Window-logical `(x, y, width, height)` of the item with this id, like
  /// `ElementRef::rect`: padding, scrolling and ancestor transforms applied,
  /// the drawing transform not, clipped to the canvas's content box. A rotated
  /// placement yields the enclosing axis-aligned box. `None` while detached,
  /// for an unknown id, or when the item lies entirely outside the canvas.
  pub fn item_window_bounds(&self, id: &str) -> Option<(f32, f32, f32, f32)> {
    let (_, bounds) = self.item_in_window(id)?;
    let [x, y, width, height] = bounds?;
    Some((x, y, width, height))
  }

  /// The item with this id and its window-logical bounds clipped to the
  /// content box. Only that item is copied and transformed.
  pub(crate) fn item_in_window(&self, id: &str) -> Option<(CanvasItem, Option<[f32; 4]>)> {
    let s = self.inner.lock();
    let item = s.items.iter().find(|item| item.id == id)?.clone();
    let bounds = s
      .window_placement()
      .and_then(|(placement, content)| clip(item.window_bounds(placement)?, content));
    Some((item, bounds))
  }

  /// Items with their window-logical bounds clipped to the content box.
  /// Bounds are `None` while detached, for non-finite geometry, or outside the canvas.
  #[cfg(feature = "mcp")]
  pub(crate) fn items_in_window(&self) -> Vec<(CanvasItem, Option<[f32; 4]>)> {
    let (items, placement) = {
      let s = self.inner.lock();
      (s.items.clone(), s.window_placement())
    };
    items
      .iter()
      .map(|item| {
        let bounds = placement.and_then(|(matrix, content)| clip(item.window_bounds(matrix)?, content));
        (item.clone(), bounds)
      })
      .collect()
  }
}

impl super::Surface {
  /// Content-to-window transform and the content box in window coordinates, while attached.
  fn window_placement(&self) -> Option<(Transform2D, [f32; 4])> {
    if !self.attached {
      return None;
    }
    let size = self.metrics.size;
    let content = CanvasItem::rect("", "", 0.0, 0.0, size.width, size.height).window_bounds(self.to_window)?;
    Some((self.to_window, content))
  }
}

/// Drop every item whose id appears again later in the list; also returns
/// the duplicated ids, sorted.
fn last_per_id(mut items: Vec<CanvasItem>) -> (Vec<CanvasItem>, Vec<String>) {
  let mut last = std::collections::HashMap::with_capacity(items.len());
  for (index, item) in items.iter().enumerate() {
    last.insert(item.id.as_str(), index);
  }
  if last.len() == items.len() {
    return (items, Vec::new());
  }
  let keep: Vec<bool> = items
    .iter()
    .enumerate()
    .map(|(index, item)| last[item.id.as_str()] == index)
    .collect();
  let mut duplicates: Vec<String> = items
    .iter()
    .zip(&keep)
    .filter(|(_, kept)| !**kept)
    .map(|(item, _)| item.id.clone())
    .collect();
  duplicates.sort();
  duplicates.dedup();
  let mut index = 0;
  items.retain(|_| {
    index += 1;
    keep[index - 1]
  });
  (items, duplicates)
}

/// Intersection of two `[x, y, width, height]` boxes; `None` when they are
/// disjoint. Touching or zero-size results are kept.
fn clip(bounds: [f32; 4], to: [f32; 4]) -> Option<[f32; 4]> {
  let left = bounds[0].max(to[0]);
  let top = bounds[1].max(to[1]);
  let right = (bounds[0] + bounds[2]).min(to[0] + to[2]);
  let bottom = (bounds[1] + bounds[3]).min(to[1] + to[3]);
  (right >= left && bottom >= top).then_some([left, top, right - left, bottom - top])
}
