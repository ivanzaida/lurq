//! The window's controls of a title bar the app draws itself, told to Windows.
//!
//! Windows asks a window what is at a point of it (`WM_NCHITTEST`). A window whose title bar is the app's answers
//! "the client area" everywhere, so the system does not know its minimize, maximize and close controls: Windows 11
//! opens its snap layouts only over what the window says is its maximize button. The shell lays a window procedure
//! of its own over winit's. Where the tree last laid out an element marked `.window_control(..)`, and nothing that
//! takes the pointer covers it (`Window::control_at`), it answers `HTMINBUTTON`, `HTMAXBUTTON` or `HTCLOSE`.
//!
//! The pointer over such an element is then in the window's non-client area: the system sends the non-client mouse
//! messages there (`WM_NCMOUSEMOVE`, `WM_NCLBUTTONDOWN`, ...) and not the client ones winit hears. The procedure
//! gives winit each of them as the client message it would have been, at the same point, so the element is under
//! the pointer, held down, clicked and given its cursor as any other element is, and it is the element's own
//! `on_click` that acts on the window. The system's own handling of a press on a caption button (it draws and
//! tracks a button of its own) is not called.
//!
//! Leaving goes the same way. The system counts the pointer over a control as out of the client area, so the
//! `WM_MOUSELEAVE` it posts when the pointer comes onto a control from the rest of the window is kept from winit;
//! when the pointer leaves a control (`WM_NCMOUSELEAVE`) for outside the window, winit is told the pointer left,
//! and when it leaves it for the rest of the window, the leave of the client area is asked for again.

use std::{
  collections::HashMap,
  sync::{LazyLock, Mutex},
};

use windows::Win32::{
  Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM},
  Graphics::Gdi::ScreenToClient,
  UI::{
    Input::KeyboardAndMouse::{
      GetKeyState, TME_LEAVE, TME_NONCLIENT, TRACKMOUSEEVENT, TRACKMOUSEEVENT_FLAGS, TrackMouseEvent, VK_CONTROL,
      VK_LBUTTON, VK_MBUTTON, VK_RBUTTON, VK_SHIFT, VK_XBUTTON1, VK_XBUTTON2,
    },
    WindowsAndMessaging::{
      CallWindowProcW, DefWindowProcW, GWLP_WNDPROC, GetCursorPos, GetWindowLongPtrW, HTCLIENT, HTCLOSE, HTMAXBUTTON,
      HTMINBUTTON, SetWindowLongPtrW, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MBUTTONDOWN, WM_MBUTTONUP, WM_MOUSEMOVE,
      WM_NCDESTROY, WM_NCHITTEST, WM_NCLBUTTONDBLCLK, WM_NCLBUTTONDOWN, WM_NCLBUTTONUP, WM_NCMBUTTONDBLCLK,
      WM_NCMBUTTONDOWN, WM_NCMBUTTONUP, WM_NCMOUSELEAVE, WM_NCMOUSEMOVE, WM_NCRBUTTONDBLCLK, WM_NCRBUTTONDOWN,
      WM_NCRBUTTONUP, WM_NCXBUTTONDBLCLK, WM_NCXBUTTONDOWN, WM_NCXBUTTONUP, WM_RBUTTONDOWN, WM_RBUTTONUP, WM_SETCURSOR,
      WM_XBUTTONDOWN, WM_XBUTTONUP, WNDPROC, WindowFromPoint,
    },
  },
};

use super::window::Window;
use crate::node::WindowControl;

/// The pointer left the client area of a window that asked to be told (`TrackMouseEvent`). The `windows` crate has
/// it with the common controls, a feature this crate does not turn on.
const WM_MOUSELEAVE: u32 = 0x02A3;

/// What the procedure needs of each window it is laid over: the procedure under it, and the window's state.
struct Laid {
  original: isize,
  window: Window,
  /// Whether the pointer was last heard over a control, in the non-client area.
  over_control: bool,
}

/// The windows the procedure is laid over, by their handle.
static LAID: LazyLock<Mutex<HashMap<isize, Laid>>> = LazyLock::new(|| Mutex::new(HashMap::new()));

/// Lays the procedure over the window `hwnd`, once; `window` is its state, where the tree leaves its controls.
pub(crate) fn lay_over(hwnd: isize, window: Window) {
  let mut laid = LAID.lock().unwrap();
  if laid.contains_key(&hwnd) {
    return;
  }
  let handle = HWND(hwnd as *mut _);
  // SAFETY: `handle` is a window of this thread; the procedure set is `procedure`, which calls the one it replaces
  // for every message it does not answer itself.
  let original = unsafe {
    let original = GetWindowLongPtrW(handle, GWLP_WNDPROC);
    SetWindowLongPtrW(handle, GWLP_WNDPROC, procedure as usize as isize);
    original
  };
  laid.insert(
    hwnd,
    Laid {
      original,
      window,
      over_control: false,
    },
  );
}

/// What the window answers the system's hit test with over a control.
fn hit_code(control: WindowControl) -> u32 {
  match control {
    WindowControl::Minimize => HTMINBUTTON,
    WindowControl::Maximize => HTMAXBUTTON,
    WindowControl::Close => HTCLOSE,
  }
}

/// Whether a hit test's answer is one of the three this procedure gives.
fn is_control_code(code: usize) -> bool {
  [HTMINBUTTON, HTMAXBUTTON, HTCLOSE].contains(&(code as u32))
}

/// The point of a mouse message's `lparam`: in screen pixels for a non-client one.
fn point(lparam: LPARAM) -> POINT {
  POINT {
    x: (lparam.0 & 0xffff) as u16 as i16 as i32,
    y: ((lparam.0 >> 16) & 0xffff) as u16 as i16 as i32,
  }
}

/// A point as the `lparam` of a client mouse message carries it.
fn point_lparam(at: POINT) -> LPARAM {
  let x = at.x as i16 as u16 as u32;
  let y = at.y as i16 as u16 as u32;
  LPARAM(((y << 16) | x) as i32 as isize)
}

/// The client message a non-client mouse button message would have been in the client area. A second press of a
/// double click is a press: winit's windows hear no double clicks.
fn client_button_message(message: u32) -> Option<u32> {
  Some(match message {
    WM_NCLBUTTONDOWN | WM_NCLBUTTONDBLCLK => WM_LBUTTONDOWN,
    WM_NCLBUTTONUP => WM_LBUTTONUP,
    WM_NCRBUTTONDOWN | WM_NCRBUTTONDBLCLK => WM_RBUTTONDOWN,
    WM_NCRBUTTONUP => WM_RBUTTONUP,
    WM_NCMBUTTONDOWN | WM_NCMBUTTONDBLCLK => WM_MBUTTONDOWN,
    WM_NCMBUTTONUP => WM_MBUTTONUP,
    WM_NCXBUTTONDOWN | WM_NCXBUTTONDBLCLK => WM_XBUTTONDOWN,
    WM_NCXBUTTONUP => WM_XBUTTONUP,
    _ => return None,
  })
}

/// The hit test's answer a non-client mouse message carries: its `wparam`, the low word of it for a message of
/// the extra buttons, whose high word says which of them.
fn message_hit_code(message: u32, wparam: WPARAM) -> usize {
  match message {
    WM_NCXBUTTONDOWN | WM_NCXBUTTONUP | WM_NCXBUTTONDBLCLK => wparam.0 & 0xffff,
    _ => wparam.0,
  }
}

/// The `wparam` of the client message a non-client one becomes: the keys and buttons held (`MK_*`), and for the
/// extra buttons which of them, kept in the high word.
fn client_wparam(message: u32, wparam: WPARAM, keys: usize) -> WPARAM {
  match message {
    WM_NCXBUTTONDOWN | WM_NCXBUTTONUP | WM_NCXBUTTONDBLCLK => WPARAM((wparam.0 & !0xffff) | keys),
    _ => WPARAM(keys),
  }
}

/// The keys and buttons held now, as a client mouse message's `wparam` says them (`MK_*`).
fn held_keys() -> usize {
  const MK: [(u16, usize); 7] = [
    (VK_LBUTTON.0, 0x01),
    (VK_RBUTTON.0, 0x02),
    (VK_SHIFT.0, 0x04),
    (VK_CONTROL.0, 0x08),
    (VK_MBUTTON.0, 0x10),
    (VK_XBUTTON1.0, 0x20),
    (VK_XBUTTON2.0, 0x40),
  ];
  MK.iter()
    // SAFETY: reads the state of a key; no pointer is passed.
    .filter(|(key, _)| unsafe { GetKeyState(i32::from(*key)) } < 0)
    .fold(0, |keys, (_, flag)| keys | flag)
}

/// The screen point `at` in the client area of `hwnd`.
fn to_client(hwnd: HWND, mut at: POINT) -> Option<POINT> {
  // SAFETY: `hwnd` is the window the message is for.
  unsafe { ScreenToClient(hwnd, &mut at) }.as_bool().then_some(at)
}

/// The control of `window` at the screen point `at`, if the tree laid one out there.
fn control_at(hwnd: HWND, window: &Window, at: POINT) -> Option<WindowControl> {
  let at = to_client(hwnd, at)?;
  window.control_at(at.x as f32, at.y as f32)
}

/// Where the pointer is on the screen.
fn cursor() -> Option<POINT> {
  let mut at = POINT::default();
  // SAFETY: `at` is a point to write to.
  unsafe { GetCursorPos(&mut at) }.ok().map(|()| at)
}

/// Whether the pointer is on `hwnd` itself: not outside it, and not on a window over it.
fn cursor_on_window(hwnd: HWND) -> Option<POINT> {
  let at = cursor()?;
  // SAFETY: asks which window is at a point.
  (unsafe { WindowFromPoint(at) } == hwnd).then_some(at)
}

/// Asks the system to tell `hwnd` when the pointer leaves it: its client area, or with `TME_NONCLIENT` the rest.
fn track_leave(hwnd: HWND, flags: TRACKMOUSEEVENT_FLAGS) {
  let mut track = TRACKMOUSEEVENT {
    cbSize: std::mem::size_of::<TRACKMOUSEEVENT>() as u32,
    dwFlags: flags,
    hwndTrack: hwnd,
    dwHoverTime: 0,
  };
  // SAFETY: `track` names this window.
  let _ = unsafe { TrackMouseEvent(&mut track) };
}

/// Sets whether the pointer was last heard over a control of `hwnd`, and returns what it was.
fn set_over_control(hwnd: HWND, over: bool) -> bool {
  LAID
    .lock()
    .ok()
    .and_then(|mut laid| {
      laid
        .get_mut(&(hwnd.0 as isize))
        .map(|laid| std::mem::replace(&mut laid.over_control, over))
    })
    .unwrap_or(false)
}

unsafe extern "system" fn procedure(hwnd: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
  let Some((original, window)) = LAID.lock().ok().and_then(|laid| {
    laid
      .get(&(hwnd.0 as isize))
      .map(|laid| (laid.original, laid.window.clone()))
  }) else {
    // SAFETY: a message of a window this procedure no longer knows goes to the system's.
    return unsafe { DefWindowProcW(hwnd, message, wparam, lparam) };
  };
  // SAFETY: `original` is the procedure this one was laid over, read with `GetWindowLongPtrW`.
  let original: WNDPROC = unsafe { std::mem::transmute::<isize, WNDPROC>(original) };
  let call =
    |message: u32, wparam: WPARAM, lparam: LPARAM| unsafe { CallWindowProcW(original, hwnd, message, wparam, lparam) };
  // A non-client mouse message over a control of the app's: the answer it carries is one this procedure gave. A
  // title bar the system draws has buttons of its own that answer the same, outside the client area.
  let over_control = |message: u32| {
    is_control_code(message_hit_code(message, wparam)) && control_at(hwnd, &window, point(lparam)).is_some()
  };
  // The client message winit would have heard at a non-client message's point.
  let client_point = || to_client(hwnd, point(lparam)).map(point_lparam);
  match message {
    WM_NCHITTEST => {
      let hit = call(message, wparam, lparam);
      if hit.0 == HTCLIENT as isize
        && let Some(control) = control_at(hwnd, &window, point(lparam))
      {
        return LRESULT(hit_code(control) as isize);
      }
      hit
    }
    WM_NCMOUSEMOVE if over_control(message) => {
      let Some(at) = client_point() else {
        return call(message, wparam, lparam);
      };
      set_over_control(hwnd, true);
      call(WM_MOUSEMOVE, WPARAM(held_keys()), at);
      // Asked for after winit heard the move: where winit asks for the leave of the client area, it does it there.
      track_leave(hwnd, TME_LEAVE | TME_NONCLIENT);
      LRESULT(0)
    }
    WM_NCLBUTTONDOWN | WM_NCLBUTTONUP | WM_NCLBUTTONDBLCLK | WM_NCRBUTTONDOWN | WM_NCRBUTTONUP | WM_NCRBUTTONDBLCLK
    | WM_NCMBUTTONDOWN | WM_NCMBUTTONUP | WM_NCMBUTTONDBLCLK | WM_NCXBUTTONDOWN | WM_NCXBUTTONUP
    | WM_NCXBUTTONDBLCLK
      if over_control(message) =>
    {
      let (Some(at), Some(client)) = (client_point(), client_button_message(message)) else {
        return call(message, wparam, lparam);
      };
      let keys = held_keys();
      // The press is where the pointer is, whether or not a move was heard there first.
      set_over_control(hwnd, true);
      call(WM_MOUSEMOVE, WPARAM(keys), at);
      track_leave(hwnd, TME_LEAVE | TME_NONCLIENT);
      call(client, client_wparam(message, wparam, keys), at);
      // An extra button's message is answered with TRUE, the others with 0.
      LRESULT(isize::from(matches!(client, WM_XBUTTONDOWN | WM_XBUTTONUP)))
    }
    WM_MOUSEMOVE => {
      let result = call(message, wparam, lparam);
      // Back in the client area from a control: winit still counts the pointer in the window, and the leave it
      // asked for was spent when the pointer went onto the control.
      if set_over_control(hwnd, false) {
        track_leave(hwnd, TME_LEAVE);
      }
      result
    }
    // A press on a control is held as the client area's (winit takes the pointer for it, and may ask for the leave
    // of the client area meanwhile). Let go over a control, the pointer is the non-client area's again.
    WM_LBUTTONUP | WM_RBUTTONUP | WM_MBUTTONUP | WM_XBUTTONUP => {
      let result = call(message, wparam, lparam);
      if cursor_on_window(hwnd).is_some_and(|at| control_at(hwnd, &window, at).is_some()) {
        set_over_control(hwnd, true);
        track_leave(hwnd, TME_LEAVE | TME_NONCLIENT);
      }
      result
    }
    // The pointer left the client area for a control: to winit it is still in the window.
    WM_MOUSELEAVE if cursor_on_window(hwnd).is_some_and(|at| control_at(hwnd, &window, at).is_some()) => LRESULT(0),
    WM_NCMOUSELEAVE => {
      if set_over_control(hwnd, false) {
        if cursor_on_window(hwnd).is_some() {
          // To the rest of the window: its moves follow, and the leave of the client area is asked for again.
          track_leave(hwnd, TME_LEAVE);
        } else {
          // Out of the window: winit is told the pointer left, as it is when it leaves the client area.
          call(WM_MOUSELEAVE, WPARAM(0), LPARAM(0));
        }
      }
      call(message, wparam, lparam)
    }
    // The cursor over a control is the element's: winit sets the app's cursor only in the client area.
    WM_SETCURSOR
      if is_control_code(lparam.0 as usize & 0xffff)
        && wparam.0 == hwnd.0 as usize
        && cursor().is_some_and(|at| control_at(hwnd, &window, at).is_some()) =>
    {
      call(message, wparam, LPARAM((lparam.0 & !0xffff) | HTCLIENT as isize))
    }
    WM_NCDESTROY => {
      if let Ok(mut laid) = LAID.lock() {
        laid.remove(&(hwnd.0 as isize));
      }
      // SAFETY: the procedure under this one is put back before it sees the window go.
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

#[cfg(test)]
mod tests;
