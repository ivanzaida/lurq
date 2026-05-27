use crate::{images::ImageData, impl_into_node};

impl_into_node!(Image);

impl Image {
  pub fn new(data: ImageData) -> Self {
    Self::from_node(crate::node::Node::image(data))
  }
}
