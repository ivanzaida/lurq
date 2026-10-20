//! Images decoded at runtime from encoded bytes or a file, off the UI
//! thread, kept in a cache keyed by content.
//!
//! [`Image::from_bytes`](crate::components::Image::from_bytes) and
//! [`Image::from_file`](crate::components::Image::from_file) build a node
//! that the runtime resolves through [`decoded_images`]: the first pass asks
//! for a decode and shows the placeholder, a worker thread decodes (PNG,
//! JPEG, the first frame of a GIF or WebP, BMP, TIFF) and downscales to the
//! requested cap, and the next pass shows the image. A chat laid out again
//! finds its thumbnails in the cache and decodes nothing; rows a
//! `VirtualizedList` builds and drops as it scrolls do the same. A broken
//! file is a [`Failed`](ImageLoadState::Failed) state with its error, never a
//! panic.

mod cache;
#[cfg(test)]
mod cache_tests;
mod worker;

use std::{
  fmt,
  hash::{Hash, Hasher},
  path::{Path, PathBuf},
  sync::Arc,
  time::Duration,
};

pub use cache::{DEFAULT_BUDGET_BYTES, DecodedImages};

use crate::images::ImageData;

/// Where the encoded image comes from.
#[derive(Clone, PartialEq, Eq)]
pub enum ImageSource {
  /// Encoded bytes (PNG, JPEG, GIF, WebP, BMP, TIFF).
  Bytes(Arc<[u8]>),
  /// A file to read and decode.
  File(PathBuf),
}

impl fmt::Debug for ImageSource {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::Bytes(bytes) => f.debug_struct("Bytes").field("len", &bytes.len()).finish(),
      Self::File(path) => f.debug_tuple("File").field(path).finish(),
    }
  }
}

/// The content key an image is cached by: the same key means the same
/// pixels. [`of_bytes`](Self::of_bytes) hashes the bytes; an app that keeps
/// a hash of its own (the one it stores an attachment under) passes it with
/// [`ImageKey::new`] and skips the hashing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ImageKey(u64);

impl ImageKey {
  /// A key the caller derived from the content (a stored hash).
  pub const fn new(key: u64) -> Self {
    Self(key)
  }

  /// A key hashed from the encoded bytes.
  pub fn of_bytes(bytes: &[u8]) -> Self {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    bytes.hash(&mut hasher);
    Self(hasher.finish())
  }

  /// A key of a file: its path, size and modification time, so a file
  /// replaced on disk decodes again and an unchanged one does not.
  pub fn of_path(path: &Path) -> Self {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    path.hash(&mut hasher);
    if let Ok(metadata) = std::fs::metadata(path) {
      metadata.len().hash(&mut hasher);
      if let Ok(modified) = metadata.modified()
        && let Ok(since_epoch) = modified.duration_since(std::time::UNIX_EPOCH)
      {
        since_epoch.hash(&mut hasher);
      }
    }
    Self(hasher.finish())
  }

  pub const fn value(self) -> u64 {
    self.0
  }
}

/// A cap on the decoded size: the image is downscaled to fit within it,
/// keeping its aspect ratio, so a thumbnail of a 4K screenshot does not keep
/// its 30 MB of RGBA.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ImageSizeCap {
  pub width: u32,
  pub height: u32,
}

impl ImageSizeCap {
  pub const fn new(width: u32, height: u32) -> Self {
    Self { width, height }
  }
}

/// Why an image did not decode.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImageDecodeError {
  message: String,
}

impl ImageDecodeError {
  pub(crate) fn new(message: impl Into<String>) -> Self {
    Self {
      message: message.into(),
    }
  }

  pub fn message(&self) -> &str {
    &self.message
  }
}

impl fmt::Display for ImageDecodeError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str(&self.message)
  }
}

impl std::error::Error for ImageDecodeError {}

/// Where a requested image is.
#[derive(Clone)]
pub enum ImageLoadState {
  /// A worker is decoding it; the placeholder shows.
  Loading,
  Ready(ImageData),
  Failed(Arc<ImageDecodeError>),
}

impl fmt::Debug for ImageLoadState {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::Loading => f.write_str("Loading"),
      Self::Ready(data) => f
        .debug_struct("Ready")
        .field("width", &data.width())
        .field("height", &data.height())
        .finish(),
      Self::Failed(error) => f.debug_tuple("Failed").field(error).finish(),
    }
  }
}

impl ImageLoadState {
  pub fn is_loading(&self) -> bool {
    matches!(self, Self::Loading)
  }

  pub fn ready(&self) -> Option<&ImageData> {
    match self {
      Self::Ready(data) => Some(data),
      _ => None,
    }
  }

  pub fn error(&self) -> Option<&ImageDecodeError> {
    match self {
      Self::Failed(error) => Some(error),
      _ => None,
    }
  }
}

/// The process-wide cache of decoded images and its workers.
pub fn decoded_images() -> &'static DecodedImages {
  static CACHE: std::sync::OnceLock<DecodedImages> = std::sync::OnceLock::new();
  CACHE.get_or_init(DecodedImages::new)
}

/// Decodes `source` on this thread and downscales it to `cap`: the first
/// frame of an animated image, as RGBA.
pub fn decode_image(source: &ImageSource, cap: Option<ImageSizeCap>) -> Result<ImageData, ImageDecodeError> {
  let file_bytes;
  let bytes: &[u8] = match source {
    ImageSource::Bytes(bytes) => bytes,
    ImageSource::File(path) => {
      file_bytes =
        std::fs::read(path).map_err(|error| ImageDecodeError::new(format!("{}: {error}", path.display())))?;
      &file_bytes
    }
  };
  let decoded = image::load_from_memory(bytes).map_err(|error| ImageDecodeError::new(error.to_string()))?;
  let decoded = match cap {
    Some(cap) if decoded.width() > cap.width || decoded.height() > cap.height => {
      // Downscale to fit, keeping the aspect ratio: never upscale.
      decoded.resize(
        cap.width.max(1),
        cap.height.max(1),
        image::imageops::FilterType::Triangle,
      )
    }
    _ => decoded,
  };
  let rgba = decoded.into_rgba8();
  let (width, height) = (rgba.width(), rgba.height());
  if width == 0 || height == 0 {
    return Err(ImageDecodeError::new("the image has no pixels"));
  }
  Ok(ImageData::from_rgba(rgba.into_raw(), width, height))
}

/// Blocks until no decode is pending, or `timeout` passes: for tests that
/// build an image from bytes and want the next pass to show it.
pub fn wait_for_decoded_images(timeout: Duration) -> bool {
  decoded_images().wait_idle(timeout)
}
