//! A press that repeats while it is held, as the buttons that step a number do (`repeat_press`). The press of the
//! primary button on such an element runs its `on_click` handlers at once; while the button stays down and the
//! pointer over the element, they run again, first after [`REPEAT_DELAY`] and then every [`REPEAT_INTERVAL`]. A
//! pointer that leaves the element pauses the repeat, and one that comes back resumes it. The release ends it and
//! clicks nothing more.

use std::time::{Duration, Instant};

use super::{Tree, find_node_by_id};
use crate::{
  app::events::{EventControl, MouseButton, MouseEvent, MouseEventKind, PointerKind},
  core::NodeId,
};

/// How long a press is held before it repeats.
pub(super) const REPEAT_DELAY: Duration = Duration::from_millis(400);

/// How often a held press repeats after the first delay.
pub(super) const REPEAT_INTERVAL: Duration = Duration::from_millis(50);

/// A press being held on an element that repeats it.
#[derive(Clone, Copy)]
pub(super) struct PressRepeat {
  node: NodeId,
  next: Instant,
}

impl Tree {
  /// The element a press of the primary button on `target_ids` (innermost first) repeats on, if it landed on one.
  pub(super) fn press_repeat_target(&self, target_ids: &[NodeId]) -> Option<NodeId> {
    let root = self.root.as_ref()?;
    target_ids
      .iter()
      .copied()
      .find(|id| find_node_by_id(root, *id).is_some_and(|node| node.events.press_repeats))
  }

  /// Runs the click of the element a press landed on (`target`), focuses what it focuses, and starts the repeat.
  /// `x` and `y` are the press, in physical pixels.
  pub(super) fn start_press_repeat(&mut self, target: NodeId, x: f32, y: f32) {
    let focuses = self
      .root
      .as_ref()
      .and_then(|root| find_node_by_id(root, target))
      .and_then(|node| node.events.focuses.clone());
    if let (Some(focuses), Some(ctx)) = (focuses, &self.root_ctx) {
      ctx.focus(&focuses);
    }
    self.press_repeat = Some(PressRepeat {
      node: target,
      next: Instant::now() + REPEAT_DELAY,
    });
    self.run_press_repeat(target, x, y);
  }

  /// Ends a held press: its button was released, or the OS took the press.
  pub(super) fn end_press_repeat(&mut self) {
    self.press_repeat = None;
  }

  /// When a held press repeats next.
  pub(super) fn next_press_repeat(&self) -> Option<Instant> {
    self.press_repeat.map(|repeat| repeat.next)
  }

  /// Repeats a held press whose time has come, if the pointer is still over its element. Returns whether it ran.
  pub(super) fn tick_press_repeat(&mut self, now: Instant) -> bool {
    let Some(repeat) = self.press_repeat else {
      return false;
    };
    if now < repeat.next {
      return false;
    }
    let exists = self
      .root
      .as_ref()
      .is_some_and(|root| find_node_by_id(root, repeat.node).is_some());
    if !exists {
      self.press_repeat = None;
      return false;
    }
    self.press_repeat = Some(PressRepeat {
      node: repeat.node,
      next: now + REPEAT_INTERVAL,
    });
    // A disabled element is hit by nothing: an element that was disabled as it repeated (a step to the end of a
    // range) stops it.
    let Some((x, y)) = self.pointer_position() else {
      return false;
    };
    if !self.hit_target_ids_at(x, y).contains(&repeat.node) {
      return false;
    }
    self.run_press_repeat(repeat.node, x, y)
  }

  /// Runs the `on_click` handlers of the element `node` as for a click at `x`, `y` (physical pixels).
  fn run_press_repeat(&mut self, node: NodeId, x: f32, y: f32) -> bool {
    let Some(handlers) = self
      .root
      .as_ref()
      .and_then(|root| find_node_by_id(root, node))
      .map(|node| node.events.on_click.clone())
    else {
      return false;
    };
    let scale = self.scale_factor();
    let event = MouseEvent {
      x: x / scale,
      y: y / scale,
      button: MouseButton::Left,
      kind: MouseEventKind::Click,
      shift: false,
      ctrl: false,
      alt: false,
      meta: false,
      from_keyboard: false,
      pointer: if self.touch_pointer.down {
        PointerKind::Touch
      } else {
        PointerKind::Mouse
      },
      target_id: node,
      control: EventControl::new(),
    };
    for handler in &handlers {
      handler.call(&event);
    }
    self.needs_redraw = true;
    !handlers.is_empty()
  }
}
