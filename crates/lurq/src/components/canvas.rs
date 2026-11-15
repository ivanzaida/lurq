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

  /// Draws the canvas with `paint` when it is first laid out, when its size or its scale changes, and when `key`
  /// changes, and not otherwise: a drawing that follows from what `key` stands for (a chart's data and the theme it
  /// is drawn in) is drawn only when that changes, whatever renders around it. `paint` draws the whole of it, from a
  /// cleared surface.
  pub fn paint(mut self, key: u64, paint: impl Fn(&crate::canvas::CanvasHandle) + Send + Sync + 'static) -> Self {
    self
      .node
      .set_canvas_painter(crate::canvas::CanvasPainter::new(key, paint));
    self
  }
}

impl Default for Canvas {
  fn default() -> Self {
    Self::new()
  }
}
