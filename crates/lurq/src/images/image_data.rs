use std::{
  io::Cursor,
  sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
  },
  time::Instant,
};

use image::{AnimationDecoder, ImageFormat, codecs};
use parking_lot::{Mutex, RwLock};

pub enum ImageKind {
  Bytes(ImageData),
  #[cfg(feature = "resources")]
  Resource(Arc<str>),
}

#[cfg(feature = "resources")]
impl From<&str> for ImageKind {
  fn from(value: &str) -> Self {
    Self::Resource(Arc::from(value))
  }
}

#[cfg(feature = "resources")]
impl From<String> for ImageKind {
  fn from(value: String) -> Self {
    Self::Resource(Arc::from(value))
  }
}

#[cfg(feature = "resources")]
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

static NEXT_IMAGE_ID: AtomicU64 = AtomicU64::new(1);
const MIN_ANIMATION_FRAME_MS: u64 = 10;
const MAX_STREAMING_RECYCLED_BUFFERS: usize = 3;

#[derive(Clone)]
pub struct ImageData {
  id: u64,
  width: u32,
  height: u32,
  frames: Arc<Vec<ImageFrame>>,
  total_duration_ms: u64,
  started_at: Instant,
  streaming: Option<Arc<StreamingImageInner>>,
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
}

#[derive(Clone)]
pub struct StreamingImage {
  image: ImageData,
}

pub(crate) struct ImageFrameData {
  pub data: Arc<Vec<u8>>,
  pub width: u32,
  pub height: u32,
  pub frame_index: usize,
  pub version: u64,
}

impl ImageData {
  pub fn from_rgba(data: Vec<u8>, width: u32, height: u32) -> Self {
    assert_eq!(data.len(), (width * height * 4) as usize);
    Self {
      id: NEXT_IMAGE_ID.fetch_add(1, Ordering::Relaxed),
      width,
      height,
      frames: Arc::new(vec![ImageFrame {
        data: Arc::new(data),
        duration_ms: 0,
      }]),
      total_duration_ms: 0,
      started_at: Instant::now(),
      streaming: None,
    }
  }

  pub fn streaming_rgba(data: Vec<u8>, width: u32, height: u32) -> Self {
    assert_eq!(data.len(), (width * height * 4) as usize);
    let data = Arc::new(data);
    Self {
      id: NEXT_IMAGE_ID.fetch_add(1, Ordering::Relaxed),
      width,
      height,
      frames: Arc::new(Vec::new()),
      total_duration_ms: 0,
      started_at: Instant::now(),
      streaming: Some(Arc::new(StreamingImageInner {
        data: RwLock::new(data),
        recycled: Mutex::new(Vec::new()),
        version: AtomicU64::new(0),
      })),
    }
  }

  pub fn from_bytes(bytes: &[u8]) -> Result<Self, image::ImageError> {
    match image::guess_format(bytes)? {
      ImageFormat::Gif => Self::from_gif_bytes(bytes),
      ImageFormat::WebP => Self::from_webp_bytes(bytes),
      _ => Self::from_static_bytes(bytes),
    }
  }

  fn from_static_bytes(bytes: &[u8]) -> Result<Self, image::ImageError> {
    let img = image::load_from_memory(bytes)?.into_rgba8();
    let width = img.width();
    let height = img.height();
    Ok(Self::from_rgba(img.into_raw(), width, height))
  }

  fn from_gif_bytes(bytes: &[u8]) -> Result<Self, image::ImageError> {
    let decoder = codecs::gif::GifDecoder::new(Cursor::new(bytes))?;
    let frames = decoder.into_frames().collect_frames()?;
    Self::from_animation_frames(frames).or_else(|_| Self::from_static_bytes(bytes))
  }

  fn from_webp_bytes(bytes: &[u8]) -> Result<Self, image::ImageError> {
    let decoder = codecs::webp::WebPDecoder::new(Cursor::new(bytes))?;
    if !decoder.has_animation() {
      return Self::from_static_bytes(bytes);
    }
    let frames = decoder.into_frames().collect_frames()?;
    Self::from_animation_frames(frames).or_else(|_| Self::from_static_bytes(bytes))
  }

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
    let frames = frames
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

    Ok(Self {
      id: NEXT_IMAGE_ID.fetch_add(1, Ordering::Relaxed),
      width,
      height,
      frames: Arc::new(frames),
      total_duration_ms,
      started_at: Instant::now(),
      streaming: None,
    })
  }

  pub fn from_file(path: impl AsRef<std::path::Path>) -> Result<Self, image::ImageError> {
    let bytes = std::fs::read(path).map_err(image::ImageError::IoError)?;
    Self::from_bytes(&bytes)
  }

  pub fn id(&self) -> u64 {
    self.id
  }

  pub fn data(&self) -> &[u8] {
    assert!(
      self.streaming.is_none(),
      "streaming images expose their latest pixels through data_arc()"
    );
    &self.frames[0].data
  }

  pub fn data_arc(&self) -> Arc<Vec<u8>> {
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

  pub fn is_animated(&self) -> bool {
    self.frames.len() > 1 && self.total_duration_ms > 0
  }

  pub fn is_streaming(&self) -> bool {
    self.streaming.is_some()
  }

  pub fn version(&self) -> u64 {
    self
      .streaming
      .as_ref()
      .map_or(0, |streaming| streaming.version.load(Ordering::Acquire))
  }

  pub fn set_streaming_rgba(&self, data: Vec<u8>) {
    assert_eq!(data.len(), (self.width * self.height * 4) as usize);
    let Some(streaming) = &self.streaming else {
      panic!("set_streaming_rgba requires ImageData::streaming_rgba");
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
    let Some(streaming) = &self.streaming else {
      panic!("take_streaming_rgba_buffer requires ImageData::streaming_rgba");
    };
    let expected_len = (self.width * self.height * 4) as usize;
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
    let mut data = streaming.data.read().as_ref().clone();
    update(&mut data);
    assert_eq!(data.len(), (self.width * self.height * 4) as usize);
    *streaming.data.write() = Arc::new(data);
    streaming.version.fetch_add(1, Ordering::Release);
  }

  pub(crate) fn frame_at(&self, now: Instant) -> ImageFrameData {
    if let Some(streaming) = &self.streaming {
      return ImageFrameData {
        data: Arc::clone(&streaming.data.read()),
        width: self.width,
        height: self.height,
        frame_index: 0,
        version: streaming.version.load(Ordering::Acquire),
      };
    }

    let frame_index = if self.is_animated() {
      let elapsed_ms = now
        .saturating_duration_since(self.started_at)
        .as_millis()
        .min(u128::from(u64::MAX)) as u64;
      let position_ms = elapsed_ms % self.total_duration_ms;
      let mut cursor_ms = 0_u64;
      self
        .frames
        .iter()
        .position(|frame| {
          cursor_ms += frame.duration_ms;
          position_ms < cursor_ms
        })
        .unwrap_or(0)
    } else {
      0
    };
    let frame = &self.frames[frame_index];

    ImageFrameData {
      data: Arc::clone(&frame.data),
      width: self.width,
      height: self.height,
      frame_index,
      version: frame_index as u64,
    }
  }
}

impl StreamingImage {
  pub fn new_rgba(data: Vec<u8>, width: u32, height: u32) -> Self {
    Self {
      image: ImageData::streaming_rgba(data, width, height),
    }
  }

  pub fn image_data(&self) -> ImageData {
    self.image.clone()
  }

  pub fn set_rgba(&self, data: Vec<u8>) {
    self.image.set_streaming_rgba(data);
  }

  pub fn take_rgba_buffer(&self) -> Option<Vec<u8>> {
    self.image.take_streaming_rgba_buffer()
  }

  pub fn update_rgba(&self, update: impl FnOnce(&mut [u8])) {
    self.image.update_streaming_rgba(update);
  }

  pub fn version(&self) -> u64 {
    self.image.version()
  }
}

fn delay_ms(delay: image::Delay) -> u64 {
  let (numerator, denominator) = delay.numer_denom_ms();
  if denominator == 0 {
    return MIN_ANIMATION_FRAME_MS;
  }
  let ms = u64::from(numerator).div_ceil(u64::from(denominator));
  ms.max(MIN_ANIMATION_FRAME_MS)
}

#[cfg(test)]
mod tests {
  use std::time::Duration;

  use image::{Delay, Frame, Rgba, RgbaImage, codecs::gif::GifEncoder};

  use super::ImageData;

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

    assert!(image.is_animated());
    assert_eq!(image.frame_at(image.started_at).frame_index, 0);
    assert_eq!(
      image.frame_at(image.started_at + Duration::from_millis(25)).frame_index,
      1
    );
    assert_eq!(image.id(), image_id);
  }

  #[test]
  fn streaming_rgba_updates_shared_clones_with_stable_image_id() {
    let image = ImageData::streaming_rgba(vec![0, 0, 0, 255], 1, 1);
    let clone = image.clone();
    let image_id = image.id();

    let initial = clone.frame_at(image.started_at);
    assert_eq!(&initial.data[..], &[0, 0, 0, 255]);
    assert_eq!(initial.version, 0);

    image.set_streaming_rgba(vec![255, 0, 0, 255]);
    let updated = clone.frame_at(image.started_at);

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

    let frame = image.frame_at(image.started_at);
    assert_eq!(&frame.data[..], &[0, 128, 0, 255]);
    assert_eq!(frame.version, 1);
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
    let held = image.frame_at(image.started_at).data;
    image.set_streaming_rgba(vec![0, 255, 0, 255]);

    assert!(image.take_streaming_rgba_buffer().is_some());
    assert!(image.take_streaming_rgba_buffer().is_none());
    drop(held);

    assert!(image.take_streaming_rgba_buffer().is_some());
  }
}
