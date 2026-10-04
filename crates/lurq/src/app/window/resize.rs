//! Resizing a native window that may be minimized, maximized or full screen, and reporting the size it really took.
//!
//! Windows does not resize a minimized window (it keeps its minimized frame), so every resize restores a minimized
//! window first. An app's resize leaves a maximized or full-screen window so; `lurq_resize`, which asks for an exact
//! size, takes the window out of those modes too. The winit shell applies every `WindowCommand::Resize` through
//! [`Window::apply_resize`]; tests drive it with a simulated window.

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

/// Which modes a resize takes the window out of before requesting the size.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ResizeRestore {
  /// Only minimized, which Windows does not resize. A maximized or full-screen window stays so: an app restoring a
  /// saved size or fitting its content must not take the user out of full screen. `WindowHandle::resize`.
  Minimized,
  /// Full screen, minimized and maximized, so the window can take exactly the requested size. `lurq_resize`.
  Every,
}

/// A queued resize: the client size in physical pixels, what it restores first, and the report waiting for it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ResizeRequest {
  pub(crate) width: u32,
  pub(crate) height: u32,
  pub(crate) restore: ResizeRestore,
  pub(crate) report: Option<ResizeTicket>,
}

/// Takes the window out of the modes `restore` names, in the order full screen, minimized, maximized (a window
/// minimized from maximized comes back maximized), and requests the client size.
pub(crate) fn resize_native(
  window: &impl NativeWindow,
  width: u32,
  height: u32,
  restore: ResizeRestore,
) -> ResizeOutcome {
  let every = restore == ResizeRestore::Every;
  let mut left = WindowModes::default();
  if every && window.modes().full_screen {
    window.leave_full_screen();
    left.full_screen = true;
  }
  if window.modes().minimized {
    window.set_minimized(false);
    left.minimized = true;
  }
  if every && window.modes().maximized {
    window.set_maximized(false);
    left.maximized = true;
  }
  window.request_inner_size(width, height);
  ResizeOutcome {
    requested: (width, height),
    size: window.inner_size(),
    left,
    remaining: window.modes(),
  }
}

/// Waits for how a resize ended. `deliver` gets `None` when there was no native window to resize (not created yet,
/// or closed); `abandoned` says that nobody waits any more (an MCP call that timed out), so the report can go.
#[cfg_attr(not(feature = "mcp"), allow(dead_code))]
pub(crate) struct ResizeReport {
  deliver: Box<dyn FnOnce(Option<ResizeOutcome>) + Send + Sync>,
  abandoned: Box<dyn Fn() -> bool + Send + Sync>,
}

impl ResizeReport {
  #[cfg_attr(not(feature = "mcp"), allow(dead_code))]
  pub(crate) fn new(
    deliver: impl FnOnce(Option<ResizeOutcome>) + Send + Sync + 'static,
    abandoned: impl Fn() -> bool + Send + Sync + 'static,
  ) -> Self {
    Self {
      deliver: Box::new(deliver),
      abandoned: Box::new(abandoned),
    }
  }
}

/// Pairs a queued `WindowCommand::Resize` with the report waiting for it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ResizeTicket(u64);

#[derive(Default)]
pub(super) struct ResizeReports {
  next: u64,
  pending: Vec<(ResizeTicket, ResizeReport)>,
}

impl Window {
  /// Queues a resize that takes the window out of every mode first, whose outcome goes to `report` once the shell
  /// has applied it. When no shell applies this window's commands (a headless tree), `report` gets `None` at once.
  ///
  /// Reports nobody waits for any more, and reports whose command left the queue without being applied, are dropped
  /// here so they cannot accumulate. Only here: the shell applies a batch of commands it took without new reports
  /// being queued in between.
  #[cfg_attr(not(feature = "mcp"), allow(dead_code))]
  pub(crate) fn resize_reported(&self, width: u32, height: u32, report: ResizeReport) {
    let mut guard = self.inner.write().unwrap();
    if !guard.shell_attached {
      drop(guard);
      (report.deliver)(None);
      return;
    }
    let inner = &mut *guard;
    let commands = &inner.commands;
    inner.resize_reports.pending.retain(|(ticket, report)| {
      let queued = commands
        .iter()
        .any(|command| matches!(command, WindowCommand::Resize(request) if request.report == Some(*ticket)));
      queued && !(report.abandoned)()
    });
    let ticket = ResizeTicket(inner.resize_reports.next);
    inner.resize_reports.next += 1;
    inner.resize_reports.pending.push((ticket, report));
    let waker = inner.queue_resize(ResizeRequest {
      width,
      height,
      restore: ResizeRestore::Every,
      report: Some(ticket),
    });
    drop(guard);
    // Outside the lock: the waker may re-enter window state.
    if let Some(waker) = waker {
      waker();
    }
  }

  /// Applies a queued resize to the native window, if there is one, and hands the outcome to the report that asked
  /// for it.
  #[cfg_attr(not(feature = "winit"), allow(dead_code))]
  pub(crate) fn apply_resize(&self, native: Option<&impl NativeWindow>, request: ResizeRequest) {
    let outcome = native.map(|native| resize_native(native, request.width, request.height, request.restore));
    let Some(ticket) = request.report else {
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
      (report.deliver)(outcome);
    }
  }

  #[cfg(test)]
  pub(crate) fn pending_resize_reports(&self) -> usize {
    self.inner.read().unwrap().resize_reports.pending.len()
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
