//! What a page loads, through the browser's `fetch`: an address, and a file or a resource of the app, which in a
//! browser are the page's own files at that path beside it.
//!
//! The browser decides what a page may read. An address on another server is read only where that server allows
//! the page's origin (CORS); where it does not, the browser refuses, and says why in its console alone: the load
//! fails with [`LoadError::Io`] and a sentence that names both causes. An app that needs such an address reads it
//! through a server of its own ([`Loader::proxy`](super::Loader::proxy)).

use std::{cell::Cell, rc::Rc, sync::Arc, time::Duration};

use js_sys::{Reflect, Uint8Array};
use wasm_bindgen::{JsCast, JsValue, closure::Closure};
use wasm_bindgen_futures::JsFuture;

use super::LoadError;

/// Why a fetch the browser refused failed: it does not tell the page which.
const REFUSED: &str =
  "the browser did not fetch it: the network failed, or the server does not let this page read it (CORS)";

/// Reads all of what is at `address`, or its first `max_bytes` where `cut`, telling `told` how much came of how
/// much: a `false` from it stops the read.
pub(super) async fn fetch(
  address: &str,
  headers: &[(Arc<str>, Arc<str>)],
  timeout: Duration,
  max_bytes: u64,
  cut: bool,
  mut told: impl FnMut(u64, Option<u64>) -> bool,
) -> Result<Vec<u8>, LoadError> {
  let window = web_sys::window().ok_or(LoadError::Unsupported("there is no page to fetch from"))?;
  let init = web_sys::RequestInit::new();
  let sent = web_sys::Headers::new().map_err(said)?;
  for (name, value) in headers {
    // A header the browser keeps to itself (`User-Agent`, `Cookie`) is not sent, and is no error.
    let _ = sent.set(name, value);
  }
  init.set_headers(&sent);
  // The time the loader waits: the request is aborted when it has passed.
  let abort = web_sys::AbortController::new().map_err(said)?;
  init.set_signal(Some(&abort.signal()));
  let timed_out = Rc::new(Cell::new(false));
  let on_time = {
    let (abort, timed_out) = (abort.clone(), timed_out.clone());
    Closure::once(move || {
      timed_out.set(true);
      abort.abort();
    })
  };
  let millis = i32::try_from(timeout.as_millis()).unwrap_or(i32::MAX);
  let timer = window
    .set_timeout_with_callback_and_timeout_and_arguments_0(on_time.as_ref().unchecked_ref(), millis)
    .ok();
  let read = read(&window, address, &init, max_bytes, cut, &mut told).await;
  if let Some(timer) = timer {
    window.clear_timeout_with_handle(timer);
  }
  drop(on_time);
  match read {
    Err(_) if timed_out.get() => Err(LoadError::TimedOut),
    read => read,
  }
}

async fn read(
  window: &web_sys::Window,
  address: &str,
  init: &web_sys::RequestInit,
  max_bytes: u64,
  cut: bool,
  told: &mut impl FnMut(u64, Option<u64>) -> bool,
) -> Result<Vec<u8>, LoadError> {
  let response: web_sys::Response = JsFuture::from(window.fetch_with_str_and_init(address, init))
    .await
    .map_err(|_| LoadError::Io(REFUSED.to_owned()))?
    .dyn_into()
    .map_err(said)?;
  match response.status() {
    404 | 410 => return Err(LoadError::NotFound),
    status if !(200..300).contains(&status) => return Err(LoadError::Status(status)),
    _ => {}
  }
  let total = response
    .headers()
    .get("content-length")
    .ok()
    .flatten()
    .and_then(|length| length.parse::<u64>().ok());
  if !cut && total.is_some_and(|total| total > max_bytes) {
    return Err(LoadError::TooLarge { limit: max_bytes });
  }
  let Some(body) = response.body() else {
    return Ok(Vec::new());
  };
  let reader: web_sys::ReadableStreamDefaultReader = body.get_reader().unchecked_into();
  let mut bytes = Vec::new();
  loop {
    let chunk = JsFuture::from(reader.read())
      .await
      .map_err(|_| LoadError::Io(REFUSED.to_owned()))?;
    let done = Reflect::get(&chunk, &JsValue::from_str("done"))
      .ok()
      .and_then(|done| done.as_bool())
      .unwrap_or(true);
    if done {
      break;
    }
    let value: Uint8Array = Reflect::get(&chunk, &JsValue::from_str("value"))
      .map_err(said)?
      .dyn_into()
      .map_err(said)?;
    let room = usize::try_from(max_bytes.saturating_sub(bytes.len() as u64)).unwrap_or(usize::MAX);
    let length = value.length() as usize;
    let taken = length.min(room);
    let start = bytes.len();
    bytes.resize(start + taken, 0);
    value.subarray(0, taken as u32).copy_to(&mut bytes[start..]);
    if taken < length {
      let _ = reader.cancel();
      if cut {
        break;
      }
      return Err(LoadError::TooLarge { limit: max_bytes });
    }
    if !told(bytes.len() as u64, total) {
      let _ = reader.cancel();
      return Err(LoadError::Cancelled);
    }
    if cut && bytes.len() as u64 >= max_bytes {
      let _ = reader.cancel();
      break;
    }
  }
  Ok(bytes)
}

/// What the browser threw, in its own words: for a log.
fn said(error: JsValue) -> LoadError {
  LoadError::Io(
    error
      .as_string()
      .or_else(|| {
        error
          .dyn_ref::<js_sys::Error>()
          .map(|error| String::from(error.message()))
      })
      .unwrap_or_else(|| "the browser failed".to_owned()),
  )
}

/// The address of a file or a resource in a browser: its path, with the page's separators, which the browser
/// resolves against the page's own address.
pub(super) fn address_of(path: &std::path::Path) -> String {
  path.to_string_lossy().replace('\\', "/")
}
