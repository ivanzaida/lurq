use crate::{images::WgpuExternalImageSlot, impl_into_node};

impl_into_node!(GpuViewport);

/// A Lurq element backed by a texture rendered through the shared WGPU device.
///
/// Standard node builders provide sizing, mouse/scroll handlers, keyboard
/// focus, and drag capture; this component only supplies the zero-copy image.
impl GpuViewport {
  pub fn new(slot: WgpuExternalImageSlot) -> Self {
    Self::from_node(crate::node::Node::image(slot.image_data()))
  }
}
