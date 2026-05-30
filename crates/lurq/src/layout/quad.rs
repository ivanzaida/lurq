use crate::{
  layout::text_style::TextStyle,
  node::{
    border::{BorderRadius, Borders},
    color::Color,
    transform::Transform2D,
  },
};

#[derive(Clone, Copy, Default)]
pub struct ClipRect {
  pub x: f32,
  pub y: f32,
  pub width: f32,
  pub height: f32,
  pub active: bool,
}

pub struct Quad {
  pub x: f32,
  pub y: f32,
  pub width: f32,
  pub height: f32,
  pub opacity: f32,
  pub transform: Transform2D,
  pub content: QuadContent,
  pub border_radius: Option<BorderRadius>,
  pub border: Option<Borders>,
  pub clip: ClipRect,
}

pub enum QuadContent {
  Rect {
    color: Color,
  },
  Text {
    text: String,
    style: TextStyle,
    wrap: bool,
  },
  #[cfg(feature = "image")]
  Image {
    data: crate::images::ImageData,
    uv_min: [f32; 2],
    uv_max: [f32; 2],
  },
  #[cfg(feature = "svg")]
  Svg {
    data: crate::svg::SvgData,
  },
  None,
}
