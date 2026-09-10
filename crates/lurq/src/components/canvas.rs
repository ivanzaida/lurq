use crate::impl_into_node;

impl_into_node!(Canvas);

impl Canvas {
  /// A persistent 2D surface. Attach an ordinary element ref and obtain its
  /// `as_canvas().context_2d()` after layout. Default size: 300 × 150 logical pixels.
  /// Use the synchronous CPU reference renderer for custom hosts and tests.
  pub fn software(self) -> Self {
    self.node.canvas_handle().unwrap().software();
    self
  }
  pub fn new() -> Self {
    Self::from_node(crate::node::Node::canvas())
  }
}

impl Default for Canvas {
  fn default() -> Self {
    Self::new()
  }
}
