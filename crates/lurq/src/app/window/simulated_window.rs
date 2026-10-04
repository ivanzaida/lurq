//! A native window for tests of the resize sequence, behaving like a Windows window: a minimized window shows a
//! sliver of client area and ignores sizes, and a maximized or full-screen one keeps its mode's frame.

use std::cell::RefCell;

use super::resize::{NativeWindow, WindowModes};

/// The client area Windows reported for a minimized window on one machine.
pub(crate) const MINIMIZED_CLIENT: (u32, u32) = (237, 39);
pub(crate) const MAXIMIZED_CLIENT: (u32, u32) = (1920, 1040);
pub(crate) const FULL_SCREEN_CLIENT: (u32, u32) = (1920, 1080);

pub(crate) struct SimulatedWindow {
  state: RefCell<State>,
}

struct State {
  normal: (u32, u32),
  modes: WindowModes,
  /// Minimized from maximized: restoring brings the window back maximized.
  maximized_under_minimized: bool,
  refuses_restore: bool,
  calls: Vec<String>,
}

impl SimulatedWindow {
  pub(crate) fn new(normal: (u32, u32), modes: WindowModes) -> Self {
    Self {
      state: RefCell::new(State {
        normal,
        modes,
        maximized_under_minimized: false,
        refuses_restore: false,
        calls: Vec::new(),
      }),
    }
  }

  /// Minimized from maximized, reporting itself only minimized until restored.
  pub(crate) fn maximized_under_minimized(self) -> Self {
    self.state.borrow_mut().maximized_under_minimized = true;
    self
  }

  /// A platform that does not restore a minimized window.
  pub(crate) fn refusing_restore(self) -> Self {
    self.state.borrow_mut().refuses_restore = true;
    self
  }

  pub(crate) fn calls(&self) -> Vec<String> {
    self.state.borrow().calls.clone()
  }
}

impl NativeWindow for SimulatedWindow {
  fn modes(&self) -> WindowModes {
    self.state.borrow().modes
  }

  fn inner_size(&self) -> (u32, u32) {
    let state = self.state.borrow();
    if state.modes.minimized {
      MINIMIZED_CLIENT
    } else if state.modes.full_screen {
      FULL_SCREEN_CLIENT
    } else if state.modes.maximized {
      MAXIMIZED_CLIENT
    } else {
      state.normal
    }
  }

  fn set_minimized(&self, minimized: bool) {
    let mut state = self.state.borrow_mut();
    state.calls.push(format!("set_minimized({minimized})"));
    if minimized || !state.refuses_restore {
      state.modes.minimized = minimized;
      if !minimized && state.maximized_under_minimized {
        state.modes.maximized = true;
      }
    }
  }

  fn set_maximized(&self, maximized: bool) {
    let mut state = self.state.borrow_mut();
    state.calls.push(format!("set_maximized({maximized})"));
    state.modes.maximized = maximized;
  }

  fn leave_full_screen(&self) {
    let mut state = self.state.borrow_mut();
    state.calls.push("leave_full_screen".into());
    state.modes.full_screen = false;
  }

  fn request_inner_size(&self, width: u32, height: u32) {
    let mut state = self.state.borrow_mut();
    state.calls.push(format!("request_inner_size({width}x{height})"));
    if !state.modes.any() {
      state.normal = (width, height);
    }
  }
}
