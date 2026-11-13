// Clips inside a clip with rounded corners, through the real render engines,
// read back from a hidden window of the test's own: what lies along an edge
// of a rounded panel (a rail along its bottom, a band across its top) is cut
// by the panel's round corners, and a child with corners of its own at the
// panel's corner by both. Rects and glyphs cover the whole window under each
// clip; a pixel well inside every clip shows them, and one well outside any
// shows the background.
//
// cargo test -p lurq --features wgpu,dx12,screenshot --lib nested_clip_readback -- --ignored --test-threads=1

use std::sync::Arc;

use super::readback_window;
use crate::{
  app::render_engine::{CapturedFrame, RenderEngine},
  layout::{
    quad::ClipRect,
    render_list::{GlyphAtlas, GlyphCmd, RectCmd, RenderList, StrokeStyle},
  },
  node::{border::BorderRadius, color::Color},
};

const WIDTH: u32 = 120;
const HEIGHT: u32 = 80;
const CLEAR: Color = Color::new(24, 24, 27, 255);
const FILL: Color = Color::new(250, 250, 250, 255);
/// How far a pixel's centre must be from every edge of a clip to be judged:
/// the edges themselves are anti-aliased.
const MARGIN: f32 = 1.5;
/// Side of the one glyph in the atlas: an 8 px opaque square inside the two
/// transparent texels the glyph atlas keeps around every glyph.
const GLYPH: f32 = 12.0;
const ATLAS: u32 = 16;

/// The panel every scene is in: a box with corners of 16.
fn panel() -> ClipRect {
  ClipRect::rounded_rect(10.0, 10.0, 100.0, 60.0, Some(BorderRadius::all(16.0)))
}

/// The clips of the scenes, and how many rounded clips each carries.
fn scenes() -> Vec<(&'static str, ClipRect, usize)> {
  vec![
    (
      "a rail along the bottom of the panel",
      panel().intersect(ClipRect::rect(10.0, 62.0, 100.0, 8.0)),
      1,
    ),
    (
      "a band across the top of the panel",
      panel().intersect(ClipRect::rect(10.0, 10.0, 100.0, 12.0)),
      1,
    ),
    (
      "a child with corners of its own at the panel's corner",
      panel().intersect(ClipRect::rounded_rect(
        10.0,
        10.0,
        50.0,
        30.0,
        Some(BorderRadius::new(4.0, 12.0, 12.0, 12.0)),
      )),
      2,
    ),
    (
      "a rounded child well inside the panel",
      panel().intersect(ClipRect::rounded_rect(
        40.0,
        25.0,
        40.0,
        30.0,
        Some(BorderRadius::all(10.0)),
      )),
      1,
    ),
  ]
}

/// The signed distance from `(x, y)` to a rounded rect's edge: negative inside.
fn rounded_distance(x: f32, y: f32, rect: [f32; 4], radius: [f32; 4]) -> f32 {
  let [rx, ry, width, height] = rect;
  let (half_x, half_y) = (width * 0.5, height * 0.5);
  let (px, py) = (x - (rx + half_x), y - (ry + half_y));
  let r = match (px >= 0.0, py >= 0.0) {
    (false, false) => radius[0],
    (true, false) => radius[1],
    (true, true) => radius[2],
    (false, true) => radius[3],
  };
  let (qx, qy) = (px.abs() - half_x + r, py.abs() - half_y + r);
  qx.max(0.0).hypot(qy.max(0.0)) + qx.max(qy).min(0.0) - r
}

/// The greatest signed distance from `(x, y)` to the edges of `clip`: its
/// rectangle and every rounded clip it carries.
fn clip_distance(clip: &ClipRect, x: f32, y: f32) -> f32 {
  let rect = rounded_distance(x, y, [clip.x, clip.y, clip.width, clip.height], [0.0; 4]);
  clip.rounded_clips().fold(rect, |distance, rounded| {
    distance.max(rounded_distance(
      x,
      y,
      [rounded.x, rounded.y, rounded.width, rounded.height],
      rounded.radius.to_array(),
    ))
  })
}

fn atlas() -> GlyphAtlas {
  let mut data = vec![0_u8; (ATLAS * ATLAS * 4) as usize];
  for y in 2..10 {
    for x in 2..10 {
      let index = ((y * ATLAS + x) * 4) as usize;
      data[index..index + 4].copy_from_slice(&[255; 4]);
    }
  }
  GlyphAtlas {
    data: Arc::from(data),
    width: ATLAS,
    height: ATLAS,
    version: 1,
    dirty_rects: Arc::from(Vec::new()),
    dirty_from_version: 0,
  }
}

fn list(rects: Vec<RectCmd>, glyphs: Vec<GlyphCmd>) -> RenderList {
  RenderList {
    clear_color: CLEAR,
    rects,
    glyphs,
    #[cfg(feature = "raster")]
    images: Vec::new(),
    #[cfg(feature = "svg")]
    svgs: Vec::new(),
    layers: Vec::new(),
    backdrops: Vec::new(),
    atlas: atlas(),
  }
}

/// A rect over the whole window, under `clip`.
fn rect_scene(clip: ClipRect) -> RenderList {
  let rect = RectCmd {
    order: 0,
    x: 0.0,
    y: 0.0,
    width: WIDTH as f32,
    height: HEIGHT as f32,
    color: FILL,
    radii: [0.0; 4],
    stroke: [0.0; 4],
    stroke_color: Color::new(0, 0, 0, 0),
    stroke_style: StrokeStyle::default(),
    transform: [1.0, 0.0, 0.0, 1.0],
    transform_origin: [0.0; 2],
    clip,
    gradient: None,
    shadow: None,
  };
  list(vec![rect], Vec::new())
}

/// Glyphs whose ink tiles the whole window, one text under `clip`.
fn glyph_scene(clip: ClipRect) -> RenderList {
  let mut glyphs = Vec::new();
  for row in 0..HEIGHT / 8 + 1 {
    for column in 0..WIDTH / 8 + 1 {
      glyphs.push(GlyphCmd {
        order: 0,
        x: (column * 8) as f32 - 2.0,
        y: (row * 8) as f32 - 2.0,
        width: GLYPH,
        height: GLYPH,
        color: FILL.to_linear_f32_array(),
        atlas_min: [0.0, 0.0],
        atlas_max: [GLYPH, GLYPH],
        transform: [1.0, 0.0, 0.0, 1.0],
        transform_origin: [0.0; 2],
        sharpness: 1.0,
        color_glyph: false,
        shadow_sigma: 0.0,
        clip,
      });
    }
  }
  list(Vec::new(), glyphs)
}

fn pixel(frame: &CapturedFrame, x: u32, y: u32) -> [u8; 3] {
  let index = ((y * frame.width + x) * 4) as usize;
  [frame.rgba[index], frame.rgba[index + 1], frame.rgba[index + 2]]
}

fn assert_cut_to(frame: &CapturedFrame, clip: &ClipRect, what: &str) {
  let clear = [CLEAR.r(), CLEAR.g(), CLEAR.b()];
  let fill = [FILL.r(), FILL.g(), FILL.b()];
  let close = |actual: [u8; 3], expected: [u8; 3]| actual.iter().zip(expected).all(|(a, e)| a.abs_diff(e) <= 2);
  let (mut inside, mut outside) = (0, 0);
  for y in 0..HEIGHT {
    for x in 0..WIDTH {
      let distance = clip_distance(clip, x as f32 + 0.5, y as f32 + 0.5);
      let painted = pixel(frame, x, y);
      if distance < -MARGIN {
        inside += 1;
        assert!(
          close(painted, fill),
          "{what}: ({x}, {y}) is inside every clip but is {painted:?}"
        );
      } else if distance > MARGIN {
        outside += 1;
        assert!(
          close(painted, clear),
          "{what}: ({x}, {y}) is outside a clip but is {painted:?}, not the background"
        );
      }
    }
  }
  assert!(
    inside > 50 && outside > 50,
    "{what}: {inside} pixels inside, {outside} outside"
  );
}

fn assert_nested_clips_cut<E: RenderEngine>(mut new_engine: impl FnMut() -> E, backend: &str) {
  for (scene, clip, rounded) in scenes() {
    assert_eq!(clip.rounded_clips().count(), rounded, "{scene}: rounded clips kept");
    let frame = readback_window::capture(&mut new_engine(), &rect_scene(clip), WIDTH, HEIGHT);
    assert_cut_to(&frame, &clip, &format!("{backend}, rects, {scene}"));
    let frame = readback_window::capture(&mut new_engine(), &glyph_scene(clip), WIDTH, HEIGHT);
    assert_cut_to(&frame, &clip, &format!("{backend}, glyphs, {scene}"));
  }
  // The rail's bottom-left pixel is in the rail and past the panel's corner: the background's.
  let (_, rail, _) = scenes()[0];
  let frame = readback_window::capture(&mut new_engine(), &rect_scene(rail), WIDTH, HEIGHT);
  assert_eq!(
    pixel(&frame, 10, 69),
    [CLEAR.r(), CLEAR.g(), CLEAR.b()],
    "{backend}: the rail's corner"
  );
  assert_eq!(
    pixel(&frame, 60, 66),
    [FILL.r(), FILL.g(), FILL.b()],
    "{backend}: the rail"
  );
}

#[cfg(feature = "wgpu")]
#[test]
#[ignore = "requires a Windows desktop and a GPU adapter; creates hidden windows"]
fn wgpu_nested_clips_cut_by_the_rounded_clip_around_them() {
  assert_nested_clips_cut(crate::app::wgpu_render::WgpuRenderEngine::new, "wgpu");
}

#[cfg(feature = "dx12")]
#[test]
#[ignore = "requires a Windows desktop and a DX12 adapter; creates hidden windows"]
fn dx12_nested_clips_cut_by_the_rounded_clip_around_them() {
  assert_nested_clips_cut(crate::app::dx12_render::Dx12RenderEngine::new, "dx12");
}
