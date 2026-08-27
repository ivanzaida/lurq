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
| `navigator(nav)` | Hand over a router `Navigator` for `lurq_navigate`. |
| `include_devtools(true)` | Expose the DevTools window to agents (hidden by default). |

## Connecting a Client

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
| `lurq_screenshot` | observe | PNG of a window, a region, or one element (by ref). |
| `lurq_read_tree` | observe | Element outline with `ref_N` handles, bounds, text, form values, `#id`/`.class` markers, and `.describe` attributes. |
| `lurq_find` | observe | Substring search over the refs from the last `read_tree` (answered without touching the app). |
| `lurq_find_by_id` | observe | Live lookup of the element with an `.id("...")`, returning a fresh actionable ref. |
| `lurq_find_by_class` | observe | Live lookup of every element with a `.class("...")`, in tree order. |
| `lurq_windows` | observe | List windows: id, name, title, kind, focus, size, scale factor. |
| `lurq_wait` | observe | Wait for N presented frames or render idle, so screenshots aren't mid-animation. |
| `lurq_logs` | observe | Recent log lines, if the app installed the [log layer](#capturing-logs). |
| `lurq_interact` | interact | Synthetic input: `click`, `double_click`, `move`, `drag`, `wheel`, `key`, `type`, `scroll_to`. |
| `lurq_set_value` | interact | Set a TextInput / Checkbox / Slider / Select value directly, no keystroke simulation. |
| `lurq_resize` | interact | Resize a window. |
| `lurq_navigate` | navigate | Push/replace a route, or go back/forward. Needs the `router` feature and a configured `Navigator`. |

### Coordinates and refs

The MCP surface speaks exactly one coordinate space: **pixels of the last screenshot** (physical pixels). `read_tree` bounds, `interact` coordinates, `screenshot` regions, and `resize` dimensions all use it; the server converts internally, so an agent can click what it sees without thinking about scale factors.

`lurq_read_tree` hands out `ref_N` handles for interactive elements, labeled elements (`.describe`, `.id`, `.class`), and form controls:

```text
window: main (800x600 @1.5x)
- Row #demo-toolbar [ref_24] @300,0 500x81
  - Row .demo-button [ref_23] "Open modal" @627,15 146x51
- TextInput [ref_68] value="Ada" @384,350 152x47
```

Refs are the preferred targeting mechanism: a ref carries its window, and ref-based actions re-resolve the element's live bounds at execution time, so a ref stays valid across scrolling. Each `read_tree` of a window replaces that window's refs (numbering is monotonic, so a stale ref errors with a re-read hint instead of silently aliasing a new element). Refs minted by `lurq_find_by_id` / `lurq_find_by_class` are appended and leave existing refs valid.

The typical agent loop:

```text
lurq_read_tree  →  lurq_interact { action: "click", ref: "ref_23" }
                →  lurq_wait { frames: 2 }
                →  lurq_screenshot   (verify)
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

## Making Your App Agent-Friendly

Agents work with what the tree shows them. Three annotation channels, all zero-cost unless a tooling feature is enabled:

```rust
Row::new()
  .id("save-button")                    // lurq_find_by_id, shown as #save-button
  .class("toolbar-action")              // lurq_find_by_class, shown as .toolbar-action
  .describe("role", "commits the form") // free-form key=value shown on the element
```

`id`/`class` are the same attributes used by `Tree::get_element_by_id` and DevTools, so one labeling effort serves tests, DevTools, and agents. `describe` is free-form and appears as `{role=commits the form}` in `read_tree` output; all three are matched by `lurq_find`.

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

`Tree` runs without a shell, so a CI harness can serve MCP against a headless tree — tree reading and synthetic input work as-is; screenshots need a render surface. Drive the drain yourself:

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
| `mcp` alone | Tree reading, input, forms, windows, custom tools. Screenshots error at call time without a render backend. |
| `mcp` + `wgpu` / `dx12` | `lurq_screenshot` returns PNG bytes captured from the GPU. |
| `mcp` + `router` | `lurq_navigate` is registered. |
| `mcp` + `devtools` | Nothing extra today; the DevTools window stays hidden from agents unless `include_devtools(true)`. |
