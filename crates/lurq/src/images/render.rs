use std::sync::Arc;

use crate::layout::quad::ClipRect;

pub struct ImageCmd {
  pub order: usize,
  pub x: f32,
  pub y: f32,
  pub width: f32,
  pub height: f32,
  pub image_id: u64,
  pub data: Arc<Vec<u8>>,
  pub image_width: u32,
  pub image_height: u32,
  pub clip: ClipRect,
}
