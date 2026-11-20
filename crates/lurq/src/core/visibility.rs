//! How much of an element shows: what a browser's `IntersectionObserver` tells, for an element with an
//! [`ElementRef`]. The share is reckoned from the layout at every pass that lays the window out, so it is right
//! whatever moved the content: a finger, its momentum, a snap to a page, a wheel, an animation that changes
//! the layout, or the app's own `set_scroll`.
//!
//! An element is measured against its *root*: the scroll container nearest around it, as much of it as the
//! containers around that one and the window show; the window, for an element nothing scrolls; or an element
//! of the app's choosing ([`VisibilityOptions::root`]). The root grown by a margin
//! ([`VisibilityOptions::margin`]) is what tells an element near the viewport from one far from it.
//!
//! The boxes are the layout's: a transform does not move them.

use std::sync::Arc;

use super::element_ref::{ElementRect, ElementRef};

/// How much of an element shows in its root, as [`ElementRef::observe_visibility`] tells it.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Visibility {
  /// The share of the element's box that is inside the root, from 0 to 1. An element of no size, a mark at
  /// the end of a list, has 1 where it lies inside the root and 0 where it does not.
  pub ratio: f32,
  /// Whether any of it is inside the root.
  pub visible: bool,
  /// The part of the element inside the root, in the window's logical pixels, its `relative_x` and
  /// `relative_y` from the root's corner; of no size where none of it is.
  pub visible_rect: ElementRect,
  /// The element's own box, placed the same way.
  pub rect: ElementRect,
  /// The box it was measured against: the root, grown by the margin.
  pub root_rect: ElementRect,
}

/// What an observer of an element's visibility is told of, and against what the element is measured.
///
/// ```
/// use lurq::core::VisibilityOptions;
///
/// // The page of a feed that all but fills its viewport is the current one.
/// let current = VisibilityOptions::new().thresholds([0.99]);
/// // A thumbnail within 200 of the viewport loads its picture.
/// let near = VisibilityOptions::new().margin(200.0);
/// ```
#[derive(Clone, Default)]
pub struct VisibilityOptions {
  thresholds: Vec<f32>,
  margin: (f32, f32),
  root: Option<ElementRef>,
}

impl std::fmt::Debug for VisibilityOptions {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("VisibilityOptions")
      .field("thresholds", &self.thresholds)
      .field("margin", &self.margin)
      .field("root", &self.root.is_some())
      .finish()
  }
}

impl VisibilityOptions {
  /// An observer told when the element comes into its root and when it leaves it: one threshold, at 0.
  pub fn new() -> Self {
    Self::default()
  }

  /// The shares, from 0 to 1, the observer is told of: it is called when the share that shows crosses one of
  /// them, either way, and not while it changes between two. `[0.99]` tells when the element all but fills
  /// its room and when it no longer does; `[0.0, 0.5, 1.0]` when it comes in, is half in and is all in. With
  /// none given there is one, at 0: the element comes in, or leaves.
  pub fn thresholds(mut self, thresholds: impl IntoIterator<Item = f32>) -> Self {
    self.thresholds = thresholds
      .into_iter()
      .filter(|share| share.is_finite())
      .map(|share| share.clamp(0.0, 1.0))
      .collect();
    self.thresholds.sort_by(f32::total_cmp);
    self.thresholds.dedup();
    self
  }

  /// Grows the root by `px` logical pixels on every side before the element is measured against it, as a
  /// browser's `rootMargin` does: with 200 an element counts as showing from 200 before it scrolls into view.
  /// A negative margin shrinks the root.
  pub fn margin(self, px: f32) -> Self {
    self.margins(px, px)
  }

  /// [`margin`](Self::margin) with one growth across and another down: a list that scrolls down opens what is
  /// a page under it with `margins(0.0, page)`.
  pub fn margins(mut self, across: f32, down: f32) -> Self {
    self.margin = (across, down);
    self
  }

  /// Measures the element against the box of `root`, an element around it, in place of the scroll container
  /// nearest around it. The root's box is taken whole, as it was last laid out, whatever of it shows.
  pub fn root(mut self, root: &ElementRef) -> Self {
    self.root = Some(root.clone());
    self
  }

  pub(crate) fn explicit_root(&self) -> Option<&ElementRef> {
    self.root.as_ref()
  }

  /// `root` grown by the margin.
  pub(crate) fn grown(&self, root: Area) -> Area {
    Area {
      x: root.x - self.margin.0,
      y: root.y - self.margin.1,
      width: (root.width + self.margin.0 * 2.0).max(0.0),
      height: (root.height + self.margin.1 * 2.0).max(0.0),
    }
  }

  /// How many of the thresholds a visibility is at or over: what changes when it crosses one. An element
  /// that does not show is over none.
  pub(crate) fn level(&self, visibility: &Visibility) -> usize {
    if !visibility.visible {
      return 0;
    }
    if self.thresholds.is_empty() {
      return 1;
    }
    self
      .thresholds
      .iter()
      .filter(|share| visibility.ratio >= **share)
      .count()
  }
}

/// A box in the window's logical pixels.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Area {
  pub x: f32,
  pub y: f32,
  pub width: f32,
  pub height: f32,
}

impl Area {
  pub(crate) fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
    Self {
      x,
      y,
      width: width.max(0.0),
      height: height.max(0.0),
    }
  }

  /// The part of this box that is inside `other`; of no size where they do not meet.
  pub(crate) fn within(self, other: Area) -> Area {
    let (x, y) = (self.x.max(other.x), self.y.max(other.y));
    let right = (self.x + self.width).min(other.x + other.width);
    let bottom = (self.y + self.height).min(other.y + other.height);
    Area::new(x, y, right - x, bottom - y)
  }

  fn rect(self, from: Area) -> ElementRect {
    ElementRect {
      x: self.x,
      y: self.y,
      relative_x: self.x - from.x,
      relative_y: self.y - from.y,
      width: self.width,
      height: self.height,
    }
  }
}

/// How much of `element` is inside `root`.
pub(crate) fn measure(element: Area, root: Area) -> Visibility {
  let right = (element.x + element.width).min(root.x + root.width);
  let bottom = (element.y + element.height).min(root.y + root.height);
  let (left, top) = (element.x.max(root.x), element.y.max(root.y));
  let shown = Area::new(left, top, right - left, bottom - top);
  let area = element.width * element.height;
  let (visible, ratio) = if area > 0.0 {
    let share = shown.width * shown.height / area;
    (share > 0.0, share.clamp(0.0, 1.0))
  } else {
    // A box of no size shows where it lies inside the root, its edges among it.
    let inside = root.width > 0.0 && root.height > 0.0 && right >= left && bottom >= top;
    (inside, if inside { 1.0 } else { 0.0 })
  };
  Visibility {
    ratio,
    visible,
    visible_rect: if visible {
      shown.rect(root)
    } else {
      Area::new(element.x, element.y, 0.0, 0.0).rect(root)
    },
    rect: element.rect(root),
    root_rect: root.rect(root),
  }
}

/// An element that is not laid out any more, dropped by what holds it: nothing of it shows.
pub(crate) fn hidden() -> Visibility {
  Visibility::default()
}

pub(crate) type VisibilityCallback = Arc<dyn Fn(Visibility) + Send + Sync>;

/// One observer of an element's visibility, as its ref keeps it.
pub(crate) struct Watch {
  pub id: u64,
  pub options: VisibilityOptions,
  pub observer: VisibilityCallback,
  /// How many thresholds the share was over when the observer was last called; `None` before it was.
  pub level: Option<usize>,
}

#[cfg(test)]
mod tests {
  use super::*;

  fn area(x: f32, y: f32, width: f32, height: f32) -> Area {
    Area::new(x, y, width, height)
  }

  #[test]
  fn the_share_is_the_part_of_the_box_inside_the_root() {
    let root = area(0.0, 0.0, 100.0, 100.0);

    let inside = measure(area(10.0, 10.0, 50.0, 50.0), root);
    assert_eq!((inside.ratio, inside.visible), (1.0, true));

    let half = measure(area(0.0, 75.0, 100.0, 50.0), root);
    assert_eq!((half.ratio, half.visible), (0.5, true));
    assert_eq!(
      (half.visible_rect.y, half.visible_rect.height),
      (75.0, 25.0),
      "the part that shows"
    );

    let below = measure(area(0.0, 100.0, 100.0, 50.0), root);
    assert_eq!(
      (below.ratio, below.visible),
      (0.0, false),
      "touching the edge is not showing"
    );
  }

  #[test]
  fn a_box_of_no_size_shows_where_it_lies_inside_the_root() {
    let root = area(0.0, 0.0, 100.0, 100.0);
    assert!(
      measure(area(0.0, 100.0, 100.0, 0.0), root).visible,
      "at the root's edge"
    );
    assert_eq!(measure(area(0.0, 40.0, 100.0, 0.0), root).ratio, 1.0);
    assert!(!measure(area(0.0, 140.0, 100.0, 0.0), root).visible, "under it");
  }

  #[test]
  fn a_margin_grows_the_root_and_a_level_counts_the_thresholds_crossed() {
    let options = VisibilityOptions::new().margin(50.0).thresholds([0.5, 0.0, 1.0, 0.5]);
    let root = options.grown(area(0.0, 0.0, 100.0, 100.0));
    assert_eq!(root, area(-50.0, -50.0, 200.0, 200.0));

    assert_eq!(
      options.level(&measure(area(0.0, 200.0, 100.0, 100.0), root)),
      0,
      "out of it"
    );
    assert_eq!(
      options.level(&measure(area(0.0, 140.0, 100.0, 100.0), root)),
      1,
      "a tenth in"
    );
    assert_eq!(
      options.level(&measure(area(0.0, 100.0, 100.0, 100.0), root)),
      2,
      "half in"
    );
    assert_eq!(
      options.level(&measure(area(0.0, 20.0, 100.0, 100.0), root)),
      3,
      "all in"
    );

    let plain = VisibilityOptions::new();
    assert_eq!(
      plain.level(&measure(area(0.0, 99.0, 100.0, 100.0), area(0.0, 0.0, 100.0, 100.0))),
      1
    );
    assert_eq!(
      plain.level(&measure(area(0.0, 100.0, 100.0, 100.0), area(0.0, 0.0, 100.0, 100.0))),
      0
    );
  }
}
