use crate::{impl_into_node, layout::StackAlignment, node::Element};

impl_into_node!(Stack);

impl Stack {
  pub fn new() -> Self {
    Self::from_node(crate::node::Node::stack(StackAlignment::TopStart, vec![]))
  }

  pub fn with(align: StackAlignment, children: impl IntoIterator<Item = impl Into<Element>>) -> Self {
    Self::new().stack_align(align).with_children(children)
  }

  pub fn child(mut self, child: impl Into<Element>) -> Self {
    self.node = self.node.child(child.into().node);
    self
  }

  pub fn with_children(mut self, children: impl IntoIterator<Item = impl Into<Element>>) -> Self {
    self.node = self
      .node
      .with_children(children.into_iter().map(|child| child.into().node));
    self
  }

  pub fn stack_align(mut self, align: StackAlignment) -> Self {
    self.node = self.node.stack_align(align);
    self
  }
}

impl Default for Stack {
  fn default() -> Self {
    Self::new()
  }
}
