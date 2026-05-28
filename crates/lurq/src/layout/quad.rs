use crate::{
  layout::text_style::TextStyle,
  node::{
    border::{Border, BorderRadius},
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
  pub border: Option<Border>,
  pub clip: ClipRect,
}

pub enum QuadContent {
  Rect {
    color: Color,
  },
  Text {
    text: String,
    style: TextStyle,
  },
  #[cfg(feature = "image")]
  Image {
    data: crate::images::ImageData,
  },
  #[cfg(feature = "svg")]
  Svg {
    data: crate::svg::SvgData,
  },
  None,
}
