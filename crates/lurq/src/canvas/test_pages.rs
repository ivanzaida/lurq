//! Pages that the GPU renderers' residency tests draw, whichever backend
//! encodes them.
use super::{CanvasFont, CanvasHandle, CanvasSnapshot, Context2D};
use crate::images::ImageData;

/// What the asset cache charges a texture of at most 64 KiB.
pub(crate) const CHARGE: usize = 64 * 1024;
/// The upload size of one of [`images`].
pub(crate) const IMAGE_BYTES: u64 = 16 * 16 * 4;
/// The shape cache's budget; its ceiling is twice it.
pub(crate) const SHAPE_BUDGET: usize = 8 * 1024 * 1024;

/// `count` distinct opaque 16 × 16 images of one colour each, distinct again
/// for each `salt`.
pub(crate) fn images(count: u8, salt: u8) -> Vec<ImageData> {
  (0..count)
    .map(|index| ImageData::from_rgba([index, salt, 255 - index, 255].repeat(16 * 16), 16, 16))
    .collect()
}

/// Draws the `at`-th image of a page in rows of sixteen 4 × 4 cells.
pub(crate) fn draw_cell(d: &Context2D, image: &ImageData, at: usize) {
  let (x, y) = ((at % 16) as f32 * 4., (at / 16) as f32 * 4.);
  d.draw_image_scaled(image, x, y, 4., 4.).unwrap();
}

/// The labels of a page charged more than [`SHAPE_BUDGET`] and less than twice
/// it, drawn on a 512 × 512 canvas by [`draw_label`].
pub(crate) fn labels() -> Vec<String> {
  (0..120).map(|index| format!("Label {index:03}")).collect()
}

/// Draws the `at`-th of [`labels`] at 64 px, in four columns.
pub(crate) fn draw_label(d: &Context2D, labels: &[String], at: usize) {
  if at == 0 {
    d.set_font(CanvasFont::new("sans-serif", 64.));
    d.set_fill_style("#e5e7eb");
  }
  let (x, y) = ((at % 4) as f32 * 128., 48. + (at / 4) as f32 * 14.);
  d.fill_text(&labels[at], x, y).unwrap();
}

/// Draws `items` with `draw` as one replacement presentation over `batches`
/// encodes, each made by `encode`, which returns the asset bytes it uploaded.
/// The last batch captures the artwork and commits. Returns the bytes uploaded.
pub(crate) fn replacement(
  canvas: &CanvasHandle,
  items: usize,
  batches: usize,
  draw: impl Fn(&Context2D, usize),
  mut encode: impl FnMut() -> u64,
) -> u64 {
  let d = canvas.context_2d();
  let token = d.begin_presentation().unwrap();
  d.reset();
  let chunk = items.div_ceil(batches);
  let mut uploaded = 0;
  for at in 0..items {
    draw(&d, at);
    let last = at + 1 == items;
    if last {
      d.capture_artwork().unwrap();
      d.commit_presentation(token).unwrap();
    }
    if last || (at + 1) % chunk == 0 {
      uploaded += encode();
    }
  }
  uploaded
}

pub(crate) fn pixel(snapshot: &CanvasSnapshot, x: u32, y: u32) -> [u8; 4] {
  let at = ((y * snapshot.width + x) * 4) as usize;
  snapshot.rgba[at..at + 4].try_into().unwrap()
}
