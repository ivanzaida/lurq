use crate::{images::ImageData, impl_into_node, node::BackgroundSize};

impl_into_node!(Video);

impl Video {
  pub fn new(data: ImageData) -> Self {
    Self::from_node(crate::node::Node::video(data))
  }

  pub fn fit(mut self, fit: BackgroundSize) -> Self {
    self.update_node(|node| node.set_video_fit(fit));
    self
  }

  pub fn stretch(self) -> Self {
    self.fit(BackgroundSize::Stretch)
  }

  pub fn cover(self) -> Self {
    self.fit(BackgroundSize::Cover)
  }

  pub fn contain(self) -> Self {
    self.fit(BackgroundSize::Contain)
  }
}
