#![allow(dead_code)]

use std::{
  num::NonZeroIsize,
  sync::{Arc, Mutex},
};

use lurq::{
  app::{App, Tree, render_engine::RenderEngine},
  layout::render_list::{RectCmd, RenderList},
  node::color::Color,
};

pub fn default_app() -> App {
  App::new()
}
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

pub fn run_pass(tree: &mut Tree) {
  let mut app = App::new();
  tree.pass(&mut app, &TestSurface);
}

#[derive(Clone, Debug)]
pub struct RenderSnapshot {
  pub rects: Vec<RectSnapshot>,
  pub glyph_count: usize,
  #[cfg(feature = "image")]
  pub image_orders: Vec<usize>,
  #[cfg(feature = "svg")]
  pub svg_orders: Vec<usize>,
}

#[derive(Clone, Copy, Debug)]
pub struct RectSnapshot {
  pub x: f32,
  pub y: f32,
  pub width: f32,
  pub height: f32,
  pub color: Color,
  pub radii: [f32; 4],
  pub stroke: [f32; 4],
  pub stroke_color: Color,
}

pub fn render_pass(tree: &mut Tree) -> RenderSnapshot {
  let capture = Arc::new(Mutex::new(None));
  let render_capture = capture.clone();
  tree.set_render_engine_factory(move || {
    Box::new(CapturingRenderEngine {
      capture: render_capture.clone(),
    })
  });
  let mut app = App::new();
  tree.pass(&mut app, &TestSurface);
  capture.lock().unwrap().clone().unwrap_or_else(empty_snapshot)
}

struct CapturingRenderEngine {
  capture: Arc<Mutex<Option<RenderSnapshot>>>,
}

impl RenderEngine for CapturingRenderEngine {
  fn resize(&mut self, _width: u32, _height: u32) {}

  fn render(&mut self, list: &RenderList, _window: WindowHandle<'_>, _display: DisplayHandle<'_>) {
    let rects = list.rects.iter().map(rect_snapshot).collect();
    *self.capture.lock().unwrap() = Some(RenderSnapshot {
      rects,
      glyph_count: list.glyphs.len(),
      #[cfg(feature = "image")]
      image_orders: list.images.iter().map(|image| image.order).collect(),
      #[cfg(feature = "svg")]
      svg_orders: list.svgs.iter().map(|svg| svg.order).collect(),
    });
  }
}

fn empty_snapshot() -> RenderSnapshot {
  RenderSnapshot {
    rects: vec![],
    glyph_count: 0,
    #[cfg(feature = "image")]
    image_orders: vec![],
    #[cfg(feature = "svg")]
    svg_orders: vec![],
  }
}

fn rect_snapshot(rect: &RectCmd) -> RectSnapshot {
  RectSnapshot {
    x: rect.x,
    y: rect.y,
    width: rect.width,
    height: rect.height,
    color: rect.color,
    radii: rect.radii,
    stroke: rect.stroke,
    stroke_color: rect.stroke_color,
  }
}
