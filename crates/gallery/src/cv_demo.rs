//! What stands for a decoder that is not lurq's, on the Video Player page, on macOS and iOS: threads that each write
//! a moving test picture in one of the ways an app's decoder gives a frame, and give it to lurq to draw. A decoder
//! of the system's (VideoToolbox, a camera) gives buffers Metal draws where they are already; one that decodes in
//! memory writes its frame into such a buffer (`MacosCvPixelBuffer::nv12`), as these do, and from the buffer on
//! the frame is lurq's and is not copied. A decoder of software that keeps its three planes in memory gives them as
//! they are (`ImageData::streaming_i420`), and they are sent to the card once a frame.
//!
//! A decoder of buffers writes into three in turn, so that the one that is drawn is not the one that is written.

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

/// How a decoder gives its frames.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
  /// A buffer of two planes, gray and color, 8 bits of the video range, which says no matrix: drawn as BT.709.
  Nv12,
  /// The same, of the full range (`420f`), which says its matrix is BT.601: the picture looks the same.
  FullRange601,
  /// A buffer of one plane of blue, green, red and alpha.
  Bgra,
  /// A buffer of two planes shown once a shared event of Metal says it is written (`show_after`).
  Signaled,
  /// Three planes in memory (I420), sent to the card as they are.
  Planes,
}

/// The decoders of the page, in the order of its table.
pub const KINDS: [Kind; 5] = [Kind::Nv12, Kind::FullRange601, Kind::Bgra, Kind::Signaled, Kind::Planes];

/// The frames of a decoder, as an image to draw. The decoder stops when it is dropped.
pub struct Decoded {
  pub kind: Kind,
  pub image: ImageData,
  stop: Arc<AtomicBool>,
}

impl Drop for Decoded {
  fn drop(&mut self) {
    self.stop.store(true, Ordering::Release);
  }
}

/// How a frame's gray and color are written: the matrix, by the weights of red and of blue, and the range.
#[derive(Clone, Copy)]
struct Coding {
  red: f32,
  blue: f32,
  full: bool,
}

/// BT.709 in the video range: what a frame that says no matrix is drawn by.
const BT709: Coding = Coding {
  red: 0.2126,
  blue: 0.0722,
  full: false,
};

/// BT.601 in the full range.
const BT601_FULL: Coding = Coding {
  red: 0.299,
  blue: 0.114,
  full: true,
};

impl Coding {
  /// The gray, blue-difference and red-difference of a color of red, green and blue from 0 to 1.
  fn of(self, [r, g, b]: [f32; 3]) -> (u8, u8, u8) {
    let gray = self.red * r + (1.0 - self.red - self.blue) * g + self.blue * b;
    let blue = (b - gray) / (2.0 * (1.0 - self.blue));
    let red = (r - gray) / (2.0 * (1.0 - self.red));
    let byte = |value: f32| value.round().clamp(0.0, 255.0) as u8;
    if self.full {
      (
        byte(gray * 255.0),
        byte(128.0 + blue * 255.0),
        byte(128.0 + red * 255.0),
      )
    } else {
      (
        byte(16.0 + gray * 219.0),
        byte(128.0 + blue * 224.0),
        byte(128.0 + red * 224.0),
      )
    }
  }
}

/// White, yellow, cyan, green, magenta, red and blue: the bars of the test picture.
const BARS: [[f32; 3]; 7] = [
  [1.0, 1.0, 1.0],
  [1.0, 1.0, 0.0],
  [0.0, 1.0, 1.0],
  [0.0, 1.0, 0.0],
  [1.0, 0.0, 1.0],
  [1.0, 0.0, 0.0],
  [0.0, 0.0, 1.0],
];

/// Where the bright bar that crosses the picture is at frame `at`: left to right, or right to left for `back`.
fn sweep(at: u32, back: bool) -> usize {
  let width = SIZE.0 as usize;
  let on = (at as usize * 4) % width;
  if back { width - 1 - on } else { on }
}

/// The test picture at frame `at`, written by `coding`: bars of color with a white bar that goes across them, in
/// the two planes of a buffer.
fn paint(
  at: u32,
  back: bool,
  coding: Coding,
  gray: &mut [u8],
  gray_stride: usize,
  color: &mut [u8],
  color_stride: usize,
) {
  let bars = BARS.map(|bar| coding.of(bar));
  let white = coding.of([1.0; 3]);
  let (width, height) = (SIZE.0 as usize, SIZE.1 as usize);
  let sweep = sweep(at, back);
  for row in 0..height {
    let line = &mut gray[row * gray_stride..row * gray_stride + width];
    for (x, pixel) in line.iter_mut().enumerate() {
      let near = x.abs_diff(sweep) < 6;
      *pixel = if near { white.0 } else { bars[x * bars.len() / width].0 };
    }
  }
  for row in 0..height / 2 {
    let line = &mut color[row * color_stride..row * color_stride + width];
    for (x, pair) in line.chunks_exact_mut(2).enumerate() {
      let x = x * 2;
      let near = x.abs_diff(sweep) < 6;
      let (_, blue, red) = if near { white } else { bars[x * bars.len() / width] };
      (pair[0], pair[1]) = (blue, red);
    }
  }
}

/// The test picture at frame `at` in three planes of memory, gray then the two of color apart: as `paint`, laid out
/// as a decoder of software lays its frame out.
fn paint_planes(at: u32, frame: &mut [u8]) {
  let (width, height) = (SIZE.0 as usize, SIZE.1 as usize);
  let (gray, color) = frame.split_at_mut(width * height);
  let (blue, red) = color.split_at_mut(width * height / 4);
  let mut pairs = vec![0_u8; width * height / 2];
  paint(at, false, BT709, gray, width, &mut pairs, width);
  for (at, pair) in pairs.chunks_exact(2).enumerate() {
    (blue[at], red[at]) = (pair[0], pair[1]);
  }
}

/// A picture of red, green and blue at frame `at`, in rows of blue, green, red and alpha: hues that run across it
/// and move, darker toward the bottom. It is not of bars, so that what an app renders by itself stands apart.
fn paint_bgra(at: u32, rows: &mut [u8], stride: usize) {
  let (width, height) = (SIZE.0 as usize, SIZE.1 as usize);
  for y in 0..height {
    let light = 1.0 - 0.6 * y as f32 / height as f32;
    let line = &mut rows[y * stride..y * stride + width * 4];
    for (x, pixel) in line.chunks_exact_mut(4).enumerate() {
      let hue = ((x + y / 2 + at as usize * 3) % width) as f32 / width as f32 * 6.0;
      let [r, g, b] = hued(hue).map(|value| (value * light * 255.0).round() as u8);
      pixel.copy_from_slice(&[b, g, r, 255]);
    }
  }
}

/// Red, green and blue of the hue `hue`, from 0 to 6 around the wheel, at its fullest.
fn hued(hue: f32) -> [f32; 3] {
  let part = hue.fract();
  match hue as u32 % 6 {
    0 => [1.0, part, 0.0],
    1 => [1.0 - part, 1.0, 0.0],
    2 => [0.0, 1.0, part],
    3 => [0.0, 1.0 - part, 1.0],
    4 => [part, 0.0, 1.0],
    _ => [1.0, 0.0, 1.0 - part],
  }
}

/// A buffer of two planes of the full range (`420f`) that says its matrix is BT.601, which Metal draws where it
/// is: what a camera gives. `None` where the system makes none.
fn full_range_601() -> Option<MacosCvPixelBuffer> {
  use core_foundation_sys::{
    base::{CFRelease, kCFAllocatorDefault},
    dictionary::{CFDictionaryCreate, kCFTypeDictionaryKeyCallBacks, kCFTypeDictionaryValueCallBacks},
    number::kCFBooleanTrue,
    string::CFStringRef,
  };
  use core_video_sys::pixel_buffer::{
    CVPixelBufferCreate, CVPixelBufferRef, kCVPixelBufferIOSurfacePropertiesKey, kCVPixelBufferMetalCompatibilityKey,
  };

  #[link(name = "CoreVideo", kind = "framework")]
  unsafe extern "C" {
    static kCVImageBufferYCbCrMatrixKey: CFStringRef;
    static kCVImageBufferYCbCrMatrix_ITU_R_601_4: CFStringRef;
    fn CVBufferSetAttachment(buffer: CVPixelBufferRef, key: CFStringRef, value: *const std::ffi::c_void, mode: u32);
  }

  // SAFETY: the dictionaries are made of the system's own keys and values, which they retain, and are let go once
  // the buffer is made; the buffer is held by its wrapper alone once it is made.
  unsafe {
    let dictionary = |keys: &[*const std::ffi::c_void], values: &[*const std::ffi::c_void]| {
      CFDictionaryCreate(
        kCFAllocatorDefault,
        keys.as_ptr(),
        values.as_ptr(),
        keys.len() as isize,
        &kCFTypeDictionaryKeyCallBacks,
        &kCFTypeDictionaryValueCallBacks,
      )
    };
    let surface = dictionary(&[], &[]);
    let attributes = dictionary(
      &[
        kCVPixelBufferIOSurfacePropertiesKey.cast(),
        kCVPixelBufferMetalCompatibilityKey.cast(),
      ],
      &[surface.cast(), kCFBooleanTrue.cast()],
    );
    let mut buffer: CVPixelBufferRef = std::ptr::null_mut();
    let status = CVPixelBufferCreate(
      kCFAllocatorDefault.cast(),
      SIZE.0 as usize,
      SIZE.1 as usize,
      u32::from_be_bytes(*b"420f"),
      attributes.cast(),
      &mut buffer,
    );
    CFRelease(attributes.cast());
    CFRelease(surface.cast());
    if status != 0 || buffer.is_null() {
      return None;
    }
    // 1 is kCVAttachmentMode_ShouldPropagate.
    CVBufferSetAttachment(
      buffer,
      kCVImageBufferYCbCrMatrixKey,
      kCVImageBufferYCbCrMatrix_ITU_R_601_4.cast(),
      1,
    );
    let held = MacosCvPixelBuffer::retain(buffer);
    CFRelease(buffer.cast());
    Some(held)
  }
}

/// Starts a decoder of the kind `kind`. `None` where nothing draws such frames yet (the window has no device, or
/// draws another way), or the system gives no buffer.
pub fn start(kind: Kind) -> Option<Decoded> {
  if !cv_pixel_buffer_frames_drawn() {
    return None;
  }
  let stop = Arc::new(AtomicBool::new(false));
  let stopped = stop.clone();
  let image = match kind {
    Kind::Planes => {
      let length = (SIZE.0 * SIZE.1 * 3 / 2) as usize;
      let image = ImageData::streaming_i420(vec![0; length], SIZE.0, SIZE.1);
      let written = image.clone();
      spawn(stopped, move |at| {
        let mut frame = written.take_streaming_i420_buffer().unwrap_or_default();
        frame.resize(length, 0);
        paint_planes(at, &mut frame);
        written.set_streaming_i420(frame);
      })?;
      image
    }
    _ => {
      let buffers: Vec<MacosCvPixelBuffer> = (0..BUFFERS)
        .map(|_| match kind {
          Kind::FullRange601 => full_range_601(),
          Kind::Bgra => MacosCvPixelBuffer::bgra(SIZE.0, SIZE.1),
          _ => MacosCvPixelBuffer::nv12(SIZE.0, SIZE.1),
        })
        .collect::<Option<_>>()?;
      let frames = CvPixelBufferFrames::new();
      let image = NativeImageData::from_cv_pixel_buffer_frames(SIZE.0, SIZE.1, frames.clone()).image_data();
      // The event the app's Metal signals once a frame is written: the gallery writes in memory, and signals it
      // itself once it has, where an app's command buffer would `encodeSignalEvent:value:`.
      let event = match kind {
        Kind::Signaled => {
          use objc2_metal::MTLDevice as _;
          Some(objc2_metal::MTLCreateSystemDefaultDevice()?.newSharedEvent()?)
        }
        _ => None,
      };
      let event = event.map(Shared);
      spawn(stopped, move |at| {
        let buffer = &buffers[at as usize % BUFFERS];
        let written = match kind {
          Kind::Bgra => buffer.write_bgra(|rows, stride| paint_bgra(at, rows, stride)),
          _ => buffer.write_nv12(|gray, gray_stride, color, color_stride| {
            let coding = if kind == Kind::FullRange601 { BT601_FULL } else { BT709 };
            paint(
              at,
              kind == Kind::Signaled,
              coding,
              gray,
              gray_stride,
              color,
              color_stride,
            )
          }),
        };
        if !written {
          return;
        }
        match &event {
          Some(Shared(event)) => {
            use objc2_metal::MTLSharedEvent as _;
            let value = u64::from(at) + 1;
            frames.show_after(buffer.clone(), event, value);
            event.setSignaledValue(value);
          }
          None => frames.show(buffer.clone()),
        }
      })?;
      image
    }
  };
  Some(Decoded { kind, image, stop })
}

/// A shared event of Metal, which any thread signals and reads.
struct Shared(objc2::rc::Retained<objc2::runtime::ProtocolObject<dyn objc2_metal::MTLSharedEvent>>);

// SAFETY: a shared event is Metal's to signal and read from any thread.
unsafe impl Send for Shared {}

/// Runs `frame` with the number of each frame, thirty a second, on a thread of its own until `stop` is set.
fn spawn(stop: Arc<AtomicBool>, mut frame: impl FnMut(u32) + Send + 'static) -> Option<()> {
  std::thread::Builder::new()
    .name("gallery-decoder".to_owned())
    .spawn(move || {
      let mut at = 0_u32;
      while !stop.load(Ordering::Acquire) {
        frame(at);
        at = at.wrapping_add(1);
        std::thread::sleep(FRAME);
      }
    })
    .ok()
    .map(|_| ())
}
