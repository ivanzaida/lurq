//! The gallery's icon: the box of the board's mark, white on a dark tile, drawn here so that the gallery needs no
//! image file. The window has it, and so the title bars the gallery shows as pictures show it too.

use lurq::app::WindowIcon;

/// The side of the icon, in pixels.
const SIDE: u32 = 64;

/// The lines of the box, as the theme's `box` glyph has them on its grid of 24: its outline, the edges that meet
/// in its middle, and the edge down from there.
const LINES: [((f32, f32), (f32, f32)); 9] = [
  ((12.0, 2.0), (21.0, 7.0)),
  ((21.0, 7.0), (21.0, 17.0)),
  ((21.0, 17.0), (12.0, 22.0)),
  ((12.0, 22.0), (3.0, 17.0)),
  ((3.0, 17.0), (3.0, 7.0)),
  ((3.0, 7.0), (12.0, 2.0)),
  ((3.3, 7.0), (12.0, 12.0)),
  ((12.0, 12.0), (20.7, 7.0)),
  ((12.0, 12.0), (12.0, 22.0)),
];

/// How far the point `p` is from the segment from `a` to `b`.
fn distance(p: (f32, f32), a: (f32, f32), b: (f32, f32)) -> f32 {
  let (dx, dy) = (b.0 - a.0, b.1 - a.1);
  let along = (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / (dx * dx + dy * dy)).clamp(0.0, 1.0);
  let (x, y) = (a.0 + along * dx - p.0, a.1 + along * dy - p.1);
  (x * x + y * y).sqrt()
}

/// How much of the pixel at `(x, y)` a rounded square of `side` with corners of `radius` covers.
fn tile(x: f32, y: f32, side: f32, radius: f32) -> f32 {
  let (cx, cy) = (x.clamp(radius, side - radius), y.clamp(radius, side - radius));
  let out = ((x - cx).powi(2) + (y - cy).powi(2)).sqrt() - radius;
  (0.5 - out).clamp(0.0, 1.0)
}

/// The gallery's icon, as the window takes it.
pub fn icon() -> WindowIcon {
  let side = SIDE as f32;
  // The glyph's grid of 24 in the middle 40 of the tile; its lines 2 wide on the grid.
  let scale = 40.0 / 24.0;
  let offset = (side - 40.0) / 2.0;
  let half_line = 1.0 * scale;
  let mut rgba = Vec::with_capacity((SIDE * SIDE * 4) as usize);
  for row in 0..SIDE {
    for column in 0..SIDE {
      let (x, y) = (column as f32 + 0.5, row as f32 + 0.5);
      let on_grid = ((x - offset) / scale, (y - offset) / scale);
      let near = LINES
        .iter()
        .map(|(a, b)| distance(on_grid, *a, *b) * scale)
        .fold(f32::MAX, f32::min);
      let line = (half_line + 0.5 - near).clamp(0.0, 1.0);
      // `#18181B` under white.
      let mix = |dark: f32| (dark + (250.0 - dark) * line).round() as u8;
      rgba.extend_from_slice(&[
        mix(24.0),
        mix(24.0),
        mix(27.0),
        (tile(x, y, side, 14.0) * 255.0).round() as u8,
      ]);
    }
  }
  WindowIcon::from_rgba(rgba, SIDE, SIDE)
}
