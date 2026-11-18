//! What the system draws over a window, laid over a capture of it.
//!
//! A capture is what the renderer drew. Where the system draws part of the window itself, over the app's frame
//! (macOS's lights, which take the place of the app's own controls: `winit_shell/macos_controls.rs`), that part
//! is not in it, and a screenshot would show an empty place where a person sees the lights. The shell gives the
//! tree a way to have the system draw those parts as pictures (`Tree::set_system_pictures`); a screenshot asks for
//! them as it is taken, and lays them over its pixels once they are read back.

use std::sync::Arc;

use crate::app::render_engine::{CapturedFrame, RenderCaptureTarget};

/// A part of the window the system drew: its pixels, premultiplied RGBA8, and where its top-left pixel is in the
/// window, in physical pixels.
#[derive(Clone, Debug, PartialEq)]
pub struct SystemPicture {
  pub x: i32,
  pub y: i32,
  pub width: u32,
  pub height: u32,
  pub rgba: Vec<u8>,
}

/// What draws the system's parts of a window, asked on the thread the tree runs on as a screenshot is taken.
pub(crate) type SystemPictures = Arc<dyn Fn() -> Vec<SystemPicture> + Send + Sync>;

/// Lays `pictures` over `pixels`, a tight RGBA8 capture `width` x `height` whose top-left pixel is window pixel
/// `origin`: each is drawn over what is under it, as it is drawn over the window.
pub(crate) fn lay_over(pixels: &mut [u8], width: u32, height: u32, origin: (u32, u32), pictures: &[SystemPicture]) {
  for picture in pictures {
    for row in 0..picture.height {
      let y = i64::from(picture.y) + i64::from(row) - i64::from(origin.1);
      if y < 0 || y >= i64::from(height) {
        continue;
      }
      for column in 0..picture.width {
        let x = i64::from(picture.x) + i64::from(column) - i64::from(origin.0);
        if x < 0 || x >= i64::from(width) {
          continue;
        }
        let from = ((row * picture.width + column) * 4) as usize;
        let Some(source) = picture.rgba.get(from..from + 4) else {
          continue;
        };
        let alpha = u32::from(source[3]);
        if alpha == 0 {
          continue;
        }
        let to = ((y as u32 * width + x as u32) * 4) as usize;
        let under = &mut pixels[to..to + 4];
        for channel in 0..4 {
          let over = u32::from(source[channel]) + (u32::from(under[channel]) * (255 - alpha) + 127) / 255;
          under[channel] = over.min(255) as u8;
        }
      }
    }
  }
}

/// `target`, with `pictures` laid over the pixels before they reach it. The capture's top-left pixel is window
/// pixel `origin`. Without pictures the target is returned as is.
pub(crate) fn laying_over(
  target: RenderCaptureTarget,
  origin: (u32, u32),
  pictures: Vec<SystemPicture>,
) -> RenderCaptureTarget {
  if pictures.is_empty() {
    return target;
  }
  RenderCaptureTarget::Bytes(Arc::new(move |outcome: Result<CapturedFrame, String>| {
    let outcome = outcome.map(|mut frame| {
      lay_over(&mut frame.rgba, frame.width, frame.height, origin, &pictures);
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
mod tests {
  use super::*;

  #[test]
  fn a_picture_is_drawn_over_the_capture_where_it_is_in_the_window() {
    // A capture 4 x 2 of gray, taken from window pixel (10, 20).
    let mut pixels = vec![100_u8; 4 * 2 * 4];
    let red = [255, 0, 0, 255];
    let half_white = [128, 128, 128, 128];
    let clear = [0, 0, 0, 0];
    // At window (11, 20), 2 x 2; its lower row is under the capture's.
    let picture = SystemPicture {
      x: 11,
      y: 20,
      width: 2,
      height: 2,
      rgba: [red, half_white, clear, red].concat(),
    };
    lay_over(&mut pixels, 4, 2, (10, 20), &[picture]);
    let at = |x: usize, y: usize| pixels[(y * 4 + x) * 4..][..4].to_vec();
    assert_eq!(at(0, 0), vec![100; 4], "outside the picture");
    assert_eq!(at(1, 0), red.to_vec(), "opaque");
    assert_eq!(at(2, 0), vec![178, 178, 178, 178], "half over gray");
    assert_eq!(at(1, 1), vec![100; 4], "clear");
    assert_eq!(at(2, 1), red.to_vec());
  }

  #[test]
  fn a_picture_past_the_capture_is_cut_to_it() {
    let mut pixels = vec![0_u8; 2 * 2 * 4];
    let picture = SystemPicture {
      x: -1,
      y: 1,
      width: 4,
      height: 4,
      rgba: [255_u8; 4 * 4 * 4].to_vec(),
    };
    lay_over(&mut pixels, 2, 2, (0, 0), &[picture]);
    assert_eq!(&pixels[..8], &[0; 8], "the row above the picture");
    assert_eq!(&pixels[8..], &[255; 8]);
  }
}
