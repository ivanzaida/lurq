//! `lurq_interact` actions that hold input across calls (`press`, `release`,
//! `key_down`, `key_up`, and `move` while something is held), and the release
//! of held input a client can no longer release itself.
//!
//! Held input goes through the same tree entry points as a real button or key
//! held down, so a held `move` is a real held-button move: the tree's own press,
//! drag and capture state decide what it does. Each call that delivers held
//! input also asks the shell to present right after the drain, as it does from
//! inside a real mouse-move stream, so every call lands in its own pass.

use serde_json::Value;

use super::{
  interact::{parse_button, parse_modifiers, resolve_point},
  windows::{canonical_window_id, requested_window, window_tree_mut},
};
use crate::{
  app::{
    Tree,
    events::MouseButton,
    synthetic_input::{self, SyntheticInput, SyntheticModifiers},
  },
  mcp::{
    McpState, Scope,
    held_input::{HeldButton, HeldInput, HeldKey, ReleaseReason, WindowHolds, button_name, key_modifiers},
    sessions::SessionId,
    shared::{McpToolOutput, McpToolResult},
  },
};

const INTERACT_TOOL: &str = "lurq_interact";

/// Runs `action` when it holds or releases input, or meets input held in its window; `None` leaves it to the
/// other actions, unchanged.
pub(super) fn held_action(
  tree: &mut Tree,
  state: &McpState,
  args: &Value,
  action: &str,
  session: Option<SessionId>,
) -> Option<McpToolResult> {
  let holding = !state.held.lock().is_empty();
  match action {
    "press" => Some(press(tree, state, args, session)),
    "release" => Some(release(tree, state, args)),
    "key_down" => Some(key_down(tree, state, args, session)),
    "key_up" => Some(key_up(tree, state, args)),
    _ if !holding => None,
    "move" => held_move(tree, state, args),
    "click" | "double_click" | "drag" => button_conflict(tree, state, args, action).map(Err),
    "key" => key_conflict(tree, state, args).map(Err),
    _ => None,
  }
}

fn press(tree: &mut Tree, state: &McpState, args: &Value, session: Option<SessionId>) -> McpToolResult {
  let session = session.ok_or_else(|| needs_session("press"))?;
  let button = held_button(args)?;
  let (window, point) = resolve_point(tree, state, args, "ref", "x", "y")?;
  let (x, y) = point.ok_or("press needs a `ref` or `x`/`y` coordinates")?;
  let window = canonical_window_id(tree, &window, state.include_devtools)?;
  let target = window_tree_mut(tree, &window, state.include_devtools)?;
  let mut held = state.held.lock();
  if held.get(&window).is_some_and(|holds| holds.holds_button(button)) {
    return Err(format!(
      "the {} button is already held in window {window:?}; release it first",
      button_name(button)
    ));
  }
  let holds = held.entry(&window, target.window().focus_losses());
  holds.buttons.push(HeldButton { button, session });
  holds.pointer = Some((x, y));
  let modifiers = union(parse_modifiers(args), holds.modifiers());
  held.settle(&window);
  let report = held.report(&window);
  drop(held);
  // Hover first, as `click` does: hit testing reads the last motion event.
  deliver(
    target,
    [
      SyntheticInput::mouse_move(x, y),
      SyntheticInput::mouse_down(x, y, button),
    ],
    modifiers,
  );
  Ok(McpToolOutput::Json(serde_json::json!({
    "ok": true, "action": "press", "window": window, "button": button_name(button), "x": x, "y": y, "held": report
  })))
}

fn release(tree: &mut Tree, state: &McpState, args: &Value) -> McpToolResult {
  let button = held_button(args)?;
  let (window, point) = resolve_point(tree, state, args, "ref", "x", "y")?;
  let window = canonical_window_id(tree, &window, state.include_devtools)?;
  let target = window_tree_mut(tree, &window, state.include_devtools)?;
  let mut held = state.held.lock();
  let Some(holds) = held.get_mut(&window).filter(|holds| holds.holds_button(button)) else {
    let what = format!("the {} button", button_name(button));
    return Err(not_held(&held, &window, &what, "press it first"));
  };
  let from = holds.pointer;
  let (x, y) = point.or(from).ok_or("release needs a `ref` or `x`/`y` coordinates")?;
  holds.buttons.retain(|held| held.button != button);
  holds.pointer = Some((x, y));
  let modifiers = union(parse_modifiers(args), holds.modifiers());
  held.settle(&window);
  let report = held.report(&window);
  drop(held);
  let mut inputs = Vec::new();
  // A release elsewhere follows a move there, as real input does.
  if from != Some((x, y)) {
    inputs.push(SyntheticInput::mouse_move(x, y));
  }
  inputs.push(SyntheticInput::mouse_up(x, y, button));
  deliver(target, inputs, modifiers);
  Ok(McpToolOutput::Json(serde_json::json!({
    "ok": true, "action": "release", "window": window, "button": button_name(button), "x": x, "y": y, "held": report
  })))
}

fn key_down(tree: &mut Tree, state: &McpState, args: &Value, session: Option<SessionId>) -> McpToolResult {
  let session = session.ok_or_else(|| needs_session("key_down"))?;
  let key = key_arg(args, "key_down")?;
  let window = canonical_window_id(tree, &requested_window(args), state.include_devtools)?;
  let target = window_tree_mut(tree, &window, state.include_devtools)?;
  let mut held = state.held.lock();
  if held.get(&window).is_some_and(|holds| holds.holds_key(key)) {
    return Err(format!(
      "key {key:?} is already held in window {window:?}; key_up it first (the `key` action presses and releases)"
    ));
  }
  let holds = held.entry(&window, target.window().focus_losses());
  holds.keys.push(HeldKey {
    key: key.to_owned(),
    session,
  });
  let modifiers = union(parse_modifiers(args), holds.modifiers());
  held.settle(&window);
  let report = held.report(&window);
  drop(held);
  deliver(target, [SyntheticInput::key_down(key)], modifiers);
  Ok(McpToolOutput::Json(serde_json::json!({
    "ok": true, "action": "key_down", "key": key, "window": window, "held": report
  })))
}

fn key_up(tree: &mut Tree, state: &McpState, args: &Value) -> McpToolResult {
  let key = key_arg(args, "key_up")?;
  let window = canonical_window_id(tree, &requested_window(args), state.include_devtools)?;
  let target = window_tree_mut(tree, &window, state.include_devtools)?;
  let mut held = state.held.lock();
  let Some(holds) = held.get_mut(&window).filter(|holds| holds.holds_key(key)) else {
    return Err(not_held(&held, &window, &format!("key {key:?}"), "key_down it first"));
  };
  holds.keys.retain(|held| held.key != key);
  let modifiers = union(parse_modifiers(args), holds.modifiers());
  held.settle(&window);
  let report = held.report(&window);
  drop(held);
  deliver(target, [SyntheticInput::key_up(key)], modifiers);
  Ok(McpToolOutput::Json(serde_json::json!({
    "ok": true, "action": "key_up", "key": key, "window": window, "held": report
  })))
}

/// A `move` in a window that holds input. Arguments the plain `move` would refuse are left to it.
fn held_move(tree: &mut Tree, state: &McpState, args: &Value) -> Option<McpToolResult> {
  let (window, point) = resolve_point(tree, state, args, "ref", "x", "y").ok()?;
  let (x, y) = point?;
  let window = canonical_window_id(tree, &window, state.include_devtools).ok()?;
  let target = window_tree_mut(tree, &window, state.include_devtools).ok()?;
  let mut held = state.held.lock();
  let holds = held.get_mut(&window)?;
  holds.pointer = Some((x, y));
  let modifiers = union(parse_modifiers(args), holds.modifiers());
  let report = holds.report();
  drop(held);
  deliver(target, [SyntheticInput::mouse_move(x, y)], modifiers);
  Some(Ok(McpToolOutput::Json(serde_json::json!({
    "ok": true, "action": "move", "window": window, "x": x, "y": y, "held": report
  }))))
}

/// `click`, `double_click` and `drag` press and release their button, which would end a hold of it.
fn button_conflict(tree: &mut Tree, state: &McpState, args: &Value, action: &str) -> Option<String> {
  let (window, _) = resolve_point(tree, state, args, "ref", "x", "y").ok()?;
  let window = canonical_window_id(tree, &window, state.include_devtools).ok()?;
  let button = parse_button(args);
  state.held.lock().get(&window)?.holds_button(button).then(|| {
    format!(
      "the {} button is held in window {window:?}; release it before {action}",
      button_name(button)
    )
  })
}

/// `key` presses and releases its key, which would end a hold of it.
fn key_conflict(tree: &mut Tree, state: &McpState, args: &Value) -> Option<String> {
  let key = args.get("key")?.as_str()?;
  let window = canonical_window_id(tree, &requested_window(args), state.include_devtools).ok()?;
  state
    .held
    .lock()
    .get(&window)?
    .holds_key(key)
    .then(|| format!("key {key:?} is held in window {window:?}; key_up it instead"))
}

/// Releases what clients hold but can no longer release themselves: what a session held once it ended, and
/// everything in a window that lost focus since its first hold or closed, or once `lurq_interact` is unavailable.
/// Returns whether anything was released.
pub(crate) fn release_unreachable_holds(root: &mut Tree, state: &McpState) -> bool {
  let ended = state.shared.sessions.take_ended();
  let shared = &state.shared;
  let unavailable = !shared.is_enabled() || !shared.has_scope(&Scope::Interact) || shared.is_denied(INTERACT_TOOL);
  let mut held = state.held.lock();
  if held.is_empty() {
    return false;
  }
  let mut released = Vec::new();
  for window in held.window_ids() {
    let reason = if unavailable {
      Some(ReleaseReason::Unavailable)
    } else {
      match held_window_tree(root, &window) {
        Some((tree, true)) => held
          .get(&window)
          .is_some_and(|holds| holds.focus_losses != tree.window().focus_losses())
          .then_some(ReleaseReason::FocusLost),
        _ => Some(ReleaseReason::WindowClosed),
      }
    };
    let taken = match reason {
      Some(reason) => held.take(&window, reason).map(|holds| (holds, reason)),
      None => held
        .take_sessions(&window, &ended)
        .map(|holds| (holds, ReleaseReason::SessionEnded)),
    };
    if let Some((holds, reason)) = taken {
      released.push(Released::new(window, holds, reason, &held));
    }
  }
  drop(held);
  let any = !released.is_empty();
  deliver_releases(root, released);
  any
}

/// Releases everything clients hold, as the server stops.
pub(crate) fn release_all_holds(root: &mut Tree, state: &McpState) {
  let mut held = state.held.lock();
  let released = held
    .window_ids()
    .into_iter()
    .filter_map(|window| {
      let holds = held.take(&window, ReleaseReason::ServerStopped)?;
      Some(Released::new(window, holds, ReleaseReason::ServerStopped, &held))
    })
    .collect();
  drop(held);
  deliver_releases(root, released);
}

/// Held input taken from a window, to end there.
struct Released {
  window: String,
  holds: WindowHolds,
  reason: ReleaseReason,
  /// The modifiers of keys still held there by other sessions.
  still: SyntheticModifiers,
}

impl Released {
  fn new(window: String, holds: WindowHolds, reason: ReleaseReason, held: &HeldInput) -> Self {
    let still = held.get(&window).map(WindowHolds::modifiers).unwrap_or_default();
    Self {
      window,
      holds,
      reason,
      still,
    }
  }
}

/// Ends each released hold as an abandoned gesture: a button the way the OS ends a press it takes over (an up
/// event, no click; a drag ends as a miss), a key with its up event.
fn deliver_releases(root: &mut Tree, released: Vec<Released>) {
  for Released {
    window,
    holds,
    reason,
    still,
  } in released
  {
    tracing::info!(
      "lurq MCP released held input in window {window:?} ({}) because {}",
      holds.report(),
      reason.describe()
    );
    let Some((tree, _)) = held_window_tree(root, &window) else {
      continue;
    };
    let (x, y) = holds.pointer.unwrap_or_default();
    for held in holds.buttons.iter().rev() {
      tree.mouse_press_taken_by_os(x, y, held.button);
    }
    let mut keys = holds.keys;
    while let Some(held) = keys.pop() {
      let modifiers = union(key_modifiers(&keys), still);
      synthetic_input::apply(tree, &SyntheticInput::key_up(held.key).with_modifiers(modifiers));
    }
    tree.request_redraw();
  }
}

/// The tree behind a canonical window id, open or closed, and whether it is open and not closing.
fn held_window_tree<'t>(root: &'t mut Tree, window: &str) -> Option<(&'t mut Tree, bool)> {
  let (tree, open) = if window == "main" {
    (root, true)
  } else {
    root.secondary_tree_by_id_mut(window.strip_prefix('w')?.parse().ok()?)?
  };
  let open = open && !tree.window().close_accepted();
  Some((tree, open))
}

fn deliver(target: &mut Tree, inputs: impl IntoIterator<Item = SyntheticInput>, modifiers: SyntheticModifiers) {
  for input in inputs {
    synthetic_input::apply(target, &input.with_modifiers(modifiers));
  }
  target.request_redraw();
  target.mcp_input_present = true;
}

fn union(a: SyntheticModifiers, b: SyntheticModifiers) -> SyntheticModifiers {
  SyntheticModifiers {
    shift: a.shift || b.shift,
    ctrl: a.ctrl || b.ctrl,
    alt: a.alt || b.alt,
    meta: a.meta || b.meta,
  }
}

/// `button` for the held actions: `left` (default), `middle` or `right`; anything else is refused.
fn held_button(args: &Value) -> Result<MouseButton, String> {
  match args.get("button") {
    None | Some(Value::Null) => Ok(MouseButton::Left),
    Some(value) => match value.as_str() {
      Some("left") => Ok(MouseButton::Left),
      Some("middle") => Ok(MouseButton::Middle),
      Some("right") => Ok(MouseButton::Right),
      _ => Err(format!("unknown button {value}; use left, middle or right")),
    },
  }
}

fn key_arg<'a>(args: &'a Value, action: &str) -> Result<&'a str, String> {
  args
    .get("key")
    .and_then(Value::as_str)
    .ok_or_else(|| format!("{action} needs `key`"))
}

fn needs_session(action: &str) -> String {
  format!(
    "{action} holds input across calls, so it needs an MCP session: initialize over streamable HTTP and send the \
     Mcp-Session-Id it returns. A stateless request has no session whose end would release what it holds."
  )
}

fn not_held(held: &HeldInput, window: &str, what: &str, hint: &str) -> String {
  match held.released_because(window) {
    Some(reason) => format!(
      "{what} is not held in window {window:?}: held input there was released because {}; {hint}",
      reason.describe()
    ),
    None => format!("{what} is not held in window {window:?}; {hint}"),
  }
}

impl Tree {
  /// Whether an MCP call delivered held input to this tree since the last call; the shell then presents it.
  #[cfg_attr(not(feature = "winit"), allow(dead_code))]
  pub(crate) fn take_mcp_input_present(&mut self) -> bool {
    std::mem::take(&mut self.mcp_input_present)
  }
}
