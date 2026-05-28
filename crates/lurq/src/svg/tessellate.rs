use lyon::{
  lyon_tessellation::{BuffersBuilder, FillOptions, FillTessellator, StrokeOptions, StrokeTessellator, VertexBuffers},
  path::Path,
};

use super::SvgData;
use crate::node::color::Color;

#[derive(Clone)]
pub struct TessellatedSvg {
  pub vertices: Vec<SvgVertex>,
  pub indices: Vec<u32>,
}

#[derive(Clone, Copy)]
pub struct SvgVertex {
  pub position: [f32; 2],
  pub color: [f32; 4],
}

pub fn tessellate(data: &SvgData, target_width: f32, target_height: f32) -> TessellatedSvg {
  let tree = data.tree();
  let vb_w = tree.size().width();
  let vb_h = tree.size().height();
  let sx = target_width / vb_w;
  let sy = target_height / vb_h;

  let opacity = data.opacity_override().unwrap_or(1.0);

  let mut all_vertices = Vec::new();
  let mut all_indices = Vec::new();

  tessellate_group(tree.root(), data, sx, sy, opacity, &mut all_vertices, &mut all_indices);

  TessellatedSvg {
    vertices: all_vertices,
    indices: all_indices,
  }
}

fn tessellate_group(
  group: &usvg::Group,
  data: &SvgData,
  sx: f32,
  sy: f32,
  opacity: f32,
  vertices: &mut Vec<SvgVertex>,
  indices: &mut Vec<u32>,
) {
  for child in group.children() {
    match child {
      usvg::Node::Group(g) => {
        let group_opacity = opacity * g.opacity().get() as f32;
        tessellate_group(g, data, sx, sy, group_opacity, vertices, indices);
      }
      usvg::Node::Path(path) => {
        tessellate_path(path, data, sx, sy, opacity, vertices, indices);
      }
      _ => {}
    }
  }
}

fn tessellate_path(
  path: &usvg::Path,
  data: &SvgData,
  sx: f32,
  sy: f32,
  opacity: f32,
  vertices: &mut Vec<SvgVertex>,
  indices: &mut Vec<u32>,
) {
  let lyon_path = usvg_path_to_lyon(path);

  if let Some(ref fill) = path.fill() {
    let color = if let Some(override_color) = data.fill_override() {
      color_to_linear(override_color, opacity)
    } else {
      paint_to_color(&fill.paint(), opacity * fill.opacity().get() as f32)
    };

    let mut geometry: VertexBuffers<SvgVertex, u32> = VertexBuffers::new();
    let mut tessellator = FillTessellator::new();
    let result = tessellator.tessellate_path(
      &lyon_path,
      &FillOptions::default().with_tolerance(0.5),
      &mut BuffersBuilder::new(&mut geometry, |vertex: lyon::tessellation::FillVertex| SvgVertex {
        position: [vertex.position().x * sx, vertex.position().y * sy],
        color,
      }),
    );

    if result.is_ok() {
      let base = vertices.len() as u32;
      vertices.extend_from_slice(&geometry.vertices);
      indices.extend(geometry.indices.iter().map(|i| i + base));
    }
  }

  if let Some(ref stroke) = path.stroke() {
    let color = if let Some(override_color) = data.stroke_override() {
      color_to_linear(override_color, opacity)
    } else {
      paint_to_color(&stroke.paint(), opacity * stroke.opacity().get() as f32)
    };

    let line_width = stroke.width().get() as f32 * ((sx + sy) * 0.5);

    let mut geometry: VertexBuffers<SvgVertex, u32> = VertexBuffers::new();
    let mut tessellator = StrokeTessellator::new();
    let result = tessellator.tessellate_path(
      &lyon_path,
      &StrokeOptions::default().with_line_width(line_width).with_tolerance(0.5),
      &mut BuffersBuilder::new(&mut geometry, |vertex: lyon::tessellation::StrokeVertex| SvgVertex {
        position: [vertex.position().x * sx, vertex.position().y * sy],
        color,
      }),
    );

    if result.is_ok() {
      let base = vertices.len() as u32;
      vertices.extend_from_slice(&geometry.vertices);
      indices.extend(geometry.indices.iter().map(|i| i + base));
    }
  }
}

fn usvg_path_to_lyon(path: &usvg::Path) -> Path {
  let mut builder = Path::builder();
  for seg in path.data().segments() {
    match seg {
      usvg::tiny_skia_path::PathSegment::MoveTo(pt) => {
        builder.begin(lyon::geom::point(pt.x, pt.y));
      }
      usvg::tiny_skia_path::PathSegment::LineTo(pt) => {
        builder.line_to(lyon::geom::point(pt.x, pt.y));
      }
      usvg::tiny_skia_path::PathSegment::QuadTo(p1, pt) => {
        builder.quadratic_bezier_to(lyon::geom::point(p1.x, p1.y), lyon::geom::point(pt.x, pt.y));
      }
      usvg::tiny_skia_path::PathSegment::CubicTo(p1, p2, pt) => {
        builder.cubic_bezier_to(
          lyon::geom::point(p1.x, p1.y),
          lyon::geom::point(p2.x, p2.y),
          lyon::geom::point(pt.x, pt.y),
        );
      }
      usvg::tiny_skia_path::PathSegment::Close => {
        builder.close();
      }
    }
  }
  builder.build()
}

fn srgb_to_linear(c: f32) -> f32 {
  if c <= 0.04045 {
    c / 12.92
  } else {
    ((c + 0.055) / 1.055).powf(2.4)
  }
}

fn color_to_linear(color: Color, opacity: f32) -> [f32; 4] {
  [
    srgb_to_linear(color.r() as f32 / 255.0),
    srgb_to_linear(color.g() as f32 / 255.0),
    srgb_to_linear(color.b() as f32 / 255.0),
    (color.a() as f32 / 255.0) * opacity,
  ]
}

fn paint_to_color(paint: &usvg::Paint, opacity: f32) -> [f32; 4] {
  match paint {
    usvg::Paint::Color(c) => [
      srgb_to_linear(c.red as f32 / 255.0),
      srgb_to_linear(c.green as f32 / 255.0),
      srgb_to_linear(c.blue as f32 / 255.0),
      opacity,
    ],
    _ => [0.0, 0.0, 0.0, opacity],
  }
}
