use std::sync::Arc;

use super::SvgData;
use crate::node::color::Color;

const SVG_IMAGE_ID_TAG: u64 = 1 << 63;

pub(crate) struct RasterizedSvg {
  pub image_id: u64,
  pub data: Arc<Vec<u8>>,
  pub width: u32,
  pub height: u32,
}

pub(crate) fn rasterize(data: &SvgData, target_width: f32, target_height: f32) -> RasterizedSvg {
  let width = target_width.ceil().max(1.0) as u32;
  let height = target_height.ceil().max(1.0) as u32;
  let mut pixmap = tiny_skia::Pixmap::new(width, height).expect("svg raster pixmap should fit");
  let sx = width as f32 / data.tree().size().width();
  let sy = height as f32 / data.tree().size().height();
  let opacity = data.opacity_override().unwrap_or(1.0);

  rasterize_group(data.tree().root(), data, &mut pixmap, sx, sy, opacity);

  let mut pixels = pixmap.take();
  unpremultiply_rgba(&mut pixels);

  RasterizedSvg {
    image_id: raster_image_id(data.id(), width, height),
    data: Arc::new(pixels),
    width,
    height,
  }
}

fn rasterize_group(
  group: &usvg::Group,
  data: &SvgData,
  pixmap: &mut tiny_skia::Pixmap,
  sx: f32,
  sy: f32,
  opacity: f32,
) {
  for child in group.children() {
    match child {
      usvg::Node::Group(group) => {
        rasterize_group(group, data, pixmap, sx, sy, opacity * group.opacity().get() as f32);
      }
      usvg::Node::Path(path) => {
        rasterize_path(path, data, pixmap, sx, sy, opacity);
      }
      _ => {}
    }
  }
}

fn rasterize_path(path: &usvg::Path, data: &SvgData, pixmap: &mut tiny_skia::Pixmap, sx: f32, sy: f32, opacity: f32) {
  let transform = path.abs_transform().post_scale(sx, sy);

  if let Some(fill) = path.fill() {
    let fill_opacity = opacity * fill.opacity().get() as f32;
    if let Some(paint) = raster_paint(data.fill_override(), fill.paint(), fill_opacity) {
      pixmap.fill_path(path.data(), &paint, fill_rule(fill.rule()), transform, None);
    }
  }

  if let Some(stroke) = path.stroke() {
    let stroke_opacity = opacity * stroke.opacity().get() as f32;
    if let Some(paint) = raster_paint(data.stroke_override(), stroke.paint(), stroke_opacity) {
      let mut tiny_stroke = stroke.to_tiny_skia();
      tiny_stroke.width *= (sx + sy) * 0.5;
      pixmap.stroke_path(path.data(), &paint, &tiny_stroke, transform, None);
    }
  }
}

fn raster_paint(override_color: Option<Color>, paint: &usvg::Paint, opacity: f32) -> Option<tiny_skia::Paint<'static>> {
  let mut tiny_paint = tiny_skia::Paint::default();
  tiny_paint.anti_alias = true;
  tiny_paint.force_hq_pipeline = true;

  if let Some(color) = override_color {
    tiny_paint.set_color_rgba8(
      color.r(),
      color.g(),
      color.b(),
      (color.a() as f32 * opacity.clamp(0.0, 1.0)).round() as u8,
    );
    return Some(tiny_paint);
  }

  match paint {
    usvg::Paint::Color(color) => {
      tiny_paint.set_color_rgba8(
        color.red,
        color.green,
        color.blue,
        (opacity.clamp(0.0, 1.0) * 255.0).round() as u8,
      );
      Some(tiny_paint)
    }
    _ => None,
  }
}

fn fill_rule(rule: usvg::FillRule) -> tiny_skia::FillRule {
  match rule {
    usvg::FillRule::NonZero => tiny_skia::FillRule::Winding,
    usvg::FillRule::EvenOdd => tiny_skia::FillRule::EvenOdd,
  }
}

fn unpremultiply_rgba(pixels: &mut [u8]) {
  for px in pixels.chunks_exact_mut(4) {
    let alpha = px[3];
    if alpha == 0 {
      px[0] = 0;
      px[1] = 0;
      px[2] = 0;
    } else if alpha < 255 {
      let alpha = u16::from(alpha);
      px[0] = ((u16::from(px[0]) * 255 + alpha / 2) / alpha).min(255) as u8;
      px[1] = ((u16::from(px[1]) * 255 + alpha / 2) / alpha).min(255) as u8;
      px[2] = ((u16::from(px[2]) * 255 + alpha / 2) / alpha).min(255) as u8;
    }
  }
}

fn raster_image_id(svg_id: u64, width: u32, height: u32) -> u64 {
  SVG_IMAGE_ID_TAG ^ svg_id.rotate_left(31) ^ ((width as u64) << 32) ^ height as u64
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn rasterizes_stroked_circle_with_antialiased_edges() {
    let svg = SvgData::from_str(
      r##"
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24">
        <circle cx="12" cy="12" r="9" fill="none" stroke="#60a5fa" stroke-width="4" stroke-linecap="round"/>
      </svg>
      "##,
    );

    let raster = rasterize(&svg, 24.0, 24.0);

    assert_eq!(raster.width, 24);
    assert_eq!(raster.height, 24);
    assert!(raster.data.chunks_exact(4).any(|px| px[3] > 0 && px[3] < 255));
  }
}
