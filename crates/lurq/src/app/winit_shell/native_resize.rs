//! The winit window as the [`NativeWindow`] a resize is applied to, the same on Windows and macOS. On the
//! event-loop thread winit applies these calls before returning on Windows; on macOS leaving full screen (and
//! deminiaturizing) is animated, which the resize outcome reports as a mode the window is still in.

use winit::{dpi::PhysicalSize, window::Window};

use crate::app::window::resize::{NativeWindow, WindowModes};

impl NativeWindow for Window {
  fn modes(&self) -> WindowModes {
    WindowModes {
      minimized: self.is_minimized() == Some(true),
      maximized: self.is_maximized(),
      full_screen: self.fullscreen().is_some(),
    }
  }

  fn inner_size(&self) -> (u32, u32) {
    let size = Window::inner_size(self);
    (size.width, size.height)
  }

  fn set_minimized(&self, minimized: bool) {
    Window::set_minimized(self, minimized);
  }

  fn set_maximized(&self, maximized: bool) {
    Window::set_maximized(self, maximized);
  }

  fn leave_full_screen(&self) {
    self.set_fullscreen(None);
  }

  fn request_inner_size(&self, width: u32, height: u32) {
    // `Some` would be the size already applied; the outcome reads the size back either way.
    let _ = Window::request_inner_size(self, PhysicalSize::new(width, height));
  }
}
