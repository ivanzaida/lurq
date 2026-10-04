// The DX12 Canvas renderer's asset residency through the real engine, drawing
// into a hidden window of the test's own. A page charged more than the budget
// is uploaded once; textures past the ceiling are drawn and retired every
// frame; a replacement drawn in batches is one cache frame; and a completed
// front outlives the textures it was drawn from.
//
// cargo test -p lurq --lib --features canvas,dx12,screenshot dx12_canvas_residency -- --ignored --test-threads=1

use std::time::Duration;

use raw_window_handle::DisplayHandle;

use super::{
  dx12_render::Dx12RenderEngine,
  readback_window::{HiddenWindow, empty_list},
  render_engine::RenderEngine,
};
use crate::{
  canvas::{
    CanvasAssetBudget, CanvasHandle, CanvasSnapshot,
    test_pages::{CHARGE, IMAGE_BYTES, draw_cell, draw_label, images, labels, pixel, replacement},
  },
  images::ImageData,
  node::transform::Transform2D,
};

struct Host {
  engine: Dx12RenderEngine,
  window: HiddenWindow,
  canvas: CanvasHandle,
}

impl Host {
  fn new(size: u32, budget: CanvasAssetBudget) -> Self {
    let window = HiddenWindow::new(size, size);
    let mut engine = Dx12RenderEngine::new().with_canvas_asset_budget(budget);
    engine.resize(size, size);
    let canvas = CanvasHandle::test_surface(size, size, 1., false);
    Self { engine, window, canvas }
  }

  /// Renders one frame and returns the Canvas asset bytes it uploaded.
  fn frame(&mut self) -> u64 {
    let before = self.canvas.status().gpu.uploaded_bytes;
    self.engine.prepare_canvases(std::slice::from_ref(&self.canvas));
    assert!(
      self
        .engine
        .render(&empty_list(), self.window.window_handle(), DisplayHandle::windows())
    );
    self.canvas.status().gpu.uploaded_bytes - before
  }

  fn take(&mut self) -> CanvasSnapshot {
    let read = self.canvas.snapshot();
    self.frame();
    read.wait_timeout(Duration::from_secs(15)).unwrap().unwrap()
  }
}

#[test]
#[ignore = "requires a Windows desktop and a DX12 adapter; creates a hidden window"]
fn dx12_canvas_residency_keeps_every_asset_of_a_frame_larger_than_the_budget() {
  let mut host = Host::new(256, CanvasAssetBudget::default());
  // 1,100 textures charged 64 KiB each are 68.75 MiB, over the 64 MiB budget.
  let page: Vec<_> = (0..1100u32)
    .map(|index| ImageData::from_rgba(vec![(index % 251) as u8; 16 * 16 * 4], 16, 16))
    .collect();
  let uploads: Vec<_> = (0..4)
    .map(|_| {
      let d = host.canvas.context_2d();
      d.clear();
      for (index, image) in page.iter().enumerate() {
        let (x, y) = ((index % 32) as f32 * 8., (index / 32) as f32 * 7.);
        d.draw_image_scaled(image, x, y, 8., 7.).unwrap();
      }
      host.frame()
    })
    .collect();
  assert_eq!(host.canvas.status().error, None);
  assert_eq!(uploads, [1100 * IMAGE_BYTES, 0, 0, 0]);
}

#[test]
#[ignore = "requires a Windows desktop and a DX12 adapter; creates a hidden window"]
fn dx12_canvas_residency_draws_and_retires_what_the_ceiling_cannot_keep() {
  // Four textures of budget and eight of ceiling; the page draws twelve.
  let mut host = Host::new(64, CanvasAssetBudget::new(4 * CHARGE));
  let page = images(12, 7);
  let mut shown = None;
  let uploads: Vec<_> = (0..4)
    .map(|_| {
      let d = host.canvas.context_2d();
      d.clear();
      for (at, image) in page.iter().enumerate() {
        d.draw_image(image, (at % 4) as f32 * 16., (at / 4) as f32 * 16.)
          .unwrap();
      }
      let uploaded = host.frame();
      shown = Some(host.take());
      uploaded / IMAGE_BYTES
    })
    .collect();
  assert_eq!(host.canvas.status().error, None);
  assert_eq!(
    uploads,
    [12, 4, 4, 4],
    "the eight kept stay, the other four are uploaded each frame"
  );
  let shown = shown.unwrap();
  for at in 0..12u8 {
    let (x, y) = (u32::from(at % 4) * 16 + 8, u32::from(at / 4) * 16 + 8);
    assert_eq!(pixel(&shown, x, y), [at, 7, 255 - at, 255], "cell {at}");
  }
  assert_eq!(pixel(&shown, 8, 56), [0, 0, 0, 0], "the row below the page stays clear");
}

#[test]
#[ignore = "requires a Windows desktop and a DX12 adapter; creates a hidden window"]
fn dx12_canvas_residency_keeps_a_replacement_drawn_in_batches_as_one_frame() {
  {
    // Sixteen textures of budget and 32 of ceiling; the page needs 24.
    let mut host = Host::new(64, CanvasAssetBudget::new(16 * CHARGE));
    let canvas = host.canvas.clone();
    let page = images(24, 0);
    let uploads: Vec<_> = (0..3)
      .map(|_| {
        replacement(
          &canvas,
          page.len(),
          3,
          |d, at| draw_cell(d, &page[at], at),
          || host.frame(),
        )
      })
      .collect();
    assert_eq!(canvas.status().error, None);
    assert_eq!(uploads, [24 * IMAGE_BYTES, 0, 0]);
  }
  // Labels end their text frames with the same frames: over the shape cache's
  // budget, they are shaped and uploaded once.
  let mut host = Host::new(512, CanvasAssetBudget::default());
  let canvas = host.canvas.clone();
  let labels = labels();
  let uploads: Vec<_> = (0..4)
    .map(|_| {
      replacement(
        &canvas,
        labels.len(),
        3,
        |d, at| draw_label(d, &labels, at),
        || host.frame(),
      )
    })
    .collect();
  assert_eq!(canvas.status().error, None);
  assert!(uploads[0] > 0);
  // The first replacement is shaped before any frame of the engine ended (see
  // the WGPU sibling), so the steady state starts with the third.
  assert_eq!(
    uploads[2..],
    [0, 0],
    "labels of the previous replacement were shaped again"
  );
}

#[test]
#[ignore = "requires a Windows desktop and a DX12 adapter; creates a hidden window"]
fn dx12_canvas_residency_keeps_a_retained_front_past_its_textures() {
  // Two textures of budget and four of ceiling.
  let mut host = Host::new(64, CanvasAssetBudget::new(2 * CHARGE));
  let d = host.canvas.context_2d();
  let red = ImageData::from_rgba([255, 0, 0, 255].repeat(16 * 16), 16, 16);
  let blue = ImageData::from_rgba([0, 0, 255, 255].repeat(16 * 16), 16, 16);
  let draw_front = || {
    d.reset();
    d.draw_image_scaled(&red, 0., 0., 64., 64.).unwrap();
    d.draw_image_scaled(&blue, 0., 0., 16., 64.).unwrap();
  };
  let front = d.begin_presentation().unwrap();
  draw_front();
  d.capture_artwork().unwrap();
  d.commit_presentation(front).unwrap();
  assert_eq!(host.frame(), 2 * IMAGE_BYTES);
  let shown = host.take();
  assert_eq!(pixel(&shown, 4, 4), [0, 0, 255, 255]);
  assert_eq!(pixel(&shown, 40, 4), [255, 0, 0, 255]);
  // Superseding replacements end a frame each: red and blue age, are evicted
  // and retired, and the frames that could still read them complete.
  for salt in 1..6 {
    d.begin_presentation().unwrap();
    d.reset();
    for (at, image) in images(4, salt).iter().enumerate() {
      draw_cell(&d, image, at);
    }
    host.frame();
  }
  let pending = host.take();
  assert_eq!((&pending.rgba, pending.revision), (&shown.rgba, shown.revision));
  let token = d.begin_presentation().unwrap();
  d.reset();
  d.draw_retained_artwork(Transform2D::translate(0., 0.)).unwrap();
  d.commit_presentation(token).unwrap();
  assert_eq!(host.frame(), 0);
  let reprojected = host.take();
  for (x, y) in [(4, 4), (8, 60), (40, 4), (60, 60)] {
    assert_eq!(pixel(&reprojected, x, y), pixel(&shown, x, y));
  }
  let token = d.begin_presentation().unwrap();
  draw_front();
  d.commit_presentation(token).unwrap();
  assert_eq!(host.frame(), 2 * IMAGE_BYTES, "red and blue had left the cache");
  assert_eq!(host.canvas.status().error, None);
}
