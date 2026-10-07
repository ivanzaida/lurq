use std::sync::Arc;

use super::{
  tests::{call, json, state},
  windows::resize_reply,
  *,
};
use crate::{
  app::window::{
    WindowCommand,
    resize::{ResizeOutcome, WindowModes},
    simulated_window::{MINIMIZED_CLIENT, SimulatedWindow},
  },
  mcp::shared::McpToolResult,
};

const MINIMIZED: WindowModes = WindowModes {
  minimized: true,
  maximized: false,
  full_screen: false,
};

/// Sends `lurq_resize` and applies the queued resize the way the winit shell does, to `native`.
fn resize(tree: &mut Tree, native: &SimulatedWindow, args: serde_json::Value) -> McpToolResult {
  let (reply, mut receiver) = tokio::sync::oneshot::channel();
  execute(
    tree,
    &mut App::new(),
    &state(),
    McpRequest {
      tool: "lurq_resize".into(),
      args,
      reply,
      session: None,
    },
  );
  assert!(receiver.try_recv().is_err(), "the reply waits for the shell");
  for command in tree.window().take_commands() {
    let WindowCommand::Resize(request) = command else {
      panic!("expected a resize, got {command:?}");
    };
    tree.window().apply_resize(Some(native), request);
  }
  receiver.try_recv().expect("the shell answered the resize")
}

fn error(result: McpToolResult) -> String {
  match result {
    Err(error) => error,
    Ok(_) => panic!("expected an error"),
  }
}

fn shown_tree() -> Tree {
  let tree = Tree::new();
  tree.window().attach_shell(Arc::new(|| {}));
  tree
}

#[test]
fn windows_report_minimized_maximized_and_full_screen() {
  let mut tree = Tree::new();
  let mut app = App::new();
  let state = state();
  let main = |tree: &mut Tree, app: &mut App| {
    json(call(tree, app, &state, "lurq_windows", serde_json::json!({})))["windows"][0].clone()
  };
  let shown = main(&mut tree, &mut app);
  assert_eq!(
    (&shown["minimized"], &shown["maximized"], &shown["full_screen"]),
    (&false.into(), &false.into(), &false.into())
  );
  tree.window().set_minimized(true);
  tree.window().set_maximized(true);
  tree.window().set_full_screen(true);
  let hidden = main(&mut tree, &mut app);
  assert_eq!(
    (&hidden["minimized"], &hidden["maximized"], &hidden["full_screen"]),
    (&true.into(), &true.into(), &true.into())
  );
}

#[test]
fn resize_restores_a_minimized_window_and_answers_with_the_size_it_took() {
  let mut tree = shown_tree();
  let native = SimulatedWindow::new((1440, 1020), MINIMIZED);

  let reply = json(resize(
    &mut tree,
    &native,
    serde_json::json!({"width": 1280, "height": 800}),
  ));

  assert_eq!(native.calls(), ["set_minimized(false)", "request_inner_size(1280x800)"]);
  assert_eq!(
    reply,
    serde_json::json!({
      "ok": true, "window": "main", "width": 1280, "height": 800,
      "restored_from": ["minimized"], "still": [],
    })
  );
}

#[test]
fn resize_that_leaves_the_window_minimized_is_an_error_with_its_real_size() {
  let mut tree = shown_tree();
  let native = SimulatedWindow::new((1440, 1020), MINIMIZED).refusing_restore();

  let error = error(resize(
    &mut tree,
    &native,
    serde_json::json!({"width": 1280, "height": 800}),
  ));

  let (width, height) = MINIMIZED_CLIENT;
  assert!(
    error.contains(&format!(
      "is {width}x{height}, not the requested 1280x800: it is still minimized"
    )),
    "{error}"
  );
}

#[test]
fn resize_reports_a_size_the_platform_limited() {
  let reply = resize_reply(
    "main",
    Some(ResizeOutcome {
      requested: (100, 100),
      size: (640, 480),
      left: WindowModes::default(),
      remaining: WindowModes::default(),
    }),
  );
  assert!(error(reply).contains("is 640x480, not the requested 100x100: the platform limited the size"));
}

#[test]
fn resize_without_a_native_window_says_so_at_once() {
  let mut tree = Tree::new();
  let error = error(call(
    &mut tree,
    &mut App::new(),
    &state(),
    "lurq_resize",
    serde_json::json!({"width": 800, "height": 600}),
  ));
  assert!(error.contains("has no native window to resize"), "{error}");
  assert!(tree.window().take_commands().is_empty());
}

#[test]
fn resize_rejects_sizes_that_are_zero_or_too_large() {
  let mut tree = shown_tree();
  for args in [
    serde_json::json!({"width": 0, "height": 600}),
    serde_json::json!({"width": 800, "height": u64::from(u32::MAX) + 1}),
    serde_json::json!({"width": 800}),
  ] {
    assert!(call(&mut tree, &mut App::new(), &state(), "lurq_resize", args).is_err());
  }
  assert!(tree.window().take_commands().is_empty());
}

#[test]
fn resize_takes_a_full_screen_window_out_of_full_screen() {
  let mut tree = shown_tree();
  let full_screen = WindowModes {
    full_screen: true,
    ..WindowModes::default()
  };
  let native = SimulatedWindow::new((1440, 1020), full_screen);

  let reply = json(resize(
    &mut tree,
    &native,
    serde_json::json!({"width": 1280, "height": 800}),
  ));

  assert_eq!(native.calls(), ["leave_full_screen", "request_inner_size(1280x800)"]);
  assert_eq!(reply["restored_from"], serde_json::json!(["full_screen"]));
}

#[test]
fn a_resize_call_that_timed_out_leaves_no_report_behind() {
  let mut tree = shown_tree();
  let (reply, receiver) = tokio::sync::oneshot::channel();
  execute(
    &mut tree,
    &mut App::new(),
    &state(),
    McpRequest {
      tool: "lurq_resize".into(),
      args: serde_json::json!({"width": 800, "height": 600}),
      reply,
      session: None,
    },
  );
  // The HTTP side gives up and drops its end.
  drop(receiver);
  assert_eq!(tree.window().pending_resize_reports(), 1);

  let native = SimulatedWindow::new((1440, 1020), WindowModes::default());
  json(resize(
    &mut tree,
    &native,
    serde_json::json!({"width": 1024, "height": 600}),
  ));
  assert_eq!(tree.window().pending_resize_reports(), 0);
}
