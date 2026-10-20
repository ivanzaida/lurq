//! The OS clipboard: text, images and files.
//!
//! Text goes through [`copy_to_clipboard`] and [`read_from_clipboard`].
//! [`read_clipboard`] reads everything the clipboard holds at once, as a
//! paste delivers it ([`ClipboardContent`]): the text, the image a screenshot
//! tool or a browser put there (Windows `CF_DIBV5`/`CF_DIB`, macOS
//! `public.png`/`public.tiff`, read through `arboard`), and the files Explorer
//! or Finder copied (`CF_HDROP`, file URLs). [`copy_image_to_clipboard`] puts
//! an image there.
//!
//! A synthetic paste (an MCP client, an applied
//! [`SyntheticInput`](crate::app::synthetic_input::SyntheticInput)) never
//! reads this clipboard: it carries its own content or is refused.

#[cfg(windows)]
mod windows;

use std::path::PathBuf;

pub use crate::app::events::{ClipboardContent, ClipboardImage, ClipboardImageError, ClipboardKinds};

#[cfg(windows)]
pub fn copy_to_clipboard(text: impl AsRef<str>) -> bool {
  windows::copy(text.as_ref())
}

#[cfg(not(windows))]
pub fn copy_to_clipboard(text: impl AsRef<str>) -> bool {
  let Ok(mut clipboard) = arboard::Clipboard::new() else {
    return false;
  };
  clipboard.set_text(text.as_ref().to_owned()).is_ok()
}

pub fn set_clipboard_text(text: impl AsRef<str>) -> bool {
  copy_to_clipboard(text)
}

pub fn read_from_clipboard() -> Option<String> {
  let mut clipboard = arboard::Clipboard::new().ok()?;
  clipboard.get_text().ok()
}

pub fn clipboard_text() -> Option<String> {
  read_from_clipboard()
}

pub fn clear_clipboard() -> bool {
  copy_to_clipboard("")
}

/// Everything the clipboard holds: text, an image and files, any of them at
/// once. `None` when the clipboard cannot be opened or holds none of them.
///
/// The image comes as RGBA whatever format the clipboard holds it in;
/// [`ClipboardImage::to_png`] encodes it to store. A text that is empty
/// counts as no text.
pub fn read_clipboard() -> Option<ClipboardContent> {
  let mut clipboard = arboard::Clipboard::new().ok()?;
  let mut content = ClipboardContent::empty();
  if let Ok(text) = clipboard.get_text() {
    content = content.with_text(text);
  }
  if let Ok(image) = clipboard.get_image()
    && let Ok(image) = clipboard_image(image)
  {
    content = content.with_image(image);
  }
  if let Ok(files) = clipboard.get().file_list() {
    content = content.with_files(files);
  }
  (!content.is_empty()).then_some(content)
}

/// Puts an RGBA image on the clipboard, as other programs paste it: Windows
/// gets a DIB, macOS a TIFF and PNG. Rows are `width * 4` bytes, top to
/// bottom. `false` for a size the bytes do not fill, or a clipboard that
/// cannot be opened.
pub fn copy_image_to_clipboard(width: u32, height: u32, rgba: &[u8]) -> bool {
  let Ok(image) = ClipboardImage::from_rgba(width, height, rgba.to_vec()) else {
    return false;
  };
  let Ok(mut clipboard) = arboard::Clipboard::new() else {
    return false;
  };
  clipboard
    .set_image(arboard::ImageData {
      width: image.width() as usize,
      height: image.height() as usize,
      bytes: image.into_rgba().into(),
    })
    .is_ok()
}

/// Puts file paths on the clipboard, as Explorer and Finder copy files.
pub fn copy_files_to_clipboard(paths: &[PathBuf]) -> bool {
  let Ok(mut clipboard) = arboard::Clipboard::new() else {
    return false;
  };
  clipboard.set().file_list(paths).is_ok()
}

fn clipboard_image(image: arboard::ImageData<'_>) -> Result<ClipboardImage, ClipboardImageError> {
  let width = u32::try_from(image.width).unwrap_or(0);
  let height = u32::try_from(image.height).unwrap_or(0);
  ClipboardImage::from_rgba(width, height, image.bytes.into_owned())
}
