//! A gesture that began goes on to its end whatever is built anew meanwhile: an element whose pan's `Start`
//! changes what is around it and in it is another node after the render, and still hears the `Change` and the
//! `End` of that pan, and its touch's end.
//!
//! The fingers are given through [`Tree::finger`] on the tree's clock of presses, as a shell gives them.

use std::{
  sync::{Arc, Mutex},
  time::{Duration, Instant},
};

use lurq::{
  app::{
    App, Tree,
    component::Component,
    ctx::Ctx,
    events::{GesturePhase, PanEvent, TouchEvent, TouchPhase},
  },
  components::{Column, Rect, Row},
  core::Signal,
  node::Element,
};

use crate::support::TestSurface;

const FINGER: u64 = 1;

#[derive(Clone, Default)]
struct Log(Arc<Mutex<Vec<String>>>);

impl PartialEq for Log {
  fn eq(&self, _: &Self) -> bool {
    true
  }
}

impl lurq::app::component::DevtoolsInspectable for Log {}

impl Log {
  fn say(&self, line: &str) {
    self.0.lock().unwrap().push(line.to_owned());
  }

  fn told(&self) -> Vec<String> {
    self.0.lock().unwrap().clone()
  }
}

/// A seek line: while it is held it has a thumb and a time beside its track, and something stands before it.
struct Line {
  held: Signal<bool>,
  log: Log,
}

impl Component for Line {
  type Props = Log;

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      held: ctx.signal(false),
      log: ctx.props::<Log>().clone(),
    }
  }

  fn render(&self, _: &mut Ctx) -> impl Into<Element> {
    let held = self.held.get();
    let (panned, touched, hold) = (self.log.clone(), self.log.clone(), self.held.clone());
    // No id and no key, as most elements have none: a node is told from another by where it stands and what
    // it looks like, and this one stands elsewhere and looks otherwise once it is held.
    let mut line = Column::new()
      .size(200.0, 40.0)
      .on_pan(move |event: PanEvent| match event.phase {
        GesturePhase::Start => {
          event.prevent_default();
          event.stop_propagation();
          panned.say("pan start");
          hold.set(true);
        }
        GesturePhase::Change => panned.say("pan change"),
        GesturePhase::End => {
          panned.say("pan end");
          hold.set(false);
        }
        _ => panned.say("pan cancelled"),
      })
      .on_touch_end(move |_: TouchEvent| touched.say("touch end"));
    // Held, it is built in another shape: a thumb before its track, and a time after it.
    if held {
      line = line.child(Rect::new(12.0, 12.0).id("thumb"));
    }
    line = line.child(Row::new().size(200.0, 4.0));
    if held {
      line = line.child(Row::new().size(60.0, 12.0));
    }
    let mut scene = Column::new().size(300.0, 300.0);
    // And something stands before it, so that it is no longer the first of its siblings.
    if held {
      scene = scene.child(Rect::new(300.0, 0.0).id("before"));
    }
    scene.child(line)
  }
}

/// A finger lands on the track, a part of the line, goes across and lifts; `frames` is whether a frame is drawn
/// after each of its events. Tells what the line heard, and whether it is let go in the end.
fn drag(frames: bool) -> (Vec<String>, bool) {
  let log = Log::default();
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.resize(300, 300);
  tree.mount_root::<Line>(&mut app, log.clone());
  let mut now = Instant::now();
  tree.set_press_clock(Some(now));
  tree.pass(&mut app, &TestSurface);
  let mut finger = |tree: &mut Tree, app: &mut App, millis: u64, phase: TouchPhase, x: f32, y: f32| {
    now += Duration::from_millis(millis);
    tree.set_press_clock(Some(now));
    tree.finger(FINGER, phase, x, y, None);
    if frames {
      // What a handler changed is rendered, laid out and drawn.
      tree.pass(app, &TestSurface);
    }
  };
  finger(&mut tree, &mut app, 0, TouchPhase::Start, 100.0, 2.0);
  finger(&mut tree, &mut app, 16, TouchPhase::Move, 120.0, 2.0);
  assert_eq!(log.told(), ["pan start"]);
  finger(&mut tree, &mut app, 16, TouchPhase::Move, 140.0, 2.0);
  assert!(
    tree.get_element_by_id("thumb").is_some(),
    "the line was built anew, held"
  );
  finger(&mut tree, &mut app, 16, TouchPhase::Move, 160.0, 2.0);
  finger(&mut tree, &mut app, 16, TouchPhase::End, 160.0, 2.0);
  tree.pass(&mut app, &TestSurface);
  (log.told(), tree.get_element_by_id("thumb").is_none())
}

#[test]
fn a_pan_whose_start_rebuilds_the_element_under_the_finger_still_tells_its_end() {
  let (told, let_go) = drag(true);
  assert_eq!(
    told,
    ["pan start", "pan change", "pan change", "touch end", "pan end"],
    "the gesture that began is told to its end"
  );
  assert!(let_go, "and the line is let go");
}

#[test]
fn it_does_where_no_frame_comes_between_two_events_of_the_gesture() {
  // A gesture played at once, as `lurq_touch` plays one: what its start changed is built, and laid out by no
  // frame before its next event.
  let (told, let_go) = drag(false);
  assert_eq!(told, ["pan start", "pan change", "pan change", "touch end", "pan end"]);
  assert!(let_go);
}
