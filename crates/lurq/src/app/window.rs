use std::{
  ops::Deref,
  sync::{Arc, RwLock},
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

  #[cfg(feature = "image")]
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

#[derive(Clone)]
pub struct WindowHandle {
  info: WindowInfo,
  window: Window,
}

impl WindowHandle {
  pub fn info(&self) -> WindowInfo {
    self.info
  }

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

  /// Captures the next rendered window frame to a PNG file.
  #[cfg(feature = "screenshot")]
  pub fn screenshot(&self, output_path: impl Into<std::path::PathBuf>) {
    self.window.push_command(WindowCommand::Screenshot(output_path.into()));
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum WindowCommand {
  Close,
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
  #[cfg(feature = "screenshot")]
  Screenshot(std::path::PathBuf),
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

struct WindowInner {
  info: WindowInfo,
  corner_radius: WindowCornerRadius,
  version: u64,
  commands: Vec<WindowCommand>,
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
      })),
      version_signal: Signal::new(0),
    }
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

  fn push_command(&self, command: WindowCommand) {
    self.inner.write().unwrap().commands.push(command);
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
