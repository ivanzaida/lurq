pub use crate::node::node_kind::TextOverflow;
use crate::{
  app::theme::TypographyStyle,
  impl_into_node,
  layout::text_style::{TextAlign, TextStyle},
  node::{TextColor, TextTransformMode},
};

impl_into_node!(Text);

impl Text {
  pub fn new(content: &str) -> Self {
    Self::from_node(crate::node::Node::text(content))
  }

  pub fn styled(content: &str, style: TextStyle) -> Self {
    Self::from_node(crate::node::Node::text_styled(content, style))
  }

  pub fn nowrap(mut self) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::text_wrap(node, false));
    self
  }

  pub fn selectable(mut self, selectable: bool) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::selectable(node, selectable));
    self
  }

  pub fn text_transform_mode(mut self, mode: TextTransformMode) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::text_transform_mode(node, mode));
    self
  }

  pub fn variant(mut self, style: impl Into<TypographyStyle>) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::text_variant(node, style));
    self
  }

  pub fn text_align(mut self, align: impl Into<TextAlign>) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::text_align(node, align));
    self
  }

  pub fn text_overflow(mut self, overflow: TextOverflow) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::text_overflow(node, overflow));
    self
  }

  pub fn color(mut self, color: impl Into<TextColor>) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::text_color(node, color));
    self
  }

  pub fn caret_color(mut self, color: impl Into<crate::node::TextColor>) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::caret_color(node, color));
    self
  }
}

impl Default for Text {
  fn default() -> Self {
    Self::new("")
  }
}
