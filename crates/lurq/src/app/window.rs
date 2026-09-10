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
        inner.commands.push(WindowCommand::Close);
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

#[derive(Clone)]
pub struct WindowHandle {
  info: WindowInfo,
  window: Window,
}

impl WindowHandle {
  pub fn info(&self) -> WindowInfo {
    self.info
  }

  /// Replace this window's OS/app close handler. Runs on the event-loop thread.
  pub fn on_close_requested(&self, handler: impl Fn(CloseRequest) + Send + Sync + 'static) {
    self.window.inner.write().unwrap().close_handler = Some(Arc::new(handler));
  }

  pub fn clear_close_requested_handler(&self) {
    self.window.inner.write().unwrap().close_handler = None;
  }

  /// Queue a vetoable request; unlike `close`, this invokes the handler.
  pub fn request_close(&self) {
    self
      .window
      .push_command(WindowCommand::RequestClose(CloseRequestSource::App));
  }

  /// Close unconditionally, without invoking the close handler.
  pub fn close(&self) {
    self.window.push_command(WindowCommand::Close);
  }

  pub fn set_minimized(&self, minimized: bool) {
    self.window.push_command(WindowCommand::SetMinimized(minimized));
  }

  pub fn set_maximized(&self, maximized: bool) {
    self.window.push_command(WindowCommand::SetMaximized(maximized));
  }

  pub fn set_full_screen(&self, full_screen: bool) {
    self.window.push_command(WindowCommand::SetFullScreen(full_screen));
  }

  pub fn set_decorated(&self, decorated: bool) {
    self.window.push_command(WindowCommand::SetDecorated(decorated));
  }

  pub fn set_decorations(&self, decorations: bool) {
    self.set_decorated(decorations);
  }

  pub fn set_title_bar_color(&self, color: impl Into<Option<Color>>) {
    self.window.push_command(WindowCommand::SetTitleBarColor(color.into()));
  }

  pub fn clear_title_bar_color(&self) {
    self.set_title_bar_color(None);
  }

  pub fn set_icon(&self, icon: impl Into<Option<WindowIcon>>) {
    self.window.push_command(WindowCommand::SetIcon(icon.into()));
  }

  pub fn clear_icon(&self) {
    self.set_icon(None);
  }

  pub fn set_corner_radius(&self, radius: WindowCornerRadius) {
    self.window.push_command(WindowCommand::SetCornerRadius(radius));
  }

  pub fn set_rounded_corners(&self, rounded: bool) {
    self.set_corner_radius(if rounded {
      WindowCornerRadius::Rounded
    } else {
      WindowCornerRadius::None
    });
  }

  pub fn reset_corner_radius(&self) {
    self.set_corner_radius(WindowCornerRadius::Default);
  }

  pub fn r#move(&self, x: i32, y: i32) {
    self.window.push_command(WindowCommand::Move { x, y });
  }

  pub fn move_to(&self, x: i32, y: i32) {
    self.r#move(x, y);
  }

  pub fn resize(&self, width: u32, height: u32) {
    self.window.push_command(WindowCommand::Resize { width, height });
  }

  pub fn start_drag(&self) {
    self.window.push_command(WindowCommand::StartDrag);
  }

  pub fn start_resize(&self, direction: WindowResizeDirection) {
    self.window.push_command(WindowCommand::StartResize(direction));
  }

  pub fn stop_drag(&self) {
    self.window.push_command(WindowCommand::StopDrag);
  }

  /// Queue one synthetic input event, delivered inside the shell's event loop
  /// exactly where the matching OS event would have been.
  ///
  /// Positions are physical pixels in the window's coordinate space — the same
  /// units a captured frame is measured in.
  pub fn inject_input(&self, input: crate::app::synthetic_input::SyntheticInput) {
    self.window.push_command(WindowCommand::InjectInput(input));
  }

  /// Queue several synthetic events in order; see [`Self::inject_input`].
  pub fn inject_inputs(&self, inputs: impl IntoIterator<Item = crate::app::synthetic_input::SyntheticInput>) {
    for input in inputs {
      self.inject_input(input);
    }
  }

  /// Captures the next rendered window frame to a PNG file.
  #[cfg(feature = "screenshot")]
  pub fn screenshot(&self, output_path: impl Into<std::path::PathBuf>) {
    self
      .window
      .push_command(WindowCommand::Screenshot(output_path.into(), None));
  }

  /// Captures the next rendered frame cropped to a logical-pixel window
  /// region. The region is converted to physical pixels and clamped to the
  /// viewport at capture time; a region with no visible area is skipped with
  /// a warning.
  #[cfg(feature = "screenshot")]
  pub fn screenshot_region(&self, output_path: impl Into<std::path::PathBuf>, region: ScreenshotRegion) {
    self
      .window
      .push_command(WindowCommand::Screenshot(output_path.into(), Some(region)));
  }

  /// Captures the next rendered frame cropped to one element's bounds, as
  /// attached with `ref_element`. The bounds are read now, from the last
  /// completed layout pass — capture after the element has painted.
  #[cfg(feature = "screenshot")]
  pub fn screenshot_node(&self, output_path: impl Into<std::path::PathBuf>, node: &crate::core::ElementRef) {
    let bounds = node.bounds();
    self.screenshot_region(
      output_path,
      ScreenshotRegion {
        x: bounds.x,
        y: bounds.y,
        width: bounds.width,
        height: bounds.height,
      },
    );
  }

  pub fn open_devtools(&self) {
    self.window.push_command(WindowCommand::OpenDevtools);
  }

  pub fn close_devtools(&self) {
    self.window.push_command(WindowCommand::CloseDevtools);
  }

  pub fn toggle_devtools(&self) {
    self.window.push_command(WindowCommand::ToggleDevtools);
  }
}

impl Deref for WindowHandle {
  type Target = WindowInfo;

  fn deref(&self) -> &Self::Target {
    &self.info
  }
}

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
  SetTitleBarColor(Option<Color>),
  SetIcon(Option<WindowIcon>),
  SetCornerRadius(WindowCornerRadius),
  Move {
    x: i32,
    y: i32,
  },
  Resize {
    width: u32,
    height: u32,
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
  info: WindowInfo,
  corner_radius: WindowCornerRadius,
  version: u64,
  commands: Vec<WindowCommand>,
  /// Registered by the shell once its event loop exists. Without it, a
  /// cross-thread `push_command` sat unprocessed while the loop idled in
  /// `ControlFlow::Wait`.
  waker: Option<WindowWaker>,
  close_handler: Option<CloseHandler>,
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
        version: 0,
        commands: Vec::new(),
        waker: None,
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
      inner.commands.push(command);
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

  /// Wakes the host for paint work without queuing a component/window mutation.
  #[cfg(feature = "canvas")]
  pub(crate) fn wake(&self) {
    let waker = self.inner.read().unwrap().waker.clone();
    if let Some(waker) = waker {
      waker();
    }
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

  fn bump_version(inner: &mut WindowInner) -> u64 {
    inner.version = inner.version.wrapping_add(1);
    inner.version
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn window_icon_from_rgba_stores_dimensions_and_pixels() {
    let icon = WindowIcon::from_rgba(vec![255, 0, 0, 255], 1, 1);

    assert_eq!(icon.width(), 1);
    assert_eq!(icon.height(), 1);
    assert_eq!(icon.rgba(), &[255, 0, 0, 255]);
  }

  #[test]
  #[should_panic]
  fn window_icon_from_rgba_rejects_wrong_pixel_count() {
    WindowIcon::from_rgba(vec![255, 0, 0], 1, 1);
  }

  #[test]
  fn window_handle_queues_title_bar_and_icon_commands() {
    let window = Window::new();
    let handle = window.handle();
    let color = Color::from_hex("#101215");
    let icon = WindowIcon::from_rgba(vec![255, 0, 0, 255], 1, 1);

    handle.set_title_bar_color(color);
    handle.set_icon(icon.clone());
    handle.set_corner_radius(WindowCornerRadius::RoundedSmall);
    handle.clear_title_bar_color();
    handle.clear_icon();
    handle.reset_corner_radius();

    assert_eq!(
      window.take_commands(),
      vec![
        WindowCommand::SetTitleBarColor(Some(color)),
        WindowCommand::SetIcon(Some(icon)),
        WindowCommand::SetCornerRadius(WindowCornerRadius::RoundedSmall),
        WindowCommand::SetTitleBarColor(None),
        WindowCommand::SetIcon(None),
        WindowCommand::SetCornerRadius(WindowCornerRadius::Default),
      ]
    );
  }
}

#[cfg(test)]
mod close_tests {
  use std::sync::{
    Mutex,
    atomic::{AtomicUsize, Ordering},
  };

  use super::*;

  #[test]
  fn close_veto_cancel_drop_and_delayed_proceed() {
    let window = Window::new();
    window.handle().on_close_requested(|request| {
      assert_eq!(request.source(), CloseRequestSource::Os);
      request.cancel();
    });
    window.dispatch_close_request(CloseRequestSource::Os);
    assert!(window.take_shell_commands().is_empty());
    window.handle().on_close_requested(drop);
    window.dispatch_close_request(CloseRequestSource::Os);
    assert!(window.take_shell_commands().is_empty());
    let pending = Arc::new(Mutex::new(None));
    let save = pending.clone();
    window
      .handle()
      .on_close_requested(move |request| *save.lock().unwrap() = Some(request));
    window.handle().request_close();
    assert!(window.take_shell_commands().is_empty());
    let request = pending.lock().unwrap().take().unwrap();
    assert_eq!(request.source(), CloseRequestSource::App);
    std::thread::spawn(move || request.proceed()).join().unwrap();
    assert_eq!(window.take_shell_commands(), vec![WindowCommand::Close]);
  }

  #[test]
  fn close_bypasses_handler_and_no_handler_preserves_behavior() {
    let window = Window::new();
    let calls = Arc::new(AtomicUsize::new(0));
    let count = calls.clone();
    window.handle().on_close_requested(move |_| {
      count.fetch_add(1, Ordering::SeqCst);
    });
    window.handle().close();
    assert_eq!(window.take_shell_commands(), vec![WindowCommand::Close]);
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    window.handle().clear_close_requested_handler();
    for source in [CloseRequestSource::Os, CloseRequestSource::App] {
      window.dispatch_close_request(source);
      assert_eq!(window.take_shell_commands(), vec![WindowCommand::Close]);
    }
  }

  #[test]
  fn proceed_wakes_event_loop_and_request_does_not_own_window() {
    let window = Window::new();
    let pending = Arc::new(Mutex::new(None));
    let save = pending.clone();
    window
      .handle()
      .on_close_requested(move |r| *save.lock().unwrap() = Some(r));
    let wakes = Arc::new(AtomicUsize::new(0));
    let count = wakes.clone();
    window.set_waker(Arc::new(move || {
      count.fetch_add(1, Ordering::SeqCst);
    }));
    window.dispatch_close_request(CloseRequestSource::Os);
    pending.lock().unwrap().take().unwrap().proceed();
    assert_eq!(wakes.load(Ordering::SeqCst), 1);
    window.take_commands();
    window.dispatch_close_request(CloseRequestSource::Os);
    let stale = pending.lock().unwrap().take().unwrap();
    drop(window);
    stale.proceed();
    assert_eq!(wakes.load(Ordering::SeqCst), 1);
  }

  #[test]
  fn reentrant_requests_are_deferred_and_registration_is_reentrant() {
    let window = Window::new();
    let handle = window.handle();
    let reentrant = handle.clone();
    handle.on_close_requested(move |r| {
      reentrant.clear_close_requested_handler();
      reentrant.request_close();
      r.cancel();
    });
    handle.request_close();
    assert!(window.take_shell_commands().is_empty());
    assert_eq!(window.take_shell_commands(), vec![WindowCommand::Close]);
  }
}
