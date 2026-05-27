use crate::{impl_into_node, layout::text_style::TextStyle};

impl_into_node!(Text);

impl Text {
  pub fn new(content: &str) -> Self {
    Self::from_node(crate::node::Node::text(content))
  }

  pub fn styled(content: &str, style: TextStyle) -> Self {
    Self::from_node(crate::node::Node::text_styled(content, style))
  }
}

impl Default for Text {
  fn default() -> Self {
    Self::new("")
  }
}
