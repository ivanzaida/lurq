//! The pointer position in a window, read from the OS.
//!
//! During a file drag from the OS the pointer belongs to the OS: no
//! `CursorMoved` reaches the window, and winit's drop events carry no
//! position. The file-drag session reads it here instead.

use winit::window::Window;

/// The pointer position in physical pixels of the window's client area, as
/// `WindowEvent::CursorMoved` reports it, or `None` where the OS does not
/// tell (Linux, or a window without a native handle).
#[cfg(windows)]
pub(super) fn position_in_window(window: &Window) -> Option<(f32, f32)> {
  use raw_window_handle::{HasWindowHandle, RawWindowHandle};
  use windows::Win32::{
    Foundation::{HWND, POINT},
    Graphics::Gdi::ScreenToClient,
    UI::WindowsAndMessaging::GetCursorPos,
  };

  let handle = window.window_handle().ok()?;
  let RawWindowHandle::Win32(handle) = handle.as_raw() else {
    return None;
  };
  let hwnd = HWND(handle.hwnd.get() as *mut _);
  if hwnd.is_invalid() {
    return None;
  }
  let mut point = POINT::default();
  // SAFETY: `point` outlives both calls, and `hwnd` is the live window's
  // handle that winit gave out.
  unsafe {
    GetCursorPos(&mut point).ok()?;
    if !ScreenToClient(hwnd, &mut point).as_bool() {
      return None;
    }
  }
  Some((point.x as f32, point.y as f32))
}

/// The pointer position in physical pixels of the window's content area.
///
/// `NSEvent::mouseLocation` is in points of the screen space, whose origin
/// is the bottom-left corner of the main display; winit's `inner_position`
/// is the content area's top-left corner in that space flipped to a top-left
/// origin, in the window's physical pixels, and the main display's height in
/// points is its primary monitor's physical height over its scale factor.
#[cfg(target_os = "macos")]
pub(super) fn position_in_window(window: &Window) -> Option<(f32, f32)> {
  let location = objc2_app_kit::NSEvent::mouseLocation();
  let primary = window.primary_monitor()?;
  let main_screen_height = f64::from(primary.size().height) / primary.scale_factor();
  let scale = window.scale_factor();
  let inner = window.inner_position().ok()?;
  let inner_x = f64::from(inner.x) / scale;
  let inner_y = f64::from(inner.y) / scale;
  let x = (location.x - inner_x) * scale;
  let y = (main_screen_height - location.y - inner_y) * scale;
  Some((x as f32, y as f32))
}

/// Not read on this platform: the file-drag session uses the last pointer
/// position the window saw.
#[cfg(not(any(windows, target_os = "macos")))]
pub(super) fn position_in_window(_window: &Window) -> Option<(f32, f32)> {
  None
}

/// Whether any mouse button is pressed, read from the OS: a file drag from
/// the OS ends with its button, so a hover whose buttons are all up has no
/// drag behind it. `None` where the OS does not tell.
#[cfg(windows)]
pub(super) fn any_mouse_button_pressed() -> Option<bool> {
  use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_LBUTTON, VK_MBUTTON, VK_RBUTTON};

  // SAFETY: `GetAsyncKeyState` reads the state of a virtual key; it takes
  // no pointer and has no preconditions.
  let pressed =
    |key: windows::Win32::UI::Input::KeyboardAndMouse::VIRTUAL_KEY| unsafe { GetAsyncKeyState(i32::from(key.0)) < 0 };
  Some(pressed(VK_LBUTTON) || pressed(VK_RBUTTON) || pressed(VK_MBUTTON))
}

/// See the Windows version: `NSEvent::pressedMouseButtons` is a bit per
/// button, as `NSEvent::buttonNumber` counts them.
#[cfg(target_os = "macos")]
pub(super) fn any_mouse_button_pressed() -> Option<bool> {
  Some(objc2_app_kit::NSEvent::pressedMouseButtons() != 0)
}

#[cfg(not(any(windows, target_os = "macos")))]
pub(super) fn any_mouse_button_pressed() -> Option<bool> {
  None
}
