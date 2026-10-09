---
title: MCP Server
description: Embedding an MCP server so AI agents can drive and inspect a running lurq app — screenshots, tree reading, clicking, typing, navigation, and custom tools.
---

# MCP Server

lurq can embed a [Model Context Protocol](https://modelcontextprotocol.io) server so an AI agent drives your running app the way browser tools drive a web page: capture screenshots, read the element tree, click and type, fill forms, and navigate — plus any app-specific tools you register.

Nothing is exposed by default. Three layers gate the server:

1. The `mcp` Cargo feature (off by default) compiles the server in.
2. `Tree::enable_mcp` starts it — nothing listens without the call.
3. The returned `McpHandle` toggles availability and permissions at runtime.

## Enable It

Build with `lurq/mcp` and call `enable_mcp` on the root tree before the event loop starts:

```rust
use lurq::mcp::{McpConfig, Scope};

let mcp = tree.enable_mcp(
  McpConfig::new()
    .app_name("my-app")
    .scopes([Scope::Observe, Scope::Interact, Scope::Navigate]),
);

println!("MCP on http://127.0.0.1:{}/mcp", mcp.port());
```

The server runs on one background thread and serves streamable HTTP on `127.0.0.1` with an ephemeral port by default (`McpConfig::port` pins one). The winit shell drains tool calls automatically each loop turn; you do not write any per-tool plumbing.

`McpConfig` options:

| Option | Effect |
| --- | --- |
| `scopes([...])` / `scope(...)` | Which tool groups are exposed. Defaults to `Observe` + `Interact`. |
| `deny_tool("lurq_resize")` | Hide a single tool on top of scope filtering. |
| `port(4839)` | Fixed port instead of an ephemeral one. |
| `app_name("my-app")` | Name in the discovery file and server info; defaults to the executable name. |
| `instructions("...")` | Extra guidance appended to the instructions the agent receives. |
| `tool(McpTool::new(...))` | Register a [custom tool](#custom-tools). |
| `file_dialogs(broker)` | Opt adapted normal picker handlers into [request-scoped file selection](../mcp-file-dialogs/), with `Scope::custom("file_dialogs")`. |
| `navigator(nav)` | Hand over a router `Navigator` for `lurq_navigate`. |
| `include_devtools(true)` | Expose the DevTools window to agents (hidden by default). |

## Connecting a Client

Opted-in file handlers expose `lurq_file_dialogs` and `lurq_file_dialog_respond`.
They list and complete individual pending selections; they do not intercept `rfd`
or automate OS dialogs. See [MCP file selection](../mcp-file-dialogs/) for the
native fallback adapter and application overwrite/validation obligations.

Every MCP-enabled app writes a discovery file while it runs — `%LOCALAPPDATA%\lurq\mcp\<pid>.json` on Windows, XDG dirs on Linux, `~/Library/Application Support/lurq/mcp/` on macOS — containing the port, the app name, and the bearer token:

```json
{
  "version": 1,
  "pid": 27712,
  "app": "my-app",
  "transport": "streamable-http",
  "port": 64271,
  "url": "http://127.0.0.1:64271/mcp",
  "token": "81965f15ba7b…"
}
```

The file is removed on graceful shutdown. Connect Claude Code with:

```sh
claude mcp add --transport http my-app http://127.0.0.1:64271/mcp --header "Authorization: Bearer <token>"
```

### Security

A localhost HTTP server is reachable by any local process, and an input-injection endpoint must not be open. Auth is therefore mandatory, not optional:

- Every request must carry the bearer token (compared in constant time). The token is random per run and lives only in the discovery file, which is user-readable only on Unix.
- The server binds `127.0.0.1` and validates the `Host` header against loopback names, closing the DNS-rebinding hole.
- Tools outside granted scopes are **not listed** to the client at all — calling one anyway reports "unknown tool" rather than leaking its existence.

## Built-in Tools

All built-in tools use the reserved `lurq_` prefix; custom tools may not.

| Tool | Scope | Does |
| --- | --- | --- |
| `lurq_screenshot` | observe | PNG of a window, a region, or one element (by ref); sensitive text is covered by a grey bar. |
| `lurq_inspect` | observe | Structured semantic tree, or matches for `query`/`role`, with roles, names, state, bounds, and `ref_N` handles. |
| `lurq_read_tree` | observe | Element outline with `ref_N` handles, bounds, text, form values, `#id`/`.class` markers, and `.describe` attributes. |
| `lurq_find` | observe | Substring search over the refs from the last `read_tree` (answered without touching the app). |
| `lurq_find_by_id` | observe | Live lookup of the element with an `.id("...")`, or else the [canvas item](#canvas-content) with that id, returning a fresh actionable ref. |
| `lurq_find_by_class` | observe | Live lookup of every element with a `.class("...")`, in tree order. |
| `lurq_windows` | observe | List windows: id, name, title, kind, focus, `minimized`/`maximized`/`full_screen`, size, scale factor, and the buttons and keys clients [hold](#holding-buttons-and-keys) there. |
| `lurq_menu` | observe | Inspect the native-menu model, command IDs, enabled state, and platform support. |
| `lurq_wait` | observe | Wait for N presented frames or render idle, so screenshots aren't mid-animation. |
| `lurq_logs` | observe | Recent log lines, if the app installed the [log layer](#capturing-logs). |
| `lurq_interact` | interact | Synthetic input: `click`, `double_click`, `move`, `drag`, `wheel`, `key`, `type`, `scroll_to`; input [held across calls](#holding-buttons-and-keys): `press`/`release`, `key_down`/`key_up`; also `request_close` and `menu_activate`. |
| `lurq_act` | interact | `invoke` (click) or `hover` a ref from `lurq_inspect`, without agent-supplied coordinates. |
| `lurq_set_value` | interact | Set a TextInput / Checkbox / Slider / Select value directly, no keystroke simulation. |
| `lurq_resize` | interact | Resize a window, restoring it first if it is minimized, maximized or full screen. |
| `lurq_navigate` | navigate | Push/replace a route, or go back/forward. Needs the `router` feature and a configured `Navigator`. |

### Coordinates and refs

Masked text inputs expose the displayed mask and `masked=true` in tree reads, lookup/find results, and set-value replies. [Sensitive text](../styling-events/#sensitive-text) (`Text::sensitive()`) shows as `•••` with `sensitive=true` in every tool, including the names it gives a button, and `lurq_screenshot` covers it with a grey bar (window, region and element captures alike), while the window keeps showing it. Direct application access and form submission still use the underlying value. App-authored annotations, custom tool responses, and logs remain the application's responsibility. See [Window lifecycle and native menus](../window-lifecycle-menus/#mcp-and-verification) for close and menu action semantics.

The MCP surface speaks exactly one coordinate space: **pixels of the last screenshot** (physical pixels). `read_tree` bounds, `interact` coordinates, `screenshot` regions, and `resize` dimensions all use it; the server converts internally, so an agent can click what it sees without thinking about scale factors.

For a task such as “click Save”, use the semantic tools first:

```text
lurq_inspect {"query":"Save","role":"button"}
→ {"window":"main","matches":[{"ref":"ref_17","role":"button","name":"Save",
   "id":"save-button","state":{},"actions":["invoke"],"bounds":[20,20,100,40],"path":[]}],
   "truncated":false}

lurq_act {"ref":"ref_17","action":"invoke"}
→ {"dispatched":true,"action":"invoke","ref":"ref_17","window":"main"}
```

Without `query` or `role`, `lurq_inspect` returns a nested `tree`. It derives button names from child text; `a11y_name` and `a11y_role` annotations set with `.describe(...)` override the derived values. Pending reactive changes are refreshed before inspection. `bounds` is `null` before the first layout pass or while a refreshed tree awaits layout. `lurq_act` requires live layout, checks that the ref's role and name have not changed and that its center is hittable, then dispatches a normal synthetic click (`invoke`) or pointer move (`hover`, which opens hover tooltips). Each node lists the actions it accepts in `actions`: `invoke` for clickable elements and form controls, `hover` for elements with mouse-move or mouse-enter handlers. `dispatched` means the input was sent; inspect again to verify the resulting app state. When several controls match, the tool returns all of them with their ancestor paths so the agent can choose deliberately. This server inspects only the Lurq app that embeds it, not other desktop windows.

`lurq_read_tree` remains useful for a compact text outline. It hands out `ref_N` handles for interactive elements, labeled elements (`.describe`, `.id`, `.class`), and form controls:

```text
window: main (800x600 @1.5x)
- Row #demo-toolbar [ref_24] @300,0 500x81
  - Row .demo-button [ref_23] "Open modal" @627,15 146x51
- TextInput [ref_68] value="Ada" @384,350 152x47
```

Refs are the preferred targeting mechanism: a ref carries its window, and ref-based actions re-resolve the element's live bounds at execution time, so a ref stays valid across scrolling. Each `inspect` or `read_tree` of a window replaces that window's refs (numbering is monotonic, so a stale ref errors with a re-read hint instead of silently aliasing a new element). Refs minted by `lurq_find_by_id` / `lurq_find_by_class` are appended and leave existing refs valid.

The typical agent loop:

```text
lurq_inspect { query: "Open modal", role: "button" }
              →  lurq_act { action: "invoke", ref: "ref_23" }
              →  lurq_inspect   (verify)
```

### Holding buttons and keys

`click`, `drag` and `key` deliver a whole gesture inside one call, between two frames. A gesture whose samples arrive one frame apart (dragging a node or a resize handle, a marquee, a slider or scrollbar thumb, a middle-button or space-drag pan) holds its input across calls instead:

| Action | Arguments | Does |
| --- | --- | --- |
| `press` | `ref` or `x`/`y`; `button`: `left` (default), `middle` or `right` | Moves the pointer there, then presses the button, which stays down. |
| `move` | `ref` or `x`/`y` | In a window that holds a button or key, a held move: the tree treats it as a real move with the button down, so drags, sliders, scrollbars and text selection follow it and keep the pointer outside their bounds. |
| `release` | `button`; optionally `ref` or `x`/`y` | Releases the button where the pointer is, moving to the target first if one is given. A left press released in place is a click. |
| `key_down` / `key_up` | `key`, `modifiers` | Holds and releases a key, with the key names of `key` (`" "` is the space bar). |

Held modifier keys apply to every action in their window, as real keys that are down do, together with the action's own `modifiers`. A held `Shift`, `Control` or `Alt` sets `shift`, `ctrl` or `alt` on every later pointer event (moves, presses, releases, clicks) and key event (`key`, `type`, `key_down`, `key_up`). A held `Meta` or `Super` sets `meta` on key events only: pointer events have no `meta`. Scroll events carry no modifiers, so `wheel` passes them only to its pointer move.

A call that delivers held input makes the shell present right after the call, before any redraw is requested, as a real mouse-move stream presents from inside its events: one `move` per call is exactly one frame per move. Between calls a held button costs nothing: a held drag changes only on input, so the event loop waits (for a real drag as well).

```text
lurq_interact {"action":"press","x":420,"y":310}
→ {"ok":true,"action":"press","window":"main","button":"left","x":420.0,"y":310.0,
   "held":{"buttons":["left"],"keys":[]}}
lurq_interact {"action":"move","x":436,"y":318}
→ {"ok":true,"action":"move","window":"main","x":436.0,"y":318.0,"held":{"buttons":["left"],"keys":[]}}
lurq_interact {"action":"release"}
→ {"ok":true,"action":"release","window":"main","button":"left","x":436.0,"y":318.0,
   "held":{"buttons":[],"keys":[]}}
```

Holds belong to a window (its `lurq_windows` id) and to the MCP session that pressed them, so holding needs a session-based connection: initialize with protocol version 2025-06-18 (or another version before 2026-07-28) and send the `Mcp-Session-Id` the server returns. Protocol 2026-07-28 serves requests without sessions, and `press` and `key_down` refuse them. Any client may release. Each held action's result and `lurq_windows` report what a window holds (`held`). A `move` in a window that holds nothing, and every other action, behave as before. Sequences that do not fit are refused, not repaired: releasing a button or key that is not held, pressing a held button again, `key_down` of a held key, and `click`, `double_click`, `drag` or `key` of a held button or key.

Held input never stays down after its client can no longer release it. lurq releases it when:

- the session that holds it ends: the client deletes it (`DELETE /mcp`), it expires after five minutes without a request, or the server stops;
- the window loses focus or closes (a press made while the window was unfocused stays until the window next loses focus);
- `lurq_interact` stops being available: `McpHandle::set_enabled(false)`, the Interact scope removed, or the tool denied.

A button released for its client ends the way the OS ends a press it takes over (`Tree::mouse_press_taken_by_os`): an up event, no click, and a drag ends without a drop. A key gets its up event. A later `release` or `key_up` says why it found nothing held:

```text
the left button is not held in window "main": held input there was released because the window lost focus; press it first
```

## Runtime Control

`enable_mcp` returns a clonable `McpHandle` — for debug menus, env-var gating, or support-session unlocks:

```rust
mcp.set_enabled(false);              // hide and reject everything, listener stays up
mcp.add_scope(Scope::Interact);      // unlock interaction at runtime
mcp.remove_scope(&Scope::Interact);
mcp.deny_tool("lurq_resize");        // per-tool trim on top of scopes
mcp.set_navigator(router.navigator());
```

Scope checks run again at call time, so revoking a scope takes effect immediately even for a client that listed tools earlier.

A call the event loop has not started when its reply deadline passes (15 s, 30 s for `lurq_screenshot`, the requested timeout plus 5 s for `lurq_wait`) is answered with a time-out error and dropped: it never runs later.

### Controls only a person may use

MCP input reaches the same entry points as OS input, so scopes and tool denies are the way to keep an agent away from the window as a whole. To keep it away from one control, such as a switch that widens an agent's own permissions, check where the input came from in the control's handler. While lurq delivers the input of an MCP call (`lurq_interact`, `lurq_act`, `lurq_set_value`, and every other tool that reaches the event loop) or a `SyntheticInput`, `lurq::app::events::input_source()` returns `InputSource::Synthetic` and `is_synthetic_input()` is `true`:

```rust
use lurq::app::events::{MouseEvent, is_synthetic_input};

Button::new("Allow file writes").on_click(move |_: MouseEvent| {
  if is_synthetic_input() {
    return; // an agent may not grant itself permissions
  }
  allow_writes.set(true);
})
```

The source is set only while lurq delivers the input: a handler and the callbacks it reaches see it, work deferred past the delivery (a spawned task, a timer, a later render) does not. `InputSource::Os` is not proof that a person acted, since another program can post OS input. An automation layer that drives a `Tree` through its input methods itself can mark that input with `with_synthetic_input(|| ...)`.

## Multiple Windows

Every window-touching tool takes a `window` argument defaulting to `"main"`. Secondary windows are addressed by a stable id (`w1`, `w2`, … — never reused, so a closed window errors as gone instead of resolving to a different one) or by an app-assigned name:

```rust
use lurq::app::WindowOptions;

opener.open_with(
  WindowOptions::new("Settings", 700, 500).window_name("settings"),
  |app, tree| tree.mount_root::<SettingsWindow>(app, props),
);
```

`window: "focused"` is accepted as a call-time alias. Ref-based calls never need `window` — the ref knows where it lives. The DevTools window is excluded from listings, tree reads, and capture unless `include_devtools(true)`; it is tooling chrome, and its tree duplicates app state in confusing form.

### Window state and resizing

`lurq_windows` reports each window's `minimized`, `maximized` and `full_screen` state. `width` and `height` are the client area, so a minimized window reports a sliver on Windows (237x39 on one Windows 11 machine), not the size it comes back at.

`lurq_resize` takes the client size in screenshot pixels. A minimized, maximized or full-screen window cannot take an exact size (Windows ignores the size of a minimized window, and the other modes keep their own frame), so the tool first restores it to a normal window: resizing is also how an agent restores a minimized window. (An app's `WindowHandle::resize` restores only a minimized window, leaving a maximized or full-screen one in its mode.) The tool answers once the shell has applied the resize, with the size the window took and the modes it left:

```text
lurq_resize {"width":1440,"height":1020}
→ {"ok":true,"window":"main","width":1440,"height":1020,"restored_from":["minimized"],"still":[]}
```

When the window did not take the requested size, the call fails with its real size and the reason: it is still in a mode the platform did not leave (or has not finished leaving: macOS animates leaving full screen, so check `lurq_windows` and retry), or the platform limited the size (the window's minimum or maximum size, or the screen). A window that has no native window yet, or a headless tree, fails at once.

## Making Your App Agent-Friendly

Agents work with what the tree shows them. `id` and `class` are ordinary runtime attributes available without tooling features. `describe` stores tooling annotations only when `mcp` or `devtools` is enabled:

```rust
Row::new()
  .id("save-button")                    // lurq_find_by_id, shown as #save-button
  .class("toolbar-action")              // lurq_find_by_class, shown as .toolbar-action
  .describe("role", "commits the form") // free-form key=value shown on the element
```

`id`/`class` are the same attributes used by `Tree::get_element_by_id` and DevTools, so one labeling effort serves tests, DevTools, and agents. `describe` is free-form and appears as `{role="commits the form"}` in `read_tree` output; all three are matched by `lurq_find`. App-provided ids, classes, roles, attribute names and values are printed as-is when they are plain words (letters, digits, `-_.:/`) and quoted with escapes otherwise, so app text can never start a new line or forge an element or ref. Attribute values, item labels and element text are cut at 80 characters and names in lookup lines at 60, ending in `…`.

### Canvas content

Canvas pixels are opaque to the tree. Describe what you drew with [canvas items](../canvas/#describing-what-you-drew) (`CanvasHandle::set_items`, `canvas` feature) and agents get one child per item under the canvas, with a ref, its role, label, value and bounds in screenshot pixels. Item ids are written `item:<id>`, never `#<id>`, so a pattern looking for an element id cannot match an item. Anchor patterns for items on the leading space, ` item:<id> [`: an element whose id is `item:delete` renders as `#item:delete [ref_N]`, which a bare `item:delete [` would also match. `lurq_find_by_id` still finds items by their id when no element has it. A canvas with items or pointer handlers shows `{items=N}`, so an agent can tell a described canvas from an opaque one; purely decorative canvases stay out of the outline. `lurq_read_tree` lists up to `max_items` items per canvas (default 200) and counts the rest in a `… +N more items` line; `lurq_inspect` with `role` or `query` checks every item without counting items against `max_nodes`, returns up to `max_nodes` matches, and mints refs only for the matches it returns. This excerpt is `lurq_read_tree` of `examples/canvas_chart.rs` at 1.5x:

```text
window: main (630x450 @1.5x)
- Chart @0,0 630x450
  - Canvas #runs-chart [ref_11] @24,24 540x300 {items=10}
    - bar item:mon [ref_1] "Mon" @54,135 75x144 {value="12 runs"}
    - label item:mon-label [ref_2] "Mon" @54,288 75x27
    - bar item:tue [ref_3] "Tue" @159,63 75x216 {value="18 runs"}
    …
  - Text #tooltip [ref_12] "Hover a bar" @24,336 124x29
```

Item ids, roles, labels and values reach every client with the observe scope as the app registered them; unlike masked text inputs, nothing is redacted, so keep secrets out of them.

In `lurq_inspect`, items are children of the canvas node (role `canvas`) with `canvas_item: true` and match `query` (label or id) and `role` like elements. In the unfiltered tree they count toward `max_nodes`. A search with `query` or `role` scans every item without counting it, returns at most `max_nodes` matches (default 500; elements and items together) and then sets `truncated: true`, and mints refs only for the matches it returns:

```text
lurq_inspect {"role":"bar","query":"thu"}
→ {"matches":[{"ref":"ref_9","role":"bar","name":"Thu","id":"thu","canvas_item":true,
   "state":{"value":"15 runs"},"actions":["hover"],"bounds":[369,99,75,180],
   "path":["canvas \"runs-chart\""]}],"truncated":false,"window":"main"}

lurq_act {"ref":"ref_9","action":"hover"}
→ {"dispatched":true,"action":"hover","ref":"ref_9","window":"main"}
```

Item bounds are what the pointer can reach: the item clipped to its canvas and to what scroll viewports, clipping ancestors and the window leave visible, with every ancestor placed through its transform as hit testing does. Under a rotated or skewed ancestor the clip is the axis-aligned box around the transformed clip rect, so bounds can include a sliver the pointer cannot reach; input stays guarded by the hit test at the item's center. An item with nothing visible is listed as `(not visible)` (`bounds: null`, `state.hidden: true` in `lurq_inspect`); input, screenshots and `lurq_act` refuse it, and `scroll_to` brings the part inside its canvas into view. Input by item ref also requires the item's center to hit its canvas, so it never lands on a control beside or above it.

Item refs work wherever element refs do. Actions re-resolve the item's live bounds through the canvas's current placement, so a ref follows its item across redraws, scrolling and ancestor transforms, and errors once the app stops registering that id. `lurq_interact` `move`/`click`/`double_click`/`drag` target the item's center, `scroll_to` scrolls the item itself into view, and `lurq_screenshot` with an item ref crops to it. Pointer input goes through the app's normal handling: the canvas's own handlers receive it and hit-test with `CanvasHandle::item_at`, so the app's hover tooltips open as they do for a mouse. The canvas's handlers decide the item's `actions`; `lurq_act` also checks that the item's role and label are unchanged and that its center is on the canvas. In `lurq_find` and lookup results an item appears as `CanvasItem role=bar item:tue name="Tue" {value="18 runs"}`.

## Custom Tools

Apps extend the server with their own tools:

```rust
use lurq::mcp::{McpConfig, McpTool, Scope};

McpConfig::new()
  .scope(Scope::custom("project"))
  .tool(
    McpTool::new("export_project")
      .description("Export the current project to disk")
      .scope(Scope::custom("project"))
      .input_schema(serde_json::json!({
        "type": "object",
        "properties": { "path": { "type": "string" } },
        "required": ["path"]
      }))
      .handler(|ctx, args| {
        let path = args["path"].as_str().ok_or("path required")?;
        // ctx.tree: &mut Tree, ctx.app: &mut App
        Ok(serde_json::json!({ "ok": true, "path": path }))
      }),
  )
```

Two handler flavors, distinct at the type level so a blocking tool can't freeze the UI by accident:

| | Runs on | Has | For |
| --- | --- | --- | --- |
| `.handler(...)` | event-loop thread | `&mut Tree` + `&mut App` via `McpToolCtx` | UI state reads/writes; same no-blocking rule as event handlers |
| `.async_handler(...)` | server's tokio runtime | arguments only, **no** tree access | I/O-bound work (network, disk) |

Custom tool names must not start with `lurq_` and must be unique; violations panic at `enable_mcp` so they surface in development, not in an agent session. Custom scopes (`Scope::custom("project")`) participate in listing, denial, and runtime toggling like the built-in ones.

## Hosting the Tools in Your Own Server

An app that runs an MCP server of its own — with its own port, token, discovery and settings — serves lurq's tools from it instead of starting lurq's. `McpConfig::hosted()` starts no listener and writes no discovery file (`McpHandle::port()` is `0`); `McpHandle::service()` returns an `McpService`, lurq's rmcp `ServerHandler` for one client session.

```rust
use lurq::mcp::{McpConfig, Scope, rmcp};

let mcp = tree.enable_mcp(McpConfig::new().scopes([Scope::Observe]).hosted());

// Mount it as the session handler of rmcp's streamable-HTTP service…
let service = rmcp::transport::streamable_http_server::StreamableHttpService::new(
  move || Ok(mcp.service()),
  Default::default(),
  Default::default(),
);
// …or keep one `McpService` per session inside your own handler and delegate
// `list_tools`, `call_tool` and `get_info` to it for `lurq_*` tools.
```

`lurq::mcp::rmcp` re-exports the rmcp lurq is built with, so the host's handler and lurq's agree on its types. Nothing else changes: scopes, `deny_tool` and `set_enabled` on the `McpHandle` decide what the service lists and accepts, calls that need the event loop are executed by `Tree::drain_mcp_requests` (the winit shell runs it every turn), and input a client holds is released when the service of its session is dropped. A call belongs to a session when its request carries `Mcp-Session-Id` in the `hyper::http::request::Parts` that rmcp's streamable-HTTP service puts in the request context. Authenticate requests in the host: the service does not check a token.

## Profiling sessions

`lurq_profile_start` starts an independent bounded CPU capture; `lurq_profile_end` finalizes only the ID it receives. Overlapping sessions keep collecting when another ends. `lurq_profile_read` reads a session snapshot, or feature/build availability when called without an ID. All three use Observe scope and the existing bearer authentication, and run on the server thread without waking/waiting for the UI. Reports distinguish completed samples from unfinished current phases, make truncation/age/feature availability explicit, and report GPU timestamps as unavailable. See [Profiling sessions](../profiling/) for bounds, nested timing semantics, lifecycle and the shared in-process API. No DevTools window is required.

## Capturing Logs

`lurq_logs` serves a ring buffer that your tracing subscriber feeds. Opt in by adding the layer:

```rust
use tracing_subscriber::layer::SubscriberExt as _;

let subscriber = tracing_subscriber::fmt()
  .with_env_filter(filter)
  .finish()
  .with(lurq::mcp::log_layer());
tracing::subscriber::set_global_default(subscriber).ok();
```

Without the layer, `lurq_logs` tells the agent that capture is not installed.

## Navigation

`lurq_navigate` needs the app's `Navigator`. Hand it over wherever your router is created — `RouterHandle::navigator()` builds one:

```rust
fn create(ctx: &mut Ctx) -> Self {
  let router = ctx.router(routes());
  if let Some(mcp) = MCP_HANDLE.get() {
    mcp.set_navigator(router.navigator());
  }
  Self { router }
}
```

(Or pass it up front with `McpConfig::navigator` if the router exists at startup.) The tool always returns the current path plus `can_back`/`can_forward`.

## Headless Use

`Tree` runs without a shell, so a CI harness can serve MCP against a headless tree — tree reading and synthetic input work as-is; screenshots need a render surface, and `lurq_resize` fails (size a headless tree with `Tree::resize`). Drive the drain yourself:

```rust
let mcp = tree.enable_mcp(McpConfig::new());
loop {
  tree.drain_mcp_requests(&mut app);
  // ... advance your harness, run passes, etc.
}
tree.shutdown_mcp(); // stops the listener, removes the discovery file
```

## Feature Interactions

| Combination | Effect |
| --- | --- |
| `mcp` alone | Tree reading, input, windows, menu-model inspection, and custom tools. Form components additionally require `form`; screenshots error without a render backend. |
| `mcp` + `wgpu` / `dx12` | `lurq_screenshot` returns PNG bytes captured from the GPU. |
| `mcp` + `router` | `lurq_navigate` is registered. |
| `mcp` + `canvas` | Canvas items appear in tree reads and lookups and accept refs. |
| `mcp` + `devtools` | Nothing extra today; the DevTools window stays hidden from agents unless `include_devtools(true)`. |
