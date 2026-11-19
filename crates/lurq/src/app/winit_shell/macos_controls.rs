//! The window's controls on macOS, in a title bar the app draws itself: the system's own three lights, where the
//! app's are.
//!
//! A window with no decorations has no lights, and an app that draws its bar draws them (`ui::WindowChrome`). Drawn
//! by the app they look right and do less: the system opens its menu of tilings under the pointer on its own zoom
//! button and nowhere else, and takes the window to full screen at a press of it. So where the tree marks controls
//! (`.window_control(..)`), the window becomes one with a title bar that is not seen (transparent, without its
//! title, its content under it), and the system's lights are moved to the middles of the app's controls, which the
//! app then leaves undrawn (`WindowInfo::native_controls`). What a press of a light does is the system's: close
//! asks the window to close, as the app's control does.
//!
//! The system puts its lights back where it has them when the window is resized or leaves full screen, so they
//! are moved again after every frame that finds them elsewhere. In full screen the lights are in the system's bar,
//! which comes down at the top of the screen, and the app's bar draws none.
//!
//! The lights are the system's, over the app's frame, so a screenshot of what the app drew has none: as one is
//! taken the system draws them as pictures, which are laid over it (`app::capture_overlay`).

use objc2::{
  msg_send,
  runtime::{AnyObject, Bool},
};
use objc2_foundation::{NSPoint, NSRect, NSSize};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use winit::window::Window;

use crate::{app::Tree, node::WindowControl};

/// The bits of a window's style: with a title bar, its three controls and edges that resize it, and its content
/// under the title bar.
const TITLED: usize = 1 | (1 << 1) | (1 << 2) | (1 << 3);
const CONTENT_UNDER_TITLE_BAR: usize = 1 << 15;

/// The system's name of each control, and the app's.
const BUTTONS: [(isize, WindowControl); 3] = [
  (0, WindowControl::Close),
  (1, WindowControl::Minimize),
  (2, WindowControl::Maximize),
];

/// What a window's lights were last told, so that the app is told only of changes.
#[derive(Default)]
pub(super) struct NativeControls {
  shown: bool,
}

impl NativeControls {
  /// Puts the system's lights where the tree last laid the app's controls out; hides them where it laid none.
  pub(super) fn apply(&mut self, window: &Window, tree: &Tree) {
    let Ok(handle) = window.window_handle() else {
      return;
    };
    let RawWindowHandle::AppKit(handle) = handle.as_raw() else {
      return;
    };
    // The system takes the window in and out of full screen by itself, at its own zoom light and its keys, and
    // tells nothing once it is done: the last the shell heard was the window's size on its way. So whether it
    // is there is read again each turn.
    let full_screen = window.fullscreen().is_some();
    if tree.window().info().is_full_screen != full_screen {
      tree.window().set_full_screen(full_screen);
      window.request_redraw();
    }
    let regions = tree.window().control_regions();
    let middles = BUTTONS.map(|(_, control)| regions.middle_of(control));
    let any = middles.iter().any(Option::is_some);
    if !any && !self.shown {
      return;
    }
    // SAFETY: on the main thread, where a window's shell runs; the view is the window's own, and what it is asked
    // for is the window's while it lives.
    unsafe {
      let view = handle.ns_view.as_ptr().cast::<AnyObject>();
      let ns_window: *mut AnyObject = msg_send![view, window];
      if ns_window.is_null() {
        return;
      }
      if any {
        let style: usize = msg_send![ns_window, styleMask];
        let wanted = style | TITLED | CONTENT_UNDER_TITLE_BAR;
        if style != wanted {
          let _: () = msg_send![ns_window, setStyleMask: wanted];
          let _: () = msg_send![ns_window, setTitlebarAppearsTransparent: Bool::YES];
          // 1 is NSWindowTitleHidden.
          let _: () = msg_send![ns_window, setTitleVisibility: 1_isize];
        }
      }
      let full_screen = window.fullscreen().is_some();
      let frame: NSRect = msg_send![ns_window, frame];
      // The bar the lights are in is as high as twice the way down to their middle, so that they are in its middle.
      let down = middles.iter().flatten().map(|(_, y)| f64::from(*y)).fold(0.0, f64::max);
      for ((button, _), middle) in BUTTONS.iter().zip(middles) {
        let light: *mut AnyObject = msg_send![ns_window, standardWindowButton: *button];
        if light.is_null() {
          continue;
        }
        // In full screen the lights are the system's to show and to place, in the bar it brings down.
        if full_screen {
          continue;
        }
        let hidden: Bool = msg_send![light, isHidden];
        if hidden.as_bool() != middle.is_none() {
          let _: () = msg_send![light, setHidden: Bool::new(middle.is_none())];
        }
        let Some((x, _)) = middle else {
          continue;
        };
        let bar: *mut AnyObject = msg_send![light, superview];
        let holder: *mut AnyObject = if bar.is_null() {
          std::ptr::null_mut()
        } else {
          msg_send![bar, superview]
        };
        if holder.is_null() {
          continue;
        }
        let height = down * 2.0;
        let held: NSRect = msg_send![holder, frame];
        let holds = NSRect::new(
          NSPoint::new(0.0, frame.size.height - height),
          NSSize::new(frame.size.width, height),
        );
        if !same(held, holds) {
          let _: () = msg_send![holder, setFrame: holds];
        }
        let at: NSRect = msg_send![light, frame];
        let origin = NSPoint::new(f64::from(x) - at.size.width / 2.0, (height - at.size.height) / 2.0);
        if (at.origin.x - origin.x).abs() > 0.25 || (at.origin.y - origin.y).abs() > 0.25 {
          let _: () = msg_send![light, setFrameOrigin: origin];
        }
      }
    }
    self.shown = any;
    // In full screen the system's lights are in its own bar, which comes down at the top of the screen, and the
    // app's bar draws none (`ui::WindowChrome::controls`).
    tree.window().set_native_controls(any && window.fullscreen().is_none());
  }
}

fn same(a: NSRect, b: NSRect) -> bool {
  (a.origin.x - b.origin.x).abs() <= 0.25
    && (a.origin.y - b.origin.y).abs() <= 0.25
    && (a.size.width - b.size.width).abs() <= 0.25
    && (a.size.height - b.size.height).abs() <= 0.25
}

/// Has a screenshot of `window` show the system's lights where they are: see `app::capture_overlay`.
#[cfg(feature = "screenshot")]
pub(super) fn draw_lights_in_screenshots(window: &Window, tree: &mut Tree) {
  let Ok(handle) = window.window_handle() else {
    return;
  };
  let RawWindowHandle::AppKit(handle) = handle.as_raw() else {
    return;
  };
  // SAFETY: on the main thread; the view is the window's own, and the window it is in is held for as long as the
  // tree may ask for its lights.
  let held = unsafe {
    let view = handle.ns_view.as_ptr().cast::<AnyObject>();
    let ns_window: *mut AnyObject = msg_send![view, window];
    objc2::rc::Retained::retain(ns_window)
  };
  let Some(held) = held else {
    return;
  };
  let held = HeldWindow(held);
  tree.set_system_pictures(std::sync::Arc::new(move || held.lights()));
}

/// A window of AppKit's, held for the tree: it is asked for its lights on the main thread alone.
#[cfg(feature = "screenshot")]
struct HeldWindow(objc2::rc::Retained<AnyObject>);

// SAFETY: the window is touched only where `lights` finds itself on the main thread.
#[cfg(feature = "screenshot")]
unsafe impl Send for HeldWindow {}
#[cfg(feature = "screenshot")]
unsafe impl Sync for HeldWindow {}

#[cfg(feature = "screenshot")]
impl HeldWindow {
  /// The lights the system shows in the window now, each drawn by the system as it is on the screen, at the
  /// window's pixels.
  fn lights(&self) -> Vec<crate::app::capture_overlay::SystemPicture> {
    if objc2::MainThreadMarker::new().is_none() {
      return Vec::new();
    }
    let window = &*self.0;
    objc2::rc::autoreleasepool(|_| {
      // SAFETY: on the main thread, with the window held; each light is the window's while it lives.
      unsafe {
        let content: *mut AnyObject = msg_send![window, contentView];
        if content.is_null() {
          return Vec::new();
        }
        let frame: NSRect = msg_send![content, frame];
        let scale: f64 = msg_send![window, backingScaleFactor];
        BUTTONS
          .iter()
          .filter_map(|(button, _)| {
            let light: *mut AnyObject = msg_send![window, standardWindowButton: *button];
            if light.is_null() {
              return None;
            }
            let hidden: Bool = msg_send![light, isHiddenOrHasHiddenAncestor];
            if hidden.as_bool() {
              return None;
            }
            picture_of(light, frame.size.height, scale)
          })
          .collect()
      }
    })
  }
}

/// `view` as the system draws it, at its place in a window whose content is `height` points high.
///
/// # Safety
///
/// On the main thread, with `view` a view of a window that lives.
#[cfg(feature = "screenshot")]
unsafe fn picture_of(
  view: *mut AnyObject,
  height: f64,
  scale: f64,
) -> Option<crate::app::capture_overlay::SystemPicture> {
  unsafe {
    let bounds: NSRect = msg_send![view, bounds];
    let in_window: NSRect = msg_send![view, convertRect: bounds, toView: std::ptr::null_mut::<AnyObject>()];
    let rep: *mut AnyObject = msg_send![view, bitmapImageRepForCachingDisplayInRect: bounds];
    if rep.is_null() {
      return None;
    }
    let _: () = msg_send![view, cacheDisplayInRect: bounds, toBitmapImageRep: rep];
    let width: isize = msg_send![rep, pixelsWide];
    let rows: isize = msg_send![rep, pixelsHigh];
    let per_row: isize = msg_send![rep, bytesPerRow];
    let samples: isize = msg_send![rep, samplesPerPixel];
    let bits: isize = msg_send![rep, bitsPerSample];
    let planar: Bool = msg_send![rep, isPlanar];
    // NSBitmapFormatAlphaFirst and NSBitmapFormatAlphaNonpremultiplied.
    let format: usize = msg_send![rep, bitmapFormat];
    let data: *const u8 = msg_send![rep, bitmapData];
    if data.is_null() || width <= 0 || rows <= 0 || samples != 4 || bits != 8 || planar.as_bool() {
      return None;
    }
    let (alpha_first, straight) = (format & 1 != 0, format & 2 != 0);
    let (width, rows, per_row) = (width as usize, rows as usize, per_row as usize);
    let bytes = std::slice::from_raw_parts(data, per_row * rows);
    let mut rgba = Vec::with_capacity(width * rows * 4);
    for row in 0..rows {
      for pixel in bytes[row * per_row..][..width * 4].chunks_exact(4) {
        let (mut color, alpha) = if alpha_first {
          ([pixel[1], pixel[2], pixel[3]], pixel[0])
        } else {
          ([pixel[0], pixel[1], pixel[2]], pixel[3])
        };
        if straight {
          color = color.map(|channel| ((u16::from(channel) * u16::from(alpha) + 127) / 255) as u8);
        }
        rgba.extend_from_slice(&[color[0], color[1], color[2], alpha]);
      }
    }
    // The window's points go up from its bottom; its pixels down from its top.
    let top = height - (in_window.origin.y + in_window.size.height);
    Some(crate::app::capture_overlay::SystemPicture {
      x: (in_window.origin.x * scale).round() as i32,
      y: (top * scale).round() as i32,
      width: width as u32,
      height: rows as u32,
      rgba,
    })
  }
}
