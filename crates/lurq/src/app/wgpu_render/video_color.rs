//! How the samples of a frame in a `CVPixelBuffer` are colors: the matrix that makes red, green and blue of its gray
//! and its two of color, the range its samples take, and how many bits they have. The decoder writes them on the
//! buffer (its pixel format, `kCVImageBufferYCbCrMatrixKey`); the NV12 shader is told the first two
//! (`shaders/yuv.wgsl`), and the planes are opened as textures of their depth.

use core_foundation_sys::{
  base::{CFEqual, CFRelease, CFTypeRef},
  string::CFStringRef,
};
use core_video_sys::pixel_buffer::{CVPixelBufferGetPixelFormatType, CVPixelBufferRef};

/// The matrix of a frame, as `shaders/yuv.wgsl` numbers it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Matrix {
  Bt709 = 0,
  Bt601 = 1,
  Bt2020 = 2,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct VideoColor {
  pub matrix: Matrix,
  /// The samples take all of their range (`420f`), not gray from 16 to 235 and color from 16 to 240 (`420v`).
  pub full_range: bool,
  /// Ten bits in each sample's sixteen (`x420`, `xf20`), where it is eight.
  pub ten_bit: bool,
}

impl VideoColor {
  /// What the shader is told, in the instance's `opacity.zw`: the matrix, and 1 for the full range.
  pub(super) fn shader(self) -> [f32; 2] {
    [self.matrix as u8 as f32, if self.full_range { 1.0 } else { 0.0 }]
  }
}

const VIDEO_RANGE: u32 = u32::from_be_bytes(*b"420v");
const FULL_RANGE: u32 = u32::from_be_bytes(*b"420f");
const TEN_BIT_VIDEO_RANGE: u32 = u32::from_be_bytes(*b"x420");
const TEN_BIT_FULL_RANGE: u32 = u32::from_be_bytes(*b"xf20");

#[link(name = "CoreVideo", kind = "framework")]
unsafe extern "C" {
  static kCVImageBufferYCbCrMatrixKey: CFStringRef;
  static kCVImageBufferYCbCrMatrix_ITU_R_601_4: CFStringRef;
  static kCVImageBufferYCbCrMatrix_ITU_R_2020: CFStringRef;
  fn CVBufferCopyAttachment(buffer: CVPixelBufferRef, key: CFStringRef, mode: *mut u32) -> CFTypeRef;
}

/// The colors of `buffer`, or `None` where it is not a frame of two planes the renderer draws.
///
/// # Safety
///
/// `buffer` is a pixel buffer that lives.
pub(super) unsafe fn of(buffer: CVPixelBufferRef) -> Option<VideoColor> {
  let (full_range, ten_bit) = match unsafe { CVPixelBufferGetPixelFormatType(buffer) } {
    VIDEO_RANGE => (false, false),
    FULL_RANGE => (true, false),
    TEN_BIT_VIDEO_RANGE => (false, true),
    TEN_BIT_FULL_RANGE => (true, true),
    _ => return None,
  };
  // A frame that says none is of BT.709, as a frame of high definition is.
  let matrix = unsafe {
    let said = CVBufferCopyAttachment(buffer, kCVImageBufferYCbCrMatrixKey, std::ptr::null_mut());
    let is = |name: CFStringRef| !said.is_null() && CFEqual(said, name.cast()) != 0;
    let matrix = if is(kCVImageBufferYCbCrMatrix_ITU_R_601_4) {
      Matrix::Bt601
    } else if is(kCVImageBufferYCbCrMatrix_ITU_R_2020) {
      Matrix::Bt2020
    } else {
      Matrix::Bt709
    };
    if !said.is_null() {
      CFRelease(said);
    }
    matrix
  };
  Some(VideoColor {
    matrix,
    full_range,
    ten_bit,
  })
}
