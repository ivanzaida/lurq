//! `lurq_screenshot`: window, region or ref captures encoded as PNG.

use std::sync::{Arc, Mutex};

use super::{
  resolve::{ref_bounds, resolve_ref},
  windows::{requested_window, window_tree_mut},
};
use crate::{
  app::Tree,
  mcp::{
    McpState,
    shared::{McpReply, McpToolOutput, McpToolResult},
  },
};

pub(super) fn screenshot_tool(tree: &mut Tree, state: &McpState, args: &serde_json::Value, reply: McpReply) {
  let result = prepare_screenshot(tree, state, args, reply);
  if let Err((reply, message)) = result {
    let _ = reply.send(Err(message));
  }
}

/// On error, gives the reply back so the caller can resolve it.
pub(super) fn prepare_screenshot(
  tree: &mut Tree,
  state: &McpState,
  args: &serde_json::Value,
  reply: McpReply,
) -> Result<(), (McpReply, String)> {
  let ref_id = args.get("ref").and_then(|value| value.as_str());

  let (window, region_logical) = if let Some(ref_id) = ref_id {
    let resolved = match resolve_ref(state, ref_id) {
      Ok(resolved) => resolved,
      Err(message) => return Err((reply, message)),
    };
    let target = match window_tree_mut(tree, &resolved.window, state.include_devtools) {
      Ok(target) => target,
      Err(message) => return Err((reply, message)),
    };
    let bounds = match ref_bounds(target, &resolved, ref_id) {
      Ok(bounds) => bounds,
      Err(message) => return Err((reply, message)),
    };
    (
      resolved.window,
      Some(crate::app::window::ScreenshotRegion {
        x: bounds[0],
        y: bounds[1],
        width: bounds[2],
        height: bounds[3],
      }),
    )
  } else {
    let window = requested_window(args);
    let region = match args.get("region") {
      Some(region_value) => {
        let field = |name: &str| {
          region_value
            .get(name)
            .and_then(|value| value.as_f64())
            .map(|value| value as f32)
        };
        match (field("x"), field("y"), field("width"), field("height")) {
          (Some(x), Some(y), Some(width), Some(height)) => Some((x, y, width, height)),
          _ => return Err((reply, "region requires numeric x, y, width, height".into())),
        }
      }
      None => None,
    };
    let scale = {
      let target = match window_tree_mut(tree, &window, state.include_devtools) {
        Ok(target) => target,
        Err(message) => return Err((reply, message)),
      };
      target.scale_factor()
    };
    // The MCP surface speaks screenshot pixels (physical); the capture
    // pipeline takes logical regions.
    let region_logical = region.map(|(x, y, width, height)| crate::app::window::ScreenshotRegion {
      x: x / scale,
      y: y / scale,
      width: width / scale,
      height: height / scale,
    });
    (window, region_logical)
  };

  let target = match window_tree_mut(tree, &window, state.include_devtools) {
    Ok(target) => target,
    Err(message) => return Err((reply, message)),
  };

  // The reply parks inside the capture callback; the capture pipeline
  // guarantees the callback fires exactly once (pixels or error), frames
  // later, on a readback thread.
  let slot = Mutex::new(Some(reply));
  let callback = Arc::new(
    move |outcome: Result<crate::app::render_engine::CapturedFrame, String>| {
      let Some(reply) = slot.lock().unwrap().take() else {
        return;
      };
      let result = outcome.and_then(|frame| encode_png(&frame));
      let _ = reply.send(result);
    },
  );
  target.request_screenshot_capture(
    crate::app::render_engine::RenderCaptureTarget::Bytes(callback),
    region_logical,
  );
  Ok(())
}

pub(super) fn encode_png(frame: &crate::app::render_engine::CapturedFrame) -> McpToolResult {
  use image::ImageEncoder as _;
  let mut png = Vec::new();
  image::codecs::png::PngEncoder::new(&mut png)
    .write_image(&frame.rgba, frame.width, frame.height, image::ExtendedColorType::Rgba8)
    .map_err(|error| format!("failed to encode screenshot PNG: {error}"))?;
  Ok(McpToolOutput::Image {
    data: png,
    mime: "image/png",
  })
}
