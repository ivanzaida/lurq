//! `lurq_touch`: every gesture, driven through the tool, reaches the handlers a touch screen's fingers reach.

use std::sync::{Arc, Mutex};

use super::{
  tests::{TestSurface, call, json, state},
  *,
};
use crate::{
  app::events::{
    CopyEvent, EdgePanEvent, EditCommand, EditEvent, GesturePhase, LongPressEvent, PanEvent, PasteEvent, PinchEvent,
    RotateEvent, SwipeEvent, TapEvent,
  },
  components::{Column, Rect},
};

type Log = Arc<Mutex<Vec<String>>>;

/// A clipboard that keeps nothing written, which the OS never sees.
struct NoClipboard;

impl crate::app::ClipboardWriter for NoClipboard {
  fn write_text(&self, _text: &str) -> bool {
    true
  }
}

fn logged(log: &Log) -> Vec<String> {
  std::mem::take(&mut *log.lock().unwrap())
}

/// A box of 400 filling a window of 400 that takes the focus and logs every gesture and editing command.
fn board() -> (Tree, App, Log) {
  let mut tree = Tree::new();
  let mut app = App::new();
  tree.resize(400, 400);
  // Nothing the gestures copy reaches the OS clipboard.
  tree.set_clipboard_writer(Some(Arc::new(NoClipboard)));
  let log = Log::default();
  let push = {
    let log = log.clone();
    move |line: String| log.lock().unwrap().push(line)
  };
  let (tap, long, pan, swipe, edge, pinch, rotate) = (
    push.clone(),
    push.clone(),
    push.clone(),
    push.clone(),
    push.clone(),
    push.clone(),
    push.clone(),
  );
  let (undo, redo, copy, paste) = (push.clone(), push.clone(), push.clone(), push);
  tree.set_root(
    Column::new().child(
      Rect::new(400.0, 400.0)
        .focusable(true)
        .on_tap(move |event: TapEvent| tap(format!("tap of {} count {}", event.fingers, event.count)))
        .on_long_press(move |_: LongPressEvent| long("long press".into()))
        .on_pan(move |event: PanEvent| {
          if event.phase == GesturePhase::End {
            pan(format!(
              "pan of {} by {:.0},{:.0}",
              event.fingers, event.translation_x, event.translation_y
            ));
          }
        })
        .on_swipe(move |event: SwipeEvent| swipe(format!("swipe {:?} of {}", event.direction, event.fingers)))
        .on_edge_pan(move |event: EdgePanEvent| {
          if event.phase == GesturePhase::End {
            edge(format!("edge {:?}", event.edge));
          }
        })
        .on_pinch(move |event: PinchEvent| {
          if event.phase == GesturePhase::End {
            pinch(format!("pinch {:.1}", event.scale));
          }
        })
        .on_rotate(move |event: RotateEvent| {
          if event.phase == GesturePhase::End {
            rotate(format!("rotate {:.0}", event.rotation.to_degrees()));
          }
        })
        .on_undo(move |_: EditEvent| undo("undo".into()))
        .on_redo(move |_: EditEvent| redo("redo".into()))
        .on_copy(move |_: CopyEvent| copy("copy".into()))
        .on_paste(move |_: PasteEvent| paste("paste".into())),
    ),
  );
  tree.pass(&mut app, &TestSurface);
  (tree, app, log)
}

fn touch(tree: &mut Tree, app: &mut App, args: serde_json::Value) -> serde_json::Value {
  let state = state();
  json(call(tree, app, &state, "lurq_touch", args))
}

#[test]
fn taps_of_fingers_and_counts() {
  let (mut tree, mut app, log) = board();
  let reply = touch(
    &mut tree,
    &mut app,
    serde_json::json!({"gesture": "tap", "x": 200, "y": 200}),
  );
  assert_eq!(reply["ok"], true);
  assert_eq!(logged(&log), ["tap of 1 count 1"]);
  touch(
    &mut tree,
    &mut app,
    serde_json::json!({"gesture": "tap", "x": 200, "y": 200, "fingers": 2}),
  );
  assert_eq!(logged(&log), ["tap of 2 count 1"]);
  touch(
    &mut tree,
    &mut app,
    serde_json::json!({"gesture": "double_tap", "x": 100, "y": 100}),
  );
  assert_eq!(logged(&log), ["tap of 1 count 1", "tap of 1 count 2"]);
}

#[test]
fn a_long_press_rests_for_its_time() {
  let (mut tree, mut app, log) = board();
  touch(
    &mut tree,
    &mut app,
    serde_json::json!({"gesture": "long_press", "x": 200, "y": 200}),
  );
  assert_eq!(logged(&log), ["long press"]);
}

#[test]
fn a_pan_and_swipes() {
  let (mut tree, mut app, log) = board();
  touch(
    &mut tree,
    &mut app,
    serde_json::json!({"gesture": "pan", "x": 200, "y": 200, "dx": 60, "dy": 40}),
  );
  assert_eq!(logged(&log), ["pan of 1 by 60,40"]);
  touch(
    &mut tree,
    &mut app,
    serde_json::json!({"gesture": "swipe", "x": 300, "y": 200, "direction": "left"}),
  );
  assert_eq!(logged(&log), ["pan of 1 by -200,0", "swipe Left of 1"]);
  touch(
    &mut tree,
    &mut app,
    serde_json::json!({"gesture": "swipe", "x": 200, "y": 100, "direction": "down", "fingers": 2, "distance": 150}),
  );
  assert_eq!(logged(&log), ["pan of 2 by 0,150", "swipe Down of 2"]);
}

#[test]
fn an_edge_pan_comes_in_from_its_edge() {
  let (mut tree, mut app, log) = board();
  let reply = touch(
    &mut tree,
    &mut app,
    serde_json::json!({"gesture": "edge_pan", "edge": "left", "distance": 200}),
  );
  assert_eq!(reply["x"], 2.0);
  assert_eq!(logged(&log), ["edge Left"]);
}

#[test]
fn pinches_rotations_and_one_fingers_zoom() {
  let (mut tree, mut app, log) = board();
  touch(
    &mut tree,
    &mut app,
    serde_json::json!({"gesture": "pinch", "x": 200, "y": 200, "scale": 2.0}),
  );
  let lines = logged(&log);
  let scale: f32 = lines[0].strip_prefix("pinch ").unwrap().parse().unwrap();
  assert!((1.6..=2.0).contains(&scale), "{lines:?}");

  touch(
    &mut tree,
    &mut app,
    serde_json::json!({"gesture": "rotate", "x": 200, "y": 200, "angle": 45}),
  );
  let lines = logged(&log);
  let turned: f32 = lines[0].strip_prefix("rotate ").unwrap().parse().unwrap();
  assert!((30.0..=45.0).contains(&turned), "{lines:?}");

  touch(
    &mut tree,
    &mut app,
    serde_json::json!({"gesture": "zoom", "x": 200, "y": 100, "scale": 2.0}),
  );
  let lines = logged(&log);
  assert_eq!(lines.last().map(String::as_str), Some("pinch 2.0"), "{lines:?}");
}

#[test]
fn three_fingers_make_the_editing_commands_and_open_their_menu() {
  let (mut tree, mut app, log) = board();
  touch(
    &mut tree,
    &mut app,
    serde_json::json!({"gesture": "tap", "x": 200, "y": 200}),
  );
  logged(&log);

  touch(
    &mut tree,
    &mut app,
    serde_json::json!({"gesture": "three_finger_swipe", "x": 250, "y": 200, "direction": "left"}),
  );
  assert!(logged(&log).contains(&"undo".to_owned()));
  touch(
    &mut tree,
    &mut app,
    serde_json::json!({"gesture": "three_finger_swipe", "x": 150, "y": 200, "direction": "right"}),
  );
  assert!(logged(&log).contains(&"redo".to_owned()));
  touch(
    &mut tree,
    &mut app,
    serde_json::json!({"gesture": "three_finger_pinch", "x": 200, "y": 200, "scale": 0.3}),
  );
  assert_eq!(logged(&log), ["copy"]);
  // A paste MCP asks for reads no clipboard: it is refused, as a synthetic Ctrl+V is.
  touch(
    &mut tree,
    &mut app,
    serde_json::json!({"gesture": "three_finger_pinch", "x": 200, "y": 200, "scale": 2.5}),
  );
  assert_eq!(logged(&log), [] as [&str; 0]);

  touch(
    &mut tree,
    &mut app,
    serde_json::json!({"gesture": "tap", "x": 200, "y": 200, "fingers": 3}),
  );
  assert_eq!(logged(&log), ["tap of 3 count 1"]);
  assert_eq!(
    tree.window().edit_menu().map(|menu| menu.commands.len()),
    Some(5),
    "Undo, Cut, Copy, Paste, Redo"
  );
  let _ = EditCommand::Undo;
}

#[test]
fn an_unknown_gesture_is_refused_with_the_list() {
  let (mut tree, mut app, _log) = board();
  let state = state();
  let Err(error) = call(
    &mut tree,
    &mut app,
    &state,
    "lurq_touch",
    serde_json::json!({"gesture": "wave", "x": 1, "y": 1}),
  ) else {
    panic!("a gesture of no name was played");
  };
  assert!(error.contains("three_finger_pinch"), "{error}");
}

#[test]
fn inspect_finds_by_name_at_any_depth() {
  let mut tree = Tree::new();
  let mut app = App::new();
  tree.resize(400, 400);
  let mut deep: crate::node::Element = crate::components::Button::new("Deep").id("deep").into();
  for _ in 0..30 {
    deep = Column::new().child(deep).into();
  }
  tree.set_root(deep);
  tree.pass(&mut app, &TestSurface);
  let state = state();
  let found = json(call(
    &mut tree,
    &mut app,
    &state,
    "lurq_inspect",
    serde_json::json!({"query": "deep", "role": "button"}),
  ));
  assert_eq!(found["matches"].as_array().map(Vec::len), Some(1), "{found}");
  // An outline stops at 12 unless told, and says it did.
  let outline = json(call(&mut tree, &mut app, &state, "lurq_inspect", serde_json::json!({})));
  assert_eq!(outline["truncated"], true);
}
