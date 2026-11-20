//! The system's appearance, light or dark, read before the app has a window.
//!
//! A shell tells the theme the system's appearance once it has made its window, and the app mounts its root
//! before that: the root's `create` and its first render are in the light palette on a dark system, and a color
//! read there and kept stays light. [`system_appearance`] asks the system itself, with no window, and
//! [`App::read_system_appearance`](crate::app::App::read_system_appearance) gives what it says to the theme, so
//! that a root mounted after it is created and first rendered in the look the window will have.
//!
//! ```no_run
//! use lurq::app::App;
//!
//! let mut app = App::new();
//! app.read_system_appearance();
//! // tree.mount_root::<Root>(&mut app, props);
//! ```
//!
//! # What each platform reads
//!
//! - **Windows**: `AppsUseLightTheme` of
//!   `HKEY_CURRENT_USER\Software\Microsoft\Windows\CurrentVersion\Themes\Personalize`, and light whatever it says while
//!   a high contrast theme is on, as the window says then.
//! - **macOS**: `AppleInterfaceStyle` of the person's defaults, which is `Dark` while the system is dark.
//! - **iOS**: the style of the main screen's traits. On the main thread, as all of UIKit is.
//! - **A browser**: the media query `prefers-color-scheme: dark`.
//! - **Linux and the others**: nothing: the window says it when it is made, as before.

use super::window::Appearance;

/// What the system is in now, light or dark, where it can be asked with no window: see the [module](self).
/// `None` where it cannot, and where the system did not say.
pub fn system_appearance() -> Option<Appearance> {
  read()
}

#[cfg(windows)]
fn read() -> Option<Appearance> {
  use windows::{
    Win32::{
      System::Registry::{HKEY_CURRENT_USER, RRF_RT_REG_DWORD, RegGetValueW},
      UI::{
        Accessibility::{HCF_HIGHCONTRASTON, HIGHCONTRASTW},
        WindowsAndMessaging::{SPI_GETHIGHCONTRAST, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS, SystemParametersInfoW},
      },
    },
    core::w,
  };
  let (mut light, mut size) = (1u32, size_of::<u32>() as u32);
  // SAFETY: the value is read into `light`, whose size `size` says.
  let read = unsafe {
    RegGetValueW(
      HKEY_CURRENT_USER,
      w!("Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize"),
      w!("AppsUseLightTheme"),
      RRF_RT_REG_DWORD,
      None,
      Some((&raw mut light).cast()),
      Some(&raw mut size),
    )
  };
  if read.is_err() {
    return None;
  }
  let mut contrast = HIGHCONTRASTW {
    cbSize: size_of::<HIGHCONTRASTW>() as u32,
    ..Default::default()
  };
  // SAFETY: the system fills `contrast`, whose size its first field says.
  let high = unsafe {
    SystemParametersInfoW(
      SPI_GETHIGHCONTRAST,
      contrast.cbSize,
      Some((&raw mut contrast).cast()),
      SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
    )
  }
  .is_ok()
    && contrast.dwFlags.contains(HCF_HIGHCONTRASTON);
  Some(if light == 0 && !high {
    Appearance::Dark
  } else {
    Appearance::Light
  })
}

#[cfg(target_os = "macos")]
fn read() -> Option<Appearance> {
  use objc2::{class, msg_send, rc::Retained, runtime::AnyObject};
  use objc2_foundation::NSString;
  // SAFETY: the messages are `NSUserDefaults`' own, with the types its headers give them.
  let style: Option<Retained<NSString>> = unsafe {
    let defaults: Option<Retained<AnyObject>> = msg_send![class!(NSUserDefaults), standardUserDefaults];
    let defaults = defaults?;
    msg_send![&*defaults, stringForKey: &*NSString::from_str("AppleInterfaceStyle")]
  };
  // The key is not there at all while the system is light.
  Some(match style {
    Some(style) if style.to_string().eq_ignore_ascii_case("dark") => Appearance::Dark,
    _ => Appearance::Light,
  })
}

#[cfg(target_os = "ios")]
fn read() -> Option<Appearance> {
  use objc2::{class, msg_send, rc::Retained, runtime::AnyObject};
  // SAFETY: the messages are UIKit's own, with the types its headers give them.
  let style: isize = unsafe {
    let screen: Option<Retained<AnyObject>> = msg_send![class!(UIScreen), mainScreen];
    let screen = screen?;
    let traits: Option<Retained<AnyObject>> = msg_send![&*screen, traitCollection];
    let traits = traits?;
    msg_send![&*traits, userInterfaceStyle]
  };
  // `UIUserInterfaceStyleLight` and `Dark`; a screen that does not say yet says neither.
  match style {
    1 => Some(Appearance::Light),
    2 => Some(Appearance::Dark),
    _ => None,
  }
}

#[cfg(target_arch = "wasm32")]
fn read() -> Option<Appearance> {
  let dark = web_sys::window()?
    .match_media("(prefers-color-scheme: dark)")
    .ok()??
    .matches();
  Some(if dark { Appearance::Dark } else { Appearance::Light })
}

#[cfg(not(any(windows, target_os = "macos", target_os = "ios", target_arch = "wasm32")))]
fn read() -> Option<Appearance> {
  None
}

#[cfg(test)]
mod tests {
  use super::*;

  #[cfg(windows)]
  #[test]
  fn windows_says_what_it_is_in_with_no_window() {
    assert!(system_appearance().is_some());
  }

  #[cfg(not(any(windows, target_os = "macos", target_os = "ios", target_arch = "wasm32")))]
  #[test]
  fn a_system_that_cannot_be_asked_says_nothing() {
    assert_eq!(system_appearance(), None);
  }
}
