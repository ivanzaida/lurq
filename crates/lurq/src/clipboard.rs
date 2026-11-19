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
//!
//! # In a browser
//!
//! A page writes the clipboard at once and reads it only through a promise.
//! [`copy_to_clipboard`] writes the text to the system clipboard; the reading
//! functions return what this page copied last, not what another program
//! did. A paste shortcut (Ctrl+V) does read the system clipboard: the text
//! arrives a turn of the loop later. Images and files do not cross.

#[cfg(target_arch = "wasm32")]
pub(crate) mod web;
#[cfg(windows)]
mod windows;

use std::path::PathBuf;

pub use crate::app::events::{ClipboardContent, ClipboardImage, ClipboardImageError, ClipboardKinds};

#[cfg(windows)]
pub fn copy_to_clipboard(text: impl AsRef<str>) -> bool {
  windows::copy(text.as_ref())
}

#[cfg(target_arch = "wasm32")]
pub fn copy_to_clipboard(text: impl AsRef<str>) -> bool {
  web::copy(text.as_ref())
}

#[cfg(not(any(windows, target_arch = "wasm32")))]
pub fn copy_to_clipboard(text: impl AsRef<str>) -> bool {
  let Ok(mut clipboard) = arboard::Clipboard::new() else {
    return false;
  };
  clipboard.set_text(text.as_ref().to_owned()).is_ok()
}

pub fn set_clipboard_text(text: impl AsRef<str>) -> bool {
  copy_to_clipboard(text)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn read_from_clipboard() -> Option<String> {
  let mut clipboard = arboard::Clipboard::new().ok()?;
  clipboard.get_text().ok()
}

/// In a browser, the text this page copied last.
#[cfg(target_arch = "wasm32")]
pub fn read_from_clipboard() -> Option<String> {
  web::copied()
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
#[cfg(not(target_arch = "wasm32"))]
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

/// In a browser, the text this page copied last.
#[cfg(target_arch = "wasm32")]
pub fn read_clipboard() -> Option<ClipboardContent> {
  let content = ClipboardContent::empty().with_text(web::copied()?);
  (!content.is_empty()).then_some(content)
}

/// Puts an RGBA image on the clipboard, as other programs paste it: Windows
/// gets a DIB, macOS a TIFF and PNG. Rows are `width * 4` bytes, top to
/// bottom. `false` for a size the bytes do not fill, or a clipboard that
/// cannot be opened. A browser takes it as a PNG.
#[cfg(not(target_arch = "wasm32"))]
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

/// In a browser, a PNG of it through the async Clipboard API; see `clipboard::web`.
#[cfg(target_arch = "wasm32")]
pub fn copy_image_to_clipboard(width: u32, height: u32, rgba: &[u8]) -> bool {
  web::copy_image(width, height, rgba)
}

/// Puts file paths on the clipboard, as Explorer and Finder copy files.
/// Always `false` in a browser, which has no paths.
#[cfg(not(target_arch = "wasm32"))]
pub fn copy_files_to_clipboard(paths: &[PathBuf]) -> bool {
  let Ok(mut clipboard) = arboard::Clipboard::new() else {
    return false;
  };
  clipboard.set().file_list(paths).is_ok()
}

#[cfg(target_arch = "wasm32")]
pub fn copy_files_to_clipboard(_paths: &[PathBuf]) -> bool {
  false
}

#[cfg(not(target_arch = "wasm32"))]
fn clipboard_image(image: arboard::ImageData<'_>) -> Result<ClipboardImage, ClipboardImageError> {
  let width = u32::try_from(image.width).unwrap_or(0);
  let height = u32::try_from(image.height).unwrap_or(0);
  ClipboardImage::from_rgba(width, height, image.bytes.into_owned())
}
