use std::{
  fmt,
  sync::{Arc, Mutex},
};

use super::slot::single_slot_child as required_single_slot_child;
use crate::{
  app::{component::Component, ctx::Ctx, events::DragEvent},
  core::{ElementRect, ElementRef},
  layout::layout_kind::LayoutKind,
  node::{Element, Node},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DragBounds {
  None,
  SelfBounds,
}

#[derive(Clone, crate::DevtoolsInspectable)]
pub struct DragContainerProps {
  pub bounds: DragBounds,
  #[devtools_ignore]
  child: Option<Element>,
}

impl fmt::Debug for DragContainerProps {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_struct("DragContainerProps")
      .field("bounds", &self.bounds)
      .field("child", &self.child.as_ref().map(|_| "<slot child>"))
      .finish()
  }
}

impl DragContainerProps {
  pub fn new() -> Self {
    Self {
      bounds: DragBounds::SelfBounds,
      child: None,
    }
  }

  pub fn bounds(mut self, bounds: DragBounds) -> Self {
    self.bounds = bounds;
    self
  }
}

impl Default for DragContainerProps {
  fn default() -> Self {
    Self::new()
  }
}

impl PartialEq for DragContainerProps {
  fn eq(&self, other: &Self) -> bool {
    self.bounds == other.bounds && self.child.is_none() && other.child.is_none()
  }
}

pub struct DragContainer;

impl DragContainer {
  pub fn mount(ctx: &mut Ctx, mut props: DragContainerProps, child: impl Into<Element>) -> Element {
    props.child = Some(child.into());
    ctx.mount::<Self>(props)
  }
}

impl Component for DragContainer {
  type Props = DragContainerProps;

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let props = ctx.props::<Self::Props>().clone();
    let mut child = explicit_child(ctx, &props);

    if props.bounds == DragBounds::SelfBounds {
      let container_ref = child.node.element_ref_handle();
      apply_container_bounds(&mut child.node, container_ref);
    }

    child
  }
}

fn apply_container_bounds(node: &mut Node, container_ref: ElementRef) {
  for child in &mut node.children {
    apply_container_bounds(child, container_ref.clone());
  }

  if !has_drag_handler(node) {
    return;
  }

  let draggable_ref = node.element_ref_handle().mutable();
  let extent_ref = drag_extent_ref(node).mutable();
  let drag_extent = Arc::new(Mutex::new(None::<DragExtent>));
  let constrained_drag = Arc::new(Mutex::new(None::<ConstrainedDrag>));

  let existing_start = node.events.on_drag_start.clone();
  node.events.on_drag_start = Some(Arc::new({
    let drag_extent = drag_extent.clone();
    let constrained_drag = constrained_drag.clone();
    let container_ref = container_ref.clone();
    move |event: &DragEvent| {
      *drag_extent.lock().unwrap() = None;
      *constrained_drag.lock().unwrap() = Some(ConstrainedDrag::start(event, &container_ref));
      if let Some(existing_start) = &existing_start {
        existing_start(event);
      }
    }
  }));

  let existing = node.events.on_drag_move.clone();
  node.events.on_drag_move = Some(Arc::new({
    let drag_extent = drag_extent.clone();
    let constrained_drag = constrained_drag.clone();
    move |event: &DragEvent| {
      let adjusted_event = constrained_event(event, &container_ref, &constrained_drag);
      let before_dragged = draggable_ref.bounds();
      let before_extent = extent_ref.bounds();
      let extent = {
        let mut drag_extent = drag_extent.lock().unwrap();
        *drag_extent.get_or_insert_with(|| DragExtent {
          offset_x: before_extent.x - before_dragged.x,
          offset_y: before_extent.y - before_dragged.y,
          width: before_extent.width,
          height: before_extent.height,
        })
      };

      if let Some(existing) = &existing {
        if let Some(adjusted_event) = adjusted_event {
          existing(&adjusted_event);
        }
      }

      clamp_to_container(&draggable_ref, &container_ref, extent);
    }
  }));

  let existing_end = node.events.on_drag_end.clone();
  node.events.on_drag_end = Some(Arc::new(move |event: &DragEvent| {
    if let Some(existing_end) = &existing_end {
      existing_end(event);
    }
    *drag_extent.lock().unwrap() = None;
    *constrained_drag.lock().unwrap() = None;
  }));
}

fn has_drag_handler(node: &Node) -> bool {
  node.events.on_drag_start.is_some() || node.events.on_drag_move.is_some() || node.events.on_drag_end.is_some()
}

#[derive(Clone, Copy)]
struct DragExtent {
  offset_x: f32,
  offset_y: f32,
  width: f32,
  height: f32,
}

#[derive(Clone, Copy)]
struct ConstrainedDrag {
  start_x: f32,
  start_y: f32,
  last_x: f32,
  last_y: f32,
  stopped: bool,
}

impl ConstrainedDrag {
  fn start(event: &DragEvent, container_ref: &ElementRef) -> Self {
    let container = container_ref.bounds();
    let (x, y) = clamp_point_to_rect(event.x, event.y, container);
    Self {
      start_x: x,
      start_y: y,
      last_x: x,
      last_y: y,
      stopped: !point_in_rect(event.x, event.y, container),
    }
  }
}

fn constrained_event(
  event: &DragEvent,
  container_ref: &ElementRef,
  constrained_drag: &Mutex<Option<ConstrainedDrag>>,
) -> Option<DragEvent> {
  let container = container_ref.bounds();
  if container.width <= 0.0 || container.height <= 0.0 {
    return Some(*event);
  }

  let mut drag = constrained_drag.lock().unwrap();
  let drag = drag.get_or_insert_with(|| ConstrainedDrag::start(event, container_ref));
  if drag.stopped {
    return None;
  }

  let inside = point_in_rect(event.x, event.y, container);
  let (x, y) = if inside {
    (event.x, event.y)
  } else {
    clamp_point_to_rect(event.x, event.y, container)
  };

  let adjusted = DragEvent {
    x,
    y,
    delta_x: x - drag.last_x,
    delta_y: y - drag.last_y,
    total_delta_x: x - drag.start_x,
    total_delta_y: y - drag.start_y,
    ..*event
  };

  drag.last_x = x;
  drag.last_y = y;
  drag.stopped = !inside;
  Some(adjusted)
}

fn point_in_rect(x: f32, y: f32, rect: ElementRect) -> bool {
  x >= rect.x && x <= rect.x + rect.width && y >= rect.y && y <= rect.y + rect.height
}

fn clamp_point_to_rect(x: f32, y: f32, rect: ElementRect) -> (f32, f32) {
  if rect.width <= 0.0 || rect.height <= 0.0 {
    return (x, y);
  }
  (
    x.clamp(rect.x, rect.x + rect.width),
    y.clamp(rect.y, rect.y + rect.height),
  )
}

fn drag_extent_ref(node: &mut Node) -> ElementRef {
  match node.layout_kind {
    LayoutKind::AbsoluteModifier {
      width: None,
      height: None,
      ..
    }
    | LayoutKind::OffsetModifier { .. }
      if node.children.len() == 1 =>
    {
      drag_extent_ref(&mut node.children[0])
    }
    _ => node.element_ref_handle(),
  }
}

fn clamp_to_container(draggable_ref: &crate::core::ElementRefMut, container_ref: &ElementRef, extent: DragExtent) {
  let dragged = draggable_ref.bounds();
  let container = container_ref.bounds();

  if extent.width <= 0.0 || extent.height <= 0.0 || container.width <= 0.0 || container.height <= 0.0 {
    return;
  }

  let extent_x = dragged.x + extent.offset_x;
  let extent_y = dragged.y + extent.offset_y;
  let min_x = container.x;
  let min_y = container.y;
  let max_x = container.x + (container.width - extent.width).max(0.0);
  let max_y = container.y + (container.height - extent.height).max(0.0);
  let clamped_x = extent_x.clamp(min_x, max_x);
  let clamped_y = extent_y.clamp(min_y, max_y);

  if clamped_x == extent_x && clamped_y == extent_y {
    return;
  }

  let adjust_x = clamped_x - extent_x;
  let adjust_y = clamped_y - extent_y;
  draggable_ref.set_bounds(ElementRect {
    x: dragged.x + adjust_x,
    y: dragged.y + adjust_y,
    relative_x: dragged.relative_x + adjust_x,
    relative_y: dragged.relative_y + adjust_y,
    width: dragged.width,
    height: dragged.height,
  });
}

fn single_slot_child(ctx: &Ctx) -> Element {
  required_single_slot_child(ctx, "DragContainer")
}

fn explicit_child(ctx: &Ctx, props: &DragContainerProps) -> Element {
  single_slot_child(ctx);
  props
    .child
    .clone()
    .expect("DragContainer requires an explicit child; use DragContainer::mount(ctx, props, child)")
}
