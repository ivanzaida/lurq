//! Resizing a native window that may be minimized, maximized or full screen, and reporting the size it really took.
//!
//! Windows does not resize a minimized window (it keeps its minimized frame), and a maximized or full-screen window
//! returns to its mode's frame, so a resize first brings the window back to a normal one. The winit shell applies
//! every `WindowCommand::Resize` through [`Window::apply_resize`]; tests drive it with a simulated window.

use super::{Window, WindowCommand};

/// The modes that keep a window from taking a size.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct WindowModes {
  pub(crate) minimized: bool,
  pub(crate) maximized: bool,
  pub(crate) full_screen: bool,
}

impl WindowModes {
  #[cfg_attr(not(feature = "mcp"), allow(dead_code))]
  pub(crate) fn any(self) -> bool {
    self.minimized || self.maximized || self.full_screen
  }

  /// The set modes by their `lurq_windows` names.
  #[cfg_attr(not(feature = "mcp"), allow(dead_code))]
  pub(crate) fn names(self) -> Vec<&'static str> {
    [
      (self.minimized, "minimized"),
      (self.maximized, "maximized"),
      (self.full_screen, "full_screen"),
    ]
    .into_iter()
    .filter_map(|(set, name)| set.then_some(name))
    .collect()
  }
}

/// What a shell's native window reports and accepts during a resize: the winit shell implements it for its window,
/// tests for a simulated one. Sizes are physical pixels of the client area.
pub(crate) trait NativeWindow {
  fn modes(&self) -> WindowModes;
  fn inner_size(&self) -> (u32, u32);
  fn set_minimized(&self, minimized: bool);
  fn set_maximized(&self, maximized: bool);
  fn leave_full_screen(&self);
  fn request_inner_size(&self, width: u32, height: u32);
}

/// How a resize ended, as the native window reported it right after.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ResizeOutcome {
  pub(crate) requested: (u32, u32),
  pub(crate) size: (u32, u32),
  /// The modes the window was taken out of so that it could take the size.
  pub(crate) left: WindowModes,
  /// The modes it is still in: the platform refused to leave them, or has not finished (macOS animates leaving full
  /// screen).
  pub(crate) remaining: WindowModes,
}

/// Leaves full screen, then minimized, then maximized (a window minimized from maximized comes back maximized), and
/// requests the client size.
pub(crate) fn resize_native(window: &impl NativeWindow, width: u32, height: u32) -> ResizeOutcome {
  let mut left = window.modes();
  if left.full_screen {
    window.leave_full_screen();
  }
  if left.minimized {
    window.set_minimized(false);
  }
  left.maximized |= window.modes().maximized;
  if left.maximized {
    window.set_maximized(false);
  }
  window.request_inner_size(width, height);
  ResizeOutcome {
    requested: (width, height),
    size: window.inner_size(),
    left,
    remaining: window.modes(),
  }
}

/// Receives how a reported resize ended: `None` when there was no native window to resize (not created yet, or
/// closed).
#[cfg_attr(not(feature = "mcp"), allow(dead_code))]
pub(crate) type ResizeReport = Box<dyn FnOnce(Option<ResizeOutcome>) + Send + Sync>;

/// Pairs a queued `WindowCommand::Resize` with the report waiting for it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ResizeTicket(u64);

#[derive(Default)]
pub(super) struct ResizeReports {
  next: u64,
  pending: Vec<(ResizeTicket, ResizeReport)>,
}

impl Window {
  /// Queues a resize whose outcome goes to `report` once the shell has applied it. When no shell applies this
  /// window's commands (a headless tree), `report` gets `None` at once.
  #[cfg_attr(not(feature = "mcp"), allow(dead_code))]
  pub(crate) fn resize_reported(&self, width: u32, height: u32, report: ResizeReport) {
    let ticket = {
      let mut inner = self.inner.write().unwrap();
      if inner.shell_attached {
        let ticket = ResizeTicket(inner.resize_reports.next);
        inner.resize_reports.next += 1;
        inner.resize_reports.pending.push((ticket, report));
        Some(ticket)
      } else {
        drop(inner);
        report(None);
        None
      }
    };
    if let Some(ticket) = ticket {
      self.push_command(WindowCommand::Resize {
        width,
        height,
        report: Some(ticket),
      });
    }
  }

  /// Applies a queued resize to the native window, if there is one, and hands the outcome to the report that asked
  /// for it.
  #[cfg_attr(not(feature = "winit"), allow(dead_code))]
  pub(crate) fn apply_resize(
    &self,
    native: Option<&impl NativeWindow>,
    width: u32,
    height: u32,
    ticket: Option<ResizeTicket>,
  ) {
    let outcome = native.map(|native| resize_native(native, width, height));
    let Some(ticket) = ticket else {
      return;
    };
    let report = {
      let mut inner = self.inner.write().unwrap();
      let pending = &mut inner.resize_reports.pending;
      pending
        .iter()
        .position(|(waiting, _)| *waiting == ticket)
        .map(|index| pending.remove(index).1)
    };
    // Outside the lock: the report may read the window.
    if let Some(report) = report {
      report(outcome);
    }
  }
}

/// The size last reported to an app's `on_size_changed`. A minimized window's client area is not its size (a sliver
/// on Windows), so it is never reported, and a size is reported once, however many events repeat it.
#[derive(Default)]
#[cfg_attr(not(feature = "winit"), allow(dead_code))]
pub(crate) struct SizeReporter {
  last: Option<(u32, u32)>,
}

impl SizeReporter {
  /// The size to report, if any, for a window now at `size` (logical pixels).
  #[cfg_attr(not(feature = "winit"), allow(dead_code))]
  pub(crate) fn next(&mut self, minimized: bool, size: (u32, u32)) -> Option<(u32, u32)> {
    if minimized || self.last == Some(size) {
      return None;
    }
    self.last = Some(size);
    Some(size)
  }
}
