use std::{
  ops::Deref,
  sync::{Arc, RwLock, Weak},
};

use crate::{core::Signal, layout::size::Size, node::color::Color};

/// A snapshot of the window's geometry, returned by `ctx.window()`.
///
/// `resolved_*` values are in physical device pixels; `logical_*` divide those
/// by the scale factor and match the coordinate space layout reasons in.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WindowInfo {
  pub x: i32,
  pub y: i32,
  pub resolved_width: f32,
  pub resolved_height: f32,
  pub scale_factor: f32,
  pub is_minimized: bool,
  pub is_maximized: bool,
  pub is_full_screen: bool,
  pub is_decorated: bool,
  pub is_focused: bool,
}

impl WindowInfo {
  pub fn position(&self) -> (i32, i32) {
    (self.x, self.y)
  }

  pub fn resolved_size(&self) -> Size {
    Size::new(self.resolved_width, self.resolved_height)
  }

  pub fn logical_size(&self) -> Size {
    let scale = self.scale_factor.max(f32::EPSILON);
    Size::new(self.resolved_width / scale, self.resolved_height / scale)
  }

  pub fn logical_width(&self) -> f32 {
    self.logical_size().width
  }

  pub fn logical_height(&self) -> f32 {
    self.logical_size().height
  }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WindowResizeDirection {
  East,
  North,
  NorthEast,
  NorthWest,
  South,
  SouthEast,
  SouthWest,
  West,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WindowCornerRadius {
  Default,
  None,
  Rounded,
  RoundedSmall,
}

/// The 1px frame the OS compositor draws around a window. Windows 11 draws it around undecorated
/// windows too (DWM `DWMWA_BORDER_COLOR`); other platforms ignore it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WindowBorderColor {
  /// The system color.
  #[default]
  Default,
  /// No compositor border.
  None,
  Color(Color),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WindowIcon {
  rgba: Vec<u8>,
  width: u32,
  height: u32,
}

impl WindowIcon {
  pub fn from_rgba(rgba: Vec<u8>, width: u32, height: u32) -> Self {
    assert_eq!(rgba.len(), (width * height * 4) as usize);
    Self { rgba, width, height }
  }

  #[cfg(feature = "raster")]
  pub fn from_image_data(image: &crate::images::ImageData) -> Self {
    Self::from_rgba((*image.data_arc()).clone(), image.width(), image.height())
  }

  pub fn rgba(&self) -> &[u8] {
    &self.rgba
  }

  pub fn width(&self) -> u32 {
    self.width
  }

  pub fn height(&self) -> u32 {
    self.height
  }

  #[cfg_attr(not(feature = "winit"), allow(dead_code))]
  pub(crate) fn into_rgba(self) -> (Vec<u8>, u32, u32) {
    (self.rgba, self.width, self.height)
  }
}

/// Origin of a vetoable close request. Programmatic `close()` bypasses this path.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CloseRequestSource {
  Os,
  App,
}

/// A one-shot decision that may be moved to another thread or retained for a dialog.
/// Dropping it, or calling `cancel`, keeps the window open. A stale request cannot
/// close a replacement window: it refers only to its original window's queue.
pub struct CloseRequest {
  window: Weak<RwLock<WindowInner>>,
  source: CloseRequestSource,
}

impl CloseRequest {
  pub fn source(&self) -> CloseRequestSource {
    self.source
  }
  pub fn proceed(self) {
    if let Some(window) = self.window.upgrade() {
      let waker = {
        let mut inner = window.write().unwrap();
        inner.queue_command(WindowCommand::Close);
        inner.waker.clone()
      };
      if let Some(waker) = waker {
        waker();
      }
    }
  }
  pub fn cancel(self) {}
}

type CloseHandler = Arc<dyn Fn(CloseRequest) + Send + Sync>;

mod handle;
pub(crate) mod resize;
#[cfg(test)]
mod resize_tests;
#[cfg(test)]
pub(crate) mod simulated_window;
#[cfg(feature = "mcp")]
pub(crate) use handle::DialogWindow;
pub use handle::WindowHandle;
use resize::ResizeTicket;

/// A logical-pixel window region for a partial frame capture, matching the
/// units of [`crate::core::ElementRect`] bounds.
#[cfg(feature = "screenshot")]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScreenshotRegion {
  pub x: f32,
  pub y: f32,
  pub width: f32,
  pub height: f32,
}

// Not `Eq`: synthetic input carries pixel positions.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum WindowCommand {
  Close,
  RequestClose(CloseRequestSource),
  SetMinimized(bool),
  SetMaximized(bool),
  SetFullScreen(bool),
  SetDecorated(bool),
  SetTitle(String),
  SetTitleBarColor(Option<Color>),
  SetBorderColor(WindowBorderColor),
  SetIcon(Option<WindowIcon>),
  SetCornerRadius(WindowCornerRadius),
  Move {
    x: i32,
    y: i32,
  },
  /// Applied after leaving minimized, maximized and full screen; `report` names the waiting
  /// [`resize::ResizeReport`], if any.
  Resize {
    width: u32,
    height: u32,
    report: Option<ResizeTicket>,
  },
  StartDrag,
  StartResize(WindowResizeDirection),
  StopDrag,
  InjectInput(crate::app::synthetic_input::SyntheticInput),
  #[cfg(feature = "screenshot")]
  Screenshot(std::path::PathBuf, Option<ScreenshotRegion>),
  OpenDevtools,
  CloseDevtools,
  ToggleDevtools,
}

/// Reactive, per-window geometry handle held by the `Tree` and injected into
/// its root `Ctx`. Reads through `ctx.window()` subscribe to changes via the
/// version signal; the shell pushes resize/scale/move updates here.
#[derive(Clone)]
pub struct Window {
  inner: Arc<RwLock<WindowInner>>,
  version_signal: Signal<u64>,
}

/// Wakes the shell's event loop so a command pushed from another thread is
/// processed without waiting for the next OS event.
pub(crate) type WindowWaker = Arc<dyn Fn() + Send + Sync>;

struct WindowInner {
  #[cfg(feature = "mcp")]
  accepted_close: bool,
  info: WindowInfo,
  corner_radius: WindowCornerRadius,
  border_color: WindowBorderColor,
  /// `None` until the shell creates the window or the app sets a title.
  title: Option<String>,
  version: u64,
  commands: Vec<WindowCommand>,
  /// Registered by the shell once its event loop exists. Without it, a
  /// cross-thread `push_command` sat unprocessed while the loop idled in
  /// `ControlFlow::Wait`.
  waker: Option<WindowWaker>,
  /// Set once a platform shell applies this window's commands; a headless tree has none.
  shell_attached: bool,
  resize_reports: resize::ResizeReports,
  close_handler: Option<CloseHandler>,
}

impl WindowInner {
  fn queue_command(&mut self, command: WindowCommand) {
    #[cfg(feature = "mcp")]
    if command == WindowCommand::Close {
      self.accepted_close = true;
    }
    self.commands.push(command);
  }
}

impl Default for Window {
  fn default() -> Self {
    Self::new()
  }
}

impl Window {
  pub fn new() -> Self {
    Self {
      inner: Arc::new(RwLock::new(WindowInner {
        #[cfg(feature = "mcp")]
        accepted_close: false,
        info: WindowInfo {
          x: 0,
          y: 0,
          resolved_width: 0.0,
          resolved_height: 0.0,
          scale_factor: 1.0,
          is_minimized: false,
          is_maximized: false,
          is_full_screen: false,
          is_decorated: true,
          is_focused: true,
        },
        corner_radius: WindowCornerRadius::Default,
        border_color: WindowBorderColor::Default,
        title: None,
        version: 0,
        commands: Vec::new(),
        waker: None,
        shell_attached: false,
        resize_reports: resize::ResizeReports::default(),
        close_handler: None,
      })),
      version_signal: Signal::new(0),
    }
  }

  /// The single event-loop entry point for OS, queued app and MCP requests.
  pub(crate) fn dispatch_close_request(&self, source: CloseRequestSource) {
    let handler = self.inner.read().unwrap().close_handler.clone();
    let request = CloseRequest {
      window: Arc::downgrade(&self.inner),
      source,
    };
    // Never hold the state lock across application code (which may close/register).
    if let Some(handler) = handler {
      handler(request);
    } else {
      request.proceed();
    }
  }

  #[cfg(feature = "mcp")]
  pub(crate) fn close_queued(&self) -> bool {
    self.inner.read().unwrap().commands.contains(&WindowCommand::Close)
  }

  pub(crate) fn track_access(&self) {
    let _ = self.version_signal.get();
  }

  pub(crate) fn info(&self) -> WindowInfo {
    self.inner.read().unwrap().info
  }

  #[allow(dead_code)]
  pub(crate) fn corner_radius(&self) -> WindowCornerRadius {
    self.inner.read().unwrap().corner_radius
  }

  pub(crate) fn handle(&self) -> WindowHandle {
    WindowHandle {
      info: self.info(),
      window: self.clone(),
    }
  }

  /// Records the title a window is created with, unless the app already set
  /// one (its `SetTitle` command is still queued and wins).
  pub(crate) fn record_initial_title(&self, title: &str) {
    let mut inner = self.inner.write().unwrap();
    if inner.title.is_none() {
      inner.title = Some(title.to_owned());
    }
  }

  #[cfg_attr(not(feature = "winit"), allow(dead_code))]
  pub(crate) fn take_commands(&self) -> Vec<WindowCommand> {
    std::mem::take(&mut self.inner.write().unwrap().commands)
  }

  /// Resolve one generation of app requests on the event-loop thread. Requests
  /// queued by a callback remain for the next turn, avoiding recursive handlers.
  #[cfg_attr(not(feature = "winit"), allow(dead_code))]
  pub(crate) fn take_shell_commands(&self) -> Vec<WindowCommand> {
    let mut result = Vec::new();
    for command in self.take_commands() {
      if let WindowCommand::RequestClose(source) = command {
        self.dispatch_close_request(source);
      } else {
        result.push(command);
      }
    }
    for command in self.take_commands() {
      if matches!(command, WindowCommand::RequestClose(_)) {
        self.push_command(command);
      } else {
        result.push(command);
      }
    }
    result
  }

  #[cfg(all(feature = "winit", target_os = "macos"))]
  pub(crate) fn requeue_command(&self, command: WindowCommand) {
    self.push_command(command);
  }

  fn push_command(&self, command: WindowCommand) {
    let waker = {
      let mut inner = self.inner.write().unwrap();
      inner.queue_command(command);
      inner.waker.clone()
    };
    // Invoked outside the lock: the waker may re-enter window state.
    if let Some(waker) = waker {
      waker();
    }
  }

  /// Registers the shell's event-loop waker; every subsequent `push_command`
  /// invokes it so commands queued from other threads are drained promptly.
  #[cfg_attr(not(feature = "winit"), allow(dead_code))]
  pub(crate) fn set_waker(&self, waker: WindowWaker) {
    self.inner.write().unwrap().waker = Some(waker);
  }

  /// Registers the platform shell that applies this window's commands, with its event-loop waker.
  #[cfg_attr(not(feature = "winit"), allow(dead_code))]
  pub(crate) fn attach_shell(&self, waker: WindowWaker) {
    self.set_waker(waker);
    self.inner.write().unwrap().shell_attached = true;
  }

  /// Wakes the host for paint work without queuing a component/window mutation.
  pub(crate) fn wake(&self) {
    let waker = self.inner.read().unwrap().waker.clone();
    if let Some(waker) = waker {
      waker();
    }
  }

  #[cfg(feature = "query")]
  pub(crate) fn query_waker(&self) -> WindowWaker {
    let window = Arc::downgrade(&self.inner);
    Arc::new(move || {
      let wake = window.upgrade().and_then(|window| window.read().unwrap().waker.clone());
      if let Some(wake) = wake {
        wake();
      }
    })
  }

  pub fn version(&self) -> u64 {
    self.inner.read().unwrap().version
  }

  pub(crate) fn set_resolved_size(&self, width: f32, height: f32) {
    let version = {
      let mut inner = self.inner.write().unwrap();
      if inner.info.resolved_width == width && inner.info.resolved_height == height {
        return;
      }
      inner.info.resolved_width = width;
      inner.info.resolved_height = height;
      Self::bump_version(&mut inner)
    };
    self.version_signal.set(version);
  }

  pub(crate) fn set_scale_factor(&self, scale_factor: f32) {
    let version = {
      let mut inner = self.inner.write().unwrap();
      if inner.info.scale_factor == scale_factor {
        return;
      }
      inner.info.scale_factor = scale_factor;
      Self::bump_version(&mut inner)
    };
    self.version_signal.set(version);
  }

  pub(crate) fn set_position(&self, x: i32, y: i32) {
    let version = {
      let mut inner = self.inner.write().unwrap();
      if inner.info.x == x && inner.info.y == y {
        return;
      }
      inner.info.x = x;
      inner.info.y = y;
      Self::bump_version(&mut inner)
    };
    self.version_signal.set(version);
  }

  #[cfg_attr(not(feature = "winit"), allow(dead_code))]
  pub(crate) fn set_minimized(&self, minimized: bool) {
    let version = {
      let mut inner = self.inner.write().unwrap();
      if inner.info.is_minimized == minimized {
        return;
      }
      inner.info.is_minimized = minimized;
      Self::bump_version(&mut inner)
    };
    self.version_signal.set(version);
  }

  #[cfg_attr(not(feature = "winit"), allow(dead_code))]
  pub(crate) fn set_maximized(&self, maximized: bool) {
    let version = {
      let mut inner = self.inner.write().unwrap();
      if inner.info.is_maximized == maximized {
        return;
      }
      inner.info.is_maximized = maximized;
      Self::bump_version(&mut inner)
    };
    self.version_signal.set(version);
  }

  #[cfg_attr(not(feature = "winit"), allow(dead_code))]
  pub(crate) fn set_full_screen(&self, full_screen: bool) {
    let version = {
      let mut inner = self.inner.write().unwrap();
      if inner.info.is_full_screen == full_screen {
        return;
      }
      inner.info.is_full_screen = full_screen;
      Self::bump_version(&mut inner)
    };
    self.version_signal.set(version);
  }

  #[cfg_attr(not(feature = "winit"), allow(dead_code))]
  pub(crate) fn set_decorated(&self, decorated: bool) {
    let version = {
      let mut inner = self.inner.write().unwrap();
      if inner.info.is_decorated == decorated {
        return;
      }
      inner.info.is_decorated = decorated;
      Self::bump_version(&mut inner)
    };
    self.version_signal.set(version);
  }

  #[cfg_attr(not(feature = "winit"), allow(dead_code))]
  pub(crate) fn set_focused(&self, focused: bool) {
    let version = {
      let mut inner = self.inner.write().unwrap();
      if inner.info.is_focused == focused {
        return;
      }
      inner.info.is_focused = focused;
      Self::bump_version(&mut inner)
    };
    self.version_signal.set(version);
  }

  #[cfg_attr(not(feature = "winit"), allow(dead_code))]
  pub(crate) fn set_corner_radius(&self, radius: WindowCornerRadius) {
    let version = {
      let mut inner = self.inner.write().unwrap();
      if inner.corner_radius == radius {
        return;
      }
      inner.corner_radius = radius;
      Self::bump_version(&mut inner)
    };
    self.version_signal.set(version);
  }

  #[cfg_attr(not(feature = "winit"), allow(dead_code))]
  pub(crate) fn set_border_color(&self, color: WindowBorderColor) {
    let version = {
      let mut inner = self.inner.write().unwrap();
      if inner.border_color == color {
        return;
      }
      inner.border_color = color;
      Self::bump_version(&mut inner)
    };
    self.version_signal.set(version);
  }

  fn bump_version(inner: &mut WindowInner) -> u64 {
    inner.version = inner.version.wrapping_add(1);
    inner.version
  }
}

#[cfg(test)]
mod tests;
