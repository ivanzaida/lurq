use crate::{
  app::ctx::Ctx,
  node::{Element, Node, node_kind::NodeKind},
};

pub struct Slot;

impl From<Slot> for Element {
  fn from(_value: Slot) -> Self {
    let mut node = Node::new();
    node.node_kind = NodeKind::Empty;
    Element::from_node(node)
  }
}

pub(crate) fn single_slot_child(ctx: &Ctx, component_name: &str) -> Element {
  let children = ctx.children();
  assert!(
    children.is_empty(),
    "{component_name} does not accept slot children; use {component_name}::mount(ctx, props, child). Got {} slot children",
    children.len()
  );
  Element::new()
}
