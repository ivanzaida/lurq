//! Windows 11's snap layouts over the maximize control of a title bar the app draws itself.
//!
//! The system opens its layouts over a window's maximize button when the window answers the system's hit test with
//! `HTMAXBUTTON` there. A window whose title bar is the app's answers `HTCLIENT` everywhere, so the shell lays a
//! window procedure of its own over winit's: over the rectangle of the element the app named its maximize control
//! (`WindowHandle::set_maximize_control`) it answers `HTMAXBUTTON`. The pointer over that rectangle is then the
//! system's: the window hears the non-client mouse messages and not the client ones, so the procedure tells the
//! app what it would have heard (`Window::set_maximize_control_look`: under the pointer, held down) for the control
//! to look so, and a press and release on it maximize or restore the window, as a press of the control does.

use std::{
  collections::HashMap,
  sync::{LazyLock, Mutex},
};

use windows::Win32::{
  Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM},
  Graphics::Gdi::ScreenToClient,
  UI::{
    Input::KeyboardAndMouse::{TME_LEAVE, TME_NONCLIENT, TRACKMOUSEEVENT, TrackMouseEvent},
    WindowsAndMessaging::{
      CallWindowProcW, GWLP_WNDPROC, GetWindowLongPtrW, HTCLIENT, HTMAXBUTTON, IsZoomed, SW_MAXIMIZE, SW_RESTORE,
      SetWindowLongPtrW, ShowWindow, WM_MOUSEMOVE, WM_NCDESTROY, WM_NCHITTEST, WM_NCLBUTTONDBLCLK, WM_NCLBUTTONDOWN,
      WM_NCLBUTTONUP, WM_NCMOUSELEAVE, WM_NCMOUSEMOVE, WNDPROC,
    },
  },
};

use super::window::{MaximizeControlLook, Window};

/// What the procedure needs of each window it is laid over: winit's procedure, and the window's state.
struct Laid {
  original: isize,
  window: Window,
}

/// The windows the procedure is laid over, by their handle.
static LAID: LazyLock<Mutex<HashMap<isize, Laid>>> = LazyLock::new(|| Mutex::new(HashMap::new()));

/// Lays the procedure over the window `hwnd`, once; `window` is its state, which names the maximize control.
pub(crate) fn lay_over(hwnd: isize, window: Window) {
  let mut laid = LAID.lock().unwrap();
  if laid.contains_key(&hwnd) {
    return;
  }
  let handle = HWND(hwnd as *mut _);
  // SAFETY: `handle` is a window of this thread, made by winit; the procedure set is `procedure`, which calls the one
  // it replaces for every message it does not answer itself.
  let original = unsafe {
    let original = GetWindowLongPtrW(handle, GWLP_WNDPROC);
    SetWindowLongPtrW(handle, GWLP_WNDPROC, procedure as usize as isize);
    original
  };
  laid.insert(hwnd, Laid { original, window });
}

/// The point of a mouse message's `lparam`, in screen pixels for a non-client one.
fn point(lparam: LPARAM) -> POINT {
  POINT {
    x: (lparam.0 & 0xffff) as u16 as i16 as i32,
    y: ((lparam.0 >> 16) & 0xffff) as u16 as i16 as i32,
  }
}

/// Whether the screen point `at` is over the app's maximize control of `window`.
fn over_control(hwnd: HWND, window: &Window, mut at: POINT) -> bool {
  let Some(rect) = window.maximize_control_rect() else {
    return false;
  };
  // SAFETY: `hwnd` is the window the message is for.
  if !unsafe { ScreenToClient(hwnd, &mut at) }.as_bool() {
    return false;
  }
  let scale = window.info().scale_factor.max(0.01);
  let (x, y) = (at.x as f32 / scale, at.y as f32 / scale);
  x >= rect.x && x < rect.x + rect.width && y >= rect.y && y < rect.y + rect.height
}

unsafe extern "system" fn procedure(hwnd: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
  let Some((original, window)) = LAID.lock().ok().and_then(|laid| {
    laid
      .get(&(hwnd.0 as isize))
      .map(|laid| (laid.original, laid.window.clone()))
  }) else {
    return LRESULT(0);
  };
  // SAFETY: `original` is the procedure winit set on this window, read with `GetWindowLongPtrW`.
  let original: WNDPROC = unsafe { std::mem::transmute::<isize, WNDPROC>(original) };
  let call =
    |message: u32, wparam: WPARAM, lparam: LPARAM| unsafe { CallWindowProcW(original, hwnd, message, wparam, lparam) };
  let on_control = wparam.0 == HTMAXBUTTON as usize;
  let look = window.maximize_control_look();
  match message {
    WM_NCHITTEST => {
      let hit = call(message, wparam, lparam);
      if hit.0 == HTCLIENT as isize && over_control(hwnd, &window, point(lparam)) {
        return LRESULT(HTMAXBUTTON as isize);
      }
      hit
    }
    WM_NCMOUSEMOVE => {
      if on_control {
        // Told when the pointer leaves the control for anywhere: the rest of the window or outside it.
        let mut track = TRACKMOUSEEVENT {
          cbSize: std::mem::size_of::<TRACKMOUSEEVENT>() as u32,
          dwFlags: TME_LEAVE | TME_NONCLIENT,
          hwndTrack: hwnd,
          dwHoverTime: 0,
        };
        // SAFETY: `track` names this window.
        let _ = unsafe { TrackMouseEvent(&mut track) };
        window.set_maximize_control_look(MaximizeControlLook {
          hovered: true,
          pressed: look.pressed,
        });
        return LRESULT(0);
      }
      window.set_maximize_control_look(MaximizeControlLook::default());
      call(message, wparam, lparam)
    }
    WM_NCMOUSELEAVE => {
      window.set_maximize_control_look(MaximizeControlLook::default());
      call(message, wparam, lparam)
    }
    WM_MOUSEMOVE => {
      if look.hovered || look.pressed {
        window.set_maximize_control_look(MaximizeControlLook::default());
      }
      call(message, wparam, lparam)
    }
    // A press on the control is the control's: held down until it is let go on it, which maximizes or restores
    // the window. The system's own handling (a classic button drawn and tracked) is not called.
    WM_NCLBUTTONDOWN | WM_NCLBUTTONDBLCLK if on_control => {
      window.set_maximize_control_look(MaximizeControlLook {
        hovered: true,
        pressed: true,
      });
      LRESULT(0)
    }
    WM_NCLBUTTONUP if on_control => {
      if look.pressed {
        // SAFETY: `hwnd` is the window the message is for.
        unsafe {
          let show = if IsZoomed(hwnd).as_bool() {
            SW_RESTORE
          } else {
            SW_MAXIMIZE
          };
          let _ = ShowWindow(hwnd, show);
        }
      }
      window.set_maximize_control_look(MaximizeControlLook {
        hovered: true,
        pressed: false,
      });
      LRESULT(0)
    }
    WM_NCDESTROY => {
      if let Ok(mut laid) = LAID.lock() {
        laid.remove(&(hwnd.0 as isize));
      }
      // SAFETY: winit's procedure is put back before it sees the window go.
      unsafe {
        SetWindowLongPtrW(
          hwnd,
          GWLP_WNDPROC,
          original.map_or(0, |procedure| procedure as usize as isize),
        );
      }
      call(message, wparam, lparam)
    }
    _ => call(message, wparam, lparam),
  }
}
