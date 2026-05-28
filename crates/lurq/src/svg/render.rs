use std::sync::Arc;

use super::tessellate::TessellatedSvg;
use crate::layout::quad::ClipRect;

pub struct SvgCmd {
  pub order: usize,
  pub x: f32,
  pub y: f32,
  pub width: f32,
  pub height: f32,
  pub svg_id: u64,
  pub mesh: Arc<TessellatedSvg>,
  pub clip: ClipRect,
}
