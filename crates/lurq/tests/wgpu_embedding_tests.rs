#![cfg(all(feature = "image", feature = "wgpu"))]

use lurq::{
  app::wgpu_render::{SharedWgpuContext, WgpuFeatures, WgpuFrameExtension, WgpuFrameInfo, WgpuRenderEngine},
  components::GpuViewport,
  images::WgpuExternalImageSlot,
};

struct EmbeddedRenderer {
  output: WgpuExternalImageSlot,
}

impl WgpuFrameExtension for EmbeddedRenderer {
  fn initialize(&mut self, gpu: &SharedWgpuContext) {
    let _ = (&gpu.instance, &gpu.adapter, &gpu.device, &gpu.queue);
  }

  fn prepare(&mut self, _gpu: &SharedWgpuContext, frame: WgpuFrameInfo<'_>) {
    let _ = frame.viewport(self.output.image_id());
  }

  fn wants_redraw(&self) -> bool {
    true
  }
}

#[test]
fn public_embedding_api_composes_a_renderer_and_viewport() {
  let output = WgpuExternalImageSlot::new();
  let _viewport = GpuViewport::new(output.clone()).size(640.0, 480.0).focusable(true);
  let _renderer = WgpuRenderEngine::new()
    .with_optional_device_features(WgpuFeatures::TEXTURE_COMPRESSION_BC)
    .with_frame_extension(EmbeddedRenderer { output });
}
