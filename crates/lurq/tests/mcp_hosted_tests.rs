//! Hosted MCP: an app with an MCP server of its own serves lurq's tools from it. lurq starts no listener and
//! writes no discovery file; the host mounts `McpHandle::service`, and scopes and `set_enabled` still decide what it
//! lists, while calls that need the event loop are executed by `Tree::drain_mcp_requests`.
#![cfg(feature = "mcp")]

mod mcp_client;

use std::{
  path::{Path, PathBuf},
  sync::Arc,
  thread::JoinHandle,
  time::{Duration, Instant},
};

use http_body_util::BodyExt;
use lurq::{
  app::{App, Tree},
  components::Column,
  mcp::{
    McpConfig, McpHandle, Scope,
    rmcp::transport::streamable_http_server::{
      StreamableHttpServerConfig, StreamableHttpService, session::local::LocalSessionManager,
    },
  },
};
use mcp_client::Client;
use serde_json::{Value, json};

#[test]
fn hosted_mcp_serves_lurq_tools_from_the_host_server() {
  let base = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("mcp-hosted-{}", std::process::id()));
  let discovery = point_discovery_at(&base);

  let mut app = App::new();
  let mut tree = Tree::new();
  tree.resize(200, 100);
  tree.set_root(Column::new().width(200.0).height(100.0));
  tree.pass_headless(&mut app);
  let mcp = tree.enable_mcp(
    McpConfig::new()
      .app_name("lurq-hosted-test")
      .scopes([Scope::Observe])
      .hosted(),
  );

  assert_eq!(mcp.port(), 0, "a hosted server binds no port");
  assert!(
    !discovery.join(format!("{}.json", std::process::id())).exists(),
    "a hosted server writes no discovery file"
  );

  let port = host_server(mcp.clone());
  let host = mcp.clone();
  let agent = std::thread::spawn(move || {
    let mut client = Client::new(port, "host token".into());
    client.initialize();
    let listed = tool_names(&client.request("tools/list", json!({})));
    let windows = client.tool_json("lurq_windows", json!({}));
    host.set_enabled(false);
    let disabled = tool_names(&client.request("tools/list", json!({})));
    (listed, windows, disabled)
  });
  let (listed, windows, disabled) = drain_until_done(&mut tree, &mut app, agent);

  assert!(
    listed.iter().any(|name| name == "lurq_screenshot"),
    "Observe tools are listed: {listed:?}"
  );
  assert!(
    !listed.iter().any(|name| name == "lurq_interact"),
    "tools outside the granted scopes are not listed: {listed:?}"
  );
  assert!(
    windows.to_string().contains("main"),
    "event-loop tools answer through the host: {windows}"
  );
  assert!(disabled.is_empty(), "a disabled server lists no tools: {disabled:?}");

  tree.shutdown_mcp();
  let _ = std::fs::remove_dir_all(&base);
}

/// The host app's server: rmcp's streamable-HTTP service with one lurq `McpService` per session, on a port of its
/// own. A real host also checks its own bearer token.
fn host_server(mcp: McpHandle) -> u16 {
  let (port_tx, port_rx) = std::sync::mpsc::channel();
  std::thread::spawn(move || {
    let runtime = tokio::runtime::Builder::new_current_thread()
      .enable_all()
      .build()
      .unwrap();
    runtime.block_on(async move {
      let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
      port_tx.send(listener.local_addr().unwrap().port()).unwrap();
      let service = StreamableHttpService::new(
        move || Ok(mcp.service()),
        Arc::new(LocalSessionManager::default()),
        StreamableHttpServerConfig::default(),
      );
      loop {
        let (stream, _) = listener.accept().await.unwrap();
        let service = service.clone();
        tokio::spawn(async move {
          let io = hyper_util::rt::TokioIo::new(stream);
          let hyper_service = hyper::service::service_fn(move |request| {
            let mut service = service.clone();
            async move {
              let response = tower_service::Service::call(&mut service, request)
                .await
                .expect("streamable http service is infallible");
              Ok::<_, std::convert::Infallible>(response.map(|body| body.map_err(std::io::Error::other).boxed()))
            }
          });
          let _ = hyper::server::conn::http1::Builder::new()
            .serve_connection(io, hyper_service)
            .await;
        });
      }
    });
  });
  port_rx
    .recv_timeout(Duration::from_secs(5))
    .expect("the host server listens")
}

/// Runs the app loop as the winit shell does until the agent thread is done.
fn drain_until_done<T>(tree: &mut Tree, app: &mut App, agent: JoinHandle<T>) -> T {
  let deadline = Instant::now() + Duration::from_secs(60);
  while !agent.is_finished() {
    assert!(Instant::now() < deadline, "the agent did not finish");
    tree.drain_mcp_requests(app);
    std::thread::sleep(Duration::from_millis(1));
  }
  agent.join().expect("agent thread")
}

fn tool_names(listed: &Value) -> Vec<String> {
  listed["tools"]
    .as_array()
    .map(|tools| {
      tools
        .iter()
        .filter_map(|tool| tool["name"].as_str().map(str::to_owned))
        .collect()
    })
    .unwrap_or_default()
}

/// Send the discovery directory into `base` for this test process.
fn point_discovery_at(base: &Path) -> PathBuf {
  let (variable, dir) = if cfg!(windows) {
    ("LOCALAPPDATA", base.join("lurq").join("mcp"))
  } else if cfg!(target_os = "macos") {
    (
      "HOME",
      base
        .join("Library")
        .join("Application Support")
        .join("lurq")
        .join("mcp"),
    )
  } else {
    ("XDG_RUNTIME_DIR", base.join("lurq").join("mcp"))
  };
  // SAFETY: this test binary runs this one test, the only reader of the variable.
  unsafe { std::env::set_var(variable, base) };
  dir
}
