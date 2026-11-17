use crate::{images::ImageData, impl_into_node, node::BackgroundSize};

impl_into_node!(
  /// The engine's video surface: the frames of a player, fitted to its box, with no controls. It is what
  /// [`ui::VideoPlayer`](crate::ui::VideoPlayer) draws its picture with; an app that plays a video normally wants
  /// that.
  Video
);

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
