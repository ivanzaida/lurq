//! Picking a color from the screen on macOS: the system's own sampler (`NSColorSampler`), with its own loupe. It
//! runs on the main thread; its answer, a color or none where the person gave up, is converted to sRGB.

use std::{cell::Cell, ffi::c_void};

use block2::RcBlock;
use objc2_app_kit::{NSColor, NSColorSampler, NSColorSpace};
use objc2_foundation::MainThreadMarker;

use super::Picked;
use crate::node::color::Color;

/// Shows the system's sampler from the main thread; `then` is called on it with what the person picked.
pub(super) fn pick(then: Box<dyn FnOnce(Picked) + Send>) {
  match MainThreadMarker::new() {
    Some(_) => show(then),
    None => on_main_thread(Box::new(move || show(then))),
  }
}

/// Shows the sampler. On the main thread only.
fn show(then: Box<dyn FnOnce(Picked) + Send>) {
  // The handler is called once, as the session ends; a block is called through `Fn`.
  let then = Cell::new(Some(then));
  let handler = RcBlock::new(move |color: *mut NSColor| {
    let Some(then) = then.take() else {
      return;
    };
    // SAFETY: the sampler gives a color it owns for the call, or nil where the person gave up.
    let color = unsafe { color.as_ref() };
    then(Ok(color.and_then(srgb)));
  });
  let sampler = NSColorSampler::new();
  // SAFETY: called on the main thread; the sampler keeps itself and the handler until the session ends.
  unsafe { sampler.showSamplerWithSelectionHandler(&handler) };
}

/// `color` in sRGB, as the engine's colors are: `None` where it has no sRGB form (a pattern).
fn srgb(color: &NSColor) -> Option<Color> {
  let color = color.colorUsingColorSpace(&NSColorSpace::sRGBColorSpace())?;
  let channel = |value: f64| (value.clamp(0.0, 1.0) * 255.0).round() as u8;
  Some(Color::new(
    channel(color.redComponent() as f64),
    channel(color.greenComponent() as f64),
    channel(color.blueComponent() as f64),
    channel(color.alphaComponent() as f64),
  ))
}

unsafe extern "C" {
  /// The main queue of libdispatch, which `dispatch_get_main_queue()` returns.
  static _dispatch_main_q: c_void;
  fn dispatch_async_f(queue: *const c_void, context: *mut c_void, work: extern "C" fn(*mut c_void));
}

/// Runs `work` on the main thread, as the main queue gets to it.
fn on_main_thread(work: Box<dyn FnOnce() + Send>) {
  extern "C" fn run(context: *mut c_void) {
    // SAFETY: `context` is the box made below, given to this call once.
    let work = unsafe { Box::from_raw(context as *mut Box<dyn FnOnce() + Send>) };
    work();
  }
  let context = Box::into_raw(Box::new(work)) as *mut c_void;
  // SAFETY: the main queue lives as long as the process; `run` takes the box back exactly once.
  unsafe { dispatch_async_f(&raw const _dispatch_main_q, context, run) };
}
