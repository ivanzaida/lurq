//! The tool registry: built-in tools plus app-registered custom tools,
//! frozen at `enable_mcp` time and shared with the server thread.

use std::{future::Future, pin::Pin, sync::Arc};

use super::{McpToolCtx, Scope};

/// Built-in tools, dispatched by the drain on the event-loop thread (except
/// `Find` and `Logs`, which the server answers without an app roundtrip).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BuiltinTool {
  Screenshot,
  ReadTree,
  Inspect,
  Find,
  FindById,
  FindByClass,
  Windows,
  Menu,
  Wait,
  Interact,
  Act,
  SetValue,
  Resize,
  Logs,
  Navigate,
  FileDialogs,
  FileDialogRespond,
  ProfileStart,
  ProfileRead,
  ProfileEnd,
}

pub(crate) type SyncToolHandler =
  dyn Fn(&mut McpToolCtx<'_>, serde_json::Value) -> Result<serde_json::Value, String> + Send + Sync;

pub(crate) type AsyncToolHandler =
  dyn Fn(serde_json::Value) -> Pin<Box<dyn Future<Output = Result<serde_json::Value, String>> + Send>> + Send + Sync;

pub(crate) enum ToolKind {
  Builtin(BuiltinTool),
  /// Runs on the event-loop thread with `&mut Tree` / `&mut App` — same
  /// powers and same no-blocking constraint as event handlers.
  Sync(Arc<SyncToolHandler>),
  /// Runs on the server's tokio runtime with no tree access, for I/O-bound
  /// work. Distinct at the type level so blocking tools can't freeze the UI
  /// by accident.
  Async(Arc<AsyncToolHandler>),
}

pub(crate) struct RegisteredTool {
  pub(crate) name: String,
  pub(crate) description: String,
  pub(crate) scope: Scope,
  pub(crate) read_only: bool,
  pub(crate) input_schema: serde_json::Value,
  pub(crate) kind: ToolKind,
}

pub(crate) struct ToolRegistry {
  pub(crate) tools: Vec<RegisteredTool>,
}

impl ToolRegistry {
  pub(crate) fn find(&self, name: &str) -> Option<&RegisteredTool> {
    self.tools.iter().find(|tool| tool.name == name)
  }
}

fn schema(value: serde_json::Value) -> serde_json::Value {
  value
}

/// The `window` property shared by window-touching tools.
const WINDOW_PROP: &str =
  "Target window id or name from `lurq_windows` (default \"main\"; \"focused\" targets the focused window)";

pub(crate) fn builtin_tools(router: bool) -> Vec<RegisteredTool> {
  use serde_json::json;

  let mut tools = vec![
    RegisteredTool {
      name: "lurq_menu".into(), description: "Read the declarative application menu, ids, enabled state and accelerators. Activate with lurq_interact action menu_activate and id.".into(),
      scope: Scope::Observe, read_only: true,
      input_schema: schema(json!({"type": "object", "properties": {}})),
      kind: ToolKind::Builtin(BuiltinTool::Menu),
    },
    RegisteredTool {
      name: "lurq_screenshot".into(),
      description: "Capture a PNG screenshot of a window, a region of it, or a single element. \
                    Coordinates and sizes everywhere in this server are pixels of the returned image. \
                    Sensitive text (sensitive=true in the tree) is covered by a grey bar."
        .into(),
      scope: Scope::Observe,
      read_only: true,
      input_schema: schema(json!({
        "type": "object",
        "properties": {
          "window": { "type": "string", "description": WINDOW_PROP },
          "region": {
            "type": "object",
            "description": "Crop rectangle in screenshot pixels of the same window",
            "properties": {
              "x": { "type": "number" }, "y": { "type": "number" },
              "width": { "type": "number" }, "height": { "type": "number" }
            },
            "required": ["x", "y", "width", "height"]
          },
          "ref": { "type": "string", "description": "Crop to this element from lurq_read_tree (overrides region/window)" }
        }
      })),
      kind: ToolKind::Builtin(BuiltinTool::Screenshot),
    },
    RegisteredTool {
      name: "lurq_read_tree".into(),
      description: "Read a window's element tree as an indented outline. Interactive elements get \
                    stable `ref_N` handles for lurq_interact / lurq_set_value / lurq_screenshot. \
                    Bounds are `@x,y WxH` in screenshot pixels. Refs are replaced on each call — \
                    re-read after significant UI changes. Masked inputs expose only their mask and masked=true. \
                    A Canvas shows {items=N}; items the app described (bars, points, labels) are listed \
                    under it as `- role item:item-id [ref_N] \"label\" @bounds {value=\"...\"}`, and their refs \
                    work with every ref-taking tool (hover with move, click, screenshot)."
        .into(),
      scope: Scope::Observe,
      read_only: true,
      input_schema: schema(json!({
        "type": "object",
        "properties": {
          "window": { "type": "string", "description": WINDOW_PROP },
          "filter": { "type": "string", "enum": ["interactive", "all"], "description": "interactive (default): prune branches without interactive elements or text; all: every element" },
          "max_depth": { "type": "integer", "description": "Limit tree depth" },
          "max_chars": { "type": "integer", "description": "Truncate output after this many characters (default 30000)" },
          "max_items": { "type": "integer", "description": "Canvas items listed per canvas; the rest are counted (default 200)" }
        }
      })),
      kind: ToolKind::Builtin(BuiltinTool::ReadTree),
    },
    RegisteredTool {
      name: "lurq_inspect".into(),
      description: "Read the live Lurq UI as structured semantic nodes (role, accessible name, state, bounds and refs), without a screenshot. With query, return matching nodes and their ancestor path. Items an app described on a Canvas (bars, points, labels) are children of the canvas node with canvas_item=true and match query and role like elements. Replaces this window's refs; call lurq_act to invoke or hover a returned ref."
        .into(),
      scope: Scope::Observe,
      read_only: true,
      input_schema: schema(json!({
        "type": "object",
        "properties": {
          "window": { "type": "string", "description": WINDOW_PROP },
          "query": { "type": "string", "description": "Case-insensitive substring of accessible name, id, or class" },
          "role": { "type": "string", "description": "Optional exact semantic role, e.g. button or textbox" },
          "max_depth": { "type": "integer", "minimum": 1, "description": "Maximum tree depth (default 12)" },
          "max_nodes": { "type": "integer", "minimum": 1, "description": "Maximum inspected nodes (default 500); with query or role, also the maximum number of matches returned (then truncated is true)" }
        }
      })),
      kind: ToolKind::Builtin(BuiltinTool::Inspect),
    },
    RegisteredTool {
      name: "lurq_find".into(),
      description: "Search the refs handed out by the last lurq_inspect or lurq_read_tree of each window (no app \
                    roundtrip). Case-insensitive substring match against tag, text, and attributes."
        .into(),
      scope: Scope::Observe,
      read_only: true,
      input_schema: schema(json!({
        "type": "object",
        "properties": {
          "query": { "type": "string", "description": "Substring to look for" }
        },
        "required": ["query"]
      })),
      kind: ToolKind::Builtin(BuiltinTool::Find),
    },
    RegisteredTool {
      name: "lurq_find_by_id".into(),
      description: "Look up the element carrying an HTML-like id (set with the `.id(...)` builder) \
                    in the live tree and return a fresh actionable ref for it. Duplicate ids \
                    resolve to the first match in tree order, like the DOM. When no element has \
                    the id, canvas item ids are searched the same way."
        .into(),
      scope: Scope::Observe,
      read_only: true,
      input_schema: schema(json!({
        "type": "object",
        "properties": {
          "id": { "type": "string", "description": "The element id to look up" },
          "window": { "type": "string", "description": WINDOW_PROP }
        },
        "required": ["id"]
      })),
      kind: ToolKind::Builtin(BuiltinTool::FindById),
    },
    RegisteredTool {
      name: "lurq_find_by_class".into(),
      description: "Look up every element carrying an HTML-like class (set with the `.class(...)`                     builder) in the live tree, in tree order, and return fresh actionable refs."
        .into(),
      scope: Scope::Observe,
      read_only: true,
      input_schema: schema(json!({
        "type": "object",
        "properties": {
          "class": { "type": "string", "description": "The class name to look up" },
          "window": { "type": "string", "description": WINDOW_PROP }
        },
        "required": ["class"]
      })),
      kind: ToolKind::Builtin(BuiltinTool::FindByClass),
    },
    RegisteredTool {
      name: "lurq_windows".into(),
      description: "List the app's windows: id, name, title, kind, focus, whether each is minimized, maximized or full \
                    screen, size (screenshot pixels; a minimized window's client area, a sliver on Windows), and \
                    scale factor."
        .into(),
      scope: Scope::Observe,
      read_only: true,
      input_schema: schema(json!({ "type": "object", "properties": {} })),
      kind: ToolKind::Builtin(BuiltinTool::Windows),
    },
    RegisteredTool {
      name: "lurq_wait".into(),
      description: "Wait until the app has rendered: `frames` waits for that many presented frames, \
                    otherwise waits for render idle (no further redraws pending). Use before \
                    screenshots so captures aren't mid-animation."
        .into(),
      scope: Scope::Observe,
      read_only: true,
      input_schema: schema(json!({
        "type": "object",
        "properties": {
          "window": { "type": "string", "description": WINDOW_PROP },
          "frames": { "type": "integer", "minimum": 1, "description": "Wait for this many presented frames" },
          "timeout_ms": { "type": "integer", "description": "Give up after this long (default 10000)" }
        }
      })),
      kind: ToolKind::Builtin(BuiltinTool::Wait),
    },
    RegisteredTool {
      name: "lurq_interact".into(),
      description: "Drive the app with synthetic input. Actions: click, double_click, move, drag, \
                    wheel, key, type, scroll_to, request_close (vetoable window close), menu_activate (menu id). Target either a `ref` from lurq_inspect/lurq_read_tree or \
                    `x`/`y` in screenshot pixels (ref carries its window; coordinates use `window`)."
        .into(),
      scope: Scope::Interact,
      read_only: false,
      input_schema: schema(json!({
        "type": "object",
        "properties": {
          "action": { "type": "string", "enum": ["click", "double_click", "move", "drag", "wheel", "key", "type", "scroll_to", "request_close", "menu_activate"] },
          "id": { "type": "string", "description": "Menu item id for menu_activate" },
          "ref": { "type": "string", "description": "Element handle from lurq_read_tree" },
          "x": { "type": "number", "description": "Screenshot-pixel X (alternative to ref)" },
          "y": { "type": "number", "description": "Screenshot-pixel Y (alternative to ref)" },
          "to_x": { "type": "number", "description": "Drag end X" },
          "to_y": { "type": "number", "description": "Drag end Y" },
          "to_ref": { "type": "string", "description": "Drag end element" },
          "button": { "type": "string", "enum": ["left", "right", "middle"] },
          "delta_x": { "type": "number", "description": "Wheel horizontal delta" },
          "delta_y": { "type": "number", "description": "Wheel vertical delta (positive scrolls content up)" },
          "key": { "type": "string", "description": "Key name for the key action, e.g. Enter, Tab, Escape, ArrowDown, a" },
          "text": { "type": "string", "description": "Text for the type action" },
          "modifiers": { "type": "array", "items": { "type": "string", "enum": ["shift", "ctrl", "alt", "meta"] } },
          "window": { "type": "string", "description": WINDOW_PROP }
        },
        "required": ["action"]
      })),
      kind: ToolKind::Builtin(BuiltinTool::Interact),
    },
    RegisteredTool {
      name: "lurq_act".into(),
      description: "Invoke (click) or hover (move the pointer onto) a semantic element or canvas item ref from lurq_inspect without supplying coordinates. Allowed actions are listed in the ref's `actions`. Re-checks its live role and name and that it is hittable before dispatch; stale or changed refs fail."
        .into(),
      scope: Scope::Interact,
      read_only: false,
      input_schema: schema(json!({
        "type": "object",
        "properties": {
          "ref": { "type": "string", "description": "Element ref from lurq_inspect" },
          "action": { "type": "string", "enum": ["invoke", "hover"], "description": "invoke clicks the element's center; hover moves the pointer there (opens hover tooltips)" }
        },
        "required": ["ref", "action"]
      })),
      kind: ToolKind::Builtin(BuiltinTool::Act),
    },
    RegisteredTool {
      name: "lurq_set_value".into(),
      description: "Set a form control's value directly (no keystroke simulation): TextInput \
                    (string), Checkbox (boolean), Slider (number), Select (option label or index). \
                    Masked input replies contain the mask and masked=true, never the underlying value."
        .into(),
      scope: Scope::Interact,
      read_only: false,
      input_schema: schema(json!({
        "type": "object",
        "properties": {
          "ref": { "type": "string", "description": "Form element handle from lurq_read_tree" },
          "value": { "description": "New value: string, boolean, or number depending on the control" }
        },
        "required": ["ref", "value"]
      })),
      kind: ToolKind::Builtin(BuiltinTool::SetValue),
    },
    RegisteredTool {
      name: "lurq_resize".into(),
      description: "Resize a window's client area to `width` x `height` screenshot pixels. A minimized, maximized or \
                    full-screen window is restored to a normal window first, so this also restores a minimized \
                    window. Answers once the window has been resized, with the size it took and the modes it left \
                    (`restored_from`); fails with the window's real size when it did not take the requested one."
        .into(),
      scope: Scope::Interact,
      read_only: false,
      input_schema: schema(json!({
        "type": "object",
        "properties": {
          "window": { "type": "string", "description": WINDOW_PROP },
          "width": { "type": "integer", "minimum": 1 },
          "height": { "type": "integer", "minimum": 1 }
        },
        "required": ["width", "height"]
      })),
      kind: ToolKind::Builtin(BuiltinTool::Resize),
    },
    RegisteredTool {
      name: "lurq_logs".into(),
      description: "Recent app log lines captured by the lurq MCP tracing layer (empty unless the \
                    app installed `lurq::mcp::log_layer()`)."
        .into(),
      scope: Scope::Observe,
      read_only: true,
      input_schema: schema(json!({
        "type": "object",
        "properties": {
          "lines": { "type": "integer", "description": "Max lines, newest last (default 200)" },
          "filter": { "type": "string", "description": "Only lines containing this substring" }
        }
      })),
      kind: ToolKind::Builtin(BuiltinTool::Logs),
    },
  ];

  if router {
    tools.push(RegisteredTool {
      name: "lurq_navigate".into(),
      description: "Navigate the app router: push a route path, or go back/forward. Always returns \
                    the current path. Requires the app to hand its Navigator to the MCP config."
        .into(),
      scope: Scope::Navigate,
      read_only: false,
      input_schema: schema(serde_json::json!({
        "type": "object",
        "properties": {
          "path": { "type": "string", "description": "Route path to push, e.g. /settings/profile" },
          "replace": { "type": "boolean", "description": "Replace the current history entry instead of pushing" },
          "back": { "type": "boolean", "description": "Go back one history entry" },
          "forward": { "type": "boolean", "description": "Go forward one history entry" }
        }
      })),
      kind: ToolKind::Builtin(BuiltinTool::Navigate),
    });
  }

  tools
}
