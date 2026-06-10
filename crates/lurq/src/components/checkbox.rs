use crate::{core::Signal, impl_into_node, node::CheckboxStyle};

impl_into_node!(Checkbox);

impl Checkbox {
  pub fn new(value: Signal<bool>) -> Self {
    Self::from_node(crate::node::Node::checkbox(value))
  }

  pub fn box_style(mut self, style: CheckboxStyle) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::checkbox_box_style(node, style));
    self
  }

  pub fn box_part(mut self, f: impl FnOnce(CheckboxStyle) -> CheckboxStyle) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::checkbox_box_style(node, f(CheckboxStyle::new())));
    self
  }

  pub fn checked_box_style(mut self, style: CheckboxStyle) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::checkbox_checked_box_style(node, style));
    self
  }

  pub fn checked_box(mut self, f: impl FnOnce(CheckboxStyle) -> CheckboxStyle) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::checkbox_checked_box_style(node, f(CheckboxStyle::new())));
    self
  }

  pub fn box_hovered_style(mut self, style: CheckboxStyle) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::checkbox_box_hovered_style(node, style));
    self
  }

  pub fn box_hovered(mut self, f: impl FnOnce(CheckboxStyle) -> CheckboxStyle) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::checkbox_box_hovered_style(node, f(CheckboxStyle::new())));
    self
  }

  pub fn checked_box_hovered_style(mut self, style: CheckboxStyle) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::checkbox_checked_box_hovered_style(node, style));
    self
  }

  pub fn checked_box_hovered(mut self, f: impl FnOnce(CheckboxStyle) -> CheckboxStyle) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::checkbox_checked_box_hovered_style(node, f(CheckboxStyle::new())));
    self
  }
}

impl Default for Checkbox {
  fn default() -> Self {
    Self::new(Signal::new(false))
  }
}
