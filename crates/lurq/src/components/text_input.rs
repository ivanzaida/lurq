pub use crate::node::node_kind::TextInputOverflow;
use crate::{
  app::theme::CaretMode,
  core::Signal,
  impl_into_node,
  layout::text_style::{TextAlign, TextStyle},
};

impl_into_node!(TextInput);

impl TextInput {
  pub fn new(value: Signal<String>) -> Self {
    Self::from_node(crate::node::Node::text_input(value))
  }

  pub fn styled(value: Signal<String>, style: TextStyle) -> Self {
    Self::from_node(crate::node::Node::text_input_styled(value, style))
  }

  pub fn text_style(mut self, style: TextStyle) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::text_input_style(node, style));
    self
  }

  pub fn placeholder_style(mut self, style: TextStyle) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::text_input_placeholder_style(node, style));
    self
  }

  pub fn text_align(mut self, align: impl Into<TextAlign>) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::text_input_align(node, align));
    self
  }

  pub fn placeholder(mut self, placeholder: &str) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::placeholder(node, placeholder));
    self
  }

  pub fn on_input(mut self, f: impl crate::node::IntoTextInputEventHandler) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::on_input(node, f));
    self
  }

  pub fn off_input(mut self, f: impl crate::node::IntoTextInputEventHandler) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::off_input(node, f));
    self
  }

  pub fn caret_color(mut self, color: impl Into<crate::node::TextColor>) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::caret_color(node, color));
    self
  }

  pub fn selection_color(mut self, color: impl Into<crate::node::TextColor>) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::selection_color(node, color));
    self
  }

  pub fn caret_mode(mut self, mode: CaretMode) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::text_input_caret_mode(node, mode));
    self
  }

  pub fn overflow(mut self, overflow: TextInputOverflow) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::text_input_overflow(node, overflow));
    self
  }

  pub fn mask(mut self) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::text_input_mask(node));
    self
  }

  pub fn mask_char(mut self, mask: char) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::text_input_mask_char(node, mask));
    self
  }

  pub fn unmask(mut self) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::text_input_unmask(node));
    self
  }

  pub fn single_line(mut self) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::text_input_overflow(node, TextInputOverflow::Scroll));
    self
  }

  pub fn multiline(mut self) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::text_input_overflow(node, TextInputOverflow::Multiline));
    self
  }

  pub fn textarea(mut self) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::text_input_rows(node, 2, 6));
    self
  }

  pub fn rows(mut self, min_rows: usize, max_rows: usize) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::text_input_rows(node, min_rows, max_rows));
    self
  }

  pub fn min_rows(mut self, min_rows: usize) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::text_input_min_rows(node, min_rows));
    self
  }

  pub fn max_rows(mut self, max_rows: usize) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::text_input_max_rows(node, max_rows));
    self
  }

  pub fn rows_exact(mut self, rows: usize) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::text_input_rows_exact(node, rows));
    self
  }
}

impl Default for TextInput {
  fn default() -> Self {
    Self::new(Signal::new(String::new()))
  }
}
