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

  /// Paints the whole canvas with `painter`, given a context of a canvas that was reset and its metrics: when the
  /// canvas is first laid out, when it is laid out at another size or scale, and when it is built with another
  /// painter (a builder built again with other data). A builder draws on a canvas with it, with no component to
  /// hold an element ref, an observer or its subscription: the painter lives with the node.
  ///
  /// ```
  /// use lurq::components::Canvas;
  ///
  /// let bar = Canvas::new().width(120.0).height(8.0).paint(|draw, metrics| {
  ///   draw.set_fill_style("#2563eb");
  ///   draw.fill_rect(0.0, 0.0, metrics.size.width * 0.7, metrics.size.height);
  /// });
  /// ```
  pub fn paint(
    mut self,
    painter: impl Fn(&crate::canvas::Context2D, crate::canvas::CanvasMetrics) + Send + Sync + 'static,
  ) -> Self {
    self.node.set_canvas_painter(std::sync::Arc::new(painter));
    self
  }
}

impl Default for Canvas {
  fn default() -> Self {
    Self::new()
  }
}
