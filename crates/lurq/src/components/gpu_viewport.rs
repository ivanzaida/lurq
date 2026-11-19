use crate::{images::WgpuExternalImageSlot, impl_into_node};

/// The size of a viewport given none, in logical pixels: a canvas's, lurq's and a browser's.
const DEFAULT_SIZE: (f32, f32) = (300.0, 150.0);

impl_into_node!(
  /// A box in the layout whose pixels the app draws itself, with its own wgpu passes on lurq's device.
  ///
  /// It shows the texture view of a [`WgpuExternalImageSlot`], which the app's renderer, a
  /// [`WgpuFrameExtension`](crate::app::wgpu_render::WgpuFrameExtension) given to the `WgpuRenderEngine`, fills
  /// before each frame's UI pass. lurq samples the view where the box is laid out, in the same frame and without a
  /// copy, under the box's radius, opacity, clips and transform.
  ///
  /// Standard node builders provide sizing, mouse/scroll handlers, keyboard focus, and drag capture; this
  /// component only supplies the zero-copy image. Given no size it is 300 by 150 logical pixels, as a
  /// [`Canvas`](crate::components::Canvas) is; given one side, the other is still the canvas's, for the app draws to
  /// whatever box it has and there is no picture's shape to keep. `.size(..)`, `.width(..)`, `.height(..)`,
  /// percentages and flex size it as any box.
  ///
  /// Drawn by the `wgpu` renderer. lurq's DirectX 12 renderer cannot sample a wgpu texture: it leaves the box
  /// empty and says so once in the log. The guide is `docs/src/content/docs/gpu-viewport.md`, and
  /// `examples/gpu_viewport.rs` a whole app.
  GpuViewport
);

impl GpuViewport {
  /// A viewport that shows `slot`'s view, stretched to its box. An empty slot draws nothing, and what is under
  /// the viewport shows through.
  pub fn new(slot: WgpuExternalImageSlot) -> Self {
    let mut node = crate::node::Node::image(slot.image_data());
    let (width, height) = DEFAULT_SIZE;
    // The slot's picture is one pixel square to the layout until a view is given, and its texture's size is
    // in physical pixels: neither is the box's size.
    node.intrinsic_size = Some(crate::layout::Size::new(width, height));
    Self::from_node(node)
  }
}
