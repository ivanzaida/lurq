use std::sync::Arc;

use super::slot::single_slot_child as required_single_slot_child;
use crate::{
  app::{component::Component, ctx::Ctx, events::DropEvent},
  node::Element,
};

type DropCallback = Arc<dyn Fn(&DropEvent) + Send + Sync>;

#[derive(Clone, Default)]
pub struct DropZoneProps {
  pub on_drop: Option<DropCallback>,
  child: Option<Element>,
}

impl DropZoneProps {
  pub fn new() -> Self {
    Self::default()
  }

  pub fn on_drop(mut self, f: impl Fn(&DropEvent) + Send + Sync + 'static) -> Self {
    self.on_drop = Some(Arc::new(f));
    self
  }
}

impl PartialEq for DropZoneProps {
  fn eq(&self, other: &Self) -> bool {
    let same_callback = match (&self.on_drop, &other.on_drop) {
      (Some(left), Some(right)) => Arc::ptr_eq(left, right),
      (None, None) => true,
      _ => false,
    };
    same_callback && self.child.is_none() && other.child.is_none()
  }
}

pub struct DropZone;

impl DropZone {
  pub fn mount(ctx: &mut Ctx, mut props: DropZoneProps, child: impl Into<Element>) -> Element {
    props.child = Some(child.into());
    ctx.mount::<Self>(props)
  }
}

impl Component for DropZone {
  type Props = DropZoneProps;

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let props = ctx.props::<Self::Props>().clone();
    let mut child = explicit_child(ctx, &props);

    if let Some(on_drop) = props.on_drop {
      child.node = child.node.on_drop(move |event| on_drop(event));
    }

    child
  }
}

fn explicit_child(ctx: &Ctx, props: &DropZoneProps) -> Element {
  required_single_slot_child(ctx, "DropZone");
  props
    .child
    .clone()
    .expect("DropZone requires an explicit child; use DropZone::mount(ctx, props, child)")
}
