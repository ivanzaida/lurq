#[cfg(feature = "image")]
use std::io::Cursor;
use std::{
  any::Any,
  sync::{
    Arc, OnceLock,
    atomic::{AtomicBool, AtomicU64, Ordering},
  },
  time::Duration,
};

#[cfg(any(target_os = "macos", target_os = "ios"))]
use core_foundation_sys::base::{CFRelease, CFRetain};
#[cfg(any(target_os = "macos", target_os = "ios"))]
use core_video_sys::pixel_buffer::CVPixelBufferRef;
#[cfg(feature = "image")]
use image::{AnimationDecoder, ImageFormat, codecs};
use parking_lot::{Mutex, RwLock};

use crate::time::Instant;

pub enum ImageKind {
  Bytes(ImageData),
  Native(NativeImageData),
  #[cfg(all(feature = "image", feature = "http"))]
  Resource(Arc<str>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImagePixelFormat {
  Rgba8,
  Nv12,
  /// Three planes, as a decoder of software gives them: gray, `width` by `height`, then blue-difference and
  /// red-difference apart, each half as wide and half as high. The `wgpu` renderer draws them as they are; the
  /// `dx12` renderer draws none yet.
  I420,
}

#[cfg(all(feature = "image", feature = "http"))]
impl From<&str> for ImageKind {
  fn from(value: &str) -> Self {
    Self::Resource(Arc::from(value))
  }
}

#[cfg(all(feature = "image", feature = "http"))]
impl From<String> for ImageKind {
  fn from(value: String) -> Self {
    Self::Resource(Arc::from(value))
  }
}

#[cfg(all(feature = "image", feature = "http"))]
impl From<Arc<str>> for ImageKind {
  fn from(value: Arc<str>) -> Self {
    Self::Resource(value)
  }
}

impl From<ImageData> for ImageKind {
  fn from(value: ImageData) -> Self {
    Self::Bytes(value)
  }
}

impl From<NativeImageData> for ImageKind {
  fn from(value: NativeImageData) -> Self {
    Self::Native(value)
  }
}

static NEXT_IMAGE_ID: AtomicU64 = AtomicU64::new(1);
static ANIMATION_EPOCH: OnceLock<Instant> = OnceLock::new();
#[cfg(feature = "image")]
const MIN_ANIMATION_FRAME_MS: u64 = 10;
const MAX_STREAMING_RECYCLED_BUFFERS: usize = 3;

fn animation_epoch() -> Instant {
  *ANIMATION_EPOCH.get_or_init(Instant::now)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativeImageBackend {
  #[cfg(feature = "canvas")]
  Canvas,
  /// An image of a test, which no renderer draws: one that every build has.
  #[cfg(test)]
  Test,
  /// Frames in textures of the graphics card, shared by handle: [`CardFrames`](super::CardFrames).
  #[cfg(windows)]
  CardFrames,
  #[cfg(feature = "wgpu")]
  WgpuExternalRgba,
  #[cfg(any(target_os = "macos", target_os = "ios"))]
  MacosCvPixelBufferNv12,
}

#[derive(Clone)]
pub struct NativeImageData {
  id: u64,
  width: u32,
  height: u32,
  format: ImagePixelFormat,
  backend: NativeImageBackend,
  payload: Arc<dyn Any + Send + Sync>,
  version: Arc<AtomicU64>,
  continuous_redraw: bool,
}

#[cfg(any(target_os = "macos", target_os = "ios"))]
pub struct MacosCvPixelBuffer {
  ptr: CVPixelBufferRef,
}

#[cfg(any(target_os = "macos", target_os = "ios"))]
unsafe impl Send for MacosCvPixelBuffer {}

#[cfg(any(target_os = "macos", target_os = "ios"))]
unsafe impl Sync for MacosCvPixelBuffer {}

#[cfg(any(target_os = "macos", target_os = "ios"))]
impl MacosCvPixelBuffer {
  /// # Safety
  ///
  /// `ptr` must be a valid `CVPixelBufferRef`. This function retains it, and
  /// the wrapper releases it when dropped.
  pub unsafe fn retain(ptr: CVPixelBufferRef) -> Self {
    assert!(!ptr.is_null());
    unsafe {
      CFRetain(ptr.cast());
    }
    Self { ptr }
  }

  pub fn as_ptr(&self) -> CVPixelBufferRef {
    self.ptr
  }

  /// A buffer of two planes, gray and color at half the size, `width` by `height` (an even number each way),
  /// that Metal draws where it is: for a decoder of the app's own that has its frames in memory. The decoder
  /// writes a frame into it ([`write_nv12`](Self::write_nv12)) and shows it
  /// ([`CvPixelBufferFrames::show`]); what the system's decoders give is such a buffer already. `None` where the
  /// system makes none of that size.
  pub fn nv12(width: u32, height: u32) -> Option<Self> {
    if !width.is_multiple_of(2) || !height.is_multiple_of(2) {
      return None;
    }
    Self::made(
      width,
      height,
      core_video_sys::pixel_buffer::kCVPixelFormatType_420YpCbCr8BiPlanarVideoRange,
    )
  }

  /// A buffer of one plane of blue, green, red and alpha (`BGRA`, a byte each, of sRGB), `width` by `height`, that
  /// Metal draws where it is: what an app renders with its own Metal or writes in memory
  /// ([`write_bgra`](Self::write_bgra)), as ScreenCaptureKit gives a screen. `None` where the system makes none of
  /// that size.
  pub fn bgra(width: u32, height: u32) -> Option<Self> {
    Self::made(width, height, u32::from_be_bytes(*b"BGRA"))
  }

  /// A buffer of the system's surface, which Metal makes textures of, in the pixel format `format`.
  fn made(width: u32, height: u32, format: u32) -> Option<Self> {
    use core_foundation_sys::{
      base::kCFAllocatorDefault,
      dictionary::{CFDictionaryCreate, kCFTypeDictionaryKeyCallBacks, kCFTypeDictionaryValueCallBacks},
      number::kCFBooleanTrue,
    };
    use core_video_sys::pixel_buffer::{
      CVPixelBufferCreate, kCVPixelBufferIOSurfacePropertiesKey, kCVPixelBufferMetalCompatibilityKey,
    };
    if width == 0 || height == 0 {
      return None;
    }
    // SAFETY: the dictionaries are made of the system's own keys and values, which they retain, and are let
    // go once the buffer is made; the buffer is this wrapper's from then on.
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
      // A surface of the system's own, which is what Metal makes a texture of.
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
        kCFAllocatorDefault,
        width as usize,
        height as usize,
        format,
        // The binding of CoreVideo names the dictionary by another version of CoreFoundation's: one type.
        attributes.cast(),
        &mut buffer,
      );
      CFRelease(attributes.cast());
      CFRelease(surface.cast());
      (status == 0 && !buffer.is_null()).then_some(Self { ptr: buffer })
    }
  }

  /// Writes a frame into a buffer of one plane of `BGRA`: `write` is given its rows and how many bytes a row is,
  /// at least four for each pixel across. `false` where the buffer is of planes, or could not be held to be written.
  pub fn write_bgra(&self, write: impl FnOnce(&mut [u8], usize)) -> bool {
    use core_video_sys::pixel_buffer::{
      CVPixelBufferGetBaseAddress, CVPixelBufferGetBytesPerRow, CVPixelBufferGetHeight, CVPixelBufferGetPlaneCount,
      CVPixelBufferLockBaseAddress, CVPixelBufferUnlockBaseAddress,
    };
    // SAFETY: the buffer is held while it is written, and holds its rows of `stride` bytes.
    unsafe {
      if CVPixelBufferGetPlaneCount(self.ptr) != 0 || CVPixelBufferLockBaseAddress(self.ptr, 0) != 0 {
        return false;
      }
      let stride = CVPixelBufferGetBytesPerRow(self.ptr);
      let rows = CVPixelBufferGetHeight(self.ptr);
      let start = CVPixelBufferGetBaseAddress(self.ptr).cast::<u8>();
      let written = !start.is_null();
      if written {
        write(std::slice::from_raw_parts_mut(start, stride * rows), stride);
      }
      CVPixelBufferUnlockBaseAddress(self.ptr, 0);
      written
    }
  }

  /// Writes a frame into a buffer of two planes: `write` is given the rows of gray and how many bytes a row is,
  /// then the rows of color (a byte of blue-difference and one of red-difference for each two pixels across
  /// and down) and how many bytes a row of those is. A row is at least as long as the picture is wide.
  /// `false` where the buffer is not of two planes, or could not be held to be written.
  pub fn write_nv12(&self, write: impl FnOnce(&mut [u8], usize, &mut [u8], usize)) -> bool {
    use core_video_sys::pixel_buffer::{
      CVPixelBufferGetBaseAddressOfPlane, CVPixelBufferGetBytesPerRowOfPlane, CVPixelBufferGetHeightOfPlane,
      CVPixelBufferGetPlaneCount, CVPixelBufferLockBaseAddress, CVPixelBufferUnlockBaseAddress,
    };
    // SAFETY: the buffer is held while its planes are written, and each plane holds its rows of `stride` bytes.
    unsafe {
      if CVPixelBufferGetPlaneCount(self.ptr) != 2 || CVPixelBufferLockBaseAddress(self.ptr, 0) != 0 {
        return false;
      }
      let plane = |index: usize| {
        let stride = CVPixelBufferGetBytesPerRowOfPlane(self.ptr, index);
        let rows = CVPixelBufferGetHeightOfPlane(self.ptr, index);
        let start = CVPixelBufferGetBaseAddressOfPlane(self.ptr, index).cast::<u8>();
        (!start.is_null()).then(|| (std::slice::from_raw_parts_mut(start, stride * rows), stride))
      };
      let written = match (plane(0), plane(1)) {
        (Some((gray, gray_stride)), Some((color, color_stride))) => {
          write(gray, gray_stride, color, color_stride);
          true
        }
        _ => false,
      };
      CVPixelBufferUnlockBaseAddress(self.ptr, 0);
      written
    }
  }
}

#[cfg(any(target_os = "macos", target_os = "ios"))]
impl Clone for MacosCvPixelBuffer {
  fn clone(&self) -> Self {
    unsafe { Self::retain(self.ptr) }
  }
}

#[cfg(any(target_os = "macos", target_os = "ios"))]
impl Drop for MacosCvPixelBuffer {
  fn drop(&mut self) {
    unsafe {
      CFRelease(self.ptr.cast());
    }
  }
}

#[cfg(any(target_os = "macos", target_os = "ios"))]
#[derive(Clone)]
pub struct MacosCvPixelBufferNv12Image {
  pub pixel_buffer: MacosCvPixelBuffer,
}

/// The frames of a video that stay on the graphics card, on macOS and iOS: the buffer of pixels the system's
/// decoder gave for the frame to show, two planes of gray and of color, which the `wgpu` renderer draws on Metal
/// where it is. What decodes shows each frame as it comes ([`show`](Self::show)); the image of these frames
/// ([`NativeImageData::from_cv_pixel_buffer_frames`]) is drawn again by itself.
///
/// The buffer is one Metal can make a texture of (`kCVPixelBufferMetalCompatibilityKey`), of the kind
/// `420v`. Whether a renderer that draws such frames is there is [`cv_pixel_buffer_frames_drawn`]; where none
/// is, what decodes copies its frames into an image of memory.
#[cfg(any(target_os = "macos", target_os = "ios"))]
#[derive(Clone, Default)]
pub struct CvPixelBufferFrames {
  state: Arc<CvPixelBufferFramesState>,
}

#[cfg(any(target_os = "macos", target_os = "ios"))]
#[derive(Default)]
struct CvPixelBufferFramesState {
  shown: Mutex<Option<MacosCvPixelBuffer>>,
  count: AtomicU64,
  /// Frames the app's card is still writing, oldest first: see [`CvPixelBufferFrames::show_after`].
  #[cfg(feature = "wgpu")]
  waiting: Mutex<std::collections::VecDeque<WaitingFrame>>,
}

/// A frame shown once its event reaches its value.
#[cfg(all(any(target_os = "macos", target_os = "ios"), feature = "wgpu"))]
struct WaitingFrame {
  buffer: MacosCvPixelBuffer,
  event: objc2::rc::Retained<objc2::runtime::ProtocolObject<dyn objc2_metal::MTLSharedEvent>>,
  value: u64,
}

// SAFETY: a shared event is Metal's to signal and read from any thread.
#[cfg(all(any(target_os = "macos", target_os = "ios"), feature = "wgpu"))]
unsafe impl Send for WaitingFrame {}

/// How many frames may wait for their card at once: one more, and the oldest is not shown.
#[cfg(all(any(target_os = "macos", target_os = "ios"), feature = "wgpu"))]
const FRAMES_WAITING: usize = 8;

#[cfg(any(target_os = "macos", target_os = "ios"))]
impl CvPixelBufferFrames {
  pub fn new() -> Self {
    Self::default()
  }

  /// `buffer` is the frame to show from now on. The buffer is whole: a decoder of the system's, or one that wrote
  /// it in memory, gives it so. A frame the app's own card writes is [`show_after`](Self::show_after).
  pub fn show(&self, buffer: MacosCvPixelBuffer) {
    #[cfg(feature = "wgpu")]
    self.state.waiting.lock().clear();
    *self.state.shown.lock() = Some(buffer);
    self.state.count.fetch_add(1, Ordering::AcqRel);
  }

  /// `buffer` is the frame to show once `event` reaches `value`: the app's Metal commands that write it signal the
  /// event when they are done (`encodeSignalEvent:value:`). Nothing waits for it: until then the frame before is
  /// drawn, and the first frame of the window after it is this one. A later frame that is ready first is shown
  /// first, and this one never. The buffer is held, and not written again by the app's pool, until it is drawn no
  /// more.
  #[cfg(feature = "wgpu")]
  pub fn show_after(
    &self,
    buffer: MacosCvPixelBuffer,
    event: &objc2::runtime::ProtocolObject<dyn objc2_metal::MTLSharedEvent>,
    value: u64,
  ) {
    let mut waiting = self.state.waiting.lock();
    if waiting.len() == FRAMES_WAITING {
      waiting.pop_front();
    }
    waiting.push_back(WaitingFrame {
      buffer,
      event: objc2::Message::retain(event),
      value,
    });
  }

  /// Shows the newest frame that waited and is written now, and lets go of those before it.
  fn settle(&self) {
    #[cfg(feature = "wgpu")]
    {
      use objc2_metal::MTLSharedEvent as _;
      let mut waiting = self.state.waiting.lock();
      if let Some(ready) = waiting
        .iter()
        .rposition(|frame| frame.event.signaledValue() >= frame.value)
      {
        let frame = waiting.drain(..=ready).last().expect("one was found");
        drop(waiting);
        *self.state.shown.lock() = Some(frame.buffer);
        self.state.count.fetch_add(1, Ordering::AcqRel);
      }
    }
  }

  /// The frame to show, where one was shown.
  pub fn shown(&self) -> Option<MacosCvPixelBuffer> {
    self.settle();
    self.state.shown.lock().clone()
  }

  /// How many frames were shown: the version of an image of these frames.
  pub(crate) fn count(&self) -> u64 {
    self.settle();
    self.state.count.load(Ordering::Acquire)
  }
}

/// Whether something draws [`CvPixelBufferFrames`] where they are: the `wgpu` renderer, once it draws on Metal.
#[cfg(any(target_os = "macos", target_os = "ios"))]
static CV_PIXEL_BUFFER_FRAMES_DRAWN: AtomicBool = AtomicBool::new(false);

/// Told by a renderer that draws [`CvPixelBufferFrames`] where they are, and by one that no longer does.
#[cfg(any(target_os = "macos", target_os = "ios"))]
pub fn set_cv_pixel_buffer_frames_drawn(drawn: bool) {
  CV_PIXEL_BUFFER_FRAMES_DRAWN.store(drawn, Ordering::Release);
}

/// Whether a renderer draws [`CvPixelBufferFrames`] where they are. Where none does, a decoder copies its frames
/// into an image of memory (`ImageData::streaming_nv12`), which every renderer draws.
#[cfg(any(target_os = "macos", target_os = "ios"))]
pub fn cv_pixel_buffer_frames_drawn() -> bool {
  CV_PIXEL_BUFFER_FRAMES_DRAWN.load(Ordering::Acquire)
}

#[derive(Clone)]
pub struct ImageData {
  id: u64,
  width: u32,
  height: u32,
  format: ImagePixelFormat,
  frames: Arc<Vec<ImageFrame>>,
  animation_frame_data: Option<Arc<Vec<Arc<Vec<u8>>>>>,
  total_duration_ms: u64,
  streaming: Option<Arc<StreamingImageInner>>,
  native: Option<NativeImageData>,
}

#[derive(Clone)]
struct ImageFrame {
  data: Arc<Vec<u8>>,
  duration_ms: u64,
}

struct StreamingImageInner {
  data: RwLock<Arc<Vec<u8>>>,
  recycled: Mutex<Vec<Arc<Vec<u8>>>>,
  version: AtomicU64,
  continuous_redraw: AtomicBool,
}

#[derive(Clone)]
pub struct StreamingImage {
  image: ImageData,
}

pub(crate) struct ImageFrameData {
  pub data: Arc<Vec<u8>>,
  pub animation_frames: Option<Arc<Vec<Arc<Vec<u8>>>>>,
  pub native: Option<NativeImageData>,
  pub width: u32,
  pub height: u32,
  pub format: ImagePixelFormat,
  pub frame_index: usize,
  pub version: u64,
  pub next_frame_at: Option<Instant>,
}

impl ImageData {
  pub fn from_rgba(data: Vec<u8>, width: u32, height: u32) -> Self {
    assert_eq!(data.len(), (width * height * 4) as usize);
    Self {
      id: NEXT_IMAGE_ID.fetch_add(1, Ordering::Relaxed),
      width,
      height,
      format: ImagePixelFormat::Rgba8,
      frames: Arc::new(vec![ImageFrame {
        data: Arc::new(data),
        duration_ms: 0,
      }]),
      animation_frame_data: None,
      total_duration_ms: 0,
      streaming: None,
      native: None,
    }
  }

  pub fn streaming_rgba(data: Vec<u8>, width: u32, height: u32) -> Self {
    assert_eq!(data.len(), (width * height * 4) as usize);
    Self::streaming(data, width, height, ImagePixelFormat::Rgba8, true)
  }

  pub fn streaming_nv12(data: Vec<u8>, width: u32, height: u32) -> Self {
    assert_eq!(data.len(), nv12_len(width, height));
    Self::streaming(data, width, height, ImagePixelFormat::Nv12, true)
  }

  /// A picture that changes, of three planes (see [`ImagePixelFormat::I420`]): `data` is the gray, then the
  /// blue-difference, then the red-difference, each row as long as its plane is wide. `width` and `height` are even.
  pub fn streaming_i420(data: Vec<u8>, width: u32, height: u32) -> Self {
    assert_eq!(data.len(), nv12_len(width, height));
    Self::streaming(data, width, height, ImagePixelFormat::I420, true)
  }

  fn streaming(data: Vec<u8>, width: u32, height: u32, format: ImagePixelFormat, continuous_redraw: bool) -> Self {
    let data = Arc::new(data);
    Self {
      id: NEXT_IMAGE_ID.fetch_add(1, Ordering::Relaxed),
      width,
      height,
      format,
      frames: Arc::new(Vec::new()),
      animation_frame_data: None,
      total_duration_ms: 0,
      streaming: Some(Arc::new(StreamingImageInner {
        data: RwLock::new(data),
        recycled: Mutex::new(Vec::new()),
        version: AtomicU64::new(0),
        continuous_redraw: AtomicBool::new(continuous_redraw),
      })),
      native: None,
    }
  }

  #[cfg(feature = "image")]
  pub fn from_bytes(bytes: &[u8]) -> Result<Self, image::ImageError> {
    match image::guess_format(bytes)? {
      ImageFormat::Gif => Self::from_gif_bytes(bytes),
      ImageFormat::WebP => Self::from_webp_bytes(bytes),
      _ => Self::from_static_bytes(bytes),
    }
  }

  #[cfg(feature = "image")]
  fn from_static_bytes(bytes: &[u8]) -> Result<Self, image::ImageError> {
    let img = image::load_from_memory(bytes)?.into_rgba8();
    let width = img.width();
    let height = img.height();
    Ok(Self::from_rgba(img.into_raw(), width, height))
  }

  #[cfg(feature = "image")]
  fn from_gif_bytes(bytes: &[u8]) -> Result<Self, image::ImageError> {
    let decoder = codecs::gif::GifDecoder::new(Cursor::new(bytes))?;
    let frames = decoder.into_frames().collect_frames()?;
    Self::from_animation_frames(frames).or_else(|_| Self::from_static_bytes(bytes))
  }

  #[cfg(feature = "image")]
  fn from_webp_bytes(bytes: &[u8]) -> Result<Self, image::ImageError> {
    let decoder = codecs::webp::WebPDecoder::new(Cursor::new(bytes))?;
    if !decoder.has_animation() {
      return Self::from_static_bytes(bytes);
    }
    let frames = decoder.into_frames().collect_frames()?;
    Self::from_animation_frames(frames).or_else(|_| Self::from_static_bytes(bytes))
  }

  #[cfg(feature = "image")]
  fn from_animation_frames(frames: Vec<image::Frame>) -> Result<Self, image::ImageError> {
    let Some(first) = frames.first() else {
      return Err(image::ImageError::Decoding(image::error::DecodingError::new(
        image::error::ImageFormatHint::Unknown,
        "animated image had no frames",
      )));
    };

    let width = first.buffer().width();
    let height = first.buffer().height();
    if frames.len() <= 1 {
      return Ok(Self::from_rgba(first.buffer().clone().into_raw(), width, height));
    }

    let mut total_duration_ms = 0_u64;
    let frames: Vec<_> = frames
      .into_iter()
      .map(|frame| {
        let duration_ms = delay_ms(frame.delay());
        total_duration_ms += duration_ms;
        ImageFrame {
          data: Arc::new(frame.into_buffer().into_raw()),
          duration_ms,
        }
      })
      .collect();
    let animation_frame_data = Arc::new(frames.iter().map(|frame| Arc::clone(&frame.data)).collect());

    Ok(Self {
      id: NEXT_IMAGE_ID.fetch_add(1, Ordering::Relaxed),
      width,
      height,
      format: ImagePixelFormat::Rgba8,
      frames: Arc::new(frames),
      animation_frame_data: Some(animation_frame_data),
      total_duration_ms,
      streaming: None,
      native: None,
    })
  }

  #[cfg(feature = "image")]
  pub fn from_file(path: impl AsRef<std::path::Path>) -> Result<Self, image::ImageError> {
    let bytes = std::fs::read(path).map_err(image::ImageError::IoError)?;
    Self::from_bytes(&bytes)
  }

  #[cfg(feature = "canvas")]
  pub(crate) fn canvas_compatible(&self) -> bool {
    self.format == ImagePixelFormat::Rgba8
      && self.streaming.is_none()
      && self.native.is_none()
      && self.frames.len() == 1
      && self.width > 0
      && self.height > 0
  }

  pub fn id(&self) -> u64 {
    self.id
  }

  pub fn data(&self) -> &[u8] {
    assert!(
      self.streaming.is_none() && self.native.is_none(),
      "streaming images expose their latest pixels through data_arc()"
    );
    &self.frames[0].data
  }

  pub fn data_arc(&self) -> Arc<Vec<u8>> {
    if self.native.is_some() {
      return Arc::new(Vec::new());
    }
    if let Some(streaming) = &self.streaming {
      return Arc::clone(&streaming.data.read());
    }
    Arc::clone(&self.frames[0].data)
  }

  pub fn width(&self) -> u32 {
    self.width
  }

  pub fn height(&self) -> u32 {
    self.height
  }

  pub fn format(&self) -> ImagePixelFormat {
    self.format
  }

  pub fn is_animated(&self) -> bool {
    self.frames.len() > 1 && self.total_duration_ms > 0
  }

  /// Whether what is behind the picture shows through it: one pixel in a hundred or more is not opaque, as in a
  /// logo or an icon drawn on nothing. A few that are not (the rounded corners of a photo) do not make it so.
  /// Only pixels in memory of [`Rgba8`](ImagePixelFormat::Rgba8) are looked at, an animation's first frame; a
  /// streaming or a native picture, and one of another format, is taken as opaque. An opaque picture has every
  /// pixel read: what asks again keeps the answer.
  pub fn has_transparency(&self) -> bool {
    if self.format != ImagePixelFormat::Rgba8 || self.streaming.is_some() || self.native.is_some() {
      return false;
    }
    let Some(frame) = self.frames.first() else {
      return false;
    };
    let enough = (frame.data.len() / 4).div_ceil(100).max(1);
    frame
      .data
      .chunks_exact(4)
      .filter(|pixel| pixel[3] < 255)
      .take(enough)
      .count()
      == enough
  }

  pub fn is_streaming(&self) -> bool {
    self.streaming.is_some()
  }

  /// What holds its pixels where they are not in memory: a texture of the graphics card, a pixel buffer of the
  /// system.
  pub fn native(&self) -> Option<&NativeImageData> {
    self.native.as_ref()
  }

  pub fn is_native(&self) -> bool {
    self.native.is_some()
  }

  pub fn requires_continuous_redraw(&self) -> bool {
    self.is_animated()
      || self
        .streaming
        .as_ref()
        .is_some_and(|streaming| streaming.continuous_redraw.load(Ordering::Acquire))
      || self
        .native
        .as_ref()
        .is_some_and(NativeImageData::requires_continuous_redraw)
  }

  pub fn version(&self) -> u64 {
    self.streaming.as_ref().map_or_else(
      || self.native.as_ref().map_or(0, NativeImageData::version),
      |streaming| streaming.version.load(Ordering::Acquire),
    )
  }

  pub fn set_streaming_rgba(&self, data: Vec<u8>) {
    assert_eq!(self.format, ImagePixelFormat::Rgba8);
    assert_eq!(data.len(), (self.width * self.height * 4) as usize);
    self.set_streaming_data(data);
  }

  pub fn set_streaming_nv12(&self, data: Vec<u8>) {
    assert_eq!(self.format, ImagePixelFormat::Nv12);
    assert_eq!(data.len(), nv12_len(self.width, self.height));
    self.set_streaming_data(data);
  }

  /// The next frame of a picture of [`streaming_i420`](Self::streaming_i420), laid out as it is.
  pub fn set_streaming_i420(&self, data: Vec<u8>) {
    assert_eq!(self.format, ImagePixelFormat::I420);
    assert_eq!(data.len(), nv12_len(self.width, self.height));
    self.set_streaming_data(data);
  }

  fn set_streaming_data(&self, data: Vec<u8>) {
    let Some(streaming) = &self.streaming else {
      panic!("set_streaming_data requires a streaming ImageData");
    };
    let old = std::mem::replace(&mut *streaming.data.write(), Arc::new(data));
    let mut recycled = streaming.recycled.lock();
    recycled.push(old);
    if recycled.len() > MAX_STREAMING_RECYCLED_BUFFERS {
      recycled.remove(0);
    }
    streaming.version.fetch_add(1, Ordering::Release);
  }

  pub fn take_streaming_rgba_buffer(&self) -> Option<Vec<u8>> {
    assert_eq!(self.format, ImagePixelFormat::Rgba8);
    self.take_streaming_buffer((self.width * self.height * 4) as usize)
  }

  pub fn take_streaming_nv12_buffer(&self) -> Option<Vec<u8>> {
    assert_eq!(self.format, ImagePixelFormat::Nv12);
    self.take_streaming_buffer(nv12_len(self.width, self.height))
  }

  /// A buffer of a frame before, no longer drawn, to write the next frame of three planes into.
  pub fn take_streaming_i420_buffer(&self) -> Option<Vec<u8>> {
    assert_eq!(self.format, ImagePixelFormat::I420);
    self.take_streaming_buffer(nv12_len(self.width, self.height))
  }

  fn take_streaming_buffer(&self, expected_len: usize) -> Option<Vec<u8>> {
    let Some(streaming) = &self.streaming else {
      panic!("take_streaming_buffer requires a streaming ImageData");
    };
    let mut recycled = streaming.recycled.lock();
    let mut pending = Vec::new();
    while let Some(candidate) = recycled.pop() {
      match Arc::try_unwrap(candidate) {
        Ok(mut data) if data.capacity() >= expected_len => {
          data.clear();
          recycled.extend(pending);
          return Some(data);
        }
        Ok(_) => {}
        Err(candidate) => {
          pending.push(candidate);
        }
      }
    }
    recycled.extend(pending);
    None
  }

  pub fn update_streaming_rgba(&self, update: impl FnOnce(&mut [u8])) {
    let Some(streaming) = &self.streaming else {
      panic!("update_streaming_rgba requires ImageData::streaming_rgba");
    };
    let expected_len = (self.width * self.height * 4) as usize;

    {
      let mut current = streaming.data.write();
      if let Some(data) = Arc::get_mut(&mut *current) {
        assert_eq!(data.len(), expected_len);
        update(data);
        streaming.version.fetch_add(1, Ordering::Release);
        return;
      }
    }

    let current = Arc::clone(&streaming.data.read());
    assert_eq!(current.len(), expected_len);
    let mut data = self
      .take_streaming_rgba_buffer()
      .unwrap_or_else(|| Vec::with_capacity(expected_len));
    data.clear();
    data.extend_from_slice(&current);
    update(&mut data);
    self.set_streaming_rgba(data);
  }

  pub(crate) fn frame_at(&self, now: Instant) -> ImageFrameData {
    if let Some(streaming) = &self.streaming {
      return ImageFrameData {
        data: Arc::clone(&streaming.data.read()),
        animation_frames: None,
        native: None,
        width: self.width,
        height: self.height,
        format: self.format,
        frame_index: 0,
        version: streaming.version.load(Ordering::Acquire),
        next_frame_at: streaming.continuous_redraw.load(Ordering::Acquire).then_some(now),
      };
    }

    if let Some(native) = &self.native {
      return ImageFrameData {
        data: Arc::new(Vec::new()),
        animation_frames: None,
        native: Some(native.clone()),
        width: self.width,
        height: self.height,
        format: self.format,
        frame_index: 0,
        version: native.version(),
        next_frame_at: native.requires_continuous_redraw().then_some(now),
      };
    }

    let mut next_frame_at = None;
    let frame_index = if self.is_animated() {
      let elapsed_ms = now
        .saturating_duration_since(animation_epoch())
        .as_millis()
        .min(u128::from(u64::MAX)) as u64;
      let position_ms = elapsed_ms % self.total_duration_ms;
      let mut cursor_ms = 0_u64;
      let frame_index = self
        .frames
        .iter()
        .position(|frame| {
          cursor_ms += frame.duration_ms;
          position_ms < cursor_ms
        })
        .unwrap_or(0);
      let remaining_ms = cursor_ms.saturating_sub(position_ms);
      next_frame_at = now.checked_add(Duration::from_millis(remaining_ms));
      frame_index
    } else {
      0
    };
    let frame = &self.frames[frame_index];

    ImageFrameData {
      data: Arc::clone(&frame.data),
      animation_frames: self.animation_frame_data.clone(),
      native: None,
      width: self.width,
      height: self.height,
      format: self.format,
      frame_index,
      version: frame_index as u64,
      next_frame_at,
    }
  }
}

impl NativeImageData {
  pub fn new<T: Any + Send + Sync>(
    width: u32,
    height: u32,
    format: ImagePixelFormat,
    backend: NativeImageBackend,
    payload: T,
  ) -> Self {
    Self::new_with_continuous_redraw(width, height, format, backend, payload, false)
  }

  pub fn new_with_continuous_redraw<T: Any + Send + Sync>(
    width: u32,
    height: u32,
    format: ImagePixelFormat,
    backend: NativeImageBackend,
    payload: T,
    continuous_redraw: bool,
  ) -> Self {
    Self {
      id: NEXT_IMAGE_ID.fetch_add(1, Ordering::Relaxed),
      width,
      height,
      format,
      backend,
      payload: Arc::new(payload),
      version: Arc::new(AtomicU64::new(0)),
      continuous_redraw,
    }
  }

  pub fn image_data(&self) -> ImageData {
    ImageData {
      id: self.id,
      width: self.width,
      height: self.height,
      format: self.format,
      frames: Arc::new(Vec::new()),
      animation_frame_data: None,
      total_duration_ms: 0,
      streaming: None,
      native: Some(self.clone()),
    }
  }

  pub fn id(&self) -> u64 {
    self.id
  }

  pub fn width(&self) -> u32 {
    self.width
  }

  pub fn height(&self) -> u32 {
    self.height
  }

  pub fn format(&self) -> ImagePixelFormat {
    self.format
  }

  pub fn backend(&self) -> NativeImageBackend {
    self.backend
  }

  pub fn payload<T: Any + Send + Sync>(&self) -> Option<&T> {
    self.payload.downcast_ref()
  }

  pub fn version(&self) -> u64 {
    #[cfg(feature = "wgpu")]
    if let Some(state) = self.payload::<super::WgpuExternalImageState>() {
      return state.version();
    }

    // An image of the card's frames is another picture with each frame shown: who shows a frame says nothing
    // more.
    #[cfg(windows)]
    if let Some(frames) = self.payload::<super::CardFrames>() {
      return frames.count();
    }

    #[cfg(any(target_os = "macos", target_os = "ios"))]
    if let Some(frames) = self.payload::<CvPixelBufferFrames>() {
      return frames.count();
    }

    self.version.load(Ordering::Acquire)
  }

  pub fn bump_version(&self) {
    #[cfg(feature = "wgpu")]
    if self.payload::<super::WgpuExternalImageState>().is_some() {
      return;
    }

    #[cfg(windows)]
    if self.payload::<super::CardFrames>().is_some() {
      return;
    }

    #[cfg(any(target_os = "macos", target_os = "ios"))]
    if self.payload::<CvPixelBufferFrames>().is_some() {
      return;
    }

    self.version.fetch_add(1, Ordering::Release);
  }

  pub fn requires_continuous_redraw(&self) -> bool {
    self.continuous_redraw
  }
}

#[cfg(windows)]
impl NativeImageData {
  /// An image of `frames`, whose picture is `width` by `height`: drawn again by itself as frames come.
  pub fn from_card_frames(width: u32, height: u32, frames: super::CardFrames) -> Self {
    let format = if frames.is_rgba() {
      ImagePixelFormat::Rgba8
    } else {
      ImagePixelFormat::Nv12
    };
    Self::new_with_continuous_redraw(width, height, format, NativeImageBackend::CardFrames, frames, true)
  }
}

#[cfg(any(target_os = "macos", target_os = "ios"))]
impl NativeImageData {
  pub fn from_macos_cv_pixel_buffer_nv12(width: u32, height: u32, pixel_buffer: MacosCvPixelBuffer) -> Self {
    Self::new_with_continuous_redraw(
      width,
      height,
      ImagePixelFormat::Nv12,
      NativeImageBackend::MacosCvPixelBufferNv12,
      MacosCvPixelBufferNv12Image { pixel_buffer },
      true,
    )
  }

  /// An image of `frames`, whose picture is `width` by `height`: drawn again by itself as frames come.
  pub fn from_cv_pixel_buffer_frames(width: u32, height: u32, frames: CvPixelBufferFrames) -> Self {
    Self::new_with_continuous_redraw(
      width,
      height,
      ImagePixelFormat::Nv12,
      NativeImageBackend::MacosCvPixelBufferNv12,
      frames,
      true,
    )
  }
}

impl StreamingImage {
  pub fn new_rgba(data: Vec<u8>, width: u32, height: u32) -> Self {
    Self {
      image: ImageData::streaming_rgba(data, width, height),
    }
  }

  pub fn new_nv12(data: Vec<u8>, width: u32, height: u32) -> Self {
    Self {
      image: ImageData::streaming_nv12(data, width, height),
    }
  }

  pub fn new_rgba_manual_redraw(data: Vec<u8>, width: u32, height: u32) -> Self {
    Self {
      image: ImageData::streaming(data, width, height, ImagePixelFormat::Rgba8, false),
    }
  }

  pub fn new_nv12_manual_redraw(data: Vec<u8>, width: u32, height: u32) -> Self {
    assert_eq!(data.len(), nv12_len(width, height));
    Self {
      image: ImageData::streaming(data, width, height, ImagePixelFormat::Nv12, false),
    }
  }

  pub fn image_data(&self) -> ImageData {
    self.image.clone()
  }

  pub fn set_rgba(&self, data: Vec<u8>) {
    self.image.set_streaming_rgba(data);
  }

  pub fn set_nv12(&self, data: Vec<u8>) {
    self.image.set_streaming_nv12(data);
  }

  pub fn take_rgba_buffer(&self) -> Option<Vec<u8>> {
    self.image.take_streaming_rgba_buffer()
  }

  pub fn take_nv12_buffer(&self) -> Option<Vec<u8>> {
    self.image.take_streaming_nv12_buffer()
  }

  pub fn update_rgba(&self, update: impl FnOnce(&mut [u8])) {
    self.image.update_streaming_rgba(update);
  }

  pub fn version(&self) -> u64 {
    self.image.version()
  }
}

#[cfg(feature = "image")]
fn delay_ms(delay: image::Delay) -> u64 {
  let (numerator, denominator) = delay.numer_denom_ms();
  if denominator == 0 {
    return MIN_ANIMATION_FRAME_MS;
  }
  let ms = u64::from(numerator).div_ceil(u64::from(denominator));
  ms.max(MIN_ANIMATION_FRAME_MS)
}

/// How many bytes a frame of two planes (NV12) or of three (I420) is: the color is a quarter of the gray, twice.
fn nv12_len(width: u32, height: u32) -> usize {
  assert!(width > 0 && height > 0 && width.is_multiple_of(2) && height.is_multiple_of(2));
  (width * height + width * height / 2) as usize
}

#[cfg(all(test, feature = "image"))]
mod tests {
  use image::{Delay, Frame, Rgba, RgbaImage, codecs::gif::GifEncoder};

  use super::{ImageData, ImagePixelFormat, NativeImageBackend, NativeImageData, StreamingImage, animation_epoch};
  use crate::time::{Duration, Instant};

  #[test]
  fn gif_decoding_preserves_animation_with_stable_image_id() {
    let mut bytes = Vec::new();
    {
      let red = RgbaImage::from_pixel(2, 2, Rgba([255, 0, 0, 255]));
      let blue = RgbaImage::from_pixel(2, 2, Rgba([0, 0, 255, 255]));
      let frames = vec![
        Frame::from_parts(red, 0, 0, Delay::from_numer_denom_ms(20, 1)),
        Frame::from_parts(blue, 0, 0, Delay::from_numer_denom_ms(20, 1)),
      ];
      GifEncoder::new(&mut bytes).encode_frames(frames).unwrap();
    }

    let image = ImageData::from_bytes(&bytes).unwrap();
    let image_id = image.id();
    let epoch = animation_epoch();

    assert!(image.is_animated());
    assert_eq!(image.frame_at(epoch).frame_index, 0);
    assert_eq!(image.frame_at(epoch + Duration::from_millis(25)).frame_index, 1);
    assert_eq!(image.id(), image_id);
  }

  #[test]
  fn animated_frame_reports_next_frame_time() {
    let mut bytes = Vec::new();
    {
      let red = RgbaImage::from_pixel(2, 2, Rgba([255, 0, 0, 255]));
      let blue = RgbaImage::from_pixel(2, 2, Rgba([0, 0, 255, 255]));
      let frames = vec![
        Frame::from_parts(red, 0, 0, Delay::from_numer_denom_ms(20, 1)),
        Frame::from_parts(blue, 0, 0, Delay::from_numer_denom_ms(20, 1)),
      ];
      GifEncoder::new(&mut bytes).encode_frames(frames).unwrap();
    }

    let image = ImageData::from_bytes(&bytes).unwrap();
    let epoch = animation_epoch();
    let frame = image.frame_at(epoch + Duration::from_millis(5));

    assert_eq!(frame.frame_index, 0);
    assert_eq!(frame.next_frame_at, Some(epoch + Duration::from_millis(20)));
  }

  #[test]
  fn separately_decoded_animated_images_share_frame_clock() {
    let mut bytes = Vec::new();
    {
      let red = RgbaImage::from_pixel(2, 2, Rgba([255, 0, 0, 255]));
      let blue = RgbaImage::from_pixel(2, 2, Rgba([0, 0, 255, 255]));
      let frames = vec![
        Frame::from_parts(red, 0, 0, Delay::from_numer_denom_ms(100, 1)),
        Frame::from_parts(blue, 0, 0, Delay::from_numer_denom_ms(100, 1)),
      ];
      GifEncoder::new(&mut bytes).encode_frames(frames).unwrap();
    }

    let image = ImageData::from_bytes(&bytes).unwrap();
    let later_decoded_image = ImageData::from_bytes(&bytes).unwrap();

    let now = animation_epoch() + Duration::from_millis(125);
    let frame = image.frame_at(now);
    let later_decoded_frame = later_decoded_image.frame_at(now);

    assert_eq!(frame.frame_index, 1);
    assert_eq!(later_decoded_frame.frame_index, frame.frame_index);
    assert_eq!(later_decoded_frame.next_frame_at, frame.next_frame_at);
  }

  #[test]
  fn streaming_rgba_updates_shared_clones_with_stable_image_id() {
    let image = ImageData::streaming_rgba(vec![0, 0, 0, 255], 1, 1);
    let clone = image.clone();
    let image_id = image.id();
    let now = Instant::now();

    let initial = clone.frame_at(now);
    assert_eq!(&initial.data[..], &[0, 0, 0, 255]);
    assert_eq!(initial.version, 0);

    image.set_streaming_rgba(vec![255, 0, 0, 255]);
    let updated = clone.frame_at(now);

    assert_eq!(clone.id(), image_id);
    assert_eq!(&updated.data[..], &[255, 0, 0, 255]);
    assert_eq!(updated.version, 1);
  }

  #[test]
  fn streaming_rgba_update_mutates_pixels_and_bumps_version() {
    let image = ImageData::streaming_rgba(vec![0, 0, 0, 255], 1, 1);

    image.update_streaming_rgba(|pixels| {
      pixels[1] = 128;
    });

    let frame = image.frame_at(Instant::now());
    assert_eq!(&frame.data[..], &[0, 128, 0, 255]);
    assert_eq!(frame.version, 1);
  }

  #[test]
  fn streaming_rgba_update_reuses_unique_current_buffer() {
    let image = ImageData::streaming_rgba(vec![0, 0, 0, 255], 1, 1);
    image.set_streaming_rgba(vec![255, 0, 0, 255]);
    let current_ptr = {
      let frame = image.frame_at(Instant::now());
      frame.data.as_ptr()
    };

    image.update_streaming_rgba(|pixels| {
      pixels[1] = 128;
    });

    let frame = image.frame_at(Instant::now());
    assert_eq!(frame.data.as_ptr(), current_ptr);
    assert_eq!(&frame.data[..], &[255, 128, 0, 255]);
    assert_eq!(frame.version, 2);
  }

  #[test]
  fn streaming_rgba_recycles_released_buffers() {
    let image = ImageData::streaming_rgba(vec![0, 0, 0, 255], 1, 1);
    image.set_streaming_rgba(vec![255, 0, 0, 255]);
    image.set_streaming_rgba(vec![0, 255, 0, 255]);

    let recycled = image.take_streaming_rgba_buffer().unwrap();

    assert!(recycled.capacity() >= 4);
    assert!(recycled.is_empty());
  }

  #[test]
  fn streaming_rgba_keeps_buffers_until_renderer_releases_them() {
    let image = ImageData::streaming_rgba(vec![0, 0, 0, 255], 1, 1);
    image.set_streaming_rgba(vec![255, 0, 0, 255]);
    let held = image.frame_at(Instant::now()).data;
    image.set_streaming_rgba(vec![0, 255, 0, 255]);

    assert!(image.take_streaming_rgba_buffer().is_some());
    assert!(image.take_streaming_rgba_buffer().is_none());
    drop(held);

    assert!(image.take_streaming_rgba_buffer().is_some());
  }

  #[test]
  fn manual_redraw_streaming_rgba_does_not_request_continuous_redraw() {
    let image = StreamingImage::new_rgba_manual_redraw(vec![0, 0, 0, 255], 1, 1).image_data();

    assert!(image.is_streaming());
    assert!(!image.requires_continuous_redraw());
  }

  #[test]
  fn continuous_native_image_requests_immediate_redraw() {
    let native =
      NativeImageData::new_with_continuous_redraw(2, 2, ImagePixelFormat::Nv12, NativeImageBackend::Test, (), true);
    let image = native.image_data();
    let now = Instant::now();

    assert!(image.requires_continuous_redraw());
    assert_eq!(image.frame_at(now).next_frame_at, Some(now));
  }
}
