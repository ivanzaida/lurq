use crate::impl_into_node;

impl_into_node!(Canvas);

impl Canvas {
  /// A persistent 2D surface. Attach an ordinary element ref and obtain its
  /// `as_canvas().context_2d()` after layout. Default size: 300 × 150 logical pixels.
  pub fn new() -> Self {
    Self::from_node(crate::node::Node::canvas())
  }
}

impl Default for Canvas {
  fn default() -> Self {
    Self::new()
  }
}
