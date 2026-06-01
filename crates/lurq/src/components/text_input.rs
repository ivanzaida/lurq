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

  pub fn single_line(mut self) -> Self {
    self.node = self.node.text_input_overflow(TextInputOverflow::Scroll);
    self
  }

  pub fn multiline(mut self) -> Self {
    self.node = self.node.text_input_overflow(TextInputOverflow::Multiline);
    self
  }

  pub fn textarea(mut self) -> Self {
    self.node = self.node.text_input_rows(2, 6);
    self
  }

  pub fn rows(mut self, min_rows: usize, max_rows: usize) -> Self {
    self.node = self.node.text_input_rows(min_rows, max_rows);
    self
  }

  pub fn min_rows(mut self, min_rows: usize) -> Self {
    self.node = self.node.text_input_min_rows(min_rows);
    self
  }

  pub fn max_rows(mut self, max_rows: usize) -> Self {
    self.node = self.node.text_input_max_rows(max_rows);
    self
  }

  pub fn rows_exact(mut self, rows: usize) -> Self {
    self.node = self.node.text_input_rows_exact(rows);
    self
  }
}

impl Default for TextInput {
  fn default() -> Self {
    Self::new(Signal::new(String::new()))
  }
}
