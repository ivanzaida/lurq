use std::{
  io::Cursor,
  sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
  },
  time::Instant,
};

use image::{AnimationDecoder, ImageFormat, codecs};

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

#[derive(Clone)]
pub struct ImageData {
  id: u64,
  width: u32,
  height: u32,
  frames: Arc<Vec<ImageFrame>>,
  total_duration_ms: u64,
  started_at: Instant,
}

#[derive(Clone)]
struct ImageFrame {
  data: Arc<Vec<u8>>,
  duration_ms: u64,
}

pub(crate) struct ImageFrameData {
  pub data: Arc<Vec<u8>>,
  pub width: u32,
  pub height: u32,
  pub frame_index: usize,
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
    &self.frames[0].data
  }

  pub fn data_arc(&self) -> Arc<Vec<u8>> {
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

  pub(crate) fn frame_at(&self, now: Instant) -> ImageFrameData {
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
    }
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
}
