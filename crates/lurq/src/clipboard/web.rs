//! The clipboard of a browser.
//!
//! A page writes the clipboard at once and reads it through a promise, which
//! the browser may ask the person about first. So a copy goes to the system
//! clipboard and is kept here too, and a paste shortcut asks for the system
//! clipboard's text ([`request_paste`]) and gets it a turn of the loop later
//! ([`take_pasted`]); when the browser refuses, the paste is what this page
//! copied last.
//!
//! Only text crosses: a page reads images and files from a `paste` event,
//! which the shortcut's key event has already cancelled.

use std::cell::RefCell;

use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;

use crate::app::window::WindowWaker;

thread_local! {
  /// What this page copied last.
  static COPIED: RefCell<Option<String>> = const { RefCell::new(None) };
  /// The text of a paste the browser has answered, until the shell takes it.
  static PASTED: RefCell<Option<String>> = const { RefCell::new(None) };
  /// Wakes the loop for [`take_pasted`].
  static WAKER: RefCell<Option<WindowWaker>> = const { RefCell::new(None) };
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

/// What this page copied last: the clipboard as far as a page reads it at once.
pub(super) fn copied() -> Option<String> {
  COPIED.with(|copied| copied.borrow().clone())
}

/// The shell's waker, called when a paste's text has arrived.
pub(crate) fn set_waker(waker: WindowWaker) {
  WAKER.with(|slot| *slot.borrow_mut() = Some(waker));
}

/// Asks the browser for the clipboard's text; [`take_pasted`] has it once the browser answers.
pub(crate) fn request_paste() {
  let promise = system().map(|clipboard| clipboard.read_text());
  wasm_bindgen_futures::spawn_local(async move {
    let system = match promise {
      Some(promise) => JsFuture::from(promise)
        .await
        .ok()
        .and_then(|text| text.dyn_into::<js_sys::JsString>().ok())
        .map(String::from),
      None => None,
    };
    let Some(text) = system.or_else(copied) else {
      return;
    };
    PASTED.with(|pasted| *pasted.borrow_mut() = Some(text));
    let waker = WAKER.with(|slot| slot.borrow().clone());
    if let Some(waker) = waker {
      waker();
    }
  });
}

/// The text of a paste the browser has answered since the last call.
pub(crate) fn take_pasted() -> Option<String> {
  PASTED.with(|pasted| pasted.borrow_mut().take())
}
