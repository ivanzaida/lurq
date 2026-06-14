use crate::{
  layout::{render_list::RenderGradient, text_style::TextStyle},
  node::{
    TextTransformMode,
    border::{BorderRadius, ResolvedBorders},
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
  pub border_radius: Option<BorderRadius>,
}

pub struct Quad {
  pub x: f32,
  pub y: f32,
  pub width: f32,
  pub height: f32,
  pub opacity: f32,
  pub transform: Transform2D,
  pub transform_origin: Option<[f32; 2]>,
  pub content: QuadContent,
  pub border_radius: Option<BorderRadius>,
  pub border: Option<ResolvedBorders>,
  pub clip: ClipRect,
}

pub enum QuadContent {
  Rect {
    color: Color,
    gradient: Option<RenderGradient>,
  },
  Text {
    text: String,
    style: TextStyle,
    wrap: bool,
    transform_mode: TextTransformMode,
  },
  RichText {
    spans: Vec<RichTextSpan>,
    wrap: bool,
    transform_mode: TextTransformMode,
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

#[derive(Clone, PartialEq)]
pub struct RichTextSpan {
  pub text: String,
  pub style: TextStyle,
}
