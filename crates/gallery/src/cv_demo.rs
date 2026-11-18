//! What stands for a decoder that is not lurq's, on the Video Player page, on macOS and iOS: a thread that writes
//! a moving test picture into buffers Metal draws where they are (`MacosCvPixelBuffer::nv12`) and shows each
//! through `lurq::images::CvPixelBufferFrames`. A decoder of the system's (VideoToolbox, a camera) gives such
//! buffers already; one that decodes in memory writes its frame into one, as this does. From the buffer on the
//! frame is lurq's, and is not copied.
//!
//! It writes into three buffers in turn, so that the one that is drawn is not the one that is written.

use std::{
  sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
  },
  time::Duration,
};

use lurq::images::{CvPixelBufferFrames, ImageData, MacosCvPixelBuffer, NativeImageData, cv_pixel_buffer_frames_drawn};

/// How large its picture is, and how many buffers it writes in turn.
const SIZE: (u32, u32) = (320, 180);
const BUFFERS: usize = 3;
/// How long a frame is shown: thirty a second.
const FRAME: Duration = Duration::from_millis(33);

/// The frames of the decoder, as an image to draw. The decoder stops when it is dropped.
pub struct Decoded {
  pub image: ImageData,
  stop: Arc<AtomicBool>,
}

impl Drop for Decoded {
  fn drop(&mut self) {
    self.stop.store(true, Ordering::Release);
  }
}

/// The test picture at frame `at`: bars of color with a bright bar that goes across them, in the two planes of a
/// buffer.
fn paint(at: u32, gray: &mut [u8], gray_stride: usize, color: &mut [u8], color_stride: usize) {
  // Gray, blue-difference and red-difference of white, yellow, cyan, green, magenta, red and blue, by BT.709 in the
  // video range: what a frame that says no matrix is drawn by.
  const BARS: [(u8, u8, u8); 7] = [
    (235, 128, 128),
    (219, 16, 138),
    (188, 154, 16),
    (173, 42, 26),
    (78, 214, 230),
    (63, 102, 240),
    (32, 240, 118),
  ];
  let (width, height) = (SIZE.0 as usize, SIZE.1 as usize);
  let sweep = (at as usize * 4) % width;
  for row in 0..height {
    let line = &mut gray[row * gray_stride..row * gray_stride + width];
    for (x, pixel) in line.iter_mut().enumerate() {
      let near = x.abs_diff(sweep) < 6;
      *pixel = if near { 235 } else { BARS[x * BARS.len() / width].0 };
    }
  }
  for row in 0..height / 2 {
    let line = &mut color[row * color_stride..row * color_stride + width];
    for (x, pair) in line.chunks_exact_mut(2).enumerate() {
      let x = x * 2;
      let (_, blue, red) = BARS[x * BARS.len() / width];
      let near = x.abs_diff(sweep) < 6;
      (pair[0], pair[1]) = if near { (128, 128) } else { (blue, red) };
    }
  }
}

/// Starts the decoder. `None` where nothing draws such frames yet (the window has no device, or draws another
/// way), or the system gives no buffer.
pub fn start() -> Option<Decoded> {
  if !cv_pixel_buffer_frames_drawn() {
    return None;
  }
  let buffers: Vec<MacosCvPixelBuffer> = (0..BUFFERS)
    .map(|_| MacosCvPixelBuffer::nv12(SIZE.0, SIZE.1))
    .collect::<Option<_>>()?;
  let frames = CvPixelBufferFrames::new();
  let image = NativeImageData::from_cv_pixel_buffer_frames(SIZE.0, SIZE.1, frames.clone()).image_data();
  let stop = Arc::new(AtomicBool::new(false));
  let stopped = stop.clone();
  std::thread::Builder::new()
    .name("gallery-decoder".to_owned())
    .spawn(move || {
      let mut at = 0_u32;
      while !stopped.load(Ordering::Acquire) {
        let buffer = &buffers[at as usize % BUFFERS];
        if buffer.write_nv12(|gray, gray_stride, color, color_stride| paint(at, gray, gray_stride, color, color_stride))
        {
          frames.show(buffer.clone());
        }
        at = at.wrapping_add(1);
        std::thread::sleep(FRAME);
      }
    })
    .ok()?;
  Some(Decoded { image, stop })
}
