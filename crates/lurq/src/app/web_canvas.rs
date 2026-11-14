//! The canvas of the page a browser's window draws to.
//!
//! The page lays the canvas out, not the app: a canvas the page already has
//! under the id [`CANVAS_ID`] is drawn to where the page put it, and without
//! one a canvas is added to the body and fills the viewport. Both the GPU
//! device and the window ask here, so they get the same one: WebGL gives a
//! device for a canvas, which therefore has to be there before the window is.

use wasm_bindgen::JsCast;

/// The id of the canvas the window draws to.
pub(crate) const CANVAS_ID: &str = "lurq";

/// The page's canvas, added to the body when the page has none.
pub(crate) fn page_canvas() -> Option<web_sys::HtmlCanvasElement> {
  let document = web_sys::window()?.document()?;
  if let Some(element) = document.get_element_by_id(CANVAS_ID) {
    return element.dyn_into().ok();
  }
  let canvas: web_sys::HtmlCanvasElement = document.create_element("canvas").ok()?.dyn_into().ok()?;
  canvas.set_id(CANVAS_ID);
  let style = canvas.style();
  for (property, value) in [
    ("position", "fixed"),
    ("left", "0"),
    ("top", "0"),
    ("width", "100vw"),
    ("height", "100vh"),
    ("display", "block"),
    ("outline", "none"),
  ] {
    let _ = style.set_property(property, value);
  }
  document.body()?.append_child(&canvas).ok()?;
  Some(canvas)
}
