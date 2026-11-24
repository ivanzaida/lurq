//! The links of an app's own scheme on macOS: `myapp://…`.
//!
//! LaunchServices gives a running app such a link as an Apple Event (`kInternetEventClass`, `kAEGetURL`), not as an
//! argument, both for the link that started it and for one that comes while it runs. Winit hears no such event, and
//! its delegate is its own, so the handler is the shared Apple Event manager's, installed before the event loop
//! runs: the event of the link that started the app is sent while the app finishes launching, after that. The
//! handler only keeps the link, as a callback of AppKit can come while winit has borrowed its handler; the shell
//! gives what it kept to the window's handler of app events (`AppEvent::OpenUrl`) at its next turn, and keeps it
//! until the window has one.

use std::sync::{Mutex, Once};

use objc2::{
  ClassType, class, msg_send,
  rc::Retained,
  runtime::{AnyClass, AnyObject, ClassBuilder, Sel},
  sel,
};
use objc2_foundation::NSObject;

use crate::app::window::AppEvent;

/// `kInternetEventClass` and `kAEGetURL`: both `'GURL'`.
const GET_URL: u32 = u32::from_be_bytes(*b"GURL");
/// `keyDirectObject`: `'----'`, the link itself.
const DIRECT_OBJECT: u32 = u32::from_be_bytes(*b"----");

/// The links the system gave since the shell last asked.
static OPENED: Mutex<Vec<String>> = Mutex::new(Vec::new());

unsafe extern "C-unwind" fn get_url(_: &AnyObject, _: Sel, event: *mut AnyObject, _reply: *mut AnyObject) {
  let Some(event) = (unsafe { event.as_ref() }) else {
    return;
  };
  let descriptor: Option<Retained<AnyObject>> = unsafe { msg_send![event, paramDescriptorForKeyword: DIRECT_OBJECT] };
  let Some(descriptor) = descriptor else {
    return;
  };
  let text: Option<Retained<AnyObject>> = unsafe { msg_send![&*descriptor, stringValue] };
  if let Some(text) = text.and_then(|text| string_of(&text)) {
    OPENED.lock().unwrap().push(text);
  }
}

fn string_of(string: &AnyObject) -> Option<String> {
  let utf8: *const std::ffi::c_char = unsafe { msg_send![string, UTF8String] };
  if utf8.is_null() {
    return None;
  }
  Some(unsafe { std::ffi::CStr::from_ptr(utf8) }.to_string_lossy().into_owned())
}

/// Makes the shared Apple Event manager give the app's links to [`take_opened`]. Once, before the event loop runs.
pub(super) fn install() {
  static INSTALLED: Once = Once::new();
  INSTALLED.call_once(|| {
    let class = AnyClass::get(c"LurqUrlReceiver").unwrap_or_else(|| {
      let mut builder = ClassBuilder::new(c"LurqUrlReceiver", NSObject::class()).expect("a class name of its own");
      unsafe {
        builder.add_method(
          sel!(handleGetURLEvent:withReplyEvent:),
          get_url as unsafe extern "C-unwind" fn(_, _, _, _),
        );
      }
      builder.register()
    });
    let receiver: Retained<AnyObject> = unsafe { msg_send![class, new] };
    let manager: Retained<AnyObject> = unsafe { msg_send![class!(NSAppleEventManager), sharedAppleEventManager] };
    unsafe {
      let _: () = msg_send![
        &*manager,
        setEventHandler: &*receiver,
        andSelector: sel!(handleGetURLEvent:withReplyEvent:),
        forEventClass: GET_URL,
        andEventID: GET_URL
      ];
    }
    // The manager does not keep its handler: the receiver lives as long as the app.
    std::mem::forget(receiver);
  });
}

/// The links the system gave since the last call, in its order, as the app's events.
pub(super) fn take_opened() -> Vec<AppEvent> {
  std::mem::take(&mut *OPENED.lock().unwrap())
    .into_iter()
    .map(AppEvent::OpenUrl)
    .collect()
}
