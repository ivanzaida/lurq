use crate::{
  app::theme::TypographyId,
  impl_into_node,
  layout::text_style::TextStyle,
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
    self.node = self.node.text_wrap(false);
    self
  }

  pub fn selectable(mut self, selectable: bool) -> Self {
    self.node = self.node.selectable(selectable);
    self
  }

  pub fn text_transform_mode(mut self, mode: TextTransformMode) -> Self {
    self.node = self.node.text_transform_mode(mode);
    self
  }

  pub fn variant(mut self, id: impl Into<TypographyId>) -> Self {
    self.node = self.node.text_variant(id);
    self
  }

  pub fn color(mut self, color: impl Into<TextColor>) -> Self {
    self.node = self.node.text_color(color);
    self
  }

  pub fn caret_color(mut self, color: impl Into<crate::node::TextColor>) -> Self {
    self.node = self.node.caret_color(color);
    self
  }
}

impl Default for Text {
  fn default() -> Self {
    Self::new("")
  }
}
