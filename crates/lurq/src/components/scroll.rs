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
  /// header) and stands before what sticks at the right (a column pinned at the end), the horizontal one runs
  /// from what sticks at the start (its first column) to what sticks at the end, and the thumb is the share of
  /// what scrolls that shows. Off unless set: a page with a banner that stays keeps its bar along the whole
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

  /// Scrolls sideways by a wheel turned up and down, where the wheel is not also turned sideways, as a browser's
  /// strip of tabs does: the wheel's turn is taken across, and nothing around it scrolls with it. Off by default:
  /// a list across inside a page lets the page take the wheel.
  pub fn wheel_across(mut self, across: bool) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::wheel_across(node, across));
    self
  }

  /// Lays the content out at least as wide as the viewport, as [`ScrollBoth::fill_width`] does: items that share
  /// the room there is, and scroll sideways only where they need more (a timeline across).
  pub fn fill_width(mut self, fills: bool) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::fills_viewport_width(node, fills));
    self
  }

  pub fn with_scroll_state(mut self, existing: ScrollState) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::with_scroll_state(node, existing));
    self
  }

  /// Keeps the bars beside what scrolls: the vertical one starts under what sticks at the top (a table's
  /// header) and stands before what sticks at the right (a column pinned at the end), the horizontal one runs
  /// from what sticks at the start (its first column) to what sticks at the end, and the thumb is the share of
  /// what scrolls that shows. Off unless set: a page with a banner that stays keeps its bar along the whole
  /// height, as a browser's does.
  pub fn scrollbars_clear_of_sticky(mut self, clear: bool) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::scrollbars_clear_of_sticky(node, clear));
    self
  }

  /// Gives the bar a lane of its own under the content while the content is wider than the view; see
  /// `ScrollBoth::reserve_bar_across`.
  pub fn reserve_bar_across(mut self, reserve: bool) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::reserve_bar_across(node, reserve));
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
  /// Scrolls sideways by a wheel turned up and down, where the wheel is not also turned sideways, as a browser's
  /// strip of tabs does: the wheel's turn is taken across, and nothing around it scrolls with it. Off by default:
  /// a list across inside a page lets the page take the wheel.
  pub fn wheel_across(mut self, across: bool) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::wheel_across(node, across));
    self
  }

  /// Lays the content out at least as wide as the viewport: what is in it fills the width it is shown in, and
  /// scrolls sideways only where it needs more. A table, whose columns share the room there is.
  pub fn fill_width(mut self, fills: bool) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::fills_viewport_width(node, fills));
    self
  }

  /// Gives the bar across a lane of its own under the content while the content is wider than the view: the
  /// viewport ends above it, so nothing it shows is ever under the bar (a table's last row). Content that fits
  /// has no lane and no strip. Off unless set: the bar lies over the content, as an overlay bar does.
  pub fn reserve_bar_across(mut self, reserve: bool) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::reserve_bar_across(node, reserve));
    self
  }

  pub fn with_scroll_state(mut self, existing: ScrollState) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::with_scroll_state(node, existing));
    self
  }

  /// Keeps the bars beside what scrolls: the vertical one starts under what sticks at the top (a table's
  /// header) and stands before what sticks at the right (a column pinned at the end), the horizontal one runs
  /// from what sticks at the start (its first column) to what sticks at the end, and the thumb is the share of
  /// what scrolls that shows. Off unless set: a page with a banner that stays keeps its bar along the whole
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
