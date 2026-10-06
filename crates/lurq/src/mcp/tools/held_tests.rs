//! Input held across `lurq_interact` calls: `press`, `release`, `key_down`,
//! `key_up`, held moves, refusals, and the release of what a client can no
//! longer release itself.

use std::sync::{Arc, Mutex};

use serde_json::{Value, json};

use super::{
  tests::{TestSurface, json as json_of, state},
  *,
};
use crate::{
  app::events::{DragEvent, KeyboardEvent, MouseButton, MouseEvent},
  components::{Column, Rect, Slider},
  core::Signal,
  mcp::{
    Scope,
    sessions::{SessionId, SessionLease},
    shared::McpToolResult,
  },
};

type Log = Arc<Mutex<Vec<String>>>;

/// A 100x100 drag handle at the window's origin inside a 400x300 column that logs hover moves and keys.
struct Fixture {
  tree: Tree,
  app: App,
  state: McpState,
  log: Log,
}

fn logger<E>(log: &Log, entry: impl Fn(E) -> String + Send + Sync + 'static) -> impl Fn(E) + Send + Sync + 'static {
  let log = log.clone();
  move |event| log.lock().unwrap().push(entry(event))
}

impl Fixture {
  fn new() -> Self {
    let log = Log::default();
    let mut tree = Tree::new();
    let mut app = App::new();
    tree.resize(400, 300);
    tree.set_root(
      Column::new()
        .width(400.0)
        .height(300.0)
        .on_mouse_move(logger(&log, |event: MouseEvent| {
          format!("hover {} shift={}", event.x, event.shift)
        }))
        .on_key_down(logger(&log, |event: KeyboardEvent| {
          format!("key_down {:?} shift={}", event.key, event.shift)
        }))
        .on_key_up(logger(&log, |event: KeyboardEvent| {
          format!("key_up {:?} shift={}", event.key, event.shift)
        }))
        .child(
          Rect::new(100.0, 100.0)
            .on_mouse_down(logger(&log, |_: MouseEvent| "down".to_owned()))
            .on_mouse_up(logger(&log, |_: MouseEvent| "up".to_owned()))
            .on_click(logger(&log, |_: MouseEvent| "click".to_owned()))
            .on_drag_start(logger(&log, |event: DragEvent| {
              format!("drag_start {}", crate::mcp::held_input::button_name(event.button))
            }))
            .on_drag_move(logger(&log, |event: DragEvent| {
              format!("drag_move {}", event.total_delta_x)
            }))
            .on_drag_end(logger(&log, |event: DragEvent| format!("drag_end {}", event.x))),
        ),
    );
    tree.pass(&mut app, &TestSurface);
    Self {
      tree,
      app,
      state: state(),
      log,
    }
  }

  fn interact(&mut self, session: Option<SessionId>, args: Value) -> McpToolResult {
    let (reply, mut receiver) = tokio::sync::oneshot::channel();
    let request = McpRequest {
      tool: "lurq_interact".into(),
      args,
      reply,
      session,
    };
    execute(&mut self.tree, &mut self.app, &self.state, request);
    receiver.try_recv().expect("lurq_interact answers at once")
  }

  fn ok(&mut self, session: SessionId, args: Value) -> Value {
    json_of(self.interact(Some(session), args))
  }

  fn refused(&mut self, session: Option<SessionId>, args: Value) -> String {
    match self.interact(session, args) {
      Ok(_) => panic!("expected a refusal"),
      Err(message) => message,
    }
  }

  /// What the shell does after a drain that delivered held input: one pass. A redraw requested before it would run
  /// a second pass, which must find nothing to draw.
  fn pass(&mut self) {
    assert!(
      self.tree.pass(&mut self.app, &TestSurface).required,
      "the call is drawn"
    );
    assert!(
      !self.tree.pass(&mut self.app, &TestSurface).required,
      "the call is drawn once"
    );
    self.log.lock().unwrap().push("pass".into());
  }

  fn sweep(&mut self) -> bool {
    release_unreachable_holds(&mut self.tree, &self.state)
  }

  fn take_log(&self) -> Vec<String> {
    std::mem::take(&mut *self.log.lock().unwrap())
  }

  fn held(&mut self) -> Value {
    json_of(super::tests::call(
      &mut self.tree,
      &mut self.app,
      &self.state,
      "lurq_windows",
      json!({}),
    ))["windows"][0]["held"]
      .clone()
  }
}

const SESSION: SessionId = 7;

#[test]
fn press_moves_and_release_are_one_drag_with_a_pass_between_calls() {
  let mut f = Fixture::new();
  let pressed = f.ok(SESSION, json!({"action": "press", "x": 50, "y": 50}));
  assert_eq!(pressed["held"], json!({"buttons": ["left"], "keys": []}));
  assert!(
    f.tree.take_mcp_input_present(),
    "the shell presents held input after the drain"
  );
  f.pass();
  // The last two moves leave the handle: the drag keeps the pointer, as a held button does.
  for x in [70, 90, 110, 130] {
    let moved = f.ok(SESSION, json!({"action": "move", "x": x, "y": 60}));
    assert_eq!(moved["held"]["buttons"], json!(["left"]));
    assert!(f.tree.take_mcp_input_present());
    f.pass();
  }
  let released = f.ok(SESSION, json!({"action": "release"}));
  assert_eq!((&released["x"], &released["y"]), (&json!(130.0), &json!(60.0)));
  assert_eq!(released["held"], json!({"buttons": [], "keys": []}));
  f.pass();

  assert_eq!(
    f.take_log(),
    [
      "hover 50 shift=false",
      "down",
      "drag_start left",
      "pass",
      "drag_move 20",
      "pass",
      "drag_move 40",
      "pass",
      "drag_move 60",
      "pass",
      "drag_move 80",
      "pass",
      "drag_end 130",
      "pass",
    ]
  );
  assert_eq!(f.held(), json!({"buttons": [], "keys": []}));
}

#[test]
fn a_held_middle_button_drags_and_is_released_by_name() {
  let mut f = Fixture::new();
  f.ok(
    SESSION,
    json!({"action": "press", "x": 10, "y": 10, "button": "middle"}),
  );
  f.ok(SESSION, json!({"action": "move", "x": 30, "y": 10}));
  assert!(
    f.refused(Some(SESSION), json!({"action": "release"}))
      .contains("the left button is not held")
  );
  let released = f.ok(
    SESSION,
    json!({"action": "release", "button": "middle", "x": 40, "y": 10}),
  );
  assert_eq!(released["button"], "middle");
  assert_eq!(
    f.take_log(),
    [
      "hover 10 shift=false",
      "down",
      "drag_start middle",
      "drag_move 20",
      "drag_move 30",
      "drag_end 40"
    ]
  );
}

#[test]
fn a_press_and_release_in_place_is_a_click() {
  let mut f = Fixture::new();
  f.ok(SESSION, json!({"action": "press", "x": 20, "y": 20}));
  f.pass();
  f.ok(SESSION, json!({"action": "release"}));
  assert_eq!(
    f.take_log(),
    [
      "hover 20 shift=false",
      "down",
      "drag_start left",
      "pass",
      "drag_end 20",
      "click"
    ]
  );
}

#[test]
fn invalid_sequences_are_refused() {
  let mut f = Fixture::new();
  let message = f.refused(Some(SESSION), json!({"action": "release"}));
  assert_eq!(
    message,
    "the left button is not held in window \"main\"; press it first"
  );
  let message = f.refused(Some(SESSION), json!({"action": "key_up", "key": "a"}));
  assert_eq!(message, "key \"a\" is not held in window \"main\"; key_down it first");
  assert!(
    f.refused(None, json!({"action": "press", "x": 5, "y": 5}))
      .contains("needs a session-based MCP connection")
  );
  assert!(
    f.refused(None, json!({"action": "key_down", "key": "a"}))
      .contains("needs a session-based MCP connection")
  );
  assert!(
    f.refused(
      Some(SESSION),
      json!({"action": "press", "x": 5, "y": 5, "button": "back"})
    )
    .contains("unknown button \"back\"")
  );
  assert!(
    f.refused(Some(SESSION), json!({"action": "press"}))
      .contains("needs a `ref` or `x`/`y`")
  );
  assert_eq!(f.held(), json!({"buttons": [], "keys": []}), "refusals hold nothing");

  f.ok(SESSION, json!({"action": "press", "x": 5, "y": 5}));
  let message = f.refused(Some(SESSION), json!({"action": "press", "x": 50, "y": 50}));
  assert_eq!(
    message,
    "the left button is already held in window \"main\"; release it first"
  );
  // Actions that would press and release the held button would end the hold behind its back.
  for action in [
    json!({"action": "click", "x": 5, "y": 5}),
    json!({"action": "double_click", "x": 5, "y": 5}),
    json!({"action": "drag", "x": 5, "y": 5, "to_x": 9, "to_y": 9}),
  ] {
    assert!(
      f.refused(None, action)
        .contains("the left button is held in window \"main\"")
    );
  }
  f.ok(SESSION, json!({"action": "key_down", "key": "a"}));
  let message = f.refused(Some(SESSION), json!({"action": "key_down", "key": "a"}));
  assert!(
    message.starts_with("key \"a\" is already held in window \"main\""),
    "{message}"
  );
  assert!(
    f.refused(None, json!({"action": "key", "key": "a"}))
      .contains("key \"a\" is held")
  );
  // Another button and another key are separate holds.
  f.ok(SESSION, json!({"action": "press", "x": 5, "y": 5, "button": "right"}));
  assert!(f.interact(None, json!({"action": "key", "key": "b"})).is_ok());
  assert_eq!(f.held(), json!({"buttons": ["left", "right"], "keys": ["a"]}));
  // Any client may release.
  for action in [
    json!({"action": "release", "button": "right"}),
    json!({"action": "release"}),
    json!({"action": "key_up", "key": "a"}),
  ] {
    assert!(f.interact(None, action).is_ok());
  }
  assert_eq!(f.held(), json!({"buttons": [], "keys": []}));
}

/// A held drag changes only on input, which requests its own redraw: held still, nothing ticks and a pass has
/// nothing to draw, so the shell's loop waits instead of polling.
fn assert_idle_while_held(f: &mut Fixture) {
  assert!(f.tree.pass(&mut f.app, &TestSurface).required, "the press is drawn");
  assert!(f.tree.has_active_input_interaction());
  assert!(
    !f.tree.has_active_tick_sources(),
    "nothing ticks while the pointer is still"
  );
  assert!(!f.tree.needs_redraw());
  assert!(!f.tree.pass(&mut f.app, &TestSurface).required);
}

#[test]
fn a_held_drag_or_slider_held_still_lets_the_loop_wait() {
  let mut f = Fixture::new();
  f.ok(SESSION, json!({"action": "press", "x": 50, "y": 50}));
  assert_idle_while_held(&mut f);
  f.ok(SESSION, json!({"action": "move", "x": 70, "y": 50}));
  assert!(f.tree.pass(&mut f.app, &TestSurface).required, "a held move is drawn");

  // A real press is the same.
  let mut f = Fixture::new();
  f.tree.mouse_down(50.0, 50.0, MouseButton::Left);
  assert_idle_while_held(&mut f);

  let value = Signal::new(0);
  let mut tree = Tree::new();
  let mut app = App::new();
  tree.set_root(Slider::new(value.clone()).range(0, 10).width(100.0).height(20.0));
  tree.pass(&mut app, &TestSurface);
  let mut f = Fixture {
    tree,
    app,
    state: state(),
    log: Log::default(),
  };
  f.ok(SESSION, json!({"action": "press", "x": 0, "y": 10}));
  assert_idle_while_held(&mut f);
  f.ok(SESSION, json!({"action": "move", "x": 75, "y": 10}));
  assert!(f.tree.pass(&mut f.app, &TestSurface).required);
  f.ok(SESSION, json!({"action": "release"}));
  assert_eq!(value.get(), 8, "the slider followed the held move");
}

/// Held modifier keys reach every action in their window, as real keys that are down do. Pointer events carry
/// shift, ctrl and alt; key events also meta.
#[test]
fn held_modifiers_reach_every_action() {
  let log = Log::default();
  let pointer = |kind: &'static str| {
    logger(&log, move |event: MouseEvent| {
      format!("{kind} shift={} ctrl={} alt={}", event.shift, event.ctrl, event.alt)
    })
  };
  let key = logger(&log, |event: KeyboardEvent| {
    format!(
      "key {:?} shift={} ctrl={} alt={} meta={}",
      event.key, event.shift, event.ctrl, event.alt, event.meta
    )
  });
  let mut tree = Tree::new();
  let mut app = App::new();
  tree.resize(400, 300);
  tree.set_root(
    Column::new()
      .width(400.0)
      .height(300.0)
      .on_mouse_down(pointer("down"))
      .on_click(pointer("click"))
      .on_key_down(key),
  );
  tree.pass(&mut app, &TestSurface);
  let mut f = Fixture {
    tree,
    app,
    state: state(),
    log,
  };

  f.ok(SESSION, json!({"action": "key_down", "key": "Control"}));
  f.ok(SESSION, json!({"action": "key", "key": "a"}));
  f.ok(SESSION, json!({"action": "key_down", "key": "Shift"}));
  f.ok(SESSION, json!({"action": "click", "x": 20, "y": 20}));
  f.ok(SESSION, json!({"action": "double_click", "x": 300, "y": 200}));
  f.ok(SESSION, json!({"action": "type", "text": "b"}));
  f.ok(SESSION, json!({"action": "key_up", "key": "Control"}));
  f.ok(SESSION, json!({"action": "key_down", "key": "Meta"}));
  // The request's own modifiers add to the held ones.
  f.ok(
    SESSION,
    json!({"action": "click", "x": 20, "y": 20, "modifiers": ["alt"]}),
  );
  f.ok(SESSION, json!({"action": "key", "key": "c"}));
  assert_eq!(
    f.take_log(),
    [
      "key \"Control\" shift=false ctrl=true alt=false meta=false",
      "key \"a\" shift=false ctrl=true alt=false meta=false",
      "key \"Shift\" shift=true ctrl=true alt=false meta=false",
      "down shift=true ctrl=true alt=false",
      "click shift=true ctrl=true alt=false",
      "down shift=true ctrl=true alt=false",
      "click shift=true ctrl=true alt=false",
      "down shift=true ctrl=true alt=false",
      "click shift=true ctrl=true alt=false",
      "key \"b\" shift=true ctrl=true alt=false meta=false",
      "key \"Meta\" shift=true ctrl=false alt=false meta=true",
      "down shift=true ctrl=false alt=true",
      "click shift=true ctrl=false alt=true",
      "key \"c\" shift=true ctrl=false alt=false meta=true",
    ]
  );
}

#[test]
fn input_actions_without_holds_are_unchanged() {
  let mut f = Fixture::new();
  let moved = f.ok(SESSION, json!({"action": "move", "x": 150, "y": 40}));
  assert_eq!(
    moved,
    json!({"ok": true, "action": "move", "window": "main", "x": 150.0, "y": 40.0})
  );
  assert!(!f.tree.take_mcp_input_present());
  f.ok(SESSION, json!({"action": "click", "x": 20, "y": 20}));
  assert!(!f.tree.take_mcp_input_present());
  assert_eq!(
    f.take_log(),
    [
      "hover 150 shift=false",
      "hover 20 shift=false",
      "down",
      "drag_start left",
      "drag_end 20",
      "click"
    ]
  );
}

#[test]
fn held_keys_stay_down_across_calls_and_modify_moves() {
  let mut f = Fixture::new();
  f.ok(SESSION, json!({"action": "key_down", "key": "Shift"}));
  let down = f.ok(SESSION, json!({"action": "key_down", "key": " "}));
  assert_eq!(down["held"], json!({"buttons": [], "keys": ["Shift", " "]}));
  f.pass();
  // A key alone makes moves held moves (space-drag panning), presented each in its own pass.
  let moved = f.ok(SESSION, json!({"action": "move", "x": 200, "y": 40}));
  assert_eq!(moved["held"]["keys"], json!(["Shift", " "]));
  assert!(f.tree.take_mcp_input_present());
  f.pass();
  f.ok(SESSION, json!({"action": "key_up", "key": " "}));
  let up = f.ok(SESSION, json!({"action": "key_up", "key": "Shift"}));
  assert_eq!(up["held"], json!({"buttons": [], "keys": []}));
  let moved = f.ok(SESSION, json!({"action": "move", "x": 210, "y": 40}));
  assert!(moved.get("held").is_none());
  assert_eq!(
    f.take_log(),
    [
      "key_down \"Shift\" shift=true",
      "key_down \" \" shift=true",
      "pass",
      "hover 200 shift=true",
      "pass",
      "key_up \" \" shift=true",
      "key_up \"Shift\" shift=false",
      "hover 210 shift=false",
    ]
  );
}

#[test]
fn what_an_ended_session_held_is_released_and_others_stay() {
  let mut f = Fixture::new();
  let (first, second) = (
    SessionLease::new(f.state.shared.clone()),
    SessionLease::new(f.state.shared.clone()),
  );
  f.ok(first.route(), json!({"action": "press", "x": 50, "y": 50}));
  f.ok(second.route(), json!({"action": "key_down", "key": "Shift"}));
  f.take_log();
  assert!(!f.sweep(), "nothing is released while both sessions live");

  drop(first);
  assert!(f.sweep());
  // The press ends as an abandoned one: an up event, the drag ends, no click.
  assert_eq!(f.take_log(), ["drag_end 50", "up"]);
  assert_eq!(f.held(), json!({"buttons": [], "keys": ["Shift"]}));
  let message = f.refused(Some(SESSION), json!({"action": "release"}));
  assert_eq!(
    message,
    "the left button is not held in window \"main\": held input there was released because the MCP session that \
     held it ended; press it first"
  );

  drop(second);
  assert!(f.sweep());
  assert_eq!(f.take_log(), ["key_up \"Shift\" shift=false"]);
  assert_eq!(f.held(), json!({"buttons": [], "keys": []}));
  assert!(!f.sweep());
}

#[test]
fn a_lease_that_routed_no_call_ends_nothing() {
  let f = Fixture::new();
  drop(SessionLease::new(f.state.shared.clone()));
  assert!(f.state.shared.sessions.take_ended().is_empty());
}

#[test]
fn losing_focus_releases_but_a_press_while_unfocused_holds() {
  let mut f = Fixture::new();
  f.tree.window().set_focused(false);
  f.ok(SESSION, json!({"action": "press", "x": 50, "y": 50}));
  f.ok(SESSION, json!({"action": "key_down", "key": "a"}));
  assert!(!f.sweep(), "an unfocused window keeps what is pressed in it");
  f.tree.window().set_focused(true);
  assert!(!f.sweep());
  f.take_log();

  f.tree.window().set_focused(false);
  assert!(f.sweep());
  assert_eq!(f.take_log(), ["drag_end 50", "up", "key_up \"a\" shift=false"]);
  assert!(
    f.refused(Some(SESSION), json!({"action": "key_up", "key": "a"}))
      .contains("released because the window lost focus")
  );
  // A client's own hold clears the explanation.
  f.ok(SESSION, json!({"action": "key_down", "key": "a"}));
  f.ok(SESSION, json!({"action": "key_up", "key": "a"}));
  assert_eq!(
    f.refused(Some(SESSION), json!({"action": "key_up", "key": "a"})),
    "key \"a\" is not held in window \"main\"; key_down it first"
  );
}

#[test]
fn closing_the_window_releases() {
  let mut f = Fixture::new();
  f.ok(SESSION, json!({"action": "press", "x": 50, "y": 50}));
  f.take_log();
  f.tree
    .window()
    .dispatch_close_request(crate::app::CloseRequestSource::App);
  assert!(f.sweep());
  assert_eq!(f.take_log(), ["drag_end 50", "up"]);
  assert!(
    f.refused(Some(SESSION), json!({"action": "release"}))
      .contains("released because the window closed")
  );
}

/// A secondary window is addressed by name but held under its id, and its tree takes the release after it closed.
#[test]
fn a_secondary_window_holds_by_id_and_closing_it_releases() {
  let mut f = Fixture::new();
  let palette_log = Log::default();
  let entries = palette_log.clone();
  f.app.window_opener().open_with(
    crate::app::WindowOptions::new("Palette", 300, 200).window_name("palette"),
    move |_, tree| {
      tree.resize(300, 200);
      tree.set_root(
        Rect::new(100.0, 100.0)
          .on_drag_end(logger(&entries, |event: DragEvent| format!("drag_end {}", event.x)))
          .on_mouse_up(logger(&entries, |_: MouseEvent| "up".to_owned())),
      );
    },
  );
  assert!(f.tree.apply_secondary_window_requests(&mut f.app));
  let palette = f.tree.secondary_window_mut(0).unwrap();
  let id = format!("w{}", palette.id());
  palette.tree_mut().pass(&mut f.app, &TestSurface);

  let pressed = f.ok(
    SESSION,
    json!({"action": "press", "x": 30, "y": 30, "window": "palette"}),
  );
  assert_eq!(pressed["window"], id);
  let windows = json_of(super::tests::call(
    &mut f.tree,
    &mut f.app,
    &f.state,
    "lurq_windows",
    json!({}),
  ));
  assert_eq!(windows["windows"][0]["held"]["buttons"], json!([]));
  assert_eq!(windows["windows"][1]["id"], id);
  assert_eq!(windows["windows"][1]["held"]["buttons"], json!(["left"]));
  assert!(!f.sweep());

  f.tree.close_secondary_window(0);
  assert!(f.tree.secondary_window(0).is_none(), "the palette closed");
  assert!(f.sweep());
  assert_eq!(*palette_log.lock().unwrap(), ["drag_end 30", "up"]);
  assert!(f.take_log().is_empty(), "the main window got nothing");
}

#[test]
fn losing_the_interact_scope_releases() {
  let mut f = Fixture::new();
  f.ok(SESSION, json!({"action": "key_down", "key": "Shift"}));
  f.take_log();
  f.state.shared.remove_scope(&Scope::Interact);
  assert!(f.sweep());
  assert_eq!(f.take_log(), ["key_up \"Shift\" shift=false"]);
}

/// Through the drain the shell runs: a call routed with its session, the end of that session, and a server stop.
#[test]
fn the_drain_releases_what_ended_sessions_held_and_shutdown_releases_the_rest() {
  let Fixture {
    mut tree,
    mut app,
    state: mut mcp,
    log,
  } = Fixture::new();
  let (sender, receiver) = std::sync::mpsc::channel();
  mcp.receiver = receiver;
  let shared = mcp.shared.clone();
  tree.mcp = Some(Box::new(mcp));
  let press = |tree: &mut Tree, app: &mut App, session: SessionId| {
    let (reply, mut answer) = tokio::sync::oneshot::channel();
    let args = json!({"action": "press", "x": 50, "y": 50});
    sender
      .send(McpRequest {
        tool: "lurq_interact".into(),
        args,
        reply,
        session: Some(session),
      })
      .unwrap();
    assert!(tree.drain_mcp_requests(app));
    json_of(answer.try_recv().unwrap())
  };

  let lease = SessionLease::new(shared.clone());
  assert_eq!(
    press(&mut tree, &mut app, lease.route())["held"]["buttons"],
    json!(["left"])
  );
  assert!(!tree.drain_mcp_requests(&mut app));
  log.lock().unwrap().clear();
  drop(lease);
  assert!(
    tree.drain_mcp_requests(&mut app),
    "the drain released the ended session's press"
  );
  assert_eq!(std::mem::take(&mut *log.lock().unwrap()), ["drag_end 50", "up"]);

  let lease = SessionLease::new(shared.clone());
  press(&mut tree, &mut app, lease.route());
  log.lock().unwrap().clear();
  tree.shutdown_mcp();
  assert_eq!(*log.lock().unwrap(), ["drag_end 50", "up"]);
  drop(lease);
}
