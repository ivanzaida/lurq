use std::{
  fmt,
  sync::{Arc, Mutex},
};

use super::slot::single_slot_child as required_single_slot_child;
use crate::{
  app::{
    component::Component,
    ctx::Ctx,
    events::{DragEvent, DropResult},
  },
  core::{ElementRect, ElementRefMut},
  node::Element,
};

type DragCallback = Arc<dyn Fn(&DragEvent) + Send + Sync>;

#[derive(Clone, Default, crate::DevtoolsInspectable)]
pub struct DraggableProps {
  #[devtools_ignore]
  pub on_drag_start: Option<DragCallback>,
  #[devtools_ignore]
  pub on_drag_move: Option<DragCallback>,
  #[devtools_ignore]
  pub on_drag_end: Option<DragCallback>,
  pub drop_miss_behavior: DropMissBehavior,
  #[devtools_ignore]
  child: Option<Element>,
}

impl fmt::Debug for DraggableProps {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_struct("DraggableProps")
      .field("on_drag_start", &self.on_drag_start.as_ref().map(|_| "<callback>"))
      .field("on_drag_move", &self.on_drag_move.as_ref().map(|_| "<callback>"))
      .field("on_drag_end", &self.on_drag_end.as_ref().map(|_| "<callback>"))
      .field("drop_miss_behavior", &self.drop_miss_behavior)
      .field("child", &self.child.as_ref().map(|_| "<slot child>"))
      .finish()
  }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, crate::DevtoolsInspectable)]
pub enum DropMissBehavior {
  #[default]
  KeepPosition,
  RevertToDragStart,
}

impl DraggableProps {
  pub fn new() -> Self {
    Self::default()
  }

  pub fn on_drag_start(mut self, f: impl Fn(&DragEvent) + Send + Sync + 'static) -> Self {
    self.on_drag_start = Some(Arc::new(f));
    self
  }

  pub fn on_drag_move(mut self, f: impl Fn(&DragEvent) + Send + Sync + 'static) -> Self {
    self.on_drag_move = Some(Arc::new(f));
    self
  }

  pub fn on_drag_end(mut self, f: impl Fn(&DragEvent) + Send + Sync + 'static) -> Self {
    self.on_drag_end = Some(Arc::new(f));
    self
  }

  pub fn drop_miss_behavior(mut self, behavior: DropMissBehavior) -> Self {
    self.drop_miss_behavior = behavior;
    self
  }
}

impl PartialEq for DraggableProps {
  fn eq(&self, other: &Self) -> bool {
    same_callback(&self.on_drag_start, &other.on_drag_start)
      && same_callback(&self.on_drag_move, &other.on_drag_move)
      && same_callback(&self.on_drag_end, &other.on_drag_end)
      && self.drop_miss_behavior == other.drop_miss_behavior
      && self.child.is_none()
      && other.child.is_none()
  }
}

pub struct Draggable;

impl Draggable {
  pub fn mount(ctx: &mut Ctx, mut props: DraggableProps, child: impl Into<Element>) -> Element {
    props.child = Some(child.into());
    ctx.mount::<Self>(props)
  }
}

impl Component for Draggable {
  type Props = DraggableProps;

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let props = ctx.props::<Self::Props>().clone();
    let element_ref = ctx.element_ref_mut();
    let drag_start_bounds = Arc::new(Mutex::new(None::<ElementRect>));
    let mut child = explicit_child(ctx, &props);

    child.node = child
      .node
      .ref_element(element_ref.clone())
      .on_drag_start({
        let on_drag_start = props.on_drag_start.clone();
        let element_ref = element_ref.clone();
        let drag_start_bounds = drag_start_bounds.clone();
        move |event| {
          *drag_start_bounds.lock().unwrap() = Some(element_ref.bounds());
          if let Some(on_drag_start) = &on_drag_start {
            on_drag_start(event);
          }
        }
      })
      .on_drag_move({
        let element_ref = element_ref.clone();
        let on_drag_move = props.on_drag_move.clone();
        move |event| {
          move_element(&element_ref, event.delta_x, event.delta_y);
          if let Some(on_drag_move) = &on_drag_move {
            on_drag_move(event);
          }
        }
      })
      .on_drag_end({
        let on_drag_end = props.on_drag_end.clone();
        let element_ref = element_ref.clone();
        let drag_start_bounds = drag_start_bounds.clone();
        let drop_miss_behavior = props.drop_miss_behavior;
        move |event| {
          if drop_miss_behavior == DropMissBehavior::RevertToDragStart && event.drop_result == Some(DropResult::Missed)
          {
            if let Some(bounds) = *drag_start_bounds.lock().unwrap() {
              element_ref.set_bounds(bounds);
            }
          }
          if let Some(on_drag_end) = &on_drag_end {
            on_drag_end(event);
          }
        }
      });

    child
  }
}

fn move_element(element_ref: &ElementRefMut, delta_x: f32, delta_y: f32) {
  let rect = element_ref.bounds();
  element_ref.set_relative_bounds(
    rect.relative_x + delta_x,
    rect.relative_y + delta_y,
    rect.width,
    rect.height,
  );
}

fn same_callback(left: &Option<DragCallback>, right: &Option<DragCallback>) -> bool {
  match (left, right) {
    (Some(left), Some(right)) => Arc::ptr_eq(left, right),
    (None, None) => true,
    _ => false,
  }
}

fn explicit_child(ctx: &Ctx, props: &DraggableProps) -> Element {
  required_single_slot_child(ctx, "Draggable");
  props
    .child
    .clone()
    .expect("Draggable requires an explicit child; use Draggable::mount(ctx, props, child)")
}
