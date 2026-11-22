//! How light what lies behind a glass is, told from a frame's commands.
//!
//! A glass that follows its backdrop is a light glass with dark words on it
//! over something light, and a dark glass with light words over something
//! dark (see [`crate::layout::glass`]). Which one is decided on the CPU,
//! before the frame is drawn, from the commands painted before the glass:
//! the colour under a few points of the glass is found by painting those
//! points alone, in order, as the render engine paints every pixel. So the
//! answer is there in the same frame, the same in every render engine, and
//! no pixel is read back from the GPU.
//!
//! What is painted exactly: the window's clear colour, filled boxes (a
//! gradient as the mean of its stops) and pictures whose pixels are in
//! memory. What is left out: words, which cover little; strokes and
//! shadows; the fading of a group that is painted as a layer; and other
//! glasses under this one. A point under a picture whose pixels are the
//! system's (a video) is not known, and a glass with no known point keeps
//! the tone it had.

use crate::layout::{
  backdrop::BackdropCmd,
  glass::GlassCmd,
  opacity_layer::RoundedBox,
  quad::ClipRect,
  render_list::{RectCmd, RenderList},
};

/// The points looked at across the glass's bounds, and down them.
const POINTS: [usize; 2] = [7, 3];

/// How light what is painted behind `glass` is, 0 (black) to 1 (white), as
/// the eye takes it: the mean over points inside the glass of the lightness
/// (CIE L*) of the colour under each. `None` when no point is known.
pub fn backdrop_lightness(list: &RenderList, backdrop: &BackdropCmd, glass: &GlassCmd) -> Option<f32> {
  lightness_behind(
    list.clear_color,
    &list.rects,
    #[cfg(feature = "raster")]
    &list.images,
    backdrop,
    glass,
  )
}

/// [`backdrop_lightness`] from a frame's commands as they are being put
/// together: everything painted before the glass is in them by the time the
/// glass's own command is made.
pub(crate) fn lightness_behind(
  clear_color: crate::node::color::Color,
  all_rects: &[RectCmd],
  #[cfg(feature = "raster")] all_images: &[crate::images::ImageCmd],
  backdrop: &BackdropCmd,
  glass: &GlassCmd,
) -> Option<f32> {
  let [x0, y0, x1, y1] = glass.extent()?;
  let mut points = Vec::with_capacity(POINTS[0] * POINTS[1]);
  for row in 0..POINTS[1] {
    for column in 0..POINTS[0] {
      let x = x0 + (x1 - x0) * (column as f32 + 0.5) / POINTS[0] as f32;
      let y = y0 + (y1 - y0) * (row as f32 + 0.5) / POINTS[1] as f32;
      if glass.distance(x, y) < 0.0 && inside_clip(&backdrop.clip, x, y) {
        points.push(Point {
          x,
          y,
          color: color_of(clear_color),
          known: true,
        });
      }
    }
  }
  if points.is_empty() {
    return None;
  }
  let bounds = [x0, y0, x1, y1];
  let rects = all_rects.iter().filter(|rect| rect.order < backdrop.order);
  #[cfg(feature = "raster")]
  {
    // Boxes and pictures, in the order they are painted.
    let mut images = all_images
      .iter()
      .filter(|image| image.order < backdrop.order)
      .peekable();
    for rect in rects {
      while let Some(image) = images.next_if(|image| image.order < rect.order) {
        paint_image(image, bounds, &mut points);
      }
      paint_rect(rect, bounds, &mut points);
    }
    for image in images {
      paint_image(image, bounds, &mut points);
    }
  }
  #[cfg(not(feature = "raster"))]
  for rect in rects {
    paint_rect(rect, bounds, &mut points);
  }
  let known = points.iter().filter(|point| point.known);
  let count = known.clone().count();
  (count > 0).then(|| known.map(|point| lightness(point.color)).sum::<f32>() / count as f32)
}

/// Whether a theme whose page is `background` is a dark one: what a glass that follows the app's appearance
/// goes by, and where the line lies for one that follows its backdrop. It is the page's own colour that says
/// so, since a theme may be dark without the system's appearance being.
pub fn dark_page(background: crate::node::color::Color) -> bool {
  lightness(color_of(background)) < 0.5
}

/// A point of the glass and the colour painted under it so far: straight
/// sRGB, 0 to 1, opaque.
struct Point {
  x: f32,
  y: f32,
  color: [f32; 3],
  /// False once something whose pixels are not known was painted here.
  known: bool,
}

fn color_of(color: crate::node::color::Color) -> [f32; 3] {
  [color.r(), color.g(), color.b()].map(|channel| f32::from(channel) / 255.0)
}

/// How light a colour is to the eye, 0 to 1: CIE lightness, by which the
/// grey halfway up a screen's steps is a little over a half, and a pure red
/// as light as it.
fn lightness([r, g, b]: [f32; 3]) -> f32 {
  let y = 0.2126 * decode(r) + 0.7152 * decode(g) + 0.0722 * decode(b);
  if y <= 216.0 / 24389.0 {
    y * 24389.0 / 2700.0
  } else {
    1.16 * y.cbrt() - 0.16
  }
}

/// A channel in sRGB's steps as linear light.
fn decode(encoded: f32) -> f32 {
  if encoded <= 0.040_45 {
    encoded / 12.92
  } else {
    ((encoded + 0.055) / 1.055).powf(2.4)
  }
}

fn inside_clip(clip: &ClipRect, x: f32, y: f32) -> bool {
  !clip.active || (x >= clip.x && x < clip.x + clip.width && y >= clip.y && y < clip.y + clip.height)
}

fn touches(x: f32, y: f32, width: f32, height: f32, transform: [f32; 4], bounds: [f32; 4]) -> bool {
  // A turned or scaled box is looked at whatever its bounds.
  transform != [1.0, 0.0, 0.0, 1.0]
    || (x < bounds[2] && x + width > bounds[0] && y < bounds[3] && y + height > bounds[1])
}

/// Lays `color` over the point by `alpha`.
fn over(point: &mut Point, color: [f32; 3], alpha: f32) {
  let alpha = alpha.clamp(0.0, 1.0);
  for (under, over) in point.color.iter_mut().zip(color) {
    *under += (over - *under) * alpha;
  }
  if alpha >= 0.999 {
    // What was not known is covered.
    point.known = true;
  }
}

fn paint_rect(rect: &RectCmd, bounds: [f32; 4], points: &mut [Point]) {
  if rect.shadow.is_some() || !touches(rect.x, rect.y, rect.width, rect.height, rect.transform, bounds) {
    return;
  }
  let (color, alpha) = match &rect.gradient {
    Some(gradient) if !gradient.stops.is_empty() => {
      let mut mean = [0.0; 4];
      for stop in &gradient.stops {
        for (sum, channel) in mean.iter_mut().zip(stop.color) {
          *sum += channel / gradient.stops.len() as f32;
        }
      }
      // The stops are in linear light.
      ([mean[0], mean[1], mean[2]].map(encode), mean[3])
    }
    _ => (color_of(rect.color), f32::from(rect.color.a()) / 255.0),
  };
  if alpha <= 0.0 {
    return;
  }
  let Some(shape) = RoundedBox::transformed(
    rect.x,
    rect.y,
    rect.width,
    rect.height,
    rect.radii,
    rect.transform,
    rect.transform_origin,
  ) else {
    return;
  };
  for point in points {
    if inside_clip(&rect.clip, point.x, point.y) {
      let coverage = shape.coverage(point.x, point.y);
      if coverage > 0.0 {
        over(point, color, alpha * coverage);
      }
    }
  }
}

/// A channel of linear light in sRGB's steps.
fn encode(linear: f32) -> f32 {
  let linear = linear.clamp(0.0, 1.0);
  if linear <= 0.003_130_8 {
    linear * 12.92
  } else {
    1.055 * linear.powf(1.0 / 2.4) - 0.055
  }
}

#[cfg(feature = "raster")]
fn paint_image(image: &crate::images::ImageCmd, bounds: [f32; 4], points: &mut [Point]) {
  if image.opacity <= 0.0 || !touches(image.x, image.y, image.width, image.height, image.transform, bounds) {
    return;
  }
  let Some(shape) = RoundedBox::transformed(
    image.x,
    image.y,
    image.width,
    image.height,
    image.radii,
    image.transform,
    image.transform_origin,
  ) else {
    return;
  };
  let (width, height) = (image.image_width as usize, image.image_height as usize);
  let pixels = (image.image_format == crate::images::ImagePixelFormat::Rgba8
    && image.native.is_none()
    && width > 0
    && height > 0
    && image.data.len() >= width * height * 4)
    .then_some(&image.data);
  for point in points {
    if !inside_clip(&image.clip, point.x, point.y) {
      continue;
    }
    let coverage = shape.coverage(point.x, point.y);
    if coverage <= 0.0 {
      continue;
    }
    let Some(pixels) = pixels else {
      point.known = false;
      continue;
    };
    // Where the point is in the box along the box's own axes, then in the picture.
    let (dx, dy) = (point.x - shape.center[0], point.y - shape.center[1]);
    let [a, b, c, d] = shape.inverse;
    let u = ((a * dx + c * dy) / shape.half[0].max(1.0e-6) * 0.5 + 0.5).clamp(0.0, 1.0);
    let v = ((b * dx + d * dy) / shape.half[1].max(1.0e-6) * 0.5 + 0.5).clamp(0.0, 1.0);
    let u = image.uv_min[0] + (image.uv_max[0] - image.uv_min[0]) * u;
    let v = image.uv_min[1] + (image.uv_max[1] - image.uv_min[1]) * v;
    let column = ((u * width as f32) as usize).min(width - 1);
    let row = ((v * height as f32) as usize).min(height - 1);
    let at = (row * width + column) * 4;
    let color = [pixels[at], pixels[at + 1], pixels[at + 2]].map(|channel| f32::from(channel) / 255.0);
    over(
      point,
      color,
      f32::from(pixels[at + 3]) / 255.0 * image.opacity * coverage,
    );
  }
}

#[cfg(test)]
mod tests {
  use std::sync::Arc;

  use super::*;
  use crate::{
    layout::{
      glass::{DEFAULT_GLASS_LIGHT, Glass},
      render_list::{GlyphAtlas, StrokeStyle},
    },
    node::color::Color,
  };

  fn rect(order: usize, [x, y, width, height]: [f32; 4], color: Color) -> RectCmd {
    RectCmd {
      order,
      x,
      y,
      width,
      height,
      color,
      radii: [0.0; 4],
      stroke: [0.0; 4],
      stroke_color: Color::new(0, 0, 0, 0),
      stroke_style: StrokeStyle::default(),
      transform: [1.0, 0.0, 0.0, 1.0],
      transform_origin: [width * 0.5, height * 0.5],
      clip: ClipRect::default(),
      gradient: None,
      shadow: None,
    }
  }

  /// A glass of 100 by 40 at (50, 50), painted at order 10 over `rects` on a black window.
  fn scene(rects: Vec<RectCmd>) -> (RenderList, BackdropCmd, GlassCmd) {
    let shape = RoundedBox::rect(50.0, 50.0, 100.0, 40.0, [20.0; 4]);
    let glass = GlassCmd::new(&Glass::regular(), &[shape], 0.0, 1.0, DEFAULT_GLASS_LIGHT, [50.0, 50.0]);
    let backdrop = BackdropCmd {
      order: 10,
      x: 50.0,
      y: 50.0,
      width: 100.0,
      height: 40.0,
      radii: [20.0; 4],
      transform: [1.0, 0.0, 0.0, 1.0],
      transform_origin: [50.0, 20.0],
      clip: ClipRect::default(),
      sigma: 4.0,
      opacity: 1.0,
      glass: Some(0),
      reach: glass.reach(),
      spread: glass.spread(),
    };
    let list = RenderList {
      clear_color: Color::new(0, 0, 0, 255),
      rects,
      glyphs: Vec::new(),
      #[cfg(feature = "raster")]
      images: Vec::new(),
      layers: Vec::new(),
      backdrops: vec![backdrop],
      glasses: vec![glass],
      atlas: GlyphAtlas {
        data: Arc::from([255_u8; 4 * 4 * 4].as_slice()),
        width: 4,
        height: 4,
        version: 1,
        dirty_rects: Arc::from(Vec::new()),
        dirty_from_version: 0,
      },
    };
    (list, backdrop, glass)
  }

  fn lightness(rects: Vec<RectCmd>) -> f32 {
    let (list, backdrop, glass) = scene(rects);
    backdrop_lightness(&list, &backdrop, &glass).unwrap()
  }

  #[test]
  fn the_window_alone_is_its_clear_colour() {
    assert_eq!(lightness(Vec::new()), 0.0);
  }

  #[test]
  fn what_is_painted_before_the_glass_counts_and_what_comes_after_does_not() {
    let white = Color::new(255, 255, 255, 255);
    assert!((lightness(vec![rect(1, [0.0, 0.0, 300.0, 300.0], white)]) - 1.0).abs() < 1e-4);
    assert_eq!(lightness(vec![rect(11, [0.0, 0.0, 300.0, 300.0], white)]), 0.0);
    // Beside the glass.
    assert_eq!(lightness(vec![rect(1, [200.0, 0.0, 300.0, 300.0], white)]), 0.0);
  }

  #[test]
  fn half_of_the_glass_over_white_is_half_light() {
    let half = lightness(vec![rect(
      1,
      [100.0, 0.0, 300.0, 300.0],
      Color::new(255, 255, 255, 255),
    )]);
    assert!((half - 0.5).abs() < 0.1, "{half}");
  }

  #[test]
  fn a_fill_that_lets_light_through_is_laid_over_what_is_under_it() {
    let veil = lightness(vec![
      rect(1, [0.0, 0.0, 300.0, 300.0], Color::new(255, 255, 255, 255)),
      rect(2, [0.0, 0.0, 300.0, 300.0], Color::new(0, 0, 0, 128)),
    ]);
    // A grey of 127 of 255.
    assert!((veil - 0.532).abs() < 0.01, "{veil}");
  }

  #[test]
  fn a_clip_cuts_what_is_painted() {
    let mut clipped = rect(1, [0.0, 0.0, 300.0, 300.0], Color::new(255, 255, 255, 255));
    clipped.clip = ClipRect::rect(0.0, 0.0, 10.0, 10.0);
    assert_eq!(lightness(vec![clipped]), 0.0);
  }
}
