use std::sync::{Arc, RwLock};

use crate::{core::Signal, layout::size::Size};

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
  version: u64,
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
        },
        version: 0,
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

  fn bump_version(inner: &mut WindowInner) -> u64 {
    inner.version = inner.version.wrapping_add(1);
    inner.version
  }
}
