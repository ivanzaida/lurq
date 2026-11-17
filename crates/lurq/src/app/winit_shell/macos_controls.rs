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
//! are moved again after every frame that finds them elsewhere. In full screen the system hides its lights with
//! its bar until the pointer is at the top of the screen, and the app draws its own again: the zoom one leaves
//! full screen.

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
    // In full screen the system hides its lights with its bar, until the pointer is at the top of the screen: the
    // app draws its own there, so that the way back is seen.
    tree.window().set_native_controls(any && window.fullscreen().is_none());
  }
}

fn same(a: NSRect, b: NSRect) -> bool {
  (a.origin.x - b.origin.x).abs() <= 0.25
    && (a.origin.y - b.origin.y).abs() <= 0.25
    && (a.size.width - b.size.width).abs() <= 0.25
    && (a.size.height - b.size.height).abs() <= 0.25
}
