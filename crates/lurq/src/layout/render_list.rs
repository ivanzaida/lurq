use crate::{layout::quad::ClipRect, node::color::Color};

pub struct RenderList {
  pub rects: Vec<RectCmd>,
  pub glyphs: Vec<GlyphCmd>,
  pub atlas: GlyphAtlas,
}

pub struct RectCmd {
  pub x: f32,
  pub y: f32,
  pub width: f32,
  pub height: f32,
  pub color: Color,
  pub radii: [f32; 4],
  pub stroke: [f32; 4],
  pub stroke_color: Color,
  pub clip: ClipRect,
}

pub struct GlyphCmd {
  pub x: f32,
  pub y: f32,
  pub width: f32,
  pub height: f32,
  pub color: [f32; 4],
  pub uv_min: [f32; 2],
  pub uv_max: [f32; 2],
  pub clip: ClipRect,
}

pub struct GlyphAtlas {
  pub data: Vec<u8>,
  pub width: u32,
  pub height: u32,
}
