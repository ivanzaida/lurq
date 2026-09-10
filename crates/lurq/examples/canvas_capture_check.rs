//! End-to-end canvas composition and texture-update check on a hidden Win32 surface.
//! cargo run -p lurq --example canvas_capture_check --features canvas,screenshot,wgpu -- wgpu
//! cargo run -p lurq --example canvas_capture_check --features canvas,screenshot,dx12 -- dx12
#[cfg(windows)]
#[path = "support/canvas_scene.rs"]
mod canvas_scene;

#[cfg(windows)]
fn main() {
  use std::{
    num::NonZeroIsize,
    sync::{Arc, mpsc},
    time::Duration,
  };

  use lurq::{
    app::{
      App, Tree,
      render_engine::{RenderCaptureTarget, RenderEngine, RenderFrameCapture},
    },
    layout::{Constraints, Size, render_list::RenderList},
  };
  use raw_window_handle::{
    DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, Win32WindowHandle, WindowHandle,
  };
  use windows::{
    Win32::{
      Foundation::{HWND, LPARAM, LRESULT, WPARAM},
      UI::WindowsAndMessaging::{
        CreateWindowExW, DefWindowProcW, DestroyWindow, RegisterClassW, WINDOW_EX_STYLE, WNDCLASSW, WS_POPUP,
      },
    },
    core::w,
  };

  extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
  }
  struct Surface(HWND);
  impl HasWindowHandle for Surface {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
      let raw = Win32WindowHandle::new(NonZeroIsize::new(self.0.0 as isize).unwrap());
      Ok(unsafe { WindowHandle::borrow_raw(raw.into()) })
    }
  }
  impl HasDisplayHandle for Surface {
    fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
      Ok(DisplayHandle::windows())
    }
  }
  impl Drop for Surface {
    fn drop(&mut self) {
      unsafe {
        let _ = DestroyWindow(self.0);
      }
    }
  }
  struct Capture {
    backend: Box<dyn RenderEngine>,
    target: RenderCaptureTarget,
  }
  impl RenderEngine for Capture {
    fn prepare_canvases(&mut self, canvases: &[lurq::canvas::CanvasHandle]) {
      self.backend.prepare_canvases(canvases);
    }
    fn resize(&mut self, w: u32, h: u32) {
      self.backend.resize(w, h);
    }
    fn render(&mut self, list: &RenderList, window: WindowHandle<'_>, display: DisplayHandle<'_>) -> bool {
      self.backend.render_with_capture(
        list,
        window,
        display,
        Some(RenderFrameCapture {
          x: 0,
          y: 0,
          width: 256,
          height: 256,
          target: self.target.clone(),
          window_clip: None,
        }),
      )
    }
  }
  let surface = Surface(unsafe {
    let class_name = w!("lurq_canvas_capture_check");
    assert_ne!(
      RegisterClassW(&WNDCLASSW {
        lpfnWndProc: Some(wndproc),
        lpszClassName: class_name,
        ..Default::default()
      }),
      0
    );
    CreateWindowExW(
      WINDOW_EX_STYLE::default(),
      class_name,
      w!("Canvas capture check"),
      WS_POPUP,
      0,
      0,
      256,
      256,
      None,
      None,
      None,
      None,
    )
    .unwrap()
  });
  let backend_name = std::env::args().nth(1).unwrap_or_else(|| "wgpu".into());
  let (sender, receiver) = mpsc::channel();
  let target = RenderCaptureTarget::Bytes(Arc::new(move |frame| {
    sender.send(frame).unwrap();
  }));
  let mut tree = Tree::new();
  let name = backend_name.clone();
  tree.set_render_engine_factory(move || {
    let backend: Box<dyn RenderEngine> = match name.as_str() {
      #[cfg(feature = "wgpu")]
      "wgpu" => Box::new(lurq::app::wgpu_render::WgpuRenderEngine::new()),
      #[cfg(feature = "dx12")]
      "dx12" => Box::new(lurq::app::dx12_render::Dx12RenderEngine::new()),
      _ => panic!("enable the selected backend feature"),
    };
    Box::new(Capture {
      backend,
      target: target.clone(),
    })
  });
  tree.resize(256, 256);
  tree.set_layout_constraints_override(Some(Constraints::loose(Size::new(256.0, 256.0))));
  let mut app = App::new();
  tree.mount_root::<canvas_scene::CanvasScene>(&mut app, ());
  assert!(tree.pass(&mut app, &surface).rendered);
  let frame = receiver.recv_timeout(Duration::from_secs(15)).unwrap().unwrap();
  let png = image::RgbaImage::from_raw(frame.width, frame.height, frame.rgba).unwrap();
  assert_pixel(&png, 30, 30, [239, 68, 68, 255]);
  assert_pixel(&png, 58, 50, [24, 36, 52, 255]);
  assert_pixel(&png, 74, 146, [52, 211, 153, 255]);
  assert_pixel(&png, 155, 192, [251, 191, 36, 255]);
  // The window renderer composites straight-alpha images in linear light.
  assert_pixel(&png, 160, 40, [49, 96, 191, 255]);
  let output = std::path::PathBuf::from("target").join(format!("canvas-{backend_name}.png"));
  png.save(&output).unwrap();
  let canvas = tree.get_element_by_id("canvas").unwrap().as_canvas().unwrap();
  canvas.context_2d().set_fill_style("#ffffff");
  canvas.context_2d().fill_rect(16.0, 16.0, 12.0, 12.0);
  let report = tree.pass(&mut app, &surface);
  assert!(report.rendered && !report.layout_updated);
  let frame = receiver.recv_timeout(Duration::from_secs(15)).unwrap().unwrap();
  let updated = image::RgbaImage::from_raw(frame.width, frame.height, frame.rgba).unwrap();
  assert_pixel(&updated, 30, 30, [255, 255, 255, 255]);
  assert_pixel(&updated, 58, 50, [24, 36, 52, 255]);
  assert!(!tree.needs_redraw());
  tree.get_element_by_id_mut("canvas").unwrap().set_corner_radius(100.0);
  assert!(tree.pass(&mut app, &surface).rendered);
  let frame = receiver.recv_timeout(Duration::from_secs(15)).unwrap().unwrap();
  let rounded = image::RgbaImage::from_raw(frame.width, frame.height, frame.rgba).unwrap();
  assert_pixel(&rounded, 30, 30, [24, 36, 52, 255]);
  assert_pixel(&rounded, 74, 146, [52, 211, 153, 255]);
  tree.get_element_by_id_mut("canvas").unwrap().set_opacity(0.0);
  assert!(tree.pass(&mut app, &surface).rendered);
  let frame = receiver.recv_timeout(Duration::from_secs(15)).unwrap().unwrap();
  let hidden = image::RgbaImage::from_raw(frame.width, frame.height, frame.rgba).unwrap();
  assert_pixel(&hidden, 74, 146, [24, 36, 52, 255]);
  tree.get_element_by_id_mut("canvas").unwrap().set_opacity(1.0);
  assert!(tree.pass(&mut app, &surface).rendered);
  let frame = receiver.recv_timeout(Duration::from_secs(15)).unwrap().unwrap();
  let visible = image::RgbaImage::from_raw(frame.width, frame.height, frame.rgba).unwrap();
  assert_pixel(&visible, 74, 146, [52, 211, 153, 255]);
  // Exercise ordered readbacks while the canvas is culled by opacity. These
  // copies come from the GPU canvas, independently of window composition.
  let reference = lurq::core::ElementRef::new();
  tree.set_layout_constraints_override(Some(Constraints::loose(Size::new(1024.0, 600.0))));
  tree.set_root(
    lurq::components::Canvas::new()
      .ref_element(reference.clone())
      .width(1024.0)
      .height(600.0)
      .opacity(0.0),
  );
  assert!(tree.pass(&mut app, &surface).rendered);
  receiver.recv_timeout(Duration::from_secs(15)).unwrap().unwrap();
  let canvas = reference.as_canvas().unwrap();
  let d = canvas.context_2d();
  d.set_fill_style("#ff000080");
  d.fill_rect(0., 0., 1024., 600.);
  let red = canvas.snapshot();
  d.clear();
  d.set_fill_style("#0000ff");
  d.fill_rect(0., 0., 1024., 600.);
  let blue = canvas.snapshot();
  assert_eq!(
    canvas.snapshot().try_take().unwrap().unwrap_err(),
    lurq::canvas::CanvasError::QueueFull
  );
  assert!(tree.pass(&mut app, &surface).rendered);
  receiver.recv_timeout(Duration::from_secs(15)).unwrap().unwrap();
  let red = red.wait_timeout(Duration::from_secs(15)).unwrap().unwrap();
  let blue = blue.wait_timeout(Duration::from_secs(15)).unwrap().unwrap();
  assert_eq!(
    &red.rgba[((520 * red.width + 510) * 4) as usize..((520 * red.width + 510) * 4 + 4) as usize],
    &[255, 0, 0, 128]
  );
  assert_eq!(
    &blue.rgba[((520 * blue.width + 514) * 4) as usize..((520 * blue.width + 514) * 4 + 4) as usize],
    &[0, 0, 255, 255]
  );
  let before = canvas.status().gpu;
  d.set_fill_style("#ffffff");
  d.fill_rect(20., 20., 64., 64.);
  let white = canvas.snapshot();
  assert!(tree.pass(&mut app, &surface).rendered);
  receiver.recv_timeout(Duration::from_secs(15)).unwrap().unwrap();
  let white = white.wait_timeout(Duration::from_secs(15)).unwrap().unwrap();
  assert_eq!(
    &white.rgba[((30 * white.width + 30) * 4) as usize..((30 * white.width + 30) * 4 + 4) as usize],
    &[255; 4]
  );
  assert_eq!(canvas.status().gpu.tiles - before.tiles, 1);
  assert_eq!(canvas.status().gpu.uploaded_bytes - before.uploaded_bytes, 0);
  assert_eq!(canvas.status().gpu_bytes, 1024 * 600 * 4);
  assert_eq!(canvas.status().pending_bytes, 0);
  // A scale-only change resamples on GPU, including when no pixels are visible.
  tree.set_scale_factor(2.0);
  assert!(tree.pass(&mut app, &surface).rendered);
  receiver.recv_timeout(Duration::from_secs(15)).unwrap().unwrap();
  let scaled = canvas.snapshot();
  assert!(tree.pass(&mut app, &surface).rendered);
  receiver.recv_timeout(Duration::from_secs(15)).unwrap().unwrap();
  let scaled = scaled.wait_timeout(Duration::from_secs(15)).unwrap().unwrap();
  assert_eq!((scaled.width, scaled.height), (2048, 1200));
  assert_eq!(
    &scaled.rgba[((60 * scaled.width + 60) * 4) as usize..((60 * scaled.width + 60) * 4 + 4) as usize],
    &[255; 4]
  );
  tree.set_root(lurq::components::Rect::new(10., 10.));
  assert!(tree.pass(&mut app, &surface).rendered);
  receiver.recv_timeout(Duration::from_secs(15)).unwrap().unwrap();
  assert_eq!(canvas.status().gpu_bytes, 0);
  d.fill_rect(0., 0., 10., 10.);
  assert_eq!(canvas.status().error, Some(lurq::canvas::CanvasError::Detached));
  println!(
    "{backend_name} GPU canvas capture, tiled updates, ordered readback, culling, and resize passed: {}",
    output.display()
  );
}

#[cfg(windows)]
fn assert_pixel(png: &image::RgbaImage, x: u32, y: u32, expected: [u8; 4]) {
  let actual = png.get_pixel(x, y).0;
  assert!(
    actual.into_iter().zip(expected).all(|(a, e)| a.abs_diff(e) <= 2),
    "pixel ({x},{y}): expected {expected:?}, got {actual:?}"
  );
}

#[cfg(not(windows))]
fn main() {
  eprintln!("This capture harness uses a Win32 surface; the canvas example runs on other platforms.");
}
