use crate::{core::Signal, impl_into_node};

impl_into_node!(Slider);

impl Slider {
  pub fn new(value: Signal<f32>) -> Self {
    Self::from_node(crate::node::Node::slider(value))
  }

  pub fn range(mut self, min: f32, max: f32) -> Self {
    self.node = self.node.range(min, max);
    self
  }
}

impl Default for Slider {
  fn default() -> Self {
    Self::new(Signal::new(0.0))
  }
}
