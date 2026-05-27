use crate::impl_into_node;

impl_into_node!(Spacer);

impl Spacer {
  pub fn new() -> Self {
    Self::from_node(crate::node::Node::new())
  }
}

impl Default for Spacer {
  fn default() -> Self {
    Self::new()
  }
}
