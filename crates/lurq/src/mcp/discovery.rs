//! Discovery files: `<data-local>/lurq/mcp/<pid>.json`, one per running
//! MCP-enabled app, so tooling (`claude mcp add`, a future `lurq-mcp` shim)
//! can enumerate running apps and their ports/tokens. Removed on shutdown;
//! files left by apps that were killed are removed by the next app that starts.

mod process;

use std::path::{Path, PathBuf};

fn discovery_dir() -> Option<PathBuf> {
  #[cfg(windows)]
  {
    std::env::var_os("LOCALAPPDATA").map(|base| PathBuf::from(base).join("lurq").join("mcp"))
  }
  #[cfg(target_os = "macos")]
  {
    std::env::var_os("HOME").map(|home| {
      PathBuf::from(home)
        .join("Library")
        .join("Application Support")
        .join("lurq")
        .join("mcp")
    })
  }
  #[cfg(all(unix, not(target_os = "macos")))]
  {
    std::env::var_os("XDG_RUNTIME_DIR")
      .map(PathBuf::from)
      .or_else(|| std::env::var_os("XDG_STATE_HOME").map(PathBuf::from))
      .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local").join("state")))
      .map(|base| base.join("lurq").join("mcp"))
  }
  #[cfg(not(any(windows, unix)))]
  {
    None
  }
}

/// Write the discovery file; returns its path for removal on shutdown.
/// The file contains the bearer token, so on Unix it is user-readable only.
pub(crate) fn write(app_name: &str, port: u16, token: &str) -> Option<PathBuf> {
  let dir = discovery_dir()?;
  if let Err(error) = std::fs::create_dir_all(&dir) {
    tracing::warn!("failed to create MCP discovery directory {}: {error}", dir.display());
    return None;
  }
  remove_stale(&dir);
  let path = dir.join(format!("{}.json", std::process::id()));
  let contents = serde_json::json!({
    "version": 1,
    "pid": std::process::id(),
    "app": app_name,
    "transport": "streamable-http",
    "port": port,
    "url": format!("http://127.0.0.1:{port}/mcp"),
    "token": token,
  });
  let payload = serde_json::to_string_pretty(&contents).ok()?;
  if let Err(error) = std::fs::write(&path, payload) {
    tracing::warn!("failed to write MCP discovery file {}: {error}", path.display());
    return None;
  }
  #[cfg(unix)]
  {
    use std::os::unix::fs::PermissionsExt;
    let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
  }
  Some(path)
}

/// Remove the discovery files of processes that no longer run. Only files named
/// `<pid>.json` are considered, never this process's own.
fn remove_stale(dir: &Path) {
  let entries = match std::fs::read_dir(dir) {
    Ok(entries) => entries,
    Err(error) => {
      tracing::warn!("failed to list MCP discovery directory {}: {error}", dir.display());
      return;
    }
  };
  let own = std::process::id();
  let stale = entries
    .filter_map(Result::ok)
    .map(|entry| entry.path())
    .filter(|path| file_pid(path).is_some_and(|pid| pid != own && !process::is_running(pid)));
  for path in stale {
    remove(&path);
  }
}

/// The pid a discovery file is named after: `<digits>.json`.
fn file_pid(path: &Path) -> Option<u32> {
  if path.extension()? != "json" {
    return None;
  }
  let stem = path.file_stem()?.to_str()?;
  if stem.is_empty() || !stem.bytes().all(|byte| byte.is_ascii_digit()) {
    return None;
  }
  stem.parse().ok()
}

pub(crate) fn remove(path: &Path) {
  if let Err(error) = std::fs::remove_file(path)
    && path.exists()
  {
    tracing::warn!("failed to remove MCP discovery file {}: {error}", path.display());
  }
}
