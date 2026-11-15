//! Picking a color from the screen on Windows, on a thread of its own.
//!
//! Two windows of the thread do it: one over every monitor, all but transparent (a layered window of alpha 1, which
//! still takes the pointer), that takes the press, the keys and the cursor; and a small one beside the pointer that
//! shows the loupe and takes nothing. The loupe is drawn whole into a bitmap of its own and given to its layered
//! window with its place in one call (`UpdateLayeredWindow`), so the screen never shows it half drawn, nor moved
//! before it is drawn again, nor before it is drawn at all; it is drawn again only when what it shows or its place
//! changes. Both are kept out of what GDI reads of the screen (`WDA_EXCLUDEFROMCAPTURE`),
//! so the pixels around the pointer are read live as it moves and ten times a second. Where the system cannot keep
//! them out, the screen is read whole once before they show. The thread is aware of each monitor's scale (per monitor
//! v2): the pointer, the monitors and what GDI reads are all in physical pixels of the virtual desktop.

use std::{
  cell::RefCell,
  sync::atomic::{AtomicBool, Ordering},
};

use windows::{
  Win32::{
    Foundation::{COLORREF, HWND, LPARAM, LRESULT, POINT, RECT, SIZE, WPARAM},
    Graphics::Gdi::{
      AC_SRC_ALPHA, AC_SRC_OVER, BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BLENDFUNCTION, BitBlt, CAPTUREBLT,
      CLEARTYPE_QUALITY, CLIP_DEFAULT_PRECIS, CreateCompatibleDC, CreateDIBSection, CreateFontW, CreateSolidBrush,
      DEFAULT_CHARSET, DIB_RGB_COLORS, DeleteDC, DeleteObject, EnumDisplayMonitors, FW_NORMAL, FillRect, FrameRect,
      GdiFlush, GetDC, GetMonitorInfoW, HBITMAP, HDC, HGDIOBJ, HMONITOR, MONITOR_DEFAULTTONEAREST, MONITORINFO,
      MonitorFromPoint, OUT_DEFAULT_PRECIS, ReleaseDC, SRCCOPY, SelectObject, SetBkMode, SetTextColor, TRANSPARENT,
      TextOutW,
    },
    System::LibraryLoader::GetModuleHandleW,
    UI::{
      HiDpi::{
        DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, GetDpiForMonitor, MDT_EFFECTIVE_DPI, SetThreadDpiAwarenessContext,
      },
      Input::KeyboardAndMouse::{
        GetKeyState, ReleaseCapture, SetCapture, SetFocus, VK_DOWN, VK_ESCAPE, VK_LEFT, VK_RETURN, VK_RIGHT, VK_SHIFT,
        VK_SPACE, VK_UP,
      },
      WindowsAndMessaging::{
        CS_HREDRAW, CS_VREDRAW, CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetCursorPos,
        GetMessageW, HTTRANSPARENT, HWND_TOPMOST, IDC_CROSS, KillTimer, LWA_ALPHA, LoadCursorW, MSG, PostQuitMessage,
        RegisterClassExW, SW_SHOW, SW_SHOWNOACTIVATE, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SetCursor, SetCursorPos,
        SetForegroundWindow, SetLayeredWindowAttributes, SetTimer, SetWindowDisplayAffinity, SetWindowPos, ShowWindow,
        TranslateMessage, ULW_ALPHA, UpdateLayeredWindow, WDA_EXCLUDEFROMCAPTURE, WM_ACTIVATE, WM_KEYDOWN,
        WM_LBUTTONDOWN, WM_MOUSEMOVE, WM_NCHITTEST, WM_RBUTTONDOWN, WM_SETCURSOR, WM_TIMER, WNDCLASSEXW, WS_EX_LAYERED,
        WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_EX_TRANSPARENT, WS_POPUP,
      },
    },
  },
  core::{BOOL, PCWSTR, w},
};

use super::{
  Look, Picked,
  geometry::{CELLS, Rect, around, cells, finish_loupe, loupe_origin, offset_in, scaled},
};
use crate::node::color::Color;

/// Whether a pick runs: a second waits for nothing.
static PICKING: AtomicBool = AtomicBool::new(false);

/// Starts a pick on a thread of its own; `then` is called on it with what the person picked.
pub(super) fn pick(look: Look, then: Box<dyn FnOnce(Picked) + Send>) {
  if PICKING.swap(true, Ordering::AcqRel) {
    then(Ok(None));
    return;
  }
  let started = std::thread::Builder::new()
    .name("lurq eyedropper".to_owned())
    .spawn(move || {
      let picked = run(look);
      PICKING.store(false, Ordering::Release);
      then(Ok(picked));
    });
  if started.is_err() {
    PICKING.store(false, Ordering::Release);
  }
}

/// What the pick of this thread holds while it runs.
struct Session {
  overlay: HWND,
  loupe: HWND,
  monitors: Vec<Rect>,
  /// The desktop read whole, where the windows cannot be kept out of what is read.
  snapshot: Option<Vec<u32>>,
  desktop: Rect,
  /// Whether the overlay was active once: losing it after gives up.
  active: bool,
  outcome: Option<Option<Color>>,
}

/// What the loupe draws, and where it stands.
#[derive(Clone, Default)]
struct Loupe {
  look: Option<Look>,
  cells: Vec<Option<Color>>,
  dpi: u32,
  size: (i32, i32),
  origin: (i32, i32),
  /// Whether it was given to its window once: what it shows is drawn again only where something of it changed.
  shown: bool,
}

thread_local! {
  static SESSION: RefCell<Option<Session>> = const { RefCell::new(None) };
  static LOUPE: RefCell<Loupe> = RefCell::new(Loupe::default());
}

/// The loupe's measures at `dpi`: a cell, the room around the cells, the strip of the hex under them, the corners.
struct Measures {
  cell: i32,
  pad: i32,
  strip: i32,
  radius: i32,
}

impl Measures {
  fn at(dpi: u32) -> Self {
    Self {
      cell: scaled(12, dpi),
      pad: scaled(6, dpi),
      strip: scaled(28, dpi),
      radius: scaled(8, dpi),
    }
  }

  fn size(&self) -> (i32, i32) {
    let grid = CELLS * self.cell;
    (grid + 2 * self.pad, grid + 2 * self.pad + self.strip)
  }
}

/// Runs a pick on this thread to its end.
fn run(look: Look) -> Option<Color> {
  unsafe {
    let _ = SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    let monitors = monitors();
    let desktop = around(&monitors);
    let Ok(module) = GetModuleHandleW(None) else {
      return None;
    };
    let instance = module.into();
    let overlay_class = w!("LurqEyeDropperOverlay");
    let loupe_class = w!("LurqEyeDropperLoupe");
    register(instance, overlay_class, overlay_proc);
    register(instance, loupe_class, loupe_proc);
    let Ok(overlay) = CreateWindowExW(
      WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_LAYERED,
      overlay_class,
      w!("Pick a color"),
      WS_POPUP,
      desktop.left,
      desktop.top,
      desktop.width(),
      desktop.height(),
      None,
      None,
      Some(instance),
      None,
    ) else {
      return None;
    };
    let Ok(loupe) = CreateWindowExW(
      WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE | WS_EX_TRANSPARENT | WS_EX_LAYERED,
      loupe_class,
      w!(""),
      WS_POPUP,
      0,
      0,
      1,
      1,
      None,
      None,
      Some(instance),
      None,
    ) else {
      let _ = DestroyWindow(overlay);
      return None;
    };
    // Kept out of what is read of the screen, the windows read what is under them; else the screen is read now.
    // `LURQ_EYEDROPPER_SNAPSHOT` takes the way of a system that cannot keep them out: to check it, and to see the
    // loupe in a screenshot, which it is kept out of otherwise.
    let live = std::env::var_os("LURQ_EYEDROPPER_SNAPSHOT").is_none()
      && SetWindowDisplayAffinity(overlay, WDA_EXCLUDEFROMCAPTURE).is_ok()
      && SetWindowDisplayAffinity(loupe, WDA_EXCLUDEFROMCAPTURE).is_ok();
    let snapshot = (!live).then(|| read(desktop)).flatten();
    let _ = SetLayeredWindowAttributes(overlay, COLORREF(0), 1, LWA_ALPHA);
    LOUPE.with(|held| {
      *held.borrow_mut() = Loupe {
        look: Some(look),
        ..Loupe::default()
      }
    });
    SESSION.with(|session| {
      *session.borrow_mut() = Some(Session {
        overlay,
        loupe,
        monitors,
        snapshot,
        desktop,
        active: false,
        outcome: None,
      })
    });
    let _ = ShowWindow(overlay, SW_SHOW);
    let _ = SetForegroundWindow(overlay);
    let _ = SetFocus(Some(overlay));
    SetCapture(overlay);
    // The loupe is drawn before it shows, and over the overlay.
    update();
    let _ = ShowWindow(loupe, SW_SHOWNOACTIVATE);
    let _ = SetWindowPos(
      loupe,
      Some(HWND_TOPMOST),
      0,
      0,
      0,
      0,
      SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
    );
    SetTimer(Some(overlay), 1, 100, None);
    let mut message = MSG::default();
    while GetMessageW(&mut message, None, 0, 0).as_bool() {
      let _ = TranslateMessage(&message);
      DispatchMessageW(&message);
    }
    // The loop ends with the pick's windows gone (`finish`); where it ended otherwise (`GetMessageW` failed), they
    // go now. A window lives no longer than its thread, nor than the process: a crash takes them with it.
    let session = SESSION.with(|session| session.borrow_mut().take());
    if let Some(session) = &session
      && session.outcome.is_none()
    {
      let _ = DestroyWindow(session.loupe);
      let _ = DestroyWindow(session.overlay);
    }
    session.and_then(|session| session.outcome).flatten()
  }
}

/// Registers a class of the thread's windows; one registered before stays.
unsafe fn register(
  instance: windows::Win32::Foundation::HINSTANCE,
  name: PCWSTR,
  proc_: unsafe extern "system" fn(HWND, u32, WPARAM, LPARAM) -> LRESULT,
) {
  unsafe {
    let class = WNDCLASSEXW {
      cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
      style: CS_HREDRAW | CS_VREDRAW,
      lpfnWndProc: Some(proc_),
      hInstance: instance,
      hCursor: LoadCursorW(None, IDC_CROSS).unwrap_or_default(),
      lpszClassName: name,
      ..Default::default()
    };
    RegisterClassExW(&class);
  }
}

/// Every monitor, in physical pixels of the virtual desktop.
fn monitors() -> Vec<Rect> {
  unsafe extern "system" fn each(_: HMONITOR, _: HDC, rect: *mut RECT, data: LPARAM) -> BOOL {
    let monitors = unsafe { &mut *(data.0 as *mut Vec<Rect>) };
    let rect = unsafe { *rect };
    monitors.push(Rect {
      left: rect.left,
      top: rect.top,
      right: rect.right,
      bottom: rect.bottom,
    });
    BOOL(1)
  }
  let mut monitors: Vec<Rect> = Vec::new();
  unsafe {
    let _ = EnumDisplayMonitors(None, None, Some(each), LPARAM(&mut monitors as *mut Vec<Rect> as isize));
  }
  monitors
}

/// The pixels of `area` of the screen, row by row, as `0x00RRGGBB`.
fn read(area: Rect) -> Option<Vec<u32>> {
  let (width, height) = (area.width(), area.height());
  if width <= 0 || height <= 0 {
    return None;
  }
  unsafe {
    let screen = GetDC(None);
    let memory = CreateCompatibleDC(Some(screen));
    let info = BITMAPINFO {
      bmiHeader: BITMAPINFOHEADER {
        biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
        biWidth: width,
        // Rows from the top.
        biHeight: -height,
        biPlanes: 1,
        biBitCount: 32,
        biCompression: BI_RGB.0,
        ..Default::default()
      },
      ..Default::default()
    };
    let mut bits = std::ptr::null_mut();
    let bitmap: Option<HBITMAP> = CreateDIBSection(Some(memory), &info, DIB_RGB_COLORS, &mut bits, None, 0).ok();
    let pixels = bitmap.and_then(|bitmap| {
      let previous = SelectObject(memory, bitmap.into());
      let copied = BitBlt(
        memory,
        0,
        0,
        width,
        height,
        Some(screen),
        area.left,
        area.top,
        SRCCOPY | CAPTUREBLT,
      );
      let pixels = copied
        .is_ok()
        .then(|| std::slice::from_raw_parts(bits as *const u32, (width * height) as usize).to_vec());
      SelectObject(memory, previous);
      let _ = DeleteObject(bitmap.into());
      pixels
    });
    let _ = DeleteDC(memory);
    ReleaseDC(None, screen);
    pixels
  }
}

fn color_of(pixel: u32) -> Color {
  Color::new((pixel >> 16) as u8, (pixel >> 8) as u8, pixel as u8, 255)
}

/// Reads around the pointer and moves the loupe to it.
fn update() {
  let mut cursor = POINT::default();
  if unsafe { GetCursorPos(&mut cursor) }.is_err() {
    return;
  }
  let cursor = (cursor.x, cursor.y);
  let Some((monitors, loupe, colors)) = SESSION.with(|session| {
    let session = session.try_borrow().ok()?;
    let session = session.as_ref()?;
    let wanted = cells(cursor, &session.monitors);
    let colors: Vec<Option<Color>> = match &session.snapshot {
      Some(snapshot) => wanted
        .iter()
        .map(|cell| {
          let offset = offset_in((*cell)?, session.desktop)?;
          snapshot.get(offset).copied().map(color_of)
        })
        .collect(),
      None => {
        let half = CELLS / 2;
        let around = read(Rect::new(cursor.0 - half, cursor.1 - half, CELLS, CELLS));
        wanted
          .iter()
          .enumerate()
          .map(|(index, cell)| cell.and_then(|_| around.as_ref()?.get(index).copied().map(color_of)))
          .collect()
      }
    };
    Some((session.monitors.clone(), session.loupe, colors))
  }) else {
    return;
  };
  let monitor = monitors
    .iter()
    .copied()
    .find(|monitor| monitor.contains(cursor))
    .unwrap_or_else(|| monitor_at(cursor));
  let dpi = dpi_at(cursor);
  let size = Measures::at(dpi).size();
  let origin = loupe_origin(cursor, size, scaled(20, dpi), monitor);
  let changed = LOUPE.with(|held| {
    let mut held = held.borrow_mut();
    let changed = !held.shown || held.cells != colors || held.dpi != dpi || held.origin != origin;
    held.cells = colors;
    held.dpi = dpi;
    held.size = size;
    held.origin = origin;
    held.shown = true;
    changed.then(|| held.clone())
  });
  // Ten times a second with the pointer still and the screen under it the same: nothing to draw.
  if let Some(drawn) = changed {
    unsafe { present(loupe, &drawn) };
  }
}

/// Draws `loupe` whole into a bitmap and gives it to its window with its place and its size in one call: the screen
/// shows the last frame until this one is whole, and then this one.
unsafe fn present(window: HWND, loupe: &Loupe) {
  let (width, height) = loupe.size;
  let Some(look) = loupe.look else {
    return;
  };
  if width <= 0 || height <= 0 {
    return;
  }
  unsafe {
    let screen = GetDC(None);
    let memory = CreateCompatibleDC(Some(screen));
    let info = BITMAPINFO {
      bmiHeader: BITMAPINFOHEADER {
        biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
        biWidth: width,
        // Rows from the top.
        biHeight: -height,
        biPlanes: 1,
        biBitCount: 32,
        biCompression: BI_RGB.0,
        ..Default::default()
      },
      ..Default::default()
    };
    let mut bits = std::ptr::null_mut();
    if let Ok(bitmap) = CreateDIBSection(Some(memory), &info, DIB_RGB_COLORS, &mut bits, None, 0) {
      let previous = SelectObject(memory, bitmap.into());
      draw(memory, loupe);
      let _ = GdiFlush();
      let pixels = std::slice::from_raw_parts_mut(bits as *mut u32, (width * height) as usize);
      let radius = Measures::at(loupe.dpi.max(96)).radius as f32;
      finish_loupe(
        pixels,
        width,
        height,
        radius,
        (look.border.r(), look.border.g(), look.border.b()),
      );
      let blend = BLENDFUNCTION {
        BlendOp: AC_SRC_OVER as u8,
        BlendFlags: 0,
        SourceConstantAlpha: 255,
        AlphaFormat: AC_SRC_ALPHA as u8,
      };
      let _ = UpdateLayeredWindow(
        window,
        Some(screen),
        Some(&POINT {
          x: loupe.origin.0,
          y: loupe.origin.1,
        }),
        Some(&SIZE { cx: width, cy: height }),
        Some(memory),
        Some(&POINT { x: 0, y: 0 }),
        COLORREF(0),
        Some(&blend),
        ULW_ALPHA,
      );
      SelectObject(memory, previous);
      let _ = DeleteObject(bitmap.into());
    }
    let _ = DeleteDC(memory);
    ReleaseDC(None, screen);
  }
}

/// The monitor nearest `point`.
fn monitor_at(point: (i32, i32)) -> Rect {
  unsafe {
    let monitor = MonitorFromPoint(POINT { x: point.0, y: point.1 }, MONITOR_DEFAULTTONEAREST);
    let mut info = MONITORINFO {
      cbSize: std::mem::size_of::<MONITORINFO>() as u32,
      ..Default::default()
    };
    let _ = GetMonitorInfoW(monitor, &mut info);
    let rect = info.rcMonitor;
    Rect {
      left: rect.left,
      top: rect.top,
      right: rect.right,
      bottom: rect.bottom,
    }
  }
}

/// The scale of the monitor at `point`, as dots per inch: 96 is 100%.
fn dpi_at(point: (i32, i32)) -> u32 {
  unsafe {
    let monitor = MonitorFromPoint(POINT { x: point.0, y: point.1 }, MONITOR_DEFAULTTONEAREST);
    let (mut x, mut y) = (96, 96);
    match GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &mut x, &mut y) {
      Ok(()) => x,
      Err(_) => 96,
    }
  }
}

/// Ends the pick with `outcome`: the windows go, and the thread's loop ends.
fn finish(outcome: Option<Color>) {
  let windows = SESSION.with(|session| {
    let mut session = session.try_borrow_mut().ok()?;
    let session = session.as_mut()?;
    if session.outcome.is_some() {
      return None;
    }
    session.outcome = Some(outcome);
    Some((session.overlay, session.loupe))
  });
  let Some((overlay, loupe)) = windows else {
    return;
  };
  unsafe {
    let _ = KillTimer(Some(overlay), 1);
    let _ = ReleaseCapture();
    let _ = DestroyWindow(loupe);
    let _ = DestroyWindow(overlay);
    PostQuitMessage(0);
  }
}

/// The color the loupe shows in its middle: the pixel under the pointer.
fn taken() -> Option<Color> {
  update();
  LOUPE.with(|held| held.borrow().cells.get((CELLS * CELLS / 2) as usize).copied().flatten())
}

unsafe extern "system" fn overlay_proc(window: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
  match message {
    WM_SETCURSOR => {
      unsafe {
        if let Ok(cross) = LoadCursorW(None, IDC_CROSS) {
          SetCursor(Some(cross));
        }
      }
      LRESULT(1)
    }
    WM_MOUSEMOVE | WM_TIMER => {
      update();
      LRESULT(0)
    }
    WM_LBUTTONDOWN => {
      finish(taken());
      LRESULT(0)
    }
    WM_RBUTTONDOWN => {
      finish(None);
      LRESULT(0)
    }
    WM_KEYDOWN => {
      let key = wparam.0 as u16;
      let step = if unsafe { GetKeyState(VK_SHIFT.0 as i32) } < 0 {
        10
      } else {
        1
      };
      let nudge = match key {
        k if k == VK_LEFT.0 => Some((-step, 0)),
        k if k == VK_RIGHT.0 => Some((step, 0)),
        k if k == VK_UP.0 => Some((0, -step)),
        k if k == VK_DOWN.0 => Some((0, step)),
        _ => None,
      };
      if key == VK_ESCAPE.0 {
        finish(None);
      } else if key == VK_RETURN.0 || key == VK_SPACE.0 {
        finish(taken());
      } else if let Some((dx, dy)) = nudge {
        let mut cursor = POINT::default();
        unsafe {
          if GetCursorPos(&mut cursor).is_ok() {
            let _ = SetCursorPos(cursor.x + dx, cursor.y + dy);
          }
        }
        update();
      }
      LRESULT(0)
    }
    WM_ACTIVATE => {
      // Another window taken to the front (Alt+Tab) gives up; the first activation is the pick's own.
      let inactive = (wparam.0 & 0xffff) == 0;
      let gave_up = SESSION.with(|session| {
        let mut session = session.try_borrow_mut().ok()?;
        let session = session.as_mut()?;
        if inactive {
          Some(session.active)
        } else {
          session.active = true;
          Some(false)
        }
      });
      if gave_up == Some(true) {
        finish(None);
      }
      LRESULT(0)
    }
    _ => unsafe { DefWindowProcW(window, message, wparam, lparam) },
  }
}

unsafe extern "system" fn loupe_proc(window: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
  match message {
    WM_NCHITTEST => LRESULT(HTTRANSPARENT as isize),
    _ => unsafe { DefWindowProcW(window, message, wparam, lparam) },
  }
}

fn colorref(color: Color) -> COLORREF {
  COLORREF(u32::from(color.r()) | (u32::from(color.g()) << 8) | (u32::from(color.b()) << 16))
}

/// Fills `rect` of `target` with `color`.
unsafe fn fill(target: HDC, rect: RECT, color: Color) {
  unsafe {
    let brush = CreateSolidBrush(colorref(color));
    FillRect(target, &rect, brush);
    let _ = DeleteObject(HGDIOBJ(brush.0));
  }
}

/// A line of one pixel around `rect` of `target` in `color`.
unsafe fn frame(target: HDC, rect: RECT, color: Color) {
  unsafe {
    let brush = CreateSolidBrush(colorref(color));
    FrameRect(target, &rect, brush);
    let _ = DeleteObject(HGDIOBJ(brush.0));
  }
}

/// Draws the loupe into `target`, a bitmap of its size: the cells magnified in its panel, the middle one outlined, and
/// under them the color taken as a swatch and its hex. Its corners and its edge are made by [`finish_loupe`].
unsafe fn draw(target: HDC, loupe: &Loupe) {
  let Some(look) = loupe.look else {
    return;
  };
  let measures = Measures::at(loupe.dpi.max(96));
  let (width, height) = loupe.size;
  let rect = |left: i32, top: i32, right: i32, bottom: i32| RECT {
    left,
    top,
    right,
    bottom,
  };
  unsafe {
    fill(target, rect(0, 0, width, height), look.border);
    fill(target, rect(1, 1, width - 1, height - 1), look.background);
    let (pad, cell) = (measures.pad, measures.cell);
    for (index, color) in loupe.cells.iter().enumerate() {
      let (column, row) = (index as i32 % CELLS, index as i32 / CELLS);
      let (left, top) = (pad + column * cell, pad + row * cell);
      if let Some(color) = color {
        fill(target, rect(left, top, left + cell, top + cell), *color);
      }
    }
    // The middle cell: a line in the foreground around it, and one in the background inside that, so that it shows
    // on any color.
    let middle = CELLS / 2;
    let (left, top) = (pad + middle * cell, pad + middle * cell);
    let ring = scaled(1, loupe.dpi.max(96)).max(1);
    for step in 0..ring {
      frame(
        target,
        rect(
          left - 1 - step,
          top - 1 - step,
          left + cell + 1 + step,
          top + cell + 1 + step,
        ),
        look.foreground,
      );
    }
    frame(target, rect(left, top, left + cell, top + cell), look.background);

    let taken = loupe.cells.get((CELLS * CELLS / 2) as usize).copied().flatten();
    let grid = CELLS * cell;
    let strip_top = pad + grid + pad / 2;
    let side = measures.strip - pad;
    if let Some(color) = taken {
      let swatch = rect(pad, strip_top, pad + side, strip_top + side);
      fill(target, swatch, color);
      frame(target, swatch, look.border);
      let font = CreateFontW(
        -scaled(13, loupe.dpi.max(96)),
        0,
        0,
        0,
        FW_NORMAL.0 as i32,
        0,
        0,
        0,
        DEFAULT_CHARSET,
        OUT_DEFAULT_PRECIS,
        CLIP_DEFAULT_PRECIS,
        CLEARTYPE_QUALITY,
        0,
        w!("Consolas"),
      );
      let previous = SelectObject(target, HGDIOBJ(font.0));
      SetBkMode(target, TRANSPARENT);
      SetTextColor(target, colorref(look.foreground));
      let text: Vec<u16> = color.to_hex_upper().encode_utf16().collect();
      let _ = TextOutW(
        target,
        pad + side + pad,
        strip_top + (side - scaled(15, loupe.dpi.max(96))) / 2,
        &text,
      );
      SelectObject(target, previous);
      let _ = DeleteObject(HGDIOBJ(font.0));
    }
  }
}

#[cfg(test)]
mod tests {
  use std::sync::mpsc;

  use windows::Win32::UI::WindowsAndMessaging::{FindWindowW, IsWindowVisible, PostMessageW};

  use super::*;

  /// The window of the class `name`, while one is open.
  fn open(name: PCWSTR) -> Option<HWND> {
    unsafe { FindWindowW(name, None) }
      .ok()
      .filter(|window| !window.is_invalid())
  }

  /// Starts a pick, waits for its windows, posts `message` to the window that takes the keys and the pointer, and
  /// gives what the pick gave.
  fn pick_ended_by(message: u32, wparam: usize) -> Picked {
    let (give, take) = mpsc::channel();
    pick(
      Look {
        background: Color::new(24, 24, 27, 255),
        foreground: Color::new(250, 250, 250, 255),
        border: Color::new(63, 63, 70, 255),
      },
      Box::new(move |picked| give.send(picked).unwrap()),
    );
    // Both windows, and the loupe shown: the pick is running.
    let overlay = (0..400)
      .find_map(|_| {
        std::thread::sleep(std::time::Duration::from_millis(5));
        let loupe = open(w!("LurqEyeDropperLoupe"))?;
        unsafe { IsWindowVisible(loupe) }.as_bool().then_some(())?;
        open(w!("LurqEyeDropperOverlay"))
      })
      .expect("the pick's windows open");
    unsafe { PostMessageW(Some(overlay), message, WPARAM(wparam), LPARAM(0)) }.unwrap();
    take
      .recv_timeout(std::time::Duration::from_secs(5))
      .expect("the pick ends")
  }

  #[test]
  #[ignore = "opens the pick's windows over the desktop for a moment"]
  fn a_pick_given_up_or_taken_leaves_no_window_behind() {
    assert_eq!(
      pick_ended_by(WM_KEYDOWN, VK_ESCAPE.0 as usize),
      Ok(None),
      "Escape gives up"
    );
    assert!(open(w!("LurqEyeDropperOverlay")).is_none() && open(w!("LurqEyeDropperLoupe")).is_none());
    let taken = pick_ended_by(WM_LBUTTONDOWN, 0);
    assert!(
      matches!(taken, Ok(Some(_))),
      "a press takes the color under the pointer: {taken:?}"
    );
    assert!(open(w!("LurqEyeDropperOverlay")).is_none() && open(w!("LurqEyeDropperLoupe")).is_none());
  }
}
