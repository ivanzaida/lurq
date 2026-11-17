//! A touch as the shell gives it to the tree on every platform winit tells touches on: the finger's own events,
//! then the pointer it stands for, once; the app's pointer made coarse by a finger and left by a pen.

use std::{
  sync::{Arc, Mutex},
  time::Instant,
};

use winit::{
  dpi::PhysicalPosition,
  event::{DeviceId, Force, Touch, TouchPhase},
};

use super::{deliver_touch, is_pen, promoted_from_touch, touch::TouchPointer};
use crate::{
  app::{
    App, Pointer, Tree,
    component::Component,
    ctx::Ctx,
    events::{MouseEvent, TapEvent},
  },
  components::Rect,
  node::Element,
};

fn touch(id: u64, phase: TouchPhase, force: Option<Force>) -> Touch {
  Touch {
    // A test's device, which nothing reads.
    device_id: unsafe { DeviceId::dummy() },
    phase,
    location: PhysicalPosition::new(50.0, 50.0),
    force,
    id,
  }
}

type Log = Arc<Mutex<Vec<String>>>;

thread_local! {
  /// Where the scene of the test on this thread logs its taps and clicks.
  static LOG: std::cell::RefCell<Log> = std::cell::RefCell::new(Log::default());
}

/// A box of 100 at the top left that logs its taps and clicks, mounted in an app, in a window where nothing
/// scrolls.
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
    let (taps, clicks) = (self.log.clone(), self.log.clone());
    Rect::new(100.0, 100.0)
      .on_tap(move |event: TapEvent| taps.lock().unwrap().push(format!("tap of {}", event.fingers)))
      .on_click(move |event: MouseEvent| clicks.lock().unwrap().push(format!("click {:?}", event.pointer_kind)))
  }
}

fn tree() -> (Tree, App, Log) {
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.resize(300, 300);
  let log = Log::default();
  LOG.with(|held| *held.borrow_mut() = log.clone());
  tree.mount_root::<Scene>(&mut app, ());
  tree.pass_headless(&mut app);
  (tree, app, log)
}

/// What a finger (or a pen, by its force) does landing and lifting, delivered as the shell delivers it.
fn tap(tree: &mut Tree, force: Option<Force>) {
  let (mut pointer, mut cursor) = (TouchPointer::default(), (0.0, 0.0));
  let now = Instant::now();
  for phase in [TouchPhase::Started, TouchPhase::Ended] {
    deliver_touch(tree, &mut pointer, &mut cursor, &touch(3, phase, force), now);
  }
}

#[test]
fn a_fingers_tap_is_its_tap_and_one_click_and_makes_the_pointer_coarse() {
  let (mut tree, mut app, log) = tree();
  app.shared.pointer.tell(Pointer::Fine, false);
  tap(&mut tree, None);
  assert_eq!(*log.lock().unwrap(), ["tap of 1", "click Touch"]);
  assert_eq!(app.pointer(), Pointer::Coarse);
  tree.pass_headless(&mut app);
}

#[test]
fn a_mouse_event_is_not_taken_for_a_touchs_copy_where_the_system_says_nothing() {
  // Windows marks the mouse messages it makes from a touch; a test's thread has no such message.
  let moved = winit::event::WindowEvent::CursorMoved {
    device_id: unsafe { DeviceId::dummy() },
    position: PhysicalPosition::new(10.0, 10.0),
  };
  assert!(!promoted_from_touch(&moved));
}

#[test]
fn a_pen_is_told_where_the_platform_tells_it() {
  let finger = touch(1, TouchPhase::Started, None);
  assert!(!is_pen(&finger), "a touch with no force is a finger's");
  let pressed = touch(1, TouchPhase::Started, Some(Force::Normalized(0.4)));
  let held = touch(
    1,
    TouchPhase::Started,
    Some(Force::Calibrated {
      force: 1.0,
      max_possible_force: 2.0,
      altitude_angle: Some(1.0),
    }),
  );
  // Windows: a finger comes as WM_TOUCH with no force, a pen as pointer input with its pressure.
  assert_eq!(is_pen(&pressed), cfg!(windows));
  // iOS: an Apple Pencil's touch has the angle it is held at.
  assert_eq!(is_pen(&held), cfg!(target_os = "ios"));
}

#[cfg(windows)]
#[test]
fn a_pens_tap_on_windows_taps_and_clicks_as_a_pen_and_leaves_the_pointer() {
  let (mut tree, app, log) = tree();
  app.shared.pointer.tell(Pointer::Fine, false);
  tap(&mut tree, Some(Force::Normalized(0.5)));
  assert_eq!(*log.lock().unwrap(), ["tap of 1", "click Pen"]);
  assert_eq!(app.pointer(), Pointer::Fine, "a pen leaves it");
}
