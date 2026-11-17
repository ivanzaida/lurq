//! Where the window's controls are, in a title bar the app draws itself.
//!
//! An element says it is one of the window's controls with `.window_control(..)`. After each layout the tree finds
//! those elements (`runtime::window_controls`) and, for a shell that asked, leaves here where each is and what covers
//! it. The shell reads it when the system asks what is at a point of the window: on Windows, the window procedure of
//! `app::window_control_hits`, at the system's hit test.

use crate::node::WindowControl;

/// A rectangle of the window, in logical pixels: from its first corner up to, and not with, its second.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Bounds {
  pub(crate) x0: f32,
  pub(crate) y0: f32,
  pub(crate) x1: f32,
  pub(crate) y1: f32,
}

impl Bounds {
  /// The whole plane: what nothing clips.
  pub(crate) const EVERYWHERE: Self = Self {
    x0: f32::NEG_INFINITY,
    y0: f32::NEG_INFINITY,
    x1: f32::INFINITY,
    y1: f32::INFINITY,
  };

  pub(crate) fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
    Self {
      x0: x,
      y0: y,
      x1: x + width,
      y1: y + height,
    }
  }

  /// What is in both rectangles; empty where they do not meet.
  pub(crate) fn intersect(self, other: Self) -> Self {
    Self {
      x0: self.x0.max(other.x0),
      y0: self.y0.max(other.y0),
      x1: self.x1.min(other.x1),
      y1: self.y1.min(other.y1),
    }
  }

  pub(crate) fn is_empty(self) -> bool {
    self.x1 <= self.x0 || self.y1 <= self.y0
  }

  pub(crate) fn contains(self, x: f32, y: f32) -> bool {
    x >= self.x0 && x < self.x1 && y >= self.y0 && y < self.y1
  }

  /// Whether all of `other` is in this rectangle.
  pub(crate) fn holds(self, other: Self) -> bool {
    self.x0 <= other.x0 && self.y0 <= other.y0 && self.x1 >= other.x1 && self.y1 >= other.y1
  }
}

/// One of the window's controls as it is laid out: what of it is seen, and the elements over it that take the
/// pointer in its place.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct WindowControlRegion {
  pub(crate) control: WindowControl,
  /// The element's box, cut by what clips it.
  pub(crate) bounds: Bounds,
  /// What is hit before the element inside its box: a zone along the window's edge that resizes it, a dialog's
  /// backdrop.
  pub(crate) covers: Vec<Bounds>,
}

impl WindowControlRegion {
  fn contains(&self, x: f32, y: f32) -> bool {
    self.bounds.contains(x, y) && !self.covers.iter().any(|cover| cover.contains(x, y))
  }
}

/// The window's controls a tree found at a layout, in the order it draws them.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct WindowControlRegions(Vec<WindowControlRegion>);

impl WindowControlRegions {
  pub(crate) fn is_empty(&self) -> bool {
    self.0.is_empty()
  }

  pub(crate) fn push(&mut self, region: WindowControlRegion) {
    self.0.push(region);
  }

  /// Where `control` is: the middle of its box, in logical pixels. Of the last drawn of them.
  #[cfg_attr(not(all(feature = "winit", target_os = "macos")), allow(dead_code))]
  pub(crate) fn middle_of(&self, control: WindowControl) -> Option<(f32, f32)> {
    self
      .0
      .iter()
      .rev()
      .find(|region| region.control == control && !region.bounds.is_empty())
      .map(|region| {
        (
          (region.bounds.x0 + region.bounds.x1) / 2.0,
          (region.bounds.y0 + region.bounds.y1) / 2.0,
        )
      })
  }

  /// The control at a point of the window, in logical pixels: the last drawn of those the point is on.
  pub(crate) fn at(&self, x: f32, y: f32) -> Option<WindowControl> {
    self
      .0
      .iter()
      .rev()
      .find(|region| region.contains(x, y))
      .map(|region| region.control)
  }
}
