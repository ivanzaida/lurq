use crate::{core::Signal, impl_into_node, node::SliderPartStyle};

impl_into_node!(Slider);

impl Slider {
  pub fn new(value: Signal<i32>) -> Self {
    Self::from_node(crate::node::Node::slider(value))
  }

  pub fn new_f32(value: Signal<f32>) -> Self {
    Self::from_node(crate::node::Node::slider_f32(value))
  }

  pub fn range(mut self, min: i32, max: i32) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::range(node, min, max));
    self
  }

  pub fn range_f32(mut self, min: f32, max: f32) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::range_f32(node, min, max));
    self
  }

  pub fn step(mut self, step: f32) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::slider_step(node, step));
    self
  }

  pub fn track_style(mut self, style: SliderPartStyle) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::slider_track_style(node, style));
    self
  }

  pub fn track(mut self, f: impl FnOnce(SliderPartStyle) -> SliderPartStyle) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::slider_track_style(node, f(SliderPartStyle::new())));
    self
  }

  pub fn track_hovered_style(mut self, style: SliderPartStyle) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::slider_track_hovered_style(node, style));
    self
  }

  pub fn track_hovered(mut self, f: impl FnOnce(SliderPartStyle) -> SliderPartStyle) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::slider_track_hovered_style(node, f(SliderPartStyle::new())));
    self
  }

  pub fn fill_style(mut self, style: SliderPartStyle) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::slider_fill_style(node, style));
    self
  }

  pub fn fill(mut self, f: impl FnOnce(SliderPartStyle) -> SliderPartStyle) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::slider_fill_style(node, f(SliderPartStyle::new())));
    self
  }

  pub fn fill_hovered_style(mut self, style: SliderPartStyle) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::slider_fill_hovered_style(node, style));
    self
  }

  pub fn fill_hovered(mut self, f: impl FnOnce(SliderPartStyle) -> SliderPartStyle) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::slider_fill_hovered_style(node, f(SliderPartStyle::new())));
    self
  }

  pub fn thumb_style(mut self, style: SliderPartStyle) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::slider_thumb_style(node, style));
    self
  }

  pub fn thumb(mut self, f: impl FnOnce(SliderPartStyle) -> SliderPartStyle) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::slider_thumb_style(node, f(SliderPartStyle::new())));
    self
  }

  pub fn thumb_hovered_style(mut self, style: SliderPartStyle) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::slider_thumb_hovered_style(node, style));
    self
  }

  pub fn thumb_hovered(mut self, f: impl FnOnce(SliderPartStyle) -> SliderPartStyle) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::slider_thumb_hovered_style(node, f(SliderPartStyle::new())));
    self
  }
}

impl Default for Slider {
  fn default() -> Self {
    Self::new(Signal::new(0))
  }
}
