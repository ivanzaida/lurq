use crate::{impl_into_node, layout::layout_kind::ScrollState, node::Element};

impl_into_node!(ScrollVertical);
impl_into_node!(ScrollHorizontal);
impl_into_node!(ScrollBoth);

impl ScrollVertical {
  pub fn new(child: impl Into<Element>) -> Self {
    Self::from_node(crate::node::dsl::scroll_vertical(child.into().node))
  }

  pub fn with_scroll_state(mut self, existing: ScrollState) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::with_scroll_state(node, existing));
    self
  }
}

impl Default for ScrollVertical {
  fn default() -> Self {
    Self::new(Element::new())
  }
}

impl ScrollHorizontal {
  pub fn new(child: impl Into<Element>) -> Self {
    Self::from_node(crate::node::dsl::scroll_horizontal(child.into().node))
  }

  pub fn with_scroll_state(mut self, existing: ScrollState) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::with_scroll_state(node, existing));
    self
  }
}

impl Default for ScrollHorizontal {
  fn default() -> Self {
    Self::new(Element::new())
  }
}

impl ScrollBoth {
  pub fn new(child: impl Into<Element>) -> Self {
    Self::from_node(crate::node::dsl::scroll_both(child.into().node))
  }

  pub fn with_scroll_state(mut self, existing: ScrollState) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::with_scroll_state(node, existing));
    self
  }
}

impl Default for ScrollBoth {
  fn default() -> Self {
    Self::new(Element::new())
  }
}
