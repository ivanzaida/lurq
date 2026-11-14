// Strokes through the real render engines, read back from a hidden window of
// the test's own and compared, pixel by pixel, with the formulas of
// `layout/stroke.rs`: rings of one width and of several, a stroke on one side
// of a rounded box, radii at the hole and at the middle of the stroke, and
// dashes.
//
// cargo test -p lurq --features wgpu,dx12,screenshot --lib stroke_readback -- --ignored --test-threads=1

use std::sync::Arc;

use super::readback_window;
use crate::{
  app::render_engine::RenderEngine,
  layout::{
    quad::ClipRect,
    render_list::{GlyphAtlas, RectCmd, RenderList, StrokeStyle},
    stroke::{StrokeShape, stroke_coverage},
  },
  node::color::Color,
};

const WIDTH: u32 = 200;
const HEIGHT: u32 = 120;
/// How far a channel may be from the reference away from an edge: the shaders work in 32-bit floats.
const TOLERANCE: u8 = 6;
/// And on a pixel at an edge. The ramp there is as wide as the distance changes over a pixel, which a GPU
/// takes from derivatives over a block of two by two pixels and the reference from the pixel's own.
const EDGE_TOLERANCE: u8 = 72;

/// A white stroke with no fill.
fn stroke(
  [x, y, width, height]: [f32; 4],
  radii: [f32; 4],
  widths: [f32; 4],
  dash: [f32; 2],
  radii_at: f32,
) -> RectCmd {
  RectCmd {
    order: 0,
    x,
    y,
    width,
    height,
    color: Color::new(0, 0, 0, 0),
    radii,
    stroke: widths,
    stroke_color: Color::new(255, 255, 255, 255),
    stroke_style: StrokeStyle {
      dash,
      radii_at,
      side_colors: None,
    },
    transform: [1.0, 0.0, 0.0, 1.0],
    transform_origin: [width * 0.5, height * 0.5],
    clip: ClipRect::default(),
    gradient: None,
    shadow: None,
  }
}

fn scenes() -> Vec<(&'static str, RectCmd)> {
  const SOLID: [f32; 2] = [0.0; 2];
  vec![
    (
      "a ring of one width",
      stroke([20.0, 20.0, 120.0, 60.0], [10.0; 4], [2.0; 4], SOLID, 0.0),
    ),
    (
      "one side of a rounded box",
      stroke([20.0, 20.0, 120.0, 60.0], [12.0; 4], [0.0, 0.0, 0.0, 4.0], SOLID, 0.0),
    ),
    (
      "one side wider than the corner",
      stroke([20.0, 20.0, 120.0, 60.0], [6.0; 4], [10.0, 0.0, 0.0, 0.0], SOLID, 0.0),
    ),
    (
      "a width per side and a radius per corner",
      stroke(
        [20.0, 20.0, 140.0, 70.0],
        [12.0, 0.0, 20.0, 6.0],
        [1.0, 3.0, 5.0, 2.0],
        SOLID,
        0.0,
      ),
    ),
    (
      "one side at a fraction of a pixel",
      stroke([20.4, 20.7, 120.3, 60.6], [9.0; 4], [0.0, 0.0, 0.0, 3.0], SOLID, 0.0),
    ),
    (
      "three sides at a fraction of a pixel",
      stroke([20.4, 20.7, 120.3, 60.6], [9.0; 4], [2.0, 2.0, 0.0, 2.0], SOLID, 0.0),
    ),
    (
      "radii at the hole",
      stroke([27.0, 27.0, 106.0, 46.0], [10.0; 4], [3.0; 4], SOLID, 1.0),
    ),
    (
      "radii at the hole, one side",
      stroke([26.0, 30.0, 104.0, 40.0], [10.0; 4], [0.0, 0.0, 0.0, 4.0], SOLID, 1.0),
    ),
    (
      "radii at the middle of the stroke",
      stroke([28.0, 28.0, 104.0, 44.0], [10.0; 4], [4.0; 4], SOLID, 0.5),
    ),
    (
      "a square box with radii at the hole",
      stroke([27.0, 27.0, 106.0, 46.0], [0.0; 4], [3.0; 4], SOLID, 1.0),
    ),
    (
      "a dashed ring",
      stroke([20.0, 20.0, 120.0, 60.0], [10.0; 4], [2.0; 4], [6.0, 4.0], 0.0),
    ),
    (
      "a dashed ring with a radius per corner",
      stroke(
        [20.0, 20.0, 140.0, 70.0],
        [12.0, 0.0, 20.0, 6.0],
        [2.0; 4],
        [7.0, 5.0],
        0.0,
      ),
    ),
    (
      "a dashed bottom side",
      stroke(
        [20.0, 20.0, 150.0, 40.0],
        [0.0; 4],
        [0.0, 0.0, 2.0, 0.0],
        [5.0, 3.0],
        0.0,
      ),
    ),
    (
      "the bottom of a rounded box",
      stroke([20.0, 20.0, 120.0, 60.0], [12.0; 4], [0.0, 0.0, 2.0, 0.0], SOLID, 0.0),
    ),
    (
      "two sides of a rounded box",
      stroke([20.0, 20.0, 120.0, 60.0], [12.0; 4], [0.0, 0.0, 2.0, 2.0], SOLID, 0.0),
    ),
    (
      "a dashed bottom of a rounded box",
      stroke(
        [20.0, 20.0, 120.0, 60.0],
        [12.0; 4],
        [0.0, 0.0, 3.0, 0.0],
        [5.0, 3.0],
        0.0,
      ),
    ),
    (
      // A box 120 by 40 with a bottom border of 2 outside it.
      "the bottom outside a pill",
      stroke([30.0, 30.0, 120.0, 42.0], [20.0; 4], [0.0, 0.0, 2.0, 0.0], SOLID, 1.0),
    ),
    (
      "the bottom centered on a rounded box",
      stroke([30.0, 30.0, 120.0, 41.0], [10.0; 4], [0.0, 0.0, 2.0, 0.0], SOLID, 0.5),
    ),
  ]
}

/// A box of 120 by 60 at (20, 20) with corners of 12: its bottom-right corner is centred at (128, 68), its
/// bottom-left one at (32, 68).
const BOX: [f32; 4] = [20.0, 20.0, 120.0, 60.0];

/// The pixels of a corner centred at `centre`, by where their centres are round it: the distance from the
/// centre, and the angle from straight down towards `side` (1 right, -1 left) in degrees.
fn corner_pixels(centre: (f32, f32), side: f32) -> impl Iterator<Item = (u32, u32, f32, f32)> {
  (0..HEIGHT).flat_map(move |y| {
    (0..WIDTH).filter_map(move |x| {
      let (dx, dy) = ((x as f32 + 0.5 - centre.0) * side, y as f32 + 0.5 - centre.1);
      (dx >= 0.0 && dy >= 0.0).then(|| (x, y, dx.hypot(dy), dx.atan2(dy).to_degrees()))
    })
  })
}

fn pixel(frame: &crate::app::render_engine::CapturedFrame, x: u32, y: u32) -> [u8; 3] {
  let index = ((y * WIDTH + x) * 4) as usize;
  [frame.rgba[index], frame.rgba[index + 1], frame.rgba[index + 2]]
}

fn linear(channel: u8) -> f32 {
  crate::node::color::Color::new(channel, 0, 0, 255).to_linear_f32_array()[0]
}

/// A border on some sides of a rounded box, read back: it goes round a corner whose other side has none at
/// its full width and fades out along the arc; two sides of two colors mix across their corner at full
/// coverage; a square corner ends the band at the box's edge; dashes fade with it.
fn assert_borders_blend_round_corners<E: RenderEngine>(mut engine: impl FnMut() -> E, backend: &str) {
  // The bottom, 3 wide: the ring round the bottom-right corner is between radius 9 and 12.
  let frame = readback_window::capture(
    &mut engine(),
    &list(stroke(BOX, [12.0; 4], [0.0, 0.0, 3.0, 0.0], [0.0; 2], 0.0)),
    WIDTH,
    HEIGHT,
  );
  let at = |x, y| pixel(&frame, x, y)[0];
  assert_eq!(at(80, 78), 255, "{backend}: the middle of the side");
  assert_eq!(at(80, 77), 255, "{backend}: the middle of the side");
  assert!(at(80, 75) <= TOLERANCE, "{backend}: above the side");
  let mut seen = [0; 3];
  for (x, y, radius, angle) in corner_pixels((128.0, 68.0), 1.0) {
    let value = at(x, y);
    let in_ring = (9.6..=11.4).contains(&radius);
    if in_ring && (5.0..=20.0).contains(&angle) {
      seen[0] += 1;
      assert!(value >= 200, "{backend}: ({x}, {y}) near the side is {value}");
    }
    if in_ring && (40.0..=50.0).contains(&angle) {
      seen[1] += 1;
      assert!(
        (95..=160).contains(&value),
        "{backend}: ({x}, {y}) half way round, the whole width, is {value}"
      );
    }
    if in_ring && (75.0..=85.0).contains(&angle) {
      seen[2] += 1;
      assert!(value <= 40, "{backend}: ({x}, {y}) near the right side is {value}");
    }
    if !(8.0..=13.0).contains(&radius) && angle < 89.0 {
      assert!(value <= TOLERANCE, "{backend}: ({x}, {y}) off the ring is {value}");
    }
  }
  assert!(seen.iter().all(|count| *count >= 3), "{backend}: {seen:?}");
  // Nothing where the arc ends on the right side, nor along the sides with no border.
  for (x, y) in [
    (139, 67),
    (139, 66),
    (138, 60),
    (139, 50),
    (80, 20),
    (80, 21),
    (20, 50),
    (21, 50),
    (139, 25),
  ] {
    assert!(at(x, y) <= TOLERANCE, "{backend}: ({x}, {y}) is {}", at(x, y));
  }

  // The bottom red and the left blue, 3 wide: round the bottom-left corner the colors mix, at full coverage.
  let mut two_colors = stroke(BOX, [12.0; 4], [0.0, 0.0, 3.0, 3.0], [0.0; 2], 0.0);
  two_colors.stroke_color = Color::new(255, 0, 0, 255);
  two_colors.stroke_style.side_colors = Some([
    Color::new(255, 0, 0, 255),
    Color::new(255, 0, 0, 255),
    Color::new(255, 0, 0, 255),
    Color::new(0, 0, 255, 255),
  ]);
  let frame = readback_window::capture(&mut engine(), &list(two_colors), WIDTH, HEIGHT);
  let mut seen = 0;
  for (x, y, radius, angle) in corner_pixels((32.0, 68.0), -1.0) {
    if !(9.6..=11.4).contains(&radius) {
      continue;
    }
    seen += 1;
    let [r, _, b] = pixel(&frame, x, y);
    let coverage = linear(r) + linear(b);
    assert!(
      coverage >= 0.93,
      "{backend}: ({x}, {y}) at {angle} degrees covers {coverage} ({r}, {b})"
    );
    if (40.0..=50.0).contains(&angle) {
      assert!(
        r >= 150 && b >= 150,
        "{backend}: ({x}, {y}) half way round is ({r}, {b})"
      );
    }
    if angle < 10.0 {
      assert!(r > b, "{backend}: ({x}, {y}) near the bottom is ({r}, {b})");
    }
    if angle > 80.0 {
      assert!(b > r, "{backend}: ({x}, {y}) near the left is ({r}, {b})");
    }
  }
  assert!(seen >= 20, "{backend}: {seen} pixels round the corner");
  assert_eq!(pixel(&frame, 80, 78), [255, 0, 0], "{backend}: the bottom");
  assert_eq!(pixel(&frame, 21, 50), [0, 0, 255], "{backend}: the left side");

  // The bottom of a square box: a plain band to the box's edges.
  let frame = readback_window::capture(
    &mut engine(),
    &list(stroke(BOX, [0.0; 4], [0.0, 0.0, 2.0, 0.0], [0.0; 2], 0.0)),
    WIDTH,
    HEIGHT,
  );
  let at = |x, y| pixel(&frame, x, y)[0];
  for (x, y) in [(20, 79), (20, 78), (139, 79), (139, 78), (80, 79)] {
    assert_eq!(at(x, y), 255, "{backend}: ({x}, {y}) in the band");
  }
  for (x, y) in [(19, 79), (140, 79), (80, 77), (80, 80), (139, 77)] {
    assert!(
      at(x, y) <= TOLERANCE,
      "{backend}: ({x}, {y}) off the band is {}",
      at(x, y)
    );
  }

  // A dashed bottom of a rounded box: dashes and gaps along the side, and round the corner the dashes fade.
  let frame = readback_window::capture(
    &mut engine(),
    &list(stroke(BOX, [12.0; 4], [0.0, 0.0, 3.0, 0.0], [5.0, 3.0], 0.0)),
    WIDTH,
    HEIGHT,
  );
  let row: Vec<u8> = (34..126).map(|x| pixel(&frame, x, 78)[0]).collect();
  assert!(row.contains(&255), "{backend}: a dash");
  assert!(row.iter().any(|value| *value <= TOLERANCE), "{backend}: a gap");
  for (x, y, radius, angle) in corner_pixels((128.0, 68.0), 1.0) {
    let value = pixel(&frame, x, y)[0];
    if (9.0..=12.0).contains(&radius) && (40.0..=50.0).contains(&angle) {
      assert!(value <= 160, "{backend}: ({x}, {y}) half way round is {value}");
    }
    if radius > 13.0 || angle >= 89.0 {
      assert!(value <= TOLERANCE, "{backend}: ({x}, {y}) off the dashes is {value}");
    }
  }
}

#[cfg(feature = "wgpu")]
#[test]
#[ignore = "requires a Windows desktop and a GPU adapter; creates a hidden window"]
fn wgpu_borders_blend_round_corners() {
  assert_borders_blend_round_corners(crate::app::wgpu_render::WgpuRenderEngine::new, "wgpu");
}

#[cfg(feature = "dx12")]
#[test]
#[ignore = "requires a Windows desktop and a DX12 adapter; creates a hidden window"]
fn dx12_borders_blend_round_corners() {
  assert_borders_blend_round_corners(crate::app::dx12_render::Dx12RenderEngine::new, "dx12");
}

fn list(rect: RectCmd) -> RenderList {
  RenderList {
    clear_color: Color::new(0, 0, 0, 255),
    rects: vec![rect],
    glyphs: Vec::new(),
    #[cfg(feature = "raster")]
    images: Vec::new(),
    #[cfg(feature = "svg")]
    svgs: Vec::new(),
    layers: Vec::new(),
    backdrops: Vec::new(),
    atlas: GlyphAtlas {
      data: Arc::from([255_u8; 4 * 4 * 4].as_slice()),
      width: 4,
      height: 4,
      version: 1,
      dirty_rects: Arc::from(Vec::new()),
      dirty_from_version: 0,
    },
  }
}

/// What the reference says each pixel of a white stroke over black is.
fn reference(rect: &RectCmd) -> Vec<u8> {
  let shape = StrokeShape {
    half: [rect.width * 0.5, rect.height * 0.5],
    radii: rect.radii,
    stroke: rect.stroke,
    style: rect.stroke_style,
  };
  let centre = (rect.x + shape.half[0], rect.y + shape.half[1]);
  let mut pixels = Vec::with_capacity((WIDTH * HEIGHT) as usize);
  for y in 0..HEIGHT {
    for x in 0..WIDTH {
      let coverage = stroke_coverage(x as f32 + 0.5 - centre.0, y as f32 + 0.5 - centre.1, &shape);
      pixels.push((coverage * 255.0).round() as u8);
    }
  }
  pixels
}

/// Whether the reference has `value` at `(x, y)` and at every pixel around it.
fn all_around(reference: &[u8], x: u32, y: u32, value: u8) -> bool {
  (-1_i32..=1).all(|dy| {
    (-1_i32..=1).all(|dx| {
      let (nx, ny) = (x as i32 + dx, y as i32 + dy);
      let inside = nx >= 0 && ny >= 0 && nx < WIDTH as i32 && ny < HEIGHT as i32;
      !inside || reference[(ny as u32 * WIDTH + nx as u32) as usize] == value
    })
  })
}

/// The stroke is where the reference has it: a pixel the reference paints whole, with whole pixels all
/// around, is painted whole, and one it leaves empty among empty ones stays empty. On the pixels at an
/// edge, where a renderer takes its ramp from its own derivatives, the two are close, and over the whole
/// frame they cover the same area.
fn assert_strokes_match_the_reference<E: RenderEngine>(mut engine: impl FnMut() -> E, backend: &str) {
  for (scene, rect) in scenes() {
    let expected = reference(&rect);
    let lit = expected.iter().filter(|value| **value > 200).count();
    assert!(lit > 40, "{scene}: the reference paints the stroke ({lit} pixels)");
    let frame = readback_window::capture(&mut engine(), &list(rect), WIDTH, HEIGHT);
    assert_eq!((frame.width, frame.height), (WIDTH, HEIGHT));
    let (mut painted_area, mut expected_area) = (0_u64, 0_u64);
    for (index, (painted, expected_value)) in frame.rgba.chunks_exact(4).zip(&expected).enumerate() {
      let (x, y) = (index as u32 % WIDTH, index as u32 / WIDTH);
      let painted = painted[0];
      painted_area += u64::from(painted);
      expected_area += u64::from(*expected_value);
      let limit = if all_around(&expected, x, y, 255) || all_around(&expected, x, y, 0) {
        TOLERANCE
      } else {
        EDGE_TOLERANCE
      };
      assert!(
        painted.abs_diff(*expected_value) <= limit,
        "{backend}, {scene}: ({x}, {y}) is {painted}, the reference says {expected_value}"
      );
    }
    let off = painted_area.abs_diff(expected_area) as f64 / expected_area as f64;
    assert!(
      off <= 0.03,
      "{backend}, {scene}: the stroke covers {painted_area}, the reference {expected_area}"
    );
  }
}

#[cfg(feature = "wgpu")]
#[test]
#[ignore = "requires a Windows desktop and a GPU adapter; creates a hidden window"]
fn wgpu_strokes_match_the_reference() {
  assert_strokes_match_the_reference(crate::app::wgpu_render::WgpuRenderEngine::new, "wgpu");
}

#[cfg(feature = "dx12")]
#[test]
#[ignore = "requires a Windows desktop and a DX12 adapter; creates a hidden window"]
fn dx12_strokes_match_the_reference() {
  assert_strokes_match_the_reference(crate::app::dx12_render::Dx12RenderEngine::new, "dx12");
}
