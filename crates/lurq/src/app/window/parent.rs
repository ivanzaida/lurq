//! The system's handle of a window, for a dialog of the system to be modal to it: what the shell records when it
//! creates the window, and what `files::FileDialog` gives `rfd` as the dialog's parent.

use raw_window_handle::{
  DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, RawDisplayHandle, RawWindowHandle, WindowHandle,
};

/// A window's raw handles, kept while the window lives.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ParentWindow {
  window: RawWindowHandle,
  display: RawDisplayHandle,
}

// The handles are names the system gives the window, not pointers lurq reads through: a dialog of the system is
// handed them on its own thread (a Win32 HWND is valid from any thread).
unsafe impl Send for ParentWindow {}
unsafe impl Sync for ParentWindow {}

impl ParentWindow {
  pub(crate) fn new(window: RawWindowHandle, display: RawDisplayHandle) -> Self {
    Self { window, display }
  }

  /// The handles of a window the shell made, where the system gives them.
  #[cfg_attr(not(all(feature = "winit", windows)), allow(dead_code))]
  pub(crate) fn of(window: &(impl HasWindowHandle + HasDisplayHandle)) -> Option<Self> {
    Some(Self::new(
      window.window_handle().ok()?.as_raw(),
      window.display_handle().ok()?.as_raw(),
    ))
  }

  #[cfg_attr(not(test), allow(dead_code))]
  pub(crate) fn raw(&self) -> (RawWindowHandle, RawDisplayHandle) {
    (self.window, self.display)
  }
}

impl HasWindowHandle for ParentWindow {
  fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
    // Safety: the shell records the handle of a window it made and clears it as the window goes; a dialog is
    // opened over a window that is there.
    Ok(unsafe { WindowHandle::borrow_raw(self.window) })
  }
}

impl HasDisplayHandle for ParentWindow {
  fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
    // Safety: as above.
    Ok(unsafe { DisplayHandle::borrow_raw(self.display) })
  }
}
