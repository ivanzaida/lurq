use crate::{core::Signal, impl_into_node};

impl_into_node!(Checkbox);

impl Checkbox {
  pub fn new(value: Signal<bool>) -> Self {
    Self::from_node(crate::node::Node::checkbox(value))
  }
}

impl Default for Checkbox {
  fn default() -> Self {
    Self::new(Signal::new(false))
  }
}
