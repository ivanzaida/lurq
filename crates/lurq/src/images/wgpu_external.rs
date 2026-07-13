use std::sync::Arc;

use parking_lot::RwLock;

use super::{ImageData, ImagePixelFormat, NativeImageBackend, NativeImageData};

#[derive(Clone)]
pub struct WgpuExternalImageSlot {
  state: WgpuExternalImageState,
  native: NativeImageData,
}

#[derive(Clone)]
pub(crate) struct WgpuExternalImageState {
  inner: Arc<RwLock<WgpuExternalImageSlotState>>,
}

struct WgpuExternalImageSlotState {
  view: Option<wgpu::TextureView>,
  width: u32,
  height: u32,
  version: u64,
}

#[derive(Clone)]
pub(crate) struct WgpuExternalImageSnapshot {
  pub view: wgpu::TextureView,
  pub width: u32,
  pub height: u32,
  pub version: u64,
}

impl Default for WgpuExternalImageSlot {
  fn default() -> Self {
    Self::new()
  }
}

impl WgpuExternalImageSlot {
  pub fn new() -> Self {
    let state = WgpuExternalImageState {
      inner: Arc::new(RwLock::new(WgpuExternalImageSlotState {
        view: None,
        width: 1,
        height: 1,
        version: 0,
      })),
    };
    let native = NativeImageData::new(
      1,
      1,
      ImagePixelFormat::Rgba8,
      NativeImageBackend::WgpuExternalRgba,
      state.clone(),
    );
    Self { state, native }
  }

  /// Stable image identifier used to find this slot in [`WgpuFrameInfo`](
  /// crate::app::wgpu_render::WgpuFrameInfo).
  pub fn image_id(&self) -> u64 {
    self.native.id()
  }

  pub fn image_data(&self) -> ImageData {
    self.native.image_data()
  }

  /// Publishes a shader-readable RGBA texture view without copying its pixels.
  ///
  /// The texture must have been created from Lurq's shared WGPU device and stay
  /// valid for as long as the slot contains the view.
  pub fn set_texture_view(&self, view: wgpu::TextureView, width: u32, height: u32) -> u64 {
    let mut state = self.state.inner.write();
    state.version = state.version.wrapping_add(1);
    state.view = Some(view);
    state.width = width.max(1);
    state.height = height.max(1);
    state.version
  }

  pub fn clear(&self) -> u64 {
    let mut state = self.state.inner.write();
    state.version = state.version.wrapping_add(1);
    state.view = None;
    state.version
  }

  pub fn version(&self) -> u64 {
    self.state.version()
  }
}

impl WgpuExternalImageState {
  pub(crate) fn version(&self) -> u64 {
    self.inner.read().version
  }

  pub(crate) fn snapshot(&self) -> Option<WgpuExternalImageSnapshot> {
    let state = self.inner.read();
    Some(WgpuExternalImageSnapshot {
      view: state.view.clone()?,
      width: state.width,
      height: state.height,
      version: state.version,
    })
  }
}

#[cfg(test)]
mod tests {
  use super::WgpuExternalImageSlot;

  #[test]
  fn slot_has_stable_image_identity_and_version() {
    let slot = WgpuExternalImageSlot::new();
    let clone = slot.clone();

    assert_eq!(slot.image_id(), clone.image_id());
    assert_eq!(slot.image_data().id(), slot.image_id());
    assert_eq!(slot.version(), 0);
    assert_eq!(clone.clear(), 1);
    assert_eq!(slot.version(), 1);
  }
}
