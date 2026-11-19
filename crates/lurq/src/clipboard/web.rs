//! The clipboard of a browser.
//!
//! A page writes the clipboard at once and reads it through a promise, which
//! the browser may ask the person about first. So a copy goes to the system
//! clipboard and is kept here too, and a paste shortcut asks for what the
//! system clipboard holds ([`request_paste`]) and gets it a turn of the loop
//! later ([`take_pasted`]); when the browser refuses, the paste is what this
//! page copied last.
//!
//! A picture crosses as a PNG: a copy writes it with the async Clipboard API
//! ([`copy_image`]), and a paste reads the clipboard's items
//! (`navigator.clipboard.read`, which the browser asks the person about the
//! first time, or answers with a button to press, as Firefox does) for a PNG
//! and for text. Files do not cross: a page reads no path.

use std::cell::RefCell;

use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;

use crate::app::{events::ClipboardImage, window::WindowWaker};

thread_local! {
  /// What this page copied last.
  static COPIED: RefCell<Option<String>> = const { RefCell::new(None) };
  /// What a paste the browser has answered holds, until the shell takes it.
  static PASTED: RefCell<Option<Pasted>> = const { RefCell::new(None) };
  /// Wakes the loop for [`take_pasted`].
  static WAKER: RefCell<Option<WindowWaker>> = const { RefCell::new(None) };
}

/// What a paste holds: the clipboard's text, its picture, or both.
#[derive(Default)]
pub(crate) struct Pasted {
  pub text: Option<String>,
  pub image: Option<ClipboardImage>,
}

/// The page's clipboard, where the browser gives one: a secure context.
fn system() -> Option<web_sys::Clipboard> {
  let clipboard = web_sys::window()?.navigator().clipboard();
  let value: &wasm_bindgen::JsValue = clipboard.as_ref();
  (!value.is_undefined() && !value.is_null()).then_some(clipboard)
}

pub(super) fn copy(text: &str) -> bool {
  COPIED.with(|copied| *copied.borrow_mut() = Some(text.to_owned()));
  if let Some(clipboard) = system() {
    // The browser settles the promise by itself; a refusal leaves the copy this page keeps.
    let promise = clipboard.write_text(text);
    wasm_bindgen_futures::spawn_local(async move {
      let _ = JsFuture::from(promise).await;
    });
  }
  true
}

/// Puts a PNG of the picture on the clipboard. `false` where the page has no clipboard or the picture cannot be
/// encoded; the browser's answer comes later and is not waited for.
pub(super) fn copy_image(width: u32, height: u32, rgba: &[u8]) -> bool {
  let Ok(image) = ClipboardImage::from_rgba(width, height, rgba.to_vec()) else {
    return false;
  };
  let (Ok(png), Some(clipboard)) = (image.to_png(), system()) else {
    return false;
  };
  let parts = js_sys::Array::of1(&js_sys::Uint8Array::from(png.as_slice()));
  let properties = web_sys::BlobPropertyBag::new();
  properties.set_type("image/png");
  let Ok(blob) = web_sys::Blob::new_with_u8_array_sequence_and_options(&parts, &properties) else {
    return false;
  };
  let record = js_sys::Object::new();
  if js_sys::Reflect::set(&record, &"image/png".into(), &blob).is_err() {
    return false;
  }
  let Ok(item) = web_sys::ClipboardItem::new_with_record_from_str_to_blob_promise(&record) else {
    return false;
  };
  let promise = clipboard.write(&js_sys::Array::of1(&item));
  wasm_bindgen_futures::spawn_local(async move {
    let _ = JsFuture::from(promise).await;
  });
  true
}

/// What this page copied last: the clipboard as far as a page reads it at once.
pub(super) fn copied() -> Option<String> {
  COPIED.with(|copied| copied.borrow().clone())
}

/// The shell's waker, called when a paste's text has arrived.
pub(crate) fn set_waker(waker: WindowWaker) {
  WAKER.with(|slot| *slot.borrow_mut() = Some(waker));
}

/// The clipboard's items, as the browser gives them: its picture (a PNG) and its text. `None` where the browser
/// refused, or reads no items.
async fn read_items(clipboard: &web_sys::Clipboard) -> Option<Pasted> {
  let items: js_sys::Array = JsFuture::from(clipboard.read()).await.ok()?.dyn_into().ok()?;
  let mut pasted = Pasted::default();
  for item in items.iter() {
    let Ok(item) = item.dyn_into::<web_sys::ClipboardItem>() else {
      continue;
    };
    let types: Vec<String> = item.types().iter().filter_map(|kind| kind.as_string()).collect();
    if pasted.image.is_none()
      && types.iter().any(|kind| kind == "image/png")
      && let Some(blob) = blob_of(&item, "image/png").await
      && let Ok(buffer) = JsFuture::from(blob.array_buffer()).await
    {
      pasted.image = decode_png(&js_sys::Uint8Array::new(&buffer).to_vec());
    }
    if pasted.text.is_none()
      && types.iter().any(|kind| kind == "text/plain")
      && let Some(blob) = blob_of(&item, "text/plain").await
    {
      pasted.text = JsFuture::from(blob.text()).await.ok().and_then(|text| text.as_string());
    }
  }
  Some(pasted)
}

async fn blob_of(item: &web_sys::ClipboardItem, kind: &str) -> Option<web_sys::Blob> {
  JsFuture::from(item.get_type(kind)).await.ok()?.dyn_into().ok()
}

/// The pixels of a PNG as RGBA, whatever its own colors.
fn decode_png(bytes: &[u8]) -> Option<ClipboardImage> {
  let mut decoder = png::Decoder::new(std::io::Cursor::new(bytes));
  decoder.set_transformations(png::Transformations::normalize_to_color8());
  let mut reader = decoder.read_info().ok()?;
  let mut buffer = vec![0; reader.output_buffer_size()?];
  let info = reader.next_frame(&mut buffer).ok()?;
  let pixels = &buffer[..info.buffer_size()];
  let rgba: Vec<u8> = match info.color_type {
    png::ColorType::Rgba => pixels.to_vec(),
    png::ColorType::Rgb => pixels.chunks_exact(3).flat_map(|p| [p[0], p[1], p[2], 255]).collect(),
    png::ColorType::GrayscaleAlpha => pixels.chunks_exact(2).flat_map(|p| [p[0], p[0], p[0], p[1]]).collect(),
    png::ColorType::Grayscale => pixels.iter().flat_map(|&gray| [gray, gray, gray, 255]).collect(),
    // `normalize_to_color8` expands a palette: none comes here.
    png::ColorType::Indexed => return None,
  };
  ClipboardImage::from_rgba(info.width, info.height, rgba).ok()
}

/// Asks the browser for what the clipboard holds; [`take_pasted`] has it once the browser answers.
pub(crate) fn request_paste() {
  let clipboard = system();
  wasm_bindgen_futures::spawn_local(async move {
    let mut pasted = match &clipboard {
      Some(clipboard) => read_items(clipboard).await.unwrap_or_default(),
      None => Pasted::default(),
    };
    // A browser that reads no items, or was refused them, may still give the text.
    if pasted.text.is_none()
      && pasted.image.is_none()
      && let Some(clipboard) = &clipboard
    {
      pasted.text = JsFuture::from(clipboard.read_text())
        .await
        .ok()
        .and_then(|text| text.dyn_into::<js_sys::JsString>().ok())
        .map(String::from);
    }
    if pasted.text.is_none() && pasted.image.is_none() {
      pasted.text = copied();
    }
    if pasted.text.is_none() && pasted.image.is_none() {
      return;
    }
    PASTED.with(|slot| *slot.borrow_mut() = Some(pasted));
    let waker = WAKER.with(|slot| slot.borrow().clone());
    if let Some(waker) = waker {
      waker();
    }
  });
}

/// What a paste the browser has answered since the last call holds.
pub(crate) fn take_pasted() -> Option<Pasted> {
  PASTED.with(|pasted| pasted.borrow_mut().take())
}
