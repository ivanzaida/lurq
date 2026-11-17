use crate::{images::WgpuExternalImageSlot, impl_into_node};

impl_into_node!(
  /// A box in the layout whose pixels the app draws itself, with its own wgpu passes on lurq's device.
  ///
  /// It shows the texture view of a [`WgpuExternalImageSlot`], which the app's renderer, a
  /// [`WgpuFrameExtension`](crate::app::wgpu_render::WgpuFrameExtension) given to the `WgpuRenderEngine`, fills
  /// before each frame's UI pass. lurq samples the view where the box is laid out, in the same frame and without a
  /// copy, under the box's radius, opacity, clips and transform.
  ///
  /// Standard node builders provide sizing, mouse/scroll handlers, keyboard focus, and drag capture; this
  /// component only supplies the zero-copy image. Give it a size: on its own it is one logical pixel square.
  ///
  /// Drawn by the `wgpu` renderer only: lurq's DirectX 12 renderer cannot sample a wgpu texture, and fails a frame
  /// that has one. The guide is `docs/src/content/docs/gpu-viewport.md`, and `examples/gpu_viewport.rs` a whole
  /// app.
  GpuViewport
);

impl GpuViewport {
  /// A viewport that shows `slot`'s view, stretched to its box. An empty slot draws nothing, and what is under
  /// the viewport shows through.
  pub fn new(slot: WgpuExternalImageSlot) -> Self {
    Self::from_node(crate::node::Node::image(slot.image_data()))
  }
}
