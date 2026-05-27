use crate::{impl_into_node, layout::Alignment, node::Element};

impl_into_node!(Column);

impl Column {
  pub fn new() -> Self {
    Self::from_node(crate::node::Node::column(0.0, Alignment::Start, vec![]))
  }

  pub fn with(spacing: f32, align: Alignment, children: impl IntoIterator<Item = impl Into<Element>>) -> Self {
    Self::new().spacing(spacing).align_items(align).with_children(children)
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

  pub fn spacing(mut self, spacing: f32) -> Self {
    self.node = self.node.spacing(spacing);
    self
  }

  pub fn align_items(mut self, align: Alignment) -> Self {
    self.node = self.node.align_items(align);
    self
  }

  pub fn justify(mut self, justify: crate::layout::layout_kind::Justify) -> Self {
    self.node = self.node.justify(justify);
    self
  }

  pub fn wrap(mut self) -> Self {
    self.node = self.node.wrap();
    self
  }
}

impl Default for Column {
  fn default() -> Self {
    Self::new()
  }
}
