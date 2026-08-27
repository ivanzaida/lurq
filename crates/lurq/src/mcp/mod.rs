//! Embeddable MCP server: lets an AI agent drive and inspect a running lurq
//! app the way browser MCP tools drive a web page — screenshot, read the
//! element tree, click/type, navigate — plus lurq-specific state
//! introspection and app-defined custom tools.
//!
//! Nothing is exposed by default. Three enablement layers:
//! 1. the `mcp` Cargo feature (off by default) compiles this module in;
//! 2. [`Tree::enable_mcp`] starts the server — nothing listens without it;
//! 3. [`McpHandle`] toggles availability and scopes at runtime.
//!
//! ```ignore
//! let mcp = tree.enable_mcp(
//!   McpConfig::new().scopes([Scope::Observe, Scope::Interact]),
//! );
//! ```
//!
//! The server speaks streamable HTTP on `127.0.0.1` with a mandatory bearer
//! token, and announces itself in a discovery file
//! (`%LOCALAPPDATA%\lurq\mcp\<pid>.json` on Windows; XDG dirs on Linux).
//! Connect with `claude mcp add --transport http http://127.0.0.1:<port>/mcp`
//! and the token from the discovery file.

use std::{
  borrow::Cow,
  collections::HashSet,
  future::Future,
  pin::Pin,
  sync::{Arc, mpsc as std_mpsc},
};

mod discovery;
mod logs;
mod registry;
mod server;
mod shared;
mod tools;

pub use logs::{McpLogLayer, log_layer};
use registry::{ToolKind, ToolRegistry};
/// rmcp's schemars, re-exported so typed custom-tool inputs derive against
/// the same version: `#[derive(schemars::JsonSchema)]` with
/// `#[schemars(crate = "lurq::mcp::schemars")]`.
pub use rmcp::schemars;
use shared::McpShared;
pub use shared::{McpToolOutput, McpToolResult};

use crate::app::{App, Tree};

/// Coarse permission groups. Tools outside granted scopes are not listed to
/// the client at all (not merely rejected). Runtime-mutable via [`McpHandle`].
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Scope {
  /// Screenshots, tree reading, window listing, waiting, logs.
  Observe,
  /// Synthetic input, form set_value, window resizing.
  Interact,
  /// Router navigation.
  Navigate,
  /// Reactive state introspection (signals/memos/contexts).
  State,
  /// App-defined scope for custom tools.
  Custom(Cow<'static, str>),
}

impl Scope {
  pub fn custom(name: impl Into<Cow<'static, str>>) -> Self {
    Self::Custom(name.into())
  }
}

/// Execution context handed to synchronous custom tools: full tree and app
/// access on the event-loop thread — the same powers and the same
/// no-blocking constraint as event handlers.
pub struct McpToolCtx<'a> {
  pub tree: &'a mut Tree,
  pub app: &'a mut App,
}

type CustomSyncHandler = Arc<registry::SyncToolHandler>;
type CustomAsyncHandler = Arc<registry::AsyncToolHandler>;

enum CustomToolKind {
  Sync(CustomSyncHandler),
  Async(CustomAsyncHandler),
}

/// A custom tool registered by the embedding app.
///
/// ```ignore
/// McpTool::new("export_project")
///   .description("Export the current project to disk")
///   .scope(Scope::custom("project"))
///   .input_schema(serde_json::json!({
///     "type": "object",
///     "properties": { "path": { "type": "string" } },
///     "required": ["path"]
///   }))
///   .handler(|ctx, args| {
///     let path = args["path"].as_str().ok_or("path required")?;
///     // ctx.tree / ctx.app ...
///     Ok(serde_json::json!({ "ok": true }))
///   })
/// ```
pub struct McpTool {
  name: String,
  description: String,
  scope: Scope,
  read_only: bool,
  input_schema: serde_json::Value,
  kind: Option<CustomToolKind>,
}

impl McpTool {
  pub fn new(name: impl Into<String>) -> Self {
    Self {
      name: name.into(),
      description: String::new(),
      scope: Scope::custom("app"),
      read_only: false,
      input_schema: serde_json::json!({ "type": "object", "properties": {} }),
      kind: None,
    }
  }

  pub fn description(mut self, description: impl Into<String>) -> Self {
    self.description = description.into();
    self
  }

  pub fn scope(mut self, scope: impl Into<Scope>) -> Self {
    self.scope = scope.into();
    self
  }

  /// Hint that the tool does not modify the app.
  pub fn read_only(mut self) -> Self {
    self.read_only = true;
    self
  }

  /// JSON Schema for the tool's arguments. For typed inputs, derive
  /// `schemars::JsonSchema` (see [`schemars`]) and pass
  /// `serde_json::to_value(schemars::schema_for!(Input)).unwrap()`.
  pub fn input_schema(mut self, schema: serde_json::Value) -> Self {
    self.input_schema = schema;
    self
  }

  /// Synchronous handler, run on the event-loop thread with
  /// [`McpToolCtx`] access. Must not block (network, sleeps, dialogs) — a
  /// blocked handler freezes the UI.
  pub fn handler<F>(mut self, handler: F) -> Self
  where
    F: Fn(&mut McpToolCtx<'_>, serde_json::Value) -> Result<serde_json::Value, String> + Send + Sync + 'static,
  {
    self.kind = Some(CustomToolKind::Sync(Arc::new(handler)));
    self
  }

  /// Asynchronous handler, run on the MCP server's tokio runtime with **no**
  /// tree access — for I/O-bound work. Distinct from [`Self::handler`] at
  /// the type level so blocking tools can't freeze the UI by accident.
  pub fn async_handler<F, Fut>(mut self, handler: F) -> Self
  where
    F: Fn(serde_json::Value) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<serde_json::Value, String>> + Send + 'static,
  {
    self.kind = Some(CustomToolKind::Async(Arc::new(move |args| {
      Box::pin(handler(args)) as Pin<Box<dyn Future<Output = Result<serde_json::Value, String>> + Send>>
    })));
    self
  }
}

impl From<&'static str> for Scope {
  fn from(name: &'static str) -> Self {
    Scope::custom(name)
  }
}

/// Configuration for [`Tree::enable_mcp`].
pub struct McpConfig {
  scopes: HashSet<Scope>,
  denied_tools: HashSet<String>,
  port: Option<u16>,
  app_name: Option<String>,
  include_devtools: bool,
  instructions: Option<String>,
  tools: Vec<McpTool>,
  #[cfg(feature = "router")]
  navigator: Option<crate::router::Navigator>,
}

impl Default for McpConfig {
  fn default() -> Self {
    Self::new()
  }
}

impl McpConfig {
  /// Defaults: `Observe` + `Interact` scopes, ephemeral port, devtools
  /// window hidden.
  pub fn new() -> Self {
    Self {
      scopes: HashSet::from([Scope::Observe, Scope::Interact]),
      denied_tools: HashSet::new(),
      port: None,
      app_name: None,
      include_devtools: false,
      instructions: None,
      tools: Vec::new(),
      #[cfg(feature = "router")]
      navigator: None,
    }
  }

  /// Replace the granted scope set.
  pub fn scopes(mut self, scopes: impl IntoIterator<Item = Scope>) -> Self {
    self.scopes = scopes.into_iter().collect();
    self
  }

  /// Grant one more scope.
  pub fn scope(mut self, scope: Scope) -> Self {
    self.scopes.insert(scope);
    self
  }

  /// Per-tool deny list on top of scopes, for fine trimming.
  pub fn deny_tool(mut self, name: impl Into<String>) -> Self {
    self.denied_tools.insert(name.into());
    self
  }

  /// Fixed port instead of an ephemeral one.
  pub fn port(mut self, port: u16) -> Self {
    self.port = Some(port);
    self
  }

  /// Name announced in the discovery file and server info; defaults to the
  /// executable name.
  pub fn app_name(mut self, name: impl Into<String>) -> Self {
    self.app_name = Some(name.into());
    self
  }

  /// Expose the devtools window to MCP (it's tooling chrome and hidden by
  /// default; the audience is lurq's own development).
  pub fn include_devtools(mut self, include: bool) -> Self {
    self.include_devtools = include;
    self
  }

  /// Extra text appended to the server's instructions for the agent.
  pub fn instructions(mut self, instructions: impl Into<String>) -> Self {
    self.instructions = Some(instructions.into());
    self
  }

  /// Register a custom tool. Built-in names are reserved under the `lurq_`
  /// prefix; duplicates and reserved names panic at [`Tree::enable_mcp`].
  pub fn tool(mut self, tool: McpTool) -> Self {
    self.tools.push(tool);
    self
  }

  /// Navigator for the built-in `lurq_navigate` tool (also settable later
  /// via [`McpHandle::set_navigator`]).
  #[cfg(feature = "router")]
  pub fn navigator(mut self, navigator: crate::router::Navigator) -> Self {
    self.navigator = Some(navigator);
    self
  }
}

/// Clonable runtime control for the MCP server: enable/disable, scope
/// changes, per-tool denies — for debug menus, env-var gating, or
/// support-session unlocks.
#[derive(Clone)]
pub struct McpHandle {
  shared: Arc<McpShared>,
  port: u16,
}

impl McpHandle {
  /// The bound port; the endpoint is `http://127.0.0.1:<port>/mcp`.
  pub fn port(&self) -> u16 {
    self.port
  }

  /// The bearer token clients must present (also in the discovery file).
  pub fn token(&self) -> &str {
    &self.shared.token
  }

  pub fn is_enabled(&self) -> bool {
    self.shared.is_enabled()
  }

  /// Disable/enable serving without tearing the listener down. While
  /// disabled every tool is hidden and rejected.
  pub fn set_enabled(&self, enabled: bool) {
    self.shared.set_enabled(enabled);
  }

  pub fn set_scopes(&self, scopes: impl IntoIterator<Item = Scope>) {
    self.shared.set_scopes(scopes.into_iter().collect());
  }

  pub fn add_scope(&self, scope: Scope) {
    self.shared.add_scope(scope);
  }

  pub fn remove_scope(&self, scope: &Scope) {
    self.shared.remove_scope(scope);
  }

  pub fn deny_tool(&self, name: impl Into<String>) {
    self.shared.deny_tool(name);
  }

  pub fn allow_tool(&self, name: &str) {
    self.shared.allow_tool(name);
  }

  /// Navigator used by the built-in `lurq_navigate` tool. Typically set from
  /// a component once the router exists: `ctx.navigator()`.
  #[cfg(feature = "router")]
  pub fn set_navigator(&self, navigator: crate::router::Navigator) {
    *self.shared.navigator.write().unwrap() = Some(navigator);
  }
}

/// Parked `lurq_wait` reply on a tree, resolved as frames complete.
pub(crate) struct McpWaitEntry {
  pub(crate) mode: McpWaitMode,
  pub(crate) reply: Option<shared::McpReply>,
}

pub(crate) enum McpWaitMode {
  Frames(u32),
  Idle,
}

/// Per-tree MCP server state; present only on the root tree.
pub struct McpState {
  pub(crate) shared: Arc<McpShared>,
  pub(crate) registry: Arc<ToolRegistry>,
  pub(crate) receiver: std_mpsc::Receiver<shared::McpRequest>,
  pub(crate) include_devtools: bool,
  server: Option<server::ServerRuntime>,
  discovery_path: Option<std::path::PathBuf>,
}

fn default_app_name() -> String {
  std::env::current_exe()
    .ok()
    .and_then(|path| path.file_stem().map(|stem| stem.to_string_lossy().into_owned()))
    .unwrap_or_else(|| "lurq-app".to_owned())
}

fn build_registry(tools: Vec<McpTool>) -> ToolRegistry {
  let mut registry = ToolRegistry {
    tools: registry::builtin_tools(cfg!(feature = "router")),
  };
  for tool in tools {
    assert!(
      !tool.name.starts_with("lurq_"),
      "custom MCP tool {:?} uses the reserved lurq_ prefix",
      tool.name
    );
    assert!(
      registry.find(&tool.name).is_none(),
      "custom MCP tool {:?} is registered twice",
      tool.name
    );
    let kind = match tool.kind {
      Some(CustomToolKind::Sync(handler)) => ToolKind::Sync(handler),
      Some(CustomToolKind::Async(handler)) => ToolKind::Async(handler),
      None => panic!("custom MCP tool {:?} has no handler", tool.name),
    };
    registry.tools.push(registry::RegisteredTool {
      name: tool.name,
      description: tool.description,
      scope: tool.scope,
      read_only: tool.read_only,
      input_schema: tool.input_schema,
      kind,
    });
  }
  registry
}

impl Tree {
  /// Start the embedded MCP server. Call once on the root tree; the returned
  /// [`McpHandle`] controls availability at runtime.
  ///
  /// The winit shell drains tool calls automatically. A headless harness
  /// (driving [`Tree::pass`] itself) must call
  /// [`Tree::drain_mcp_requests`] in its loop instead — tree reading and
  /// input work as-is; screenshots need a render surface.
  ///
  /// Panics on duplicate/reserved custom tool names or if called twice.
  pub fn enable_mcp(&mut self, config: McpConfig) -> McpHandle {
    assert!(self.mcp.is_none(), "enable_mcp called twice on this tree");

    let registry = Arc::new(build_registry(config.tools));
    let app_name = config.app_name.unwrap_or_else(default_app_name);
    let token = format!("{}{}", uuid::Uuid::new_v4().simple(), uuid::Uuid::new_v4().simple());
    let shared = Arc::new(McpShared::new(
      config.scopes,
      config.denied_tools,
      token,
      app_name,
      config.instructions,
    ));
    #[cfg(feature = "router")]
    if let Some(navigator) = config.navigator {
      *shared.navigator.write().unwrap() = Some(navigator);
    }

    let (sender, receiver) = std_mpsc::channel();
    let server = match server::spawn(shared.clone(), registry.clone(), sender, config.port) {
      Ok(server) => server,
      Err(message) => {
        // A dead listener with a live handle beats poisoning app startup.
        tracing::error!("failed to start MCP server: {message}");
        self.mcp = Some(Box::new(McpState {
          shared: shared.clone(),
          registry,
          receiver,
          include_devtools: config.include_devtools,
          server: None,
          discovery_path: None,
        }));
        return McpHandle { shared, port: 0 };
      }
    };

    let port = server.port;
    let discovery_path = discovery::write(&shared.app_name, port, &shared.token);
    tracing::info!(
      "lurq MCP server listening on http://127.0.0.1:{port}/mcp{}",
      discovery_path
        .as_deref()
        .map(|path| format!(" (discovery: {})", path.display()))
        .unwrap_or_default()
    );

    self.mcp = Some(Box::new(McpState {
      shared: shared.clone(),
      registry,
      receiver,
      include_devtools: config.include_devtools,
      server: Some(server),
      discovery_path,
    }));
    McpHandle { shared, port }
  }

  /// Registered by the shell so request enqueues wake the idle event loop.
  #[cfg_attr(not(feature = "winit"), allow(dead_code))]
  pub(crate) fn set_mcp_waker(&self, waker: Arc<dyn Fn() + Send + Sync>) {
    if let Some(mcp) = &self.mcp {
      *mcp.shared.waker.lock().unwrap() = Some(waker);
    }
  }

  /// Execute all queued MCP tool calls against this (root) tree. The winit
  /// shell calls this every loop turn; headless harnesses call it between
  /// [`Tree::pass`] calls. Returns whether any request was handled.
  pub fn drain_mcp_requests(&mut self, app: &mut App) -> bool {
    let Some(state) = self.mcp.take() else {
      return false;
    };
    let mut did_work = false;
    while let Ok(request) = state.receiver.try_recv() {
      did_work = true;
      tools::execute(self, app, &state, request);
    }
    self.mcp = Some(state);
    did_work
  }

  /// Resolve parked `lurq_wait` replies after a pass; runs on every tree.
  pub(crate) fn mcp_notify_pass(&mut self, report: &crate::app::runtime::PassReport) {
    if self.mcp_wait_entries.is_empty() {
      return;
    }
    let rendered = report.rendered;
    let idle = !self.needs_redraw() && !self.has_active_timeline();
    let mut keep_producing_frames = false;
    self.mcp_wait_entries.retain_mut(|entry| {
      let Some(reply) = entry.reply.take() else {
        return false;
      };
      if reply.is_closed() {
        // The HTTP side timed out and hung up; drop the entry.
        return false;
      }
      let done = match &mut entry.mode {
        McpWaitMode::Frames(remaining) => {
          if rendered {
            *remaining = remaining.saturating_sub(1);
          }
          *remaining == 0
        }
        McpWaitMode::Idle => rendered && idle,
      };
      if done {
        let _ = reply.send(Ok(McpToolOutput::Text("done".into())));
        false
      } else {
        if matches!(entry.mode, McpWaitMode::Frames(_)) {
          keep_producing_frames = true;
        }
        entry.reply = Some(reply);
        true
      }
    });
    // Only frame waits force more frames; idle waits must let the loop
    // settle or they would never resolve.
    if keep_producing_frames {
      self.request_redraw();
    }
  }

  /// Stop the MCP server and remove the discovery file. The shell calls
  /// this when the event loop exits; explicit callers (headless harnesses)
  /// may call it directly.
  pub fn shutdown_mcp(&mut self) {
    let Some(state) = self.mcp.take() else {
      return;
    };
    if let Some(path) = &state.discovery_path {
      discovery::remove(path);
    }
    if let Some(server) = state.server {
      server.cancel.cancel();
      // Wake the accept loop is not needed — cancellation resolves the
      // select. Give the thread a moment but never hang shutdown.
      let _ = server.join.join();
    }
  }
}
