use crate::{impl_into_node, layout::layout_kind::ScrollState, node::Element};

impl_into_node!(ScrollVertical);
impl_into_node!(ScrollHorizontal);
impl_into_node!(ScrollBoth);

impl ScrollVertical {
  pub fn new(child: impl Into<Element>) -> Self {
    Self::from_node(crate::node::dsl::scroll_vertical(child.into().into_node()))
  }

  pub fn with_scroll_state(mut self, existing: ScrollState) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::with_scroll_state(node, existing));
    self
  }

  /// Keeps the bars beside what scrolls: the vertical one starts under what sticks at the top (a table's
  /// header), the horizontal one after what sticks at the start (its first column), and the thumb is the share
  /// of what scrolls that shows. Off unless set: a page with a banner that stays keeps its bar along the whole
  /// height, as a browser's does.
  pub fn scrollbars_clear_of_sticky(mut self, clear: bool) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::scrollbars_clear_of_sticky(node, clear));
    self
  }

  pub fn culling(mut self, enabled: bool) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::culling(node, enabled));
    self
  }

  /// Middle-button autoscroll, off by default; see
  /// `Node::middle_button_autoscroll`.
  pub fn middle_button_autoscroll(mut self, enabled: bool) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::middle_button_autoscroll(node, enabled));
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
    Self::from_node(crate::node::dsl::scroll_horizontal(child.into().into_node()))
  }

  pub fn with_scroll_state(mut self, existing: ScrollState) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::with_scroll_state(node, existing));
    self
  }

  /// Keeps the bars beside what scrolls: the vertical one starts under what sticks at the top (a table's
  /// header), the horizontal one after what sticks at the start (its first column), and the thumb is the share
  /// of what scrolls that shows. Off unless set: a page with a banner that stays keeps its bar along the whole
  /// height, as a browser's does.
  pub fn scrollbars_clear_of_sticky(mut self, clear: bool) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::scrollbars_clear_of_sticky(node, clear));
    self
  }

  pub fn culling(mut self, enabled: bool) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::culling(node, enabled));
    self
  }

  /// Middle-button autoscroll, off by default; see
  /// `Node::middle_button_autoscroll`.
  pub fn middle_button_autoscroll(mut self, enabled: bool) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::middle_button_autoscroll(node, enabled));
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
    Self::from_node(crate::node::dsl::scroll_both(child.into().into_node()))
  }

  /// Lays the content out at least as wide as the viewport: what is in it fills the width it is shown in, and
  /// scrolls sideways only where it needs more. A table, whose columns share the room there is.
  pub fn fill_width(mut self, fills: bool) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::fills_viewport_width(node, fills));
    self
  }

  pub fn with_scroll_state(mut self, existing: ScrollState) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::with_scroll_state(node, existing));
    self
  }

  /// Keeps the bars beside what scrolls: the vertical one starts under what sticks at the top (a table's
  /// header), the horizontal one after what sticks at the start (its first column), and the thumb is the share
  /// of what scrolls that shows. Off unless set: a page with a banner that stays keeps its bar along the whole
  /// height, as a browser's does.
  pub fn scrollbars_clear_of_sticky(mut self, clear: bool) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::scrollbars_clear_of_sticky(node, clear));
    self
  }

  pub fn culling(mut self, enabled: bool) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::culling(node, enabled));
    self
  }

  /// Middle-button autoscroll, off by default; see
  /// `Node::middle_button_autoscroll`.
  pub fn middle_button_autoscroll(mut self, enabled: bool) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::middle_button_autoscroll(node, enabled));
    self
  }
}

impl Default for ScrollBoth {
  fn default() -> Self {
    Self::new(Element::new())
  }
}
