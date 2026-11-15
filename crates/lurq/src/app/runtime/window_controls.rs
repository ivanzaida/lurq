//! The window's controls in a title bar the app draws itself (`.window_control(..)`): where they are after a
//! layout, for a shell that tells the system.
//!
//! On Windows the system asks a window what is at a point of it, and opens the snap layouts of Windows 11 over
//! what the window says is its maximize button. The winit shell answers from what this module leaves in the
//! window's state after each layout (`app::window::controls`): for each element marked as a control, its box cut by
//! what clips it, and the boxes of what is hit before it there. The walk follows the pointer's own hit test
//! (`app::hit_test`): a child of a stack that is hit covers the children before it, what sticks covers its
//! siblings, an element that clips cuts what it holds, and what takes no pointer (`HitTestBehavior::None`, inert,
//! dropped from its row) neither is a control nor covers one. A disabled control is none. Like an element ref's
//! bounds, the boxes are those of the layout: a transform is not followed.
//!
//! Nothing is walked until an element of the process is marked, and nothing is left in the window's state unless
//! a shell asked for it ([`Tree::publish_window_controls`]): on the other platforms the mark costs nothing.

use super::Tree;
use crate::{
  app::window::controls::{Bounds, WindowControlRegion, WindowControlRegions},
  layout::{
    layout_kind::{LayoutKind, Overflow},
    layout_result::LayoutResult,
  },
  node::{HitTestBehavior, Node, WindowControl, window_controls_in_use},
};

impl Tree {
  /// Which of the window's controls is at a point of the window, in physical pixels as the pointer's own: the
  /// element marked with `.window_control(..)` that is laid out there, where nothing that takes the pointer covers
  /// it. `None` anywhere else, and before the first layout.
  ///
  /// For a shell that drives a `Tree` itself and answers the system's hit test of its window (`WM_NCHITTEST` on
  /// Windows); the winit shell does it by itself.
  pub fn window_control_at(&self, x: f32, y: f32) -> Option<WindowControl> {
    let scale = self.scale_factor();
    self.window_control_regions().at(x / scale, y / scale)
  }

  /// Has the tree leave where the window's controls are in the window's state after each layout, for the shell
  /// to answer the system from.
  #[cfg_attr(not(all(feature = "winit", windows)), allow(dead_code))]
  pub(crate) fn publish_window_controls(&mut self) {
    self.publishes_window_controls = true;
    self.leave_window_controls();
  }

  /// Leaves where the window's controls are in the window's state, if a shell asked for it. Called after a layout.
  pub(super) fn leave_window_controls(&self) {
    if self.publishes_window_controls {
      self.window.set_control_regions(self.window_control_regions());
    }
  }

  fn window_control_regions(&self) -> WindowControlRegions {
    let mut regions = WindowControlRegions::default();
    if !window_controls_in_use() {
      return regions;
    }
    if let (Some(root), Some(layout)) = (&self.root, &self.last_layout) {
      // From the window's corner, as the pointer's hit test and the element refs' bounds are.
      find(
        root,
        layout,
        0.0,
        0.0,
        Bounds::EVERYWHERE,
        &mut Vec::new(),
        &mut regions,
      );
    }
    regions
  }
}

/// A step down the tree to a control: the element stepped from, where it is, what clips its children, and which
/// of them the step went to.
struct Step<'a> {
  parent: &'a Node,
  layout: &'a LayoutResult,
  x: f32,
  y: f32,
  clip: Bounds,
  child: usize,
}

/// Whether the pointer's hit test passes the node and everything in it by.
fn takes_no_pointer(node: &Node, layout: &LayoutResult) -> bool {
  node.hit_test_behavior() == HitTestBehavior::None || layout.dropped || node.is_inert()
}

/// What clips the children of a node at `bounds` that is itself clipped by `clip`.
fn children_clip(node: &Node, bounds: Bounds, clip: Bounds) -> Bounds {
  if node.overflow == Overflow::Visible {
    clip
  } else {
    clip.intersect(bounds)
  }
}

/// Finds the controls in `node` and what it holds, laid out as `layout` at `(x, y)` under `clip`.
fn find<'a>(
  node: &'a Node,
  layout: &'a LayoutResult,
  x: f32,
  y: f32,
  clip: Bounds,
  path: &mut Vec<Step<'a>>,
  regions: &mut WindowControlRegions,
) {
  // A disabled element stands for everything in it, and takes nothing: no control is in it.
  if takes_no_pointer(node, layout) || node.is_disabled() {
    return;
  }
  let bounds = Bounds::new(x, y, layout.size.width, layout.size.height);
  if let Some(control) = node.window_control()
    && node.hit_test_behavior() == HitTestBehavior::Auto
  {
    let seen = bounds.intersect(clip);
    if !seen.is_empty() {
      let covers = covers(path, seen);
      // A control wholly under something else is none.
      if !covers.iter().any(|cover| cover.holds(seen)) {
        regions.push(WindowControlRegion {
          control,
          bounds: seen,
          covers,
        });
      }
    }
  }
  let clip = children_clip(node, bounds, clip);
  if clip.is_empty() {
    return;
  }
  for (child, (child_layout, child_node)) in layout.children.iter().zip(node.children()).enumerate() {
    path.push(Step {
      parent: node,
      layout,
      x,
      y,
      clip,
      child,
    });
    find(
      child_node,
      &child_layout.result,
      x + child_layout.offset.x,
      y + child_layout.offset.y,
      clip,
      path,
      regions,
    );
    path.pop();
  }
}

/// The boxes of what is hit before a control whose seen box is `within`, found down `path`: at each step, the
/// siblings the hit test tries first and stops at.
fn covers(path: &[Step<'_>], within: Bounds) -> Vec<Bounds> {
  let mut covers = Vec::new();
  for step in path {
    let children = step.parent.children();
    let Some(own) = children.get(step.child) else {
      continue;
    };
    let stack = matches!(step.parent.layout_kind(), LayoutKind::Stack { .. });
    for (index, (child_layout, child_node)) in step.layout.children.iter().zip(children).enumerate() {
      if index == step.child {
        continue;
      }
      // What sticks is hit first, the last of it before the rest of it; in a stack, a child before those under it.
      let before = if own.is_sticky() {
        child_node.is_sticky() && index > step.child
      } else {
        child_node.is_sticky() || (stack && index > step.child)
      };
      if before {
        boxes(
          child_node,
          &child_layout.result,
          step.x + child_layout.offset.x,
          step.y + child_layout.offset.y,
          step.clip,
          within,
          &mut covers,
        );
      }
    }
  }
  covers
}

/// The boxes, inside `within`, of the elements in `node` and what it holds that take the pointer.
fn boxes(node: &Node, layout: &LayoutResult, x: f32, y: f32, clip: Bounds, within: Bounds, out: &mut Vec<Bounds>) {
  if takes_no_pointer(node, layout) {
    return;
  }
  let bounds = Bounds::new(x, y, layout.size.width, layout.size.height);
  let seen = bounds.intersect(clip).intersect(within);
  if node.hit_test_behavior() == HitTestBehavior::Auto && !seen.is_empty() {
    out.push(seen);
    // What it holds inside its own box adds nothing; what it lets out of its box may.
    if node.overflow != Overflow::Visible {
      return;
    }
  }
  let clip = children_clip(node, bounds, clip);
  if clip.intersect(within).is_empty() {
    return;
  }
  for (child_layout, child_node) in layout.children.iter().zip(node.children()) {
    boxes(
      child_node,
      &child_layout.result,
      x + child_layout.offset.x,
      y + child_layout.offset.y,
      clip,
      within,
      out,
    );
  }
}
