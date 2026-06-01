pub use crate::node::node_kind::TextInputOverflow;
use crate::{core::Signal, impl_into_node};

impl_into_node!(TextInput);

impl TextInput {
  pub fn new(value: Signal<String>) -> Self {
    Self::from_node(crate::node::Node::text_input(value))
  }

  pub fn placeholder(mut self, placeholder: &str) -> Self {
    self.node = self.node.placeholder(placeholder);
    self
  }

  pub fn overflow(mut self, overflow: TextInputOverflow) -> Self {
    self.node = self.node.text_input_overflow(overflow);
    self
  }
}

impl Default for TextInput {
  fn default() -> Self {
    Self::new(Signal::new(String::new()))
  }
}
