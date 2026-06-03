pub use crate::node::node_kind::TextInputOverflow;
use crate::{core::Signal, impl_into_node, layout::text_style::TextStyle};

impl_into_node!(TextInput);

impl TextInput {
  pub fn new(value: Signal<String>) -> Self {
    Self::from_node(crate::node::Node::text_input(value))
  }

  pub fn styled(value: Signal<String>, style: TextStyle) -> Self {
    Self::from_node(crate::node::Node::text_input_styled(value, style))
  }

  pub fn text_style(mut self, style: TextStyle) -> Self {
    self.node = self.node.text_input_style(style);
    self
  }

  pub fn placeholder_style(mut self, style: TextStyle) -> Self {
    self.node = self.node.text_input_placeholder_style(style);
    self
  }

  pub fn placeholder(mut self, placeholder: &str) -> Self {
    self.node = self.node.placeholder(placeholder);
    self
  }

  pub fn caret_color(mut self, color: impl Into<crate::node::color::Color>) -> Self {
    self.node = self.node.caret_color(color);
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
