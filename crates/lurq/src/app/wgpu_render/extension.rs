use crate::layout::render_list::RenderList;

/// Cloneable handles to the WGPU objects owned by Lurq's renderer.
///
/// Frame extensions must use these handles instead of creating a second
/// instance, adapter, device, or queue. This keeps embedded renderers on the
/// same GPU timeline as Lurq and allows their texture views to be sampled
/// directly by the UI renderer.
#[derive(Clone)]
pub struct SharedWgpuContext {
  pub instance: wgpu::Instance,
  pub adapter: wgpu::Adapter,
  pub device: wgpu::Device,
  pub queue: wgpu::Queue,
  pub surface_format: wgpu::TextureFormat,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WgpuViewportRect {
  pub x: f32,
  pub y: f32,
  pub width: f32,
  pub height: f32,
}

/// Read-only information for the Lurq frame an extension is preparing.
#[derive(Clone, Copy)]
pub struct WgpuFrameInfo<'a> {
  #[cfg_attr(not(feature = "image"), allow(dead_code))]
  render_list: &'a RenderList,
  pub surface_width: u32,
  pub surface_height: u32,
}

impl<'a> WgpuFrameInfo<'a> {
  pub(crate) fn new(render_list: &'a RenderList, surface_width: u32, surface_height: u32) -> Self {
    Self {
      render_list,
      surface_width,
      surface_height,
    }
  }

  /// Returns the physical-pixel rectangle occupied by an external image.
  ///
  /// Pass [`WgpuExternalImageSlot::image_id`](crate::images::WgpuExternalImageSlot::image_id)
  /// to locate the corresponding `GpuViewport` in the current render list.
  #[cfg(feature = "image")]
  pub fn viewport(&self, image_id: u64) -> Option<WgpuViewportRect> {
    self
      .render_list
      .images
      .iter()
      .find(|image| image.image_id == image_id)
      .map(|image| WgpuViewportRect {
        x: image.x,
        y: image.y,
        width: image.width,
        height: image.height,
      })
  }
}

/// An in-process renderer that participates in Lurq's WGPU frame lifecycle.
///
/// `initialize` runs once after Lurq creates its device. `prepare` runs before
/// Lurq encodes the UI pass, so an embedded renderer can update external
/// textures that the UI samples in the same frame.
pub trait WgpuFrameExtension {
  fn initialize(&mut self, _gpu: &SharedWgpuContext) {}

  fn prepare(&mut self, _gpu: &SharedWgpuContext, _frame: WgpuFrameInfo<'_>) {}

  /// Keep the host event loop producing frames while the extension is active.
  fn wants_redraw(&self) -> bool {
    false
  }

  fn shutdown(&mut self) {}
}

pub(crate) struct WgpuFrameExtensionEntry {
  pub extension: Box<dyn WgpuFrameExtension>,
  pub initialized: bool,
}

impl WgpuFrameExtensionEntry {
  pub fn new(extension: Box<dyn WgpuFrameExtension>) -> Self {
    Self {
      extension,
      initialized: false,
    }
  }
}
