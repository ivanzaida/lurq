use std::sync::Arc;

use crate::layout::quad::ClipRect;
use super::tessellate::TessellatedSvg;

pub struct SvgCmd {
  pub x: f32,
  pub y: f32,
  pub width: f32,
  pub height: f32,
  pub svg_id: u64,
  pub mesh: Arc<TessellatedSvg>,
  pub clip: ClipRect,
}
