//! Sensitive text in captures made for inspection.
//!
//! [`Text::sensitive`](crate::components::Text::sensitive) text is painted on
//! screen as usual, so a frame read back from the GPU holds its glyphs. A
//! capture made for an inspector (`lurq_screenshot`, a DevTools node
//! screenshot) has every pixel a sensitive text run can touch replaced by an
//! opaque grey bar before the pixels leave the capture pipeline, so the window
//! keeps showing the text to the person while the image shows none of it.
//! Layout is unchanged: the bar covers the run where it was painted.

use std::sync::Arc;

use crate::{
  app::render_engine::{CapturedFrame, RenderCaptureTarget},
  layout::{quad::ClipRect, render_list::GlyphCmd},
};

/// The bar painted over sensitive text: opaque, so neither colour nor
/// coverage of the glyphs underneath survives.
pub(crate) const REDACTION_FILL: [u8; 4] = [128, 128, 128, 255];

/// How far past its rect a glyph's anti-aliased or resampled edge can reach,
/// in physical pixels.
const GLYPH_EDGE_REACH: f32 = 2.0;

/// Whole physical window pixels `x0..x1` by `y0..y1` covering one sensitive
/// text run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct RedactedArea {
  pub(crate) x0: u32,
  pub(crate) y0: u32,
  pub(crate) x1: u32,
  pub(crate) y1: u32,
}

impl RedactedArea {
  /// The pixels the glyphs of one text run can paint, text shadow included,
  /// each glyph limited by its clip. `None` when nothing of the run is on
  /// screen.
  pub(crate) fn of_glyph_run(glyphs: &[GlyphCmd]) -> Option<Self> {
    let mut bounds: Option<[f32; 4]> = None;
    for glyph in glyphs {
      let Some(glyph_bounds) = clip_bounds(glyph_bounds(glyph), glyph.clip) else {
        continue;
      };
      bounds = Some(match bounds {
        Some(b) => [
          b[0].min(glyph_bounds[0]),
          b[1].min(glyph_bounds[1]),
          b[2].max(glyph_bounds[2]),
          b[3].max(glyph_bounds[3]),
        ],
        None => glyph_bounds,
      });
    }
    let [x0, y0, x1, y1] = bounds?;
    let area = Self {
      x0: x0.floor().max(0.0) as u32,
      y0: y0.floor().max(0.0) as u32,
      x1: x1.ceil().max(0.0) as u32,
      y1: y1.ceil().max(0.0) as u32,
    };
    (area.x0 < area.x1 && area.y0 < area.y1).then_some(area)
  }
}

/// A glyph's painted extent `[x0, y0, x1, y1]`: its rect, grown by the blur
/// reach of a text-shadow instance, through its transform the way the glyph
/// vertex stages place it.
fn glyph_bounds(glyph: &GlyphCmd) -> [f32; 4] {
  let pad = if glyph.shadow_sigma > 0.0 {
    (glyph.shadow_sigma * 2.0).ceil() + 1.0
  } else {
    0.0
  } + GLYPH_EDGE_REACH;
  let [a, b, c, d] = glyph.transform;
  let [origin_x, origin_y] = glyph.transform_origin;
  let corners = [
    (-pad, -pad),
    (glyph.width + pad, -pad),
    (-pad, glyph.height + pad),
    (glyph.width + pad, glyph.height + pad),
  ];
  let mut bounds = [f32::INFINITY, f32::INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY];
  for (local_x, local_y) in corners {
    let (cx, cy) = (local_x - origin_x, local_y - origin_y);
    let x = glyph.x + a * cx + c * cy + origin_x;
    let y = glyph.y + b * cx + d * cy + origin_y;
    bounds = [bounds[0].min(x), bounds[1].min(y), bounds[2].max(x), bounds[3].max(y)];
  }
  bounds
}

fn clip_bounds(bounds: [f32; 4], clip: ClipRect) -> Option<[f32; 4]> {
  let bounds = if clip.active {
    [
      bounds[0].max(clip.x),
      bounds[1].max(clip.y),
      bounds[2].min(clip.x + clip.width),
      bounds[3].min(clip.y + clip.height),
    ]
  } else {
    bounds
  };
  (bounds[0] < bounds[2] && bounds[1] < bounds[3]).then_some(bounds)
}

/// Paints the bar over `areas` in `pixels`, a tight RGBA8 capture `width` x
/// `height` whose top-left pixel is window pixel `origin`.
pub(crate) fn redact_pixels(pixels: &mut [u8], width: u32, height: u32, origin: (u32, u32), areas: &[RedactedArea]) {
  let (origin_x, origin_y) = origin;
  for area in areas {
    let x0 = area.x0.saturating_sub(origin_x).min(width);
    let x1 = area.x1.saturating_sub(origin_x).min(width);
    let y0 = area.y0.saturating_sub(origin_y).min(height);
    let y1 = area.y1.saturating_sub(origin_y).min(height);
    for y in y0..y1 {
      let row = (y * width) as usize * 4;
      for pixel in pixels[row + x0 as usize * 4..row + x1 as usize * 4].chunks_exact_mut(4) {
        pixel.copy_from_slice(&REDACTION_FILL);
      }
    }
  }
}

/// `target`, with `areas` redacted from the pixels before they reach it. The
/// capture's top-left pixel is window pixel `origin`. Without areas the
/// target is returned as is.
pub(crate) fn redacting_target(
  target: RenderCaptureTarget,
  origin: (u32, u32),
  areas: Vec<RedactedArea>,
) -> RenderCaptureTarget {
  if areas.is_empty() {
    return target;
  }
  RenderCaptureTarget::Bytes(Arc::new(move |outcome: Result<CapturedFrame, String>| {
    let outcome = outcome.map(|mut frame| {
      redact_pixels(&mut frame.rgba, frame.width, frame.height, origin, &areas);
      frame
    });
    match (&target, outcome) {
      (RenderCaptureTarget::Bytes(callback), outcome) => callback(outcome),
      (RenderCaptureTarget::Path(path), Ok(frame)) => {
        crate::app::frame_capture::save_capture_png(path, &frame.rgba, frame.width, frame.height);
      }
      (RenderCaptureTarget::Path(_), Err(reason)) => target.fail(reason),
    }
  }))
}

#[cfg(test)]
mod tests;
