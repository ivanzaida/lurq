//! A trackpad's gestures as winit tells them on a Mac: `PinchGesture`, `RotationGesture` and `DoubleTapGesture`
//! (smart zoom, a double tap of two fingers). The tree hears them as a finger's pinch, rotation and double tap
//! at the pointer (`Tree::trackpad_pinch` and the rest), so what a finger zooms on a touch screen zooms with the
//! trackpad. On iOS winit recognizes none of them unless told to, which lurq does not: fingers come as touches.
//!
//! Windows tells a touchpad's pinch as the wheel with Ctrl held, which the shell gives with the wheel's event.

use winit::event::{TouchPhase, WindowEvent};

use crate::app::{Tree, events::GesturePhase};

/// Gives a trackpad's gesture to the tree, at the pointer `at` in physical pixels. `false` when the event is
/// none: it is left to the rest of the shell.
pub(super) fn deliver(tree: &mut Tree, event: &WindowEvent, at: (f64, f64)) -> bool {
  let (x, y) = (at.0 as f32, at.1 as f32);
  match *event {
    // AppKit's magnification: how much larger since the last event, 0 none. It may be NaN.
    WindowEvent::PinchGesture { delta, phase, .. } => {
      let delta = if delta.is_finite() { 1.0 + delta as f32 } else { 1.0 };
      tree.trackpad_pinch(x, y, gesture_phase(phase), delta);
    }
    // Degrees, counterclockwise; a rotation is told in radians, clockwise on the screen.
    WindowEvent::RotationGesture { delta, phase, .. } => {
      let delta = if delta.is_finite() { -delta.to_radians() } else { 0.0 };
      tree.trackpad_rotate(x, y, gesture_phase(phase), delta);
    }
    WindowEvent::DoubleTapGesture { .. } => {
      tree.trackpad_double_tap(x, y);
    }
    _ => return false,
  }
  true
}

fn gesture_phase(phase: TouchPhase) -> GesturePhase {
  match phase {
    TouchPhase::Started => GesturePhase::Start,
    TouchPhase::Moved => GesturePhase::Change,
    TouchPhase::Ended | TouchPhase::Cancelled => GesturePhase::End,
  }
}

#[cfg(test)]
mod tests {
  use std::sync::{Arc, Mutex};

  use winit::event::{DeviceId, TouchPhase, WindowEvent};

  use super::deliver;
  use crate::{
    app::{
      App, Tree,
      component::Component,
      ctx::Ctx,
      events::{GesturePhase, PinchEvent, RotateEvent, TapEvent},
    },
    components::Rect,
    node::Element,
  };

  type Log = Arc<Mutex<Vec<String>>>;

  thread_local! {
    static LOG: std::cell::RefCell<Log> = std::cell::RefCell::new(Log::default());
  }

  /// A box of 100 at the top left that logs its pinches, rotations and double taps.
  struct Scene {
    log: Log,
  }

  impl Component for Scene {
    type Props = ();

    fn create(_ctx: &mut Ctx) -> Self {
      Self {
        log: LOG.with(|log| log.borrow().clone()),
      }
    }

    fn render(&self, _ctx: &mut Ctx) -> impl Into<Element> {
      let (pinches, turns, taps) = (self.log.clone(), self.log.clone(), self.log.clone());
      Rect::new(100.0, 100.0)
        .on_pinch(move |event: PinchEvent| {
          pinches.lock().unwrap().push(format!(
            "pinch {:?} {:.3} {:.3} at {} {}",
            event.phase, event.scale, event.delta, event.x, event.y
          ));
        })
        .on_rotate(move |event: RotateEvent| {
          turns.lock().unwrap().push(format!(
            "rotate {:?} {:.3} {:.3} at {} {}",
            event.phase, event.rotation, event.delta, event.x, event.y
          ));
        })
        .on_double_tap(move |event: TapEvent| {
          taps
            .lock()
            .unwrap()
            .push(format!("double tap of {} at {} {}", event.fingers, event.x, event.y));
        })
    }
  }

  /// The scene in a window of 200 at a scale of 2: a point of the window in physical pixels is twice the
  /// logical one.
  fn scene() -> (Tree, Log) {
    let log = Log::default();
    LOG.with(|cell| *cell.borrow_mut() = log.clone());
    let mut app = App::new();
    let mut tree = Tree::new();
    tree.set_scale_factor(2.0);
    tree.resize(400, 400);
    tree.mount_root::<Scene>(&mut app, ());
    tree.pass_headless(&mut app);
    (tree, log)
  }

  fn device() -> DeviceId {
    // A test's device, which nothing reads.
    unsafe { DeviceId::dummy() }
  }

  fn pinch(delta: f64, phase: TouchPhase) -> WindowEvent {
    WindowEvent::PinchGesture {
      device_id: device(),
      delta,
      phase,
    }
  }

  fn taken(log: &Log) -> Vec<String> {
    std::mem::take(&mut *log.lock().unwrap())
  }

  #[test]
  fn a_trackpads_pinch_is_a_pinch_about_the_pointer() {
    let (mut tree, log) = scene();
    let at = (60.0, 80.0);
    assert!(deliver(&mut tree, &pinch(0.0, TouchPhase::Started), at));
    assert!(deliver(&mut tree, &pinch(0.5, TouchPhase::Moved), at));
    assert!(deliver(&mut tree, &pinch(-0.2, TouchPhase::Moved), at));
    assert!(deliver(&mut tree, &pinch(0.0, TouchPhase::Ended), at));
    assert_eq!(
      taken(&log),
      [
        "pinch Start 1.000 1.000 at 30 40",
        "pinch Change 1.500 1.500 at 30 40",
        "pinch Change 1.200 0.800 at 30 40",
        "pinch End 1.200 1.000 at 30 40",
      ],
      "the scale is AppKit's magnifications one on another, at the pointer in logical pixels"
    );
  }

  #[test]
  fn a_pinch_whose_start_was_lost_starts_with_its_first_change() {
    let (mut tree, log) = scene();
    deliver(&mut tree, &pinch(0.25, TouchPhase::Moved), (20.0, 20.0));
    deliver(&mut tree, &pinch(f64::NAN, TouchPhase::Moved), (20.0, 20.0));
    deliver(&mut tree, &pinch(0.0, TouchPhase::Cancelled), (20.0, 20.0));
    assert_eq!(
      taken(&log),
      [
        "pinch Start 1.000 1.000 at 10 10",
        "pinch Change 1.250 1.250 at 10 10",
        "pinch Change 1.250 1.000 at 10 10",
        "pinch End 1.250 1.000 at 10 10",
      ]
    );
  }

  #[test]
  fn a_pinch_over_nothing_that_listens_tells_nothing() {
    let (mut tree, log) = scene();
    deliver(&mut tree, &pinch(0.0, TouchPhase::Started), (300.0, 300.0));
    deliver(&mut tree, &pinch(0.5, TouchPhase::Moved), (300.0, 300.0));
    deliver(&mut tree, &pinch(0.0, TouchPhase::Ended), (300.0, 300.0));
    assert!(taken(&log).is_empty());
  }

  #[test]
  fn a_trackpads_rotation_is_a_rotation_clockwise_in_radians() {
    let (mut tree, log) = scene();
    let at = (40.0, 40.0);
    let turn = |delta: f32, phase| WindowEvent::RotationGesture {
      device_id: device(),
      delta,
      phase,
    };
    deliver(&mut tree, &turn(0.0, TouchPhase::Started), at);
    deliver(&mut tree, &turn(-90.0, TouchPhase::Moved), at);
    deliver(&mut tree, &turn(0.0, TouchPhase::Ended), at);
    assert_eq!(
      taken(&log),
      [
        "rotate Start 0.000 0.000 at 20 20",
        "rotate Change 1.571 1.571 at 20 20",
        "rotate End 1.571 0.000 at 20 20",
      ],
      "winit's degrees counterclockwise, turned clockwise"
    );
  }

  #[test]
  fn smart_zoom_is_a_double_tap_at_the_pointer() {
    let (mut tree, log) = scene();
    assert!(deliver(
      &mut tree,
      &WindowEvent::DoubleTapGesture { device_id: device() },
      (50.0, 30.0)
    ));
    assert_eq!(taken(&log), ["double tap of 1 at 25 15"]);
  }

  #[test]
  fn other_events_are_left_to_the_shell() {
    let (mut tree, _) = scene();
    assert!(!deliver(&mut tree, &WindowEvent::Focused(true), (0.0, 0.0)));
  }

  #[test]
  fn gesture_phases_follow_winits() {
    assert_eq!(super::gesture_phase(TouchPhase::Started), GesturePhase::Start);
    assert_eq!(super::gesture_phase(TouchPhase::Cancelled), GesturePhase::End);
  }
}
