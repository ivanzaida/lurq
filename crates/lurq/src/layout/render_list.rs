use crate::{
  app::theme::ThemePalette,
  layout::quad::ClipRect,
  node::{
    color::Color,
    gradient::{Gradient, GradientKind},
  },
};

pub struct RenderList {
  pub clear_color: Color,
  pub rects: Vec<RectCmd>,
  pub glyphs: Vec<GlyphCmd>,
  #[cfg(feature = "image")]
  pub images: Vec<crate::images::ImageCmd>,
  #[cfg(feature = "svg")]
  pub svgs: Vec<crate::svg::SvgCmd>,
  pub atlas: GlyphAtlas,
}

pub struct RectCmd {
  pub order: usize,
  pub x: f32,
  pub y: f32,
  pub width: f32,
  pub height: f32,
  pub color: Color,
  pub radii: [f32; 4],
  pub stroke: [f32; 4],
  pub stroke_color: Color,
  pub transform: [f32; 4],
  pub transform_origin: [f32; 2],
  pub clip: ClipRect,
  pub gradient: Option<RenderGradient>,
}

/// A palette-resolved, encode-ready gradient attached to a rect fill.
///
/// `kind`: 0 = linear, 1 = radial, 2 = conic. `flags` bit0 marks a radial
/// ellipse (vs. circle). `dir` is the linear unit direction; `center` is the
/// radial/conic center normalized in `0..1`; `from_angle` is the conic start
/// angle in radians.
#[derive(Clone)]
pub struct RenderGradient {
  pub kind: u32,
  pub flags: u32,
  pub dir: [f32; 2],
  pub center: [f32; 2],
  pub from_angle: f32,
  pub stops: Vec<RenderGradientStop>,
}

#[derive(Clone, Copy)]
pub struct RenderGradientStop {
  pub color: [f32; 4],
  pub position: f32,
}

impl RenderGradient {
  pub fn resolve(gradient: &Gradient, palette: &ThemePalette) -> Option<Self> {
    if gradient.stops.is_empty() {
      return None;
    }
    let positions = normalize_positions(gradient);
    let stops = gradient
      .stops
      .iter()
      .zip(positions)
      .map(|(stop, position)| RenderGradientStop {
        color: stop
          .color
          .resolve(palette)
          .unwrap_or(Color::new(0, 0, 0, 0))
          .to_linear_f32_array(),
        position,
      })
      .collect();
    let (kind, flags) = match gradient.kind {
      GradientKind::Linear => (0, 0),
      GradientKind::Radial => (1, u32::from(gradient.radial_ellipse)),
      GradientKind::Conic => (2, 0),
    };
    let theta = gradient.angle_deg.to_radians();
    Some(Self {
      kind,
      flags,
      dir: [theta.sin(), -theta.cos()],
      center: [gradient.center.0, gradient.center.1],
      from_angle: theta,
      stops,
    })
  }
}

/// Resolve CSS stop positions: endpoints pin to 0/1 when omitted, defined
/// stops are clamped non-decreasing, and runs of omitted stops are spread
/// evenly between their defined neighbors.
fn normalize_positions(gradient: &Gradient) -> Vec<f32> {
  let n = gradient.stops.len();
  let mut pos: Vec<Option<f32>> = gradient.stops.iter().map(|s| s.position).collect();
  pos[0] = Some(pos[0].unwrap_or(0.0));
  pos[n - 1] = Some(pos[n - 1].unwrap_or(1.0));

  let mut last = 0.0_f32;
  for slot in pos.iter_mut() {
    if let Some(value) = slot {
      let clamped = value.clamp(0.0, 1.0).max(last);
      *slot = Some(clamped);
      last = clamped;
    }
  }

  let mut i = 0;
  while i < n {
    if pos[i].is_some() {
      i += 1;
      continue;
    }
    let start = i - 1;
    let start_val = pos[start].unwrap();
    let mut j = i;
    while j < n && pos[j].is_none() {
      j += 1;
    }
    let end_val = pos[j].unwrap();
    let segments = (j - start) as f32;
    for (offset, k) in (i..j).enumerate() {
      let frac = (offset + 1) as f32 / segments;
      pos[k] = Some(start_val + (end_val - start_val) * frac);
    }
    i = j;
  }

  pos.into_iter().map(|p| p.unwrap_or(0.0)).collect()
}

/// Append a gradient to a `vec4` storage buffer and return the `vec4` index of
/// its header. Layout (shared by both shader backends):
/// `[count, kind, flags, from_angle]`, `[dir.x, dir.y, center.x, center.y]`,
/// then per stop `[r, g, b, a]` and `[position, 0, 0, 0]`.
pub fn encode_gradient(buffer: &mut Vec<[f32; 4]>, gradient: &RenderGradient) -> f32 {
  let offset = buffer.len() as f32;
  buffer.push([
    gradient.stops.len() as f32,
    gradient.kind as f32,
    gradient.flags as f32,
    gradient.from_angle,
  ]);
  buffer.push([gradient.dir[0], gradient.dir[1], gradient.center[0], gradient.center[1]]);
  for stop in &gradient.stops {
    buffer.push(stop.color);
    buffer.push([stop.position, 0.0, 0.0, 0.0]);
  }
  offset
}

pub struct GlyphCmd {
  pub order: usize,
  pub x: f32,
  pub y: f32,
  pub width: f32,
  pub height: f32,
  pub color: [f32; 4],
  pub uv_min: [f32; 2],
  pub uv_max: [f32; 2],
  pub transform: [f32; 4],
  pub transform_origin: [f32; 2],
  pub sharpness: f32,
  pub color_glyph: bool,
  pub clip: ClipRect,
}

pub struct GlyphAtlas {
  pub data: std::sync::Arc<[u8]>,
  pub width: u32,
  pub height: u32,
  pub version: u64,
  pub dirty_rects: std::sync::Arc<[GlyphAtlasDirtyRect]>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GlyphAtlasDirtyRect {
  pub x: u32,
  pub y: u32,
  pub width: u32,
  pub height: u32,
}
