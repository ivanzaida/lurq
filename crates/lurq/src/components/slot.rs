use crate::node::{node_kind::NodeKind, Element, Node};

pub struct Slot;

impl From<Slot> for Element {
  fn from(_value: Slot) -> Self {
    let mut node = Node::new();
    node.node_kind = NodeKind::Empty;
    Element::from_node(node)
  }
}
