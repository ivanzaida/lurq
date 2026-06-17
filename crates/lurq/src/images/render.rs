use std::sync::Arc;

use super::{ImagePixelFormat, NativeImageData};
use crate::layout::quad::ClipRect;

#[derive(Clone)]
pub struct ImageCmd {
  pub order: usize,
  pub x: f32,
  pub y: f32,
  pub width: f32,
  pub height: f32,
  pub image_id: u64,
  pub frame_index: usize,
  pub version: u64,
  pub data: Arc<Vec<u8>>,
  pub animation_frames: Option<Arc<Vec<Arc<Vec<u8>>>>>,
  pub native: Option<NativeImageData>,
  pub image_width: u32,
  pub image_height: u32,
  pub image_format: ImagePixelFormat,
  pub uv_min: [f32; 2],
  pub uv_max: [f32; 2],
  pub radii: [f32; 4],
  pub transform: [f32; 4],
  pub transform_origin: [f32; 2],
  pub clip: ClipRect,
}
