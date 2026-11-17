//! Opening an address in the app the system keeps for it: a web page in the browser, `mailto:` in the mail app.
//!
//! [`open_url`] hands the address to the system and returns: what opens it, and when, is the system's. It is what a
//! link that leaves the app does when it is followed (`ui::LinkPreview`, `ui::Link::href`), and an app calls it
//! itself from a handler, or as [`Ctx::open_url`](crate::app::ctx::Ctx::open_url) and
//! [`App::open_url`](crate::app::App::open_url).
//!
//! ```no_run
//! use lurq::app::open_url;
//!
//! if let Err(error) = open_url("https://lurq.dev/docs") {
//!   eprintln!("the page was not opened: {error}");
//! }
//! ```
//!
//! # What each platform does
//!
//! - **Windows**: `ShellExecuteW` with the verb `open`.
//! - **macOS**: `NSWorkspace`'s `openURL:`.
//! - **iOS**: `UIApplication`'s `openURL:options:completionHandler:`.
//! - **Linux and the other Unix systems**: `xdg-open`, which is left to run by itself.
//! - **A browser**: `window.open` in a new tab.
//!
//! Only an address with a scheme is opened (`https:`, `mailto:`, `tel:`), never a path or a `file:` address: a text
//! that came from a stranger does not start a program on the person's machine.
//!
//! A test, or a tool that drives the app, puts a stand-in in place of the system with [`stand_in`]: what would
//! have been opened is given to it, and nothing leaves the app.

use std::{cell::RefCell, fmt, sync::Arc};

/// Why an address was not opened.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OpenUrlError {
  /// It is not an address with a scheme, or it is a `file:` one: nothing was asked of the system.
  NotAnAddress,
  /// The platform opens no address: one this build does not know.
  Unsupported,
  /// The system did not open it, in its own words: for a log.
  Failed(String),
}

impl fmt::Display for OpenUrlError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::NotAnAddress => f.write_str("it is not an address the system opens"),
      Self::Unsupported => f.write_str("this platform opens no address"),
      Self::Failed(reason) => f.write_str(reason),
    }
  }
}

impl std::error::Error for OpenUrlError {}

type Opener = Arc<dyn Fn(&str) + Send + Sync>;

thread_local! {
  /// What is given the addresses opened from this thread in place of the system, where a test put one.
  static STAND_IN: RefCell<Option<Opener>> = const { RefCell::new(None) };
}

/// Puts `opener` in place of the system for the addresses opened from this thread (the UI thread of a test's
/// tree, where every handler runs) until the guard it gives is dropped: [`open_url`] gives it the address and
/// opens nothing.
///
/// ```
/// use std::sync::{Arc, Mutex};
///
/// use lurq::app::open_url;
///
/// let opened = Arc::new(Mutex::new(Vec::new()));
/// let log = opened.clone();
/// let _held = open_url::stand_in(move |url| log.lock().unwrap().push(url.to_owned()));
/// open_url("https://lurq.dev").unwrap();
/// assert_eq!(*opened.lock().unwrap(), ["https://lurq.dev"]);
/// ```
pub fn stand_in(opener: impl Fn(&str) + Send + Sync + 'static) -> StandIn {
  let before = STAND_IN.with(|held| held.borrow_mut().replace(Arc::new(opener)));
  StandIn { before }
}

/// Keeps a [`stand_in`] in place: dropped, the one before it is back.
#[must_use = "the stand-in is taken away when this is dropped"]
pub struct StandIn {
  before: Option<Opener>,
}

impl Drop for StandIn {
  fn drop(&mut self) {
    let before = self.before.take();
    STAND_IN.with(|held| *held.borrow_mut() = before);
  }
}

/// Opens `url` in the app the system keeps for its scheme; see the [module](self).
pub fn open_url(url: &str) -> Result<(), OpenUrlError> {
  let url = url.trim();
  if !has_scheme(url) {
    return Err(OpenUrlError::NotAnAddress);
  }
  if let Some(opener) = STAND_IN.with(|held| held.borrow().clone()) {
    opener(url);
    return Ok(());
  }
  system::open(url)
}

/// Whether `url` starts with a scheme other than `file:`: letters, then letters, digits, `+`, `-` or `.`, then
/// `:`. A drive letter (`C:\`) is not one.
fn has_scheme(url: &str) -> bool {
  let Some((scheme, rest)) = url.split_once(':') else {
    return false;
  };
  let mut chars = scheme.chars();
  let well_formed = chars.next().is_some_and(|first| first.is_ascii_alphabetic())
    && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'));
  well_formed && scheme.len() > 1 && !scheme.eq_ignore_ascii_case("file") && !rest.is_empty()
}

#[cfg(windows)]
mod system {
  use windows::{
    Win32::UI::{Shell::ShellExecuteW, WindowsAndMessaging::SW_SHOWNORMAL},
    core::{PCWSTR, w},
  };

  use super::OpenUrlError;

  pub(super) fn open(url: &str) -> Result<(), OpenUrlError> {
    let wide: Vec<u16> = url.encode_utf16().chain(std::iter::once(0)).collect();
    // SAFETY: both strings end in a zero and live through the call; no window owns the request.
    let instance = unsafe {
      ShellExecuteW(
        None,
        w!("open"),
        PCWSTR(wide.as_ptr()),
        PCWSTR::null(),
        PCWSTR::null(),
        SW_SHOWNORMAL,
      )
    };
    // What it gives back is a number over 32 where it opened the address, and an error under it.
    let code = instance.0 as isize;
    if code > 32 {
      Ok(())
    } else {
      Err(OpenUrlError::Failed(format!("ShellExecuteW answered {code}")))
    }
  }
}

#[cfg(target_os = "macos")]
mod system {
  use objc2::{class, msg_send, rc::Retained, runtime::AnyObject};
  use objc2_foundation::{NSString, NSURL};

  use super::OpenUrlError;

  pub(super) fn open(url: &str) -> Result<(), OpenUrlError> {
    let Some(address) = NSURL::URLWithString(&NSString::from_str(url)) else {
      return Err(OpenUrlError::NotAnAddress);
    };
    // SAFETY: `sharedWorkspace` and `openURL:` are NSWorkspace's, and take and give what is said here.
    let opened: bool = unsafe {
      let workspace: Retained<AnyObject> = msg_send![class!(NSWorkspace), sharedWorkspace];
      msg_send![&*workspace, openURL: &*address]
    };
    if opened {
      Ok(())
    } else {
      Err(OpenUrlError::Failed("NSWorkspace did not open it".to_owned()))
    }
  }
}

#[cfg(target_os = "ios")]
mod system {
  use objc2::{
    class, msg_send,
    rc::Retained,
    runtime::{AnyObject, Bool},
  };
  use objc2_foundation::{NSString, NSURL};

  use super::OpenUrlError;

  pub(super) fn open(url: &str) -> Result<(), OpenUrlError> {
    let Some(address) = NSURL::URLWithString(&NSString::from_str(url)) else {
      return Err(OpenUrlError::NotAnAddress);
    };
    // SAFETY: `sharedApplication`, `dictionary` and `openURL:options:completionHandler:` are UIKit's and
    // Foundation's, and take what is said here; a handler of none is allowed.
    unsafe {
      let application: Retained<AnyObject> = msg_send![class!(UIApplication), sharedApplication];
      let options: Retained<AnyObject> = msg_send![class!(NSDictionary), dictionary];
      let _: () = msg_send![
        &*application,
        openURL: &*address,
        options: &*options,
        completionHandler: None::<&block2::Block<dyn Fn(Bool)>>
      ];
    }
    Ok(())
  }
}

#[cfg(all(unix, not(any(target_os = "macos", target_os = "ios", target_os = "android"))))]
mod system {
  use super::OpenUrlError;

  pub(super) fn open(url: &str) -> Result<(), OpenUrlError> {
    let mut child = std::process::Command::new("xdg-open")
      .arg(url)
      .stdin(std::process::Stdio::null())
      .stdout(std::process::Stdio::null())
      .stderr(std::process::Stdio::null())
      .spawn()
      .map_err(|error| OpenUrlError::Failed(error.to_string()))?;
    // It ends by itself once it has handed the address on: waited for away from the UI thread.
    std::thread::spawn(move || {
      let _ = child.wait();
    });
    Ok(())
  }
}

#[cfg(target_arch = "wasm32")]
mod system {
  use super::OpenUrlError;

  pub(super) fn open(url: &str) -> Result<(), OpenUrlError> {
    let window = web_sys::window().ok_or(OpenUrlError::Unsupported)?;
    match window.open_with_url_and_target(url, "_blank") {
      Ok(_) => Ok(()),
      Err(error) => Err(OpenUrlError::Failed(format!("{error:?}"))),
    }
  }
}

#[cfg(not(any(
  windows,
  target_os = "macos",
  target_os = "ios",
  all(unix, not(target_os = "android")),
  target_arch = "wasm32"
)))]
mod system {
  use super::OpenUrlError;

  pub(super) fn open(_url: &str) -> Result<(), OpenUrlError> {
    Err(OpenUrlError::Unsupported)
  }
}

#[cfg(test)]
mod tests {
  use std::sync::Mutex;

  use super::*;

  #[test]
  fn only_an_address_with_a_scheme_is_opened() {
    for url in [
      "https://lurq.dev",
      "http://a.b/c?d",
      "mailto:ivan@lurq.dev",
      "tel:+380",
      "x-app:open",
    ] {
      assert!(has_scheme(url), "{url}");
    }
    for url in [
      "lurq.dev",
      "C:\\Windows\\notepad.exe",
      "file:///etc/passwd",
      "FILE:x",
      "/usr/bin/x",
      ":x",
      "https:",
      "1a:b",
    ] {
      assert!(!has_scheme(url), "{url}");
    }
  }

  #[test]
  fn a_stand_in_is_given_the_address_and_the_one_before_it_is_back_once_it_is_dropped() {
    let opened = Arc::new(Mutex::new(Vec::<String>::new()));
    let (outer_log, inner_log) = (opened.clone(), opened.clone());
    let outer = stand_in(move |url| outer_log.lock().unwrap().push(format!("outer {url}")));
    {
      let _inner = stand_in(move |url| inner_log.lock().unwrap().push(format!("inner {url}")));
      open_url(" https://lurq.dev/a ").unwrap();
    }
    open_url("https://lurq.dev/b").unwrap();
    assert_eq!(open_url("notes.txt"), Err(OpenUrlError::NotAnAddress));
    drop(outer);
    assert_eq!(
      *opened.lock().unwrap(),
      ["inner https://lurq.dev/a", "outer https://lurq.dev/b"]
    );
  }
}
