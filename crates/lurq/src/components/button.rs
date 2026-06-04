use crate::{
  impl_into_node,
  layout::Alignment,
  node::{ButtonKind, Element},
};

impl_into_node!(Button);

impl Button {
  pub fn new(label: &str) -> Self {
    Self::from_node(
      crate::node::Node::row(0.0, Alignment::Center, vec![crate::node::Node::text(label)])
        .button_kind(ButtonKind::Button),
    )
  }

  pub fn empty() -> Self {
    Self::from_node(crate::node::Node::row(0.0, Alignment::Center, vec![]).button_kind(ButtonKind::Button))
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

  #[cfg(feature = "form")]
  pub fn submit(mut self) -> Self {
    self.node = self.node.button_kind(ButtonKind::Submit);
    self
  }

  pub fn button(mut self) -> Self {
    self.node = self.node.button_kind(ButtonKind::Button);
    self
  }

  pub fn kind(mut self, kind: ButtonKind) -> Self {
    self.node = self.node.button_kind(kind);
    self
  }

  pub fn spacing(mut self, spacing: impl Into<crate::node::SpacingValue>) -> Self {
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
}
