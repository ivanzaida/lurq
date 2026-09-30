//! Window addressing (`window` arguments), `lurq_windows`, `lurq_resize` and the `lurq_menu` model.

use crate::{
  app::Tree,
  mcp::{
    McpState,
    shared::{McpToolOutput, McpToolResult},
  },
};

pub(super) fn requested_window(args: &serde_json::Value) -> String {
  args
    .get("window")
    .and_then(|value| value.as_str())
    .filter(|window| !window.is_empty())
    .unwrap_or("main")
    .to_owned()
}

pub(super) fn is_devtools_index(tree: &Tree, index: usize) -> bool {
  #[cfg(feature = "devtools")]
  {
    tree
      .devtools
      .as_ref()
      .is_some_and(|devtools| devtools.secondary_index == index)
  }
  #[cfg(not(feature = "devtools"))]
  {
    let _ = (tree, index);
    false
  }
}

/// Secondary indexes visible to MCP, in order.
pub(super) fn visible_secondary_indexes(tree: &Tree, include_devtools: bool) -> Vec<usize> {
  (0..tree.secondary_window_count())
    .filter(|index| tree.secondary_window(*index).is_some())
    .filter(|index| include_devtools || !is_devtools_index(tree, *index))
    .collect()
}

pub(super) fn find_secondary_index(tree: &Tree, window: &str, include_devtools: bool) -> Option<usize> {
  visible_secondary_indexes(tree, include_devtools)
    .into_iter()
    .find(|index| {
      tree
        .secondary_window(*index)
        .is_some_and(|secondary| secondary.name() == Some(window) || format!("w{}", secondary.id()) == window)
    })
}

/// Resolve a `window` argument to its tree. `"main"` is the root tree;
/// secondaries resolve by name or `w<id>`; closed or unknown windows error
/// as gone rather than falling back to a different window.
pub(super) fn window_tree_mut<'t>(
  root: &'t mut Tree,
  window: &str,
  include_devtools: bool,
) -> Result<&'t mut Tree, String> {
  if window == "main" {
    return Ok(root);
  }
  if window == "focused" {
    if root.window().info().is_focused {
      return Ok(root);
    }
    let focused = visible_secondary_indexes(root, include_devtools)
      .into_iter()
      .find(|index| {
        root
          .secondary_window(*index)
          .is_some_and(|secondary| secondary.tree().window().info().is_focused)
      });
    return match focused {
      Some(index) => Ok(
        root
          .secondary_window_mut(index)
          .expect("index just resolved")
          .tree_mut(),
      ),
      // Focus races with real user activity; fall back to the main window.
      None => Ok(root),
    };
  }
  match find_secondary_index(root, window, include_devtools) {
    Some(index) => Ok(
      root
        .secondary_window_mut(index)
        .expect("index just resolved")
        .tree_mut(),
    ),
    None => Err(format!(
      "window {window:?} not found or closed; list windows with lurq_windows"
    )),
  }
}

pub(super) fn windows_tool(tree: &Tree, state: &McpState) -> McpToolResult {
  let mut windows = Vec::new();
  let main_info = tree.window().info();
  windows.push(serde_json::json!({
    "id": "main",
    "kind": "main",
    "title": tree.window().handle().title(),
    "open": true,
    "focused": main_info.is_focused,
    "width": main_info.resolved_width.round(),
    "height": main_info.resolved_height.round(),
    "scale_factor": main_info.scale_factor,
  }));
  for index in visible_secondary_indexes(tree, state.include_devtools) {
    let Some(secondary) = tree.secondary_window(index) else {
      continue;
    };
    let info = secondary.tree().window().info();
    windows.push(serde_json::json!({
      "id": format!("w{}", secondary.id()),
      "name": secondary.name(),
      "title": secondary.tree().window().handle().title().unwrap_or_else(|| secondary.title().to_owned()),
      "kind": if is_devtools_index(tree, index) { "devtools" } else { "secondary" },
      "open": true,
      "focused": info.is_focused,
      "width": info.resolved_width.round(),
      "height": info.resolved_height.round(),
      "scale_factor": info.scale_factor,
    }));
  }
  Ok(McpToolOutput::Json(serde_json::json!({ "windows": windows })))
}

pub(super) fn resize_tool(tree: &mut Tree, state: &McpState, args: &serde_json::Value) -> McpToolResult {
  let width = args
    .get("width")
    .and_then(|value| value.as_u64())
    .ok_or("`width` is required")?;
  let height = args
    .get("height")
    .and_then(|value| value.as_u64())
    .ok_or("`height` is required")?;
  if width == 0 || height == 0 {
    return Err("width and height must be positive".into());
  }
  let window = requested_window(args);
  let target = window_tree_mut(tree, &window, state.include_devtools)?;
  target.window().handle().resize(width as u32, height as u32);
  Ok(McpToolOutput::Json(serde_json::json!({
    "ok": true, "window": window, "width": width, "height": height,
    "note": "resize is applied by the OS asynchronously; lurq_wait then lurq_windows to confirm"
  })))
}

pub(super) fn menu_json(bar: &crate::app::MenuBar) -> serde_json::Value {
  use serde_json::{Value, json};

  use crate::app::{Menu, MenuAction, MenuItem};
  fn action(a: &MenuAction) -> Value {
    json!({"id": a.id.as_ref(), "label": a.label.as_ref(), "enabled": a.enabled,
      "accelerator": a.accelerator.as_ref().map(|a| json!({"key": a.key.as_ref(),
        "display": a.display(), "shift": a.modifiers.shift, "ctrl": a.modifiers.ctrl,
        "alt": a.modifiers.alt, "meta": a.modifiers.meta}))})
  }
  fn menu(m: &Menu) -> Value {
    json!({"title": m.title.as_ref(), "items": m.items.iter().map(|i| match i {
      MenuItem::Item(a) => action(a), MenuItem::Separator => json!({"separator": true}),
      MenuItem::Submenu(m) => menu(m),
    }).collect::<Vec<_>>()})
  }
  json!({"application": {"name": bar.application.name.as_ref(),
    "about": bar.application.about.as_ref().map(action),
    "preferences": bar.application.preferences.as_ref().map(action), "quit": action(&bar.application.quit)},
    "menus": bar.menus.iter().map(menu).collect::<Vec<_>>()})
}
