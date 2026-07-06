use std::{
  any::Any,
  fmt,
  sync::{Arc, Mutex},
};

use super::slot::single_slot_child as required_single_slot_child;
use crate::{
  app::{
    component::Component,
    ctx::Ctx,
    events::{DragEvent, DragPayload, DropResult, MouseButtonMask},
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
  pub start_drag_buttons: MouseButtonMask,
  pub drop_miss_behavior: DropMissBehavior,
  pub override_policy: DragOverridePolicy,
  /// An externally owned ref for the dragged element; when absent the
  /// component allocates one internally.
  #[devtools_ignore]
  pub element_ref: Option<ElementRefMut>,
  /// Elements that move in lockstep with the dragged one (e.g. the rest of
  /// a multi-selection), and share its revert/override handling.
  #[devtools_ignore]
  pub followers: Vec<ElementRefMut>,
  /// Data delivered to the drop target's handler via `DropEvent::payload`.
  #[devtools_ignore]
  pub payload: Option<DragPayload>,
  #[devtools_ignore]
  child: Option<Element>,
}

impl fmt::Debug for DraggableProps {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_struct("DraggableProps")
      .field("on_drag_start", &self.on_drag_start.as_ref().map(|_| "<callback>"))
      .field("on_drag_move", &self.on_drag_move.as_ref().map(|_| "<callback>"))
      .field("on_drag_end", &self.on_drag_end.as_ref().map(|_| "<callback>"))
      .field("start_drag_buttons", &self.start_drag_buttons)
      .field("drop_miss_behavior", &self.drop_miss_behavior)
      .field("override_policy", &self.override_policy)
      .field("followers", &self.followers.len())
      .field("payload", &self.payload.as_ref().map(|_| "<payload>"))
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

/// What happens to the drag's bounds overrides once the drag ends.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, crate::DevtoolsInspectable)]
pub enum DragOverridePolicy {
  /// The overrides persist — the elements stay where the drag left them.
  #[default]
  Keep,
  /// The overrides are cleared at drag end, handing position authority back
  /// to layout. For elements whose position re-renders from state the drop
  /// target's handler commits: an accepted drop settles on the committed
  /// coordinates, a missed drop reverts.
  Clear,
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

  pub fn start_drag_buttons(mut self, buttons: MouseButtonMask) -> Self {
    self.start_drag_buttons = buttons;
    self
  }

  pub fn drop_miss_behavior(mut self, behavior: DropMissBehavior) -> Self {
    self.drop_miss_behavior = behavior;
    self
  }

  pub fn override_policy(mut self, policy: DragOverridePolicy) -> Self {
    self.override_policy = policy;
    self
  }

  pub fn element_ref(mut self, element_ref: ElementRefMut) -> Self {
    self.element_ref = Some(element_ref);
    self
  }

  pub fn follower(mut self, follower: ElementRefMut) -> Self {
    self.followers.push(follower);
    self
  }

  pub fn followers(mut self, followers: impl IntoIterator<Item = ElementRefMut>) -> Self {
    self.followers.extend(followers);
    self
  }

  pub fn payload(mut self, payload: impl Any + Send + Sync) -> Self {
    self.payload = Some(Arc::new(payload));
    self
  }
}

impl PartialEq for DraggableProps {
  fn eq(&self, other: &Self) -> bool {
    same_callback(&self.on_drag_start, &other.on_drag_start)
      && same_callback(&self.on_drag_move, &other.on_drag_move)
      && same_callback(&self.on_drag_end, &other.on_drag_end)
      && self.start_drag_buttons == other.start_drag_buttons
      && self.drop_miss_behavior == other.drop_miss_behavior
      && self.override_policy == other.override_policy
      && same_element_ref(&self.element_ref, &other.element_ref)
      && self.followers.len() == other.followers.len()
      && self
        .followers
        .iter()
        .zip(&other.followers)
        .all(|(left, right)| left.as_ref().same_handle(&right.as_ref()))
      && same_payload(&self.payload, &other.payload)
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
    // Allocate the internal ref unconditionally so the ctx ref cursor stays
    // stable when the prop flips between renders.
    let internal_ref = ctx.element_ref_mut();
    let element_ref = props.element_ref.clone().unwrap_or(internal_ref);
    let followers = Arc::new(props.followers.clone());
    let start_bounds = Arc::new(Mutex::new(Vec::<(ElementRefMut, ElementRect)>::new()));
    let mut child = explicit_child(ctx, &props);

    child.node = child
      .node
      .ref_element(element_ref.clone())
      .start_drag_buttons(props.start_drag_buttons)
      .on_drag_start({
        let on_drag_start = props.on_drag_start.clone();
        let element_ref = element_ref.clone();
        let followers = followers.clone();
        let start_bounds = start_bounds.clone();
        move |event| {
          let mut starts = start_bounds.lock().unwrap();
          starts.clear();
          starts.push((element_ref.clone(), element_ref.bounds()));
          for follower in attached(&followers) {
            starts.push((follower.clone(), follower.bounds()));
          }
          drop(starts);
          if let Some(on_drag_start) = &on_drag_start {
            on_drag_start(&event);
          }
        }
      })
      .on_drag_move({
        let element_ref = element_ref.clone();
        let followers = followers.clone();
        let on_drag_move = props.on_drag_move.clone();
        move |event: DragEvent| {
          move_element(&element_ref, event.delta_x, event.delta_y);
          for follower in attached(&followers) {
            move_element(follower, event.delta_x, event.delta_y);
          }
          if let Some(on_drag_move) = &on_drag_move {
            on_drag_move(&event);
          }
        }
      })
      .on_drag_end({
        let on_drag_end = props.on_drag_end.clone();
        let element_ref = element_ref.clone();
        let followers = followers.clone();
        let start_bounds = start_bounds.clone();
        let drop_miss_behavior = props.drop_miss_behavior;
        let override_policy = props.override_policy;
        move |event: DragEvent| {
          if drop_miss_behavior == DropMissBehavior::RevertToDragStart && event.drop_result == Some(DropResult::Missed)
          {
            for (dragged, bounds) in start_bounds.lock().unwrap().drain(..) {
              dragged.set_bounds(bounds);
            }
          }
          if override_policy == DragOverridePolicy::Clear {
            element_ref.clear_bounds_override();
            for follower in followers.iter() {
              follower.clear_bounds_override();
            }
          }
          if let Some(on_drag_end) = &on_drag_end {
            on_drag_end(&event);
          }
        }
      });

    if let Some(payload) = props.payload.clone() {
      child.node = child.node.drag_payload(payload);
    }

    child
  }
}

fn attached(followers: &Arc<Vec<ElementRefMut>>) -> impl Iterator<Item = &ElementRefMut> {
  followers.iter().filter(|follower| follower.is_attached())
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

fn same_element_ref(left: &Option<ElementRefMut>, right: &Option<ElementRefMut>) -> bool {
  match (left, right) {
    (Some(left), Some(right)) => left.as_ref().same_handle(&right.as_ref()),
    (None, None) => true,
    _ => false,
  }
}

fn same_payload(left: &Option<DragPayload>, right: &Option<DragPayload>) -> bool {
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
