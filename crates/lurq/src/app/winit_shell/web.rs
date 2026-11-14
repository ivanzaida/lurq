//! The window in a browser: the canvas of the page (`app::web_canvas`).
//!
//! The sizes the app asks for (`with_size`, `with_min_size`, `with_max_size`)
//! are a desktop's and are not given to the canvas, whose size is the page's.

use wasm_bindgen::JsCast;
use winit::{
  dpi::PhysicalSize,
  platform::web::{WindowAttributesExtWebSys, WindowExtWebSys},
  window::{Window, WindowAttributes},
};

/// The attributes of a window that is the canvas of the page.
pub(super) fn window_attributes(mut attrs: WindowAttributes) -> WindowAttributes {
  attrs.inner_size = None;
  attrs.min_inner_size = None;
  attrs.max_inner_size = None;
  attrs.position = None;
  match crate::app::web_canvas::page_canvas() {
    Some(canvas) => attrs.with_canvas(Some(canvas)),
    // No document to look in: winit makes a canvas of its own.
    None => attrs.with_append(true),
  }
}

/// Whether the browser tells a wheel's turn as the distance the system wants scrolled, as it does on Apple's
/// systems, and not as a notch's worth of a page; see `web_wheel`.
pub(super) fn scrolls_the_systems_distance() -> bool {
  thread_local! {
    static APPLE: bool = web_sys::window()
      .and_then(|window| window.navigator().user_agent().ok())
      .is_some_and(|agent| agent.contains("Mac OS"));
  }
  APPLE.with(|apple| *apple)
}

/// Where the wheel last turned over the canvas, in the page's pixels.
///
/// winit tells where the pointer is when it moves. A page that has just loaded under a pointer at rest has
/// seen no move, and a wheel turned then scrolled what is at the window's corner. The wheel's own event says
/// where it is, and is heard here before winit hears it.
pub(super) struct WheelPointer {
  position: std::rc::Rc<std::cell::Cell<Option<(f64, f64)>>>,
  _listener: wasm_bindgen::closure::Closure<dyn FnMut(web_sys::WheelEvent)>,
}

impl WheelPointer {
  pub(super) fn watch(window: &Window) -> Option<Self> {
    let canvas = window.canvas()?;
    let position = std::rc::Rc::new(std::cell::Cell::new(None));
    let heard = position.clone();
    let listener =
      wasm_bindgen::closure::Closure::<dyn FnMut(web_sys::WheelEvent)>::new(move |event: web_sys::WheelEvent| {
        heard.set(Some((f64::from(event.offset_x()), f64::from(event.offset_y()))))
      });
    // Capturing listeners of the canvas run before winit's own.
    canvas
      .add_event_listener_with_callback_and_bool("wheel", listener.as_ref().unchecked_ref(), true)
      .ok()?;
    Some(Self {
      position,
      _listener: listener,
    })
  }

  /// Where the wheel turned since the last call, in physical pixels.
  pub(super) fn take(&self, scale: f64) -> Option<(f64, f64)> {
    self.position.take().map(|(x, y)| (x * scale, y * scale))
  }
}

/// The canvas's size in physical pixels as the page lays it out now. winit learns the size from an observer,
/// which has not run yet when the window is made.
pub(super) fn canvas_size(window: &Window) -> Option<PhysicalSize<u32>> {
  let canvas = window.canvas()?;
  let scale = window.scale_factor();
  let width = (f64::from(canvas.client_width()) * scale).round();
  let height = (f64::from(canvas.client_height()) * scale).round();
  (width >= 1.0 && height >= 1.0).then(|| PhysicalSize::new(width as u32, height as u32))
}
