//! What a clipboard holds, as a paste delivers it: text, an image and files,
//! any of them at once.
//!
//! The types live here, outside the `clipboard` feature, because a paste
//! event carries them and a headless test builds them without an OS
//! clipboard. `lurq::clipboard::read_clipboard` fills them from the OS.

use std::{fmt, path::PathBuf};

/// An image a clipboard holds, as 8-bit RGBA rows without padding.
#[derive(Clone, PartialEq, Eq)]
pub struct ClipboardImage {
  width: u32,
  height: u32,
  rgba: Vec<u8>,
}

/// Why RGBA bytes do not make a [`ClipboardImage`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ClipboardImageError {
  /// A zero width or height.
  EmptySize { width: u32, height: u32 },
  /// `rgba` is not `width * height * 4` bytes long.
  WrongLength { expected: usize, actual: usize },
}

impl fmt::Display for ClipboardImageError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::EmptySize { width, height } => write!(f, "a clipboard image needs a size; got {width}x{height}"),
      Self::WrongLength { expected, actual } => {
        write!(f, "a clipboard image needs {expected} RGBA bytes; got {actual}")
      }
    }
  }
}

impl std::error::Error for ClipboardImageError {}

impl ClipboardImage {
  /// `rgba` holds `width * height` pixels, 4 bytes each, row by row.
  pub fn from_rgba(width: u32, height: u32, rgba: Vec<u8>) -> Result<Self, ClipboardImageError> {
    if width == 0 || height == 0 {
      return Err(ClipboardImageError::EmptySize { width, height });
    }
    let expected = (width as usize) * (height as usize) * 4;
    if rgba.len() != expected {
      return Err(ClipboardImageError::WrongLength {
        expected,
        actual: rgba.len(),
      });
    }
    Ok(Self { width, height, rgba })
  }

  pub fn width(&self) -> u32 {
    self.width
  }

  pub fn height(&self) -> u32 {
    self.height
  }

  /// The pixels, `width * height * 4` bytes, rows top to bottom.
  pub fn rgba(&self) -> &[u8] {
    &self.rgba
  }

  /// The pixels, to keep without a copy.
  pub fn into_rgba(self) -> Vec<u8> {
    self.rgba
  }

  /// The image as PNG bytes, to store it.
  #[cfg(feature = "clipboard")]
  pub fn to_png(&self) -> Result<Vec<u8>, png::EncodingError> {
    let mut out = Vec::new();
    {
      let mut encoder = png::Encoder::new(&mut out, self.width, self.height);
      encoder.set_color(png::ColorType::Rgba);
      encoder.set_depth(png::BitDepth::Eight);
      let mut writer = encoder.write_header()?;
      writer.write_image_data(&self.rgba)?;
    }
    Ok(out)
  }
}

impl fmt::Debug for ClipboardImage {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_struct("ClipboardImage")
      .field("width", &self.width)
      .field("height", &self.height)
      .field("bytes", &self.rgba.len())
      .finish()
  }
}

/// The kinds of content a clipboard holds at once.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ClipboardKinds {
  pub text: bool,
  pub image: bool,
  pub files: bool,
}

impl ClipboardKinds {
  pub fn any(self) -> bool {
    self.text || self.image || self.files
  }
}

/// What a clipboard holds: text, an image and files, any of them at once.
/// Explorer and Finder put copied files on the clipboard; a screenshot tool
/// may put an image and its text form; a browser puts text and its HTML.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ClipboardContent {
  text: Option<String>,
  image: Option<ClipboardImage>,
  files: Vec<PathBuf>,
}

impl ClipboardContent {
  /// Holds nothing.
  pub fn empty() -> Self {
    Self::default()
  }

  pub fn text(text: impl Into<String>) -> Self {
    Self::empty().with_text(text)
  }

  pub fn image(image: ClipboardImage) -> Self {
    Self::empty().with_image(image)
  }

  pub fn files(files: impl IntoIterator<Item = PathBuf>) -> Self {
    Self::empty().with_files(files)
  }

  pub fn with_text(mut self, text: impl Into<String>) -> Self {
    self.text = Some(text.into());
    self
  }

  pub fn with_image(mut self, image: ClipboardImage) -> Self {
    self.image = Some(image);
    self
  }

  pub fn with_files(mut self, files: impl IntoIterator<Item = PathBuf>) -> Self {
    self.files = files.into_iter().collect();
    self
  }

  /// The text, if the clipboard holds some; an empty string counts as none.
  pub fn text_content(&self) -> Option<&str> {
    self.text.as_deref().filter(|text| !text.is_empty())
  }

  pub fn image_content(&self) -> Option<&ClipboardImage> {
    self.image.as_ref()
  }

  /// The files, as Explorer or Finder copied them; empty without any.
  pub fn file_paths(&self) -> &[PathBuf] {
    &self.files
  }

  pub fn kinds(&self) -> ClipboardKinds {
    ClipboardKinds {
      text: self.text_content().is_some(),
      image: self.image.is_some(),
      files: !self.files.is_empty(),
    }
  }

  pub fn is_empty(&self) -> bool {
    !self.kinds().any()
  }

  /// Takes the parts apart: text, image and files.
  pub fn into_parts(self) -> (Option<String>, Option<ClipboardImage>, Vec<PathBuf>) {
    (self.text.filter(|text| !text.is_empty()), self.image, self.files)
  }
}
