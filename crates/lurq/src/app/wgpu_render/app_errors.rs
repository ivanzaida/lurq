//! The errors of the app's own wgpu calls, told apart from lurq's.
//!
//! wgpu answers an invalid call with an error that a scope of the device catches, and that otherwise goes to the
//! device's one handler, which lurq sets ([`super::device_loss::watch`]). An error of lurq's own work is a fault of
//! lurq's and ends the program. One of the app's ends nothing: it is logged and given to the extensions
//! (`WgpuFrameExtension::error`), and what failed does nothing.
//!
//! What an extension does in `initialize`, `prepare`, `device_lost` and `shutdown` is run in scopes of every kind
//! ([`caught`]), so its errors are its own. What the app does elsewhere, on a thread of its own, reaches the handler;
//! wgpu calls the handler on the thread that made the call, and lurq marks its own work on the thread that draws
//! ([`OwnWork`]): an error outside it is the app's, kept for the extensions ([`Untold`]). In a browser the handler is
//! called later, by the page, and nothing tells whose an error is: there only the extensions' calls are told apart.

use std::{cell::Cell, sync::Arc};

use parking_lot::Mutex;

use super::WgpuAppError;

thread_local! {
  /// Whether lurq's own work is being done on this thread now.
  static OWN: Cell<bool> = const { Cell::new(false) };
}

/// lurq's own work on the thread that draws, while this lives: an uncaptured error in it is lurq's.
pub(super) struct OwnWork(bool);

impl OwnWork {
  pub(super) fn begin() -> Self {
    Self(OWN.replace(true))
  }
}

impl Drop for OwnWork {
  fn drop(&mut self) {
    OWN.set(self.0);
  }
}

/// Whether an uncaptured error that comes now, on this thread, is of lurq's own work.
pub(super) fn is_own() -> bool {
  OWN.get()
}

/// Runs `call` as the app's: its errors are its own.
pub(super) fn apps<R>(call: impl FnOnce() -> R) -> R {
  let own = OWN.replace(false);
  let result = call();
  OWN.set(own);
  result
}

/// The errors of the app's own calls on a device that came to its handler and were given to no extension yet.
/// Shared by the engines that draw with the device.
#[derive(Clone, Default)]
pub(super) struct Untold(Arc<Mutex<Vec<WgpuAppError>>>);

impl Untold {
  pub(super) fn push(&self, error: WgpuAppError) {
    let mut untold = self.0.lock();
    // An app that fails in every frame is told of each, but nothing is kept without end.
    if untold.len() < 64 {
      untold.push(error);
    }
  }

  pub(super) fn take(&self) -> Vec<WgpuAppError> {
    std::mem::take(&mut *self.0.lock())
  }
}

/// Says in the log what the app's call did wrong.
pub(super) fn log(error: &WgpuAppError) {
  tracing::error!("a wgpu call of the app failed, which ends nothing: {error}");
}

/// Runs `call`, an extension's, as the app's, in scopes of every kind of error, and gives what it caught. In a
/// browser a scope is answered later: what it caught is put in `untold`, for the next frame.
pub(super) fn caught<R>(device: &wgpu::Device, untold: &Untold, call: impl FnOnce() -> R) -> (R, Vec<WgpuAppError>) {
  let validation = device.push_error_scope(wgpu::ErrorFilter::Validation);
  let memory = device.push_error_scope(wgpu::ErrorFilter::OutOfMemory);
  let internal = device.push_error_scope(wgpu::ErrorFilter::Internal);
  let result = apps(call);
  // Popped in the order they were pushed in reverse, as wgpu asks.
  let scopes = [internal.pop(), memory.pop(), validation.pop()];
  #[cfg(not(target_arch = "wasm32"))]
  {
    let _ = untold;
    let errors = scopes
      .into_iter()
      .filter_map(pollster::block_on)
      .map(WgpuAppError::from)
      .collect::<Vec<_>>();
    errors.iter().for_each(log);
    (result, errors)
  }
  #[cfg(target_arch = "wasm32")]
  {
    for scope in scopes {
      let untold = untold.clone();
      wasm_bindgen_futures::spawn_local(async move {
        if let Some(error) = scope.await {
          let error = WgpuAppError::from(error);
          log(&error);
          untold.push(error);
        }
      });
    }
    (result, Vec::new())
  }
}
