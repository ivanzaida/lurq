//! The gallery as a page of a browser: `scripts/gallery-web.py` builds it.
//!
//! A page has to ask for two things a desktop gives at once, and waits for
//! both before the window starts:
//!
//! - the GPU device, which the browser hands over through a promise: WebGPU's, or WebGL2's where there is none;
//! - the fonts, since a page cannot read the system's. They are fetched from `fonts/` beside the page:
//!   `fonts/index.txt` names the files, one a line, and the families the generic ones draw with, as `sans-serif = Segoe
//!   UI`.
//!
//! The page to open on is the address's fragment: `index.html#/button`.

use lurq::{
  app::{
    App,
    wgpu_render::{WgpuBackends, WgpuDevice, WgpuRenderEngine},
  },
  layout::text_style::GenericFamily,
};
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;

/// The element of the page that says what the gallery waits for, and why it did not start.
const STATUS_ID: &str = "status";
const FONTS: &str = "fonts";

pub fn start() {
  std::panic::set_hook(Box::new(|panic| {
    let message = panic.to_string();
    web_sys::console::error_1(&JsValue::from_str(&message));
    status(Some(&format!("The gallery stopped: {message}")));
  }));
  wasm_bindgen_futures::spawn_local(async {
    let device = match WgpuDevice::request_with(backends()).await {
      Ok(device) => device,
      Err(error) => {
        status(Some(&format!(
          "The gallery draws with WebGPU or WebGL2, and this browser gave it neither: {error}"
        )));
        return;
      }
    };
    web_sys::console::log_1(&JsValue::from_str(&format!(
      "lurq gallery: drawing through {:?}",
      device.backend()
    )));
    let mut app = App::new();
    if let Err(error) = load_fonts(&mut app).await {
      status(Some(&format!("The gallery's fonts did not load: {error}")));
      return;
    }
    status(None);
    crate::run(app, route(), move || {
      WgpuRenderEngine::new().with_device(device.clone())
    });
  });
}

/// What the page may draw through: WebGPU where the browser has it and WebGL2 where it has not, or the one
/// the address asks for, `?backend=webgl` or `?backend=webgpu`, to see the gallery through it.
fn backends() -> WgpuBackends {
  let query = web_sys::window()
    .and_then(|window| window.location().search().ok())
    .unwrap_or_default();
  match query
    .trim_start_matches('?')
    .split('&')
    .find_map(|pair| pair.strip_prefix("backend="))
  {
    Some("webgl") => WgpuBackends::GL,
    Some("webgpu") => WgpuBackends::BROWSER_WEBGPU,
    _ => WgpuBackends::all(),
  }
}

/// The page the address asks for: the fragment, or the overview.
fn route() -> String {
  web_sys::window()
    .and_then(|window| window.location().hash().ok())
    .map(|hash| hash.trim_start_matches('#').to_owned())
    .filter(|route| route.starts_with('/'))
    .unwrap_or_else(|| "/".to_owned())
}

/// Says `message` on the page, or takes the line away once the gallery draws.
fn status(message: Option<&str>) {
  let Some(document) = web_sys::window().and_then(|window| window.document()) else {
    return;
  };
  let element = document.get_element_by_id(STATUS_ID);
  match (message, element) {
    (Some(message), Some(element)) => element.set_text_content(Some(message)),
    // The line was taken away when the gallery started, and something has stopped it since.
    (Some(message), None) => {
      let (Ok(element), Some(body)) = (document.create_element("div"), document.body()) else {
        return;
      };
      element.set_id(STATUS_ID);
      element.set_text_content(Some(message));
      let _ = body.append_child(&element);
    }
    (None, Some(element)) => element.remove(),
    (None, None) => {}
  }
}

async fn fetch(path: &str) -> Result<web_sys::Response, String> {
  let window = web_sys::window().ok_or("no window")?;
  let response: web_sys::Response = JsFuture::from(window.fetch_with_str(path))
    .await
    .map_err(|_| format!("{path} could not be fetched"))?
    .dyn_into()
    .map_err(|_| format!("{path} did not answer"))?;
  if !response.ok() {
    return Err(format!("{path} answered {}", response.status()));
  }
  Ok(response)
}

async fn fetch_text(path: &str) -> Result<String, String> {
  let text = fetch(path).await?.text().map_err(|_| format!("{path} has no text"))?;
  JsFuture::from(text)
    .await
    .ok()
    .and_then(|text| text.as_string())
    .ok_or_else(|| format!("{path} has no text"))
}

async fn fetch_bytes(path: &str) -> Result<Vec<u8>, String> {
  let buffer = fetch(path)
    .await?
    .array_buffer()
    .map_err(|_| format!("{path} has no bytes"))?;
  let buffer = JsFuture::from(buffer)
    .await
    .map_err(|_| format!("{path} has no bytes"))?;
  Ok(js_sys::Uint8Array::new(&buffer).to_vec())
}

/// Loads the fonts `fonts/index.txt` names and gives the generic families the ones it says.
async fn load_fonts(app: &mut App) -> Result<(), String> {
  let index = fetch_text(&format!("{FONTS}/index.txt")).await?;
  let mut families = Vec::new();
  let mut loaded = 0;
  for line in index.lines().map(str::trim) {
    if line.is_empty() || line.starts_with('#') {
      continue;
    }
    if let Some((generic, family)) = line.split_once('=') {
      let generic = GenericFamily::from_name(generic.trim())
        .ok_or_else(|| format!("{} is not a generic family", generic.trim()))?;
      families.push((generic, family.trim().to_owned()));
      continue;
    }
    app.load_font(fetch_bytes(&format!("{FONTS}/{line}")).await?);
    loaded += 1;
  }
  if loaded == 0 {
    return Err(format!("{FONTS}/index.txt names no font"));
  }
  for (generic, family) in families {
    app.set_generic_family(generic, &family);
  }
  Ok(())
}
