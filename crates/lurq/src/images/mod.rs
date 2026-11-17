#[cfg(windows)]
mod card_frames;
#[cfg(feature = "image")]
mod decoded;
mod image_data;
mod render;
#[cfg(feature = "wgpu")]
mod wgpu_external;

#[cfg(windows)]
pub use card_frames::{
  CardCrop, CardFrame, CardFrames, CardFramesWay, CardLease, CardSurface, card_frames_adapter, card_frames_nv12,
  set_card_frames_adapter, set_card_frames_nv12,
};
#[cfg(all(windows, any(feature = "wgpu", feature = "dx12")))]
pub(crate) use card_frames::part_of;
#[cfg(feature = "image")]
pub use decoded::{
  DEFAULT_BUDGET_BYTES, DecodedImages, ImageDecodeError, ImageKey, ImageLoadState, ImageSizeCap, ImageSource,
  decode_image, decoded_images, wait_for_decoded_images,
};
pub use image_data::{ImageData, ImageKind, ImagePixelFormat, NativeImageBackend, NativeImageData, StreamingImage};
#[cfg(target_os = "macos")]
pub use image_data::{MacosCvPixelBuffer, MacosCvPixelBufferNv12Image};
pub(crate) use render::ImageCmd;
#[cfg(feature = "wgpu")]
pub use wgpu_external::WgpuExternalImageSlot;
#[cfg(feature = "wgpu")]
pub(crate) use wgpu_external::{
  WgpuExternalImageSnapshot, WgpuExternalImageState, WgpuExternalImageWeak, for_device, next_in_order,
};
