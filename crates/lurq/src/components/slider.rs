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
    self.node = self.node.range(min, max);
    self
  }

  pub fn range_f32(mut self, min: f32, max: f32) -> Self {
    self.node = self.node.range_f32(min, max);
    self
  }

  pub fn step(mut self, step: f32) -> Self {
    self.node = self.node.slider_step(step);
    self
  }

  pub fn track_style(mut self, style: SliderPartStyle) -> Self {
    self.node = self.node.slider_track_style(style);
    self
  }

  pub fn track(mut self, f: impl FnOnce(SliderPartStyle) -> SliderPartStyle) -> Self {
    self.node = self.node.slider_track_style(f(SliderPartStyle::new()));
    self
  }

  pub fn track_hovered_style(mut self, style: SliderPartStyle) -> Self {
    self.node = self.node.slider_track_hovered_style(style);
    self
  }

  pub fn track_hovered(mut self, f: impl FnOnce(SliderPartStyle) -> SliderPartStyle) -> Self {
    self.node = self.node.slider_track_hovered_style(f(SliderPartStyle::new()));
    self
  }

  pub fn thumb_style(mut self, style: SliderPartStyle) -> Self {
    self.node = self.node.slider_thumb_style(style);
    self
  }

  pub fn thumb(mut self, f: impl FnOnce(SliderPartStyle) -> SliderPartStyle) -> Self {
    self.node = self.node.slider_thumb_style(f(SliderPartStyle::new()));
    self
  }

  pub fn thumb_hovered_style(mut self, style: SliderPartStyle) -> Self {
    self.node = self.node.slider_thumb_hovered_style(style);
    self
  }

  pub fn thumb_hovered(mut self, f: impl FnOnce(SliderPartStyle) -> SliderPartStyle) -> Self {
    self.node = self.node.slider_thumb_hovered_style(f(SliderPartStyle::new()));
    self
  }
}

impl Default for Slider {
  fn default() -> Self {
    Self::new(Signal::new(0))
  }
}
