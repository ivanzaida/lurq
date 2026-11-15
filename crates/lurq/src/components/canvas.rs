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

  /// Draws the canvas as [`Canvas::paint`] does, and moves it: when it is first shown, and when `value` changes
  /// (`of`, its hash, tells), `paint` is given where the drawing is on its way, a [`CanvasMove`] from what it was
  /// drawn from before to `value`, and it is drawn again at each frame until it is there, for the `motion`'s
  /// duration; at rest it is drawn only as `paint` would be. Motion is off where it is reduced
  /// (`Theme::reduced_motion`), and a resize ends it: the drawing is drawn where it goes. `key` is still the hash of
  /// everything the drawing follows from, `value` among it.
  ///
  /// [`CanvasMove`]: crate::canvas::CanvasMove
  pub fn paint_moving<T: Send + Sync + 'static>(
    mut self,
    key: u64,
    motion: crate::canvas::CanvasMotion,
    of: u64,
    value: impl Into<std::sync::Arc<T>>,
    paint: impl Fn(&crate::canvas::CanvasHandle, crate::canvas::CanvasMove<'_, T>) + Send + Sync + 'static,
  ) -> Self {
    self.node.set_canvas_painter(crate::canvas::CanvasPainter::moving(
      key,
      motion,
      of,
      value.into(),
      paint,
    ));
    self
  }
}

impl Default for Canvas {
  fn default() -> Self {
    Self::new()
  }
}
