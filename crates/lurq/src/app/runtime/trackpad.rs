//! A trackpad's gestures, as a Mac tells them: a pinch, a rotation and a smart zoom (a double tap of two
//! fingers), where the pointer is. winit gives them as `PinchGesture`, `RotationGesture` and
//! `DoubleTapGesture`, and the shell hands them to the methods here.
//!
//! They are a finger's gestures to the app: a pinch is `on_pinch`, a rotation `on_rotate`, about the pointer,
//! which stays where it is while the fingers move on the trackpad; a smart zoom is `on_double_tap`, of one
//! finger, at the pointer: what zooms on a touch screen zooms there too. The elements under the pointer when
//! the gesture starts hear it to its end, innermost first, as two fingers' gesture goes to what both landed on.
//!
//! Windows tells a touchpad's pinch as the wheel with Ctrl held instead ([`ScrollEvent::ctrl`]), as a browser
//! does: there it reaches what zooms by the wheel.
//!
//! [`ScrollEvent::ctrl`]: crate::app::events::ScrollEvent::ctrl

use super::{
  Tree, find_node_by_id, hit_test_tree,
  touch_pointer::{send_pinch, send_rotate},
};
use crate::{
  app::events::{EventControl, GesturePhase, TapEvent},
  core::NodeId,
};

/// A trackpad's gesture under way.
struct Held {
  /// The elements under the pointer when it started, from the innermost out.
  chain: Vec<NodeId>,
  /// The scale or the rotation told last.
  value: f32,
}

/// The trackpad's pinch and rotation under way.
#[derive(Default)]
pub(super) struct TrackpadGestures {
  pinch: Option<Held>,
  rotate: Option<Held>,
}

impl Tree {
  /// A trackpad's pinch at the pointer, `(x, y)` in physical pixels: `delta` is how much larger it made what
  /// is shown since the last event, to multiply it by (1.1 a tenth larger), as [`PinchEvent::delta`] is. The
  /// elements under the pointer hear `on_pinch` about it: at `Start` with a scale of 1, then a change with
  /// every event, and the end. A change with no start is one.
  ///
  /// [`PinchEvent::delta`]: crate::app::events::PinchEvent::delta
  pub fn trackpad_pinch(&mut self, x: f32, y: f32, phase: GesturePhase, delta: f32) {
    let at = self.trackpad_point(x, y);
    if phase == GesturePhase::Start || self.trackpad.pinch.is_none() {
      self.trackpad.pinch = None;
      if phase == GesturePhase::End {
        return;
      }
      let chain = self.trackpad_chain(at, |node| !node.events.on_pinch.is_empty());
      if chain.is_empty() {
        return;
      }
      if let Some(root) = &self.root {
        send_pinch(root, &chain, GesturePhase::Start, 1.0, 1.0, at);
      }
      self.trackpad.pinch = Some(Held { chain, value: 1.0 });
      if phase == GesturePhase::Start && delta == 1.0 {
        self.apply_reactive_updates_after_event();
        return;
      }
    }
    let Some(held) = &mut self.trackpad.pinch else {
      return;
    };
    let (phase, delta) = match phase {
      GesturePhase::End => (GesturePhase::End, 1.0),
      // AppKit's magnification goes below -1 in no event, but a scale is never 0 or less.
      _ => (GesturePhase::Change, delta.max(0.01)),
    };
    held.value *= delta;
    if let Some(root) = &self.root {
      send_pinch(root, &held.chain, phase, held.value, delta, at);
    }
    if phase == GesturePhase::End {
      self.trackpad.pinch = None;
    }
    self.apply_reactive_updates_after_event();
  }

  /// A trackpad's rotation at the pointer, `(x, y)` in physical pixels: `delta` is how far it turned since the
  /// last event, in radians, clockwise on the screen, as [`RotateEvent::delta`] is. The elements under the
  /// pointer hear `on_rotate` about it, as [`trackpad_pinch`](Self::trackpad_pinch) tells a pinch.
  ///
  /// [`RotateEvent::delta`]: crate::app::events::RotateEvent::delta
  pub fn trackpad_rotate(&mut self, x: f32, y: f32, phase: GesturePhase, delta: f32) {
    let at = self.trackpad_point(x, y);
    if phase == GesturePhase::Start || self.trackpad.rotate.is_none() {
      self.trackpad.rotate = None;
      if phase == GesturePhase::End {
        return;
      }
      let chain = self.trackpad_chain(at, |node| !node.events.on_rotate.is_empty());
      if chain.is_empty() {
        return;
      }
      if let Some(root) = &self.root {
        send_rotate(root, &chain, GesturePhase::Start, 0.0, 0.0, at);
      }
      self.trackpad.rotate = Some(Held { chain, value: 0.0 });
      if phase == GesturePhase::Start && delta == 0.0 {
        self.apply_reactive_updates_after_event();
        return;
      }
    }
    let Some(held) = &mut self.trackpad.rotate else {
      return;
    };
    let (phase, delta) = match phase {
      GesturePhase::End => (GesturePhase::End, 0.0),
      _ => (GesturePhase::Change, delta),
    };
    held.value += delta;
    if let Some(root) = &self.root {
      send_rotate(root, &held.chain, phase, held.value, delta, at);
    }
    if phase == GesturePhase::End {
      self.trackpad.rotate = None;
    }
    self.apply_reactive_updates_after_event();
  }

  /// A trackpad's smart zoom, a double tap of two fingers, at the pointer, `(x, y)` in physical pixels: the
  /// elements under the pointer hear `on_double_tap` of one finger there, innermost first, as a touch screen's
  /// double tap that zooms. `true` when a handler took it (prevented its default or stopped it).
  pub fn trackpad_double_tap(&mut self, x: f32, y: f32) -> bool {
    let at = self.trackpad_point(x, y);
    let chain = self.trackpad_chain(at, |node| !node.events.on_double_tap.is_empty());
    let Some(root) = &self.root else {
      return false;
    };
    let control = EventControl::new();
    'bubble: for target in chain.iter().copied() {
      let Some(node) = find_node_by_id(root, target) else {
        continue;
      };
      let event = TapEvent {
        fingers: 1,
        count: 2,
        x: at.0,
        y: at.1,
        target_id: target,
        control: control.clone(),
      };
      for handler in &node.events.on_double_tap {
        handler.call(&event);
        if event.immediate_propagation_stopped() {
          break 'bubble;
        }
      }
      if event.propagation_stopped() {
        break;
      }
    }
    let taken = control.default_prevented() || control.propagation_stopped();
    self.apply_reactive_updates_after_event();
    taken
  }

  /// A point of the window in physical pixels, in logical ones, with what was rebuilt laid out to find what
  /// is there.
  fn trackpad_point(&mut self, x: f32, y: f32) -> (f32, f32) {
    self.lay_out_rebuilt_for_input();
    let scale = self.scale_factor();
    (x / scale, y / scale)
  }

  /// The elements at a point in logical pixels, from the innermost out, where one of them `listens`: none
  /// where none does.
  fn trackpad_chain(&self, at: (f32, f32), listens: impl Fn(&crate::node::Node) -> bool) -> Vec<NodeId> {
    let (Some(root), Some(layout)) = (&self.root, &self.last_layout) else {
      return Vec::new();
    };
    let mut hits = Vec::new();
    hit_test_tree(
      root,
      layout,
      0.0,
      0.0,
      at.0,
      at.1,
      &self.layout_engine.radii(),
      &mut hits,
    );
    // A gesture over a disabled element is still over the elements it is in.
    super::take_disabled_hit(&mut hits);
    if !hits.iter().any(|(node, _)| listens(node)) {
      return Vec::new();
    }
    hits.iter().map(|(node, _)| node.node_id()).collect()
  }
}
