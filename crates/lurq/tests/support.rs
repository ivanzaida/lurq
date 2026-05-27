#![allow(dead_code)]

use std::{
  num::NonZeroIsize,
  sync::{Arc, Mutex},
};

use lurq::{
  app::{Runtime, render_engine::RenderEngine},
  layout::render_list::{RectCmd, RenderList},
  node::color::Color,
};
use raw_window_handle::{
  DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, Win32WindowHandle, WindowHandle, WindowsDisplayHandle,
};

pub struct TestSurface;

impl HasWindowHandle for TestSurface {
  fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
    let handle = Win32WindowHandle::new(NonZeroIsize::new(1).unwrap());
    Ok(unsafe { WindowHandle::borrow_raw(handle.into()) })
  }
}

impl HasDisplayHandle for TestSurface {
  fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
    Ok(unsafe { DisplayHandle::borrow_raw(WindowsDisplayHandle::new().into()) })
  }
}

pub fn run_pass(runtime: &mut Runtime) {
  runtime.pass(&TestSurface);
}

#[derive(Clone, Debug)]
pub struct RenderSnapshot {
  pub rects: Vec<RectSnapshot>,
}

#[derive(Clone, Copy, Debug)]
pub struct RectSnapshot {
  pub width: f32,
  pub height: f32,
  pub color: Color,
}

pub fn render_pass(runtime: &mut Runtime) -> RenderSnapshot {
  let capture = Arc::new(Mutex::new(None));
  runtime.set_render_engine(Box::new(CapturingRenderEngine {
    capture: capture.clone(),
  }));
  runtime.pass(&TestSurface);
  capture
    .lock()
    .unwrap()
    .clone()
    .unwrap_or(RenderSnapshot { rects: vec![] })
}

struct CapturingRenderEngine {
  capture: Arc<Mutex<Option<RenderSnapshot>>>,
}

impl RenderEngine for CapturingRenderEngine {
  fn resize(&mut self, _width: u32, _height: u32) {}

  fn render(&mut self, list: &RenderList, _window: WindowHandle<'_>, _display: DisplayHandle<'_>) {
    let rects = list.rects.iter().map(rect_snapshot).collect();
    *self.capture.lock().unwrap() = Some(RenderSnapshot { rects });
  }
}

fn rect_snapshot(rect: &RectCmd) -> RectSnapshot {
  RectSnapshot {
    width: rect.width,
    height: rect.height,
    color: rect.color,
  }
}
