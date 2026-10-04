//! The Canvas caches across replacement presentations. A replacement drawn in
//! bounded batches over several encodes is one cache frame. A pass that only
//! reprojects retained artwork ages nothing. Evicting every texture a completed
//! front was drawn from leaves the front and its artwork intact.
use std::time::Duration;

use super::{tests::device, *};
use crate::{
  canvas::test_pages::{CHARGE, IMAGE_BYTES, SHAPE_BUDGET, draw_cell, draw_label, images, labels, pixel, replacement},
  images::ImageData,
  node::transform::Transform2D,
};

/// One encode of whatever the canvas queued; returns the asset bytes uploaded.
fn encode(canvas: &CanvasHandle, renderer: &mut Renderer, gpu: &(Device, Queue)) -> u64 {
  let before = canvas.status().gpu.uploaded_bytes;
  renderer.process(&gpu.0, &gpu.1, std::slice::from_ref(canvas));
  gpu.0.poll(PollType::wait_indefinitely()).unwrap();
  canvas.status().gpu.uploaded_bytes - before
}

fn take(canvas: &CanvasHandle, renderer: &mut Renderer, gpu: &(Device, Queue)) -> CanvasSnapshot {
  let read = canvas.snapshot();
  encode(canvas, renderer, gpu);
  read.wait_timeout(Duration::from_secs(15)).unwrap().unwrap()
}

#[test]
#[ignore = "requires a GPU adapter; run explicitly and separately from context-creation tests"]
fn gpu_replacement_drawn_in_batches_is_one_cache_frame_and_artwork_passes_age_nothing() {
  let gpu = device();
  // Sixteen textures of budget and 32 of ceiling; the page needs 24.
  let mut renderer = Renderer::new(&gpu.0, &gpu.1).with_asset_budget(CanvasAssetBudget::new(16 * CHARGE));
  let canvas = CanvasHandle::test_surface(64, 64, 1., false);
  let page = images(24, 0);
  let draw_page = |renderer: &mut Renderer| {
    replacement(
      &canvas,
      page.len(),
      3,
      |d, at| draw_cell(d, &page[at], at),
      || encode(&canvas, renderer, &gpu),
    )
  };
  let uploads: Vec<_> = (0..3).map(|_| draw_page(&mut renderer)).collect();
  assert_eq!(canvas.status().error, None);
  assert_eq!(
    uploads,
    [24 * IMAGE_BYTES, 0, 0],
    "a later batch evicted textures an earlier batch of the same page drew"
  );
  assert_eq!(renderer.assets.bytes(), 24 * CHARGE, "the page stretches the cache");
  // A camera pass presents the retained artwork and uses no cached texture.
  let d = canvas.context_2d();
  for shift in 1..6 {
    let token = d.begin_presentation().unwrap();
    d.reset();
    d.draw_retained_artwork(Transform2D::translate(shift as f32, 0.))
      .unwrap();
    d.commit_presentation(token).unwrap();
    assert_eq!(encode(&canvas, &mut renderer, &gpu), 0);
  }
  assert_eq!(
    draw_page(&mut renderer),
    0,
    "artwork passes aged the page out of the cache"
  );
  assert_eq!(canvas.status().error, None);
}

#[test]
#[ignore = "requires a GPU adapter; run explicitly and separately from context-creation tests"]
fn gpu_labels_of_a_replacement_drawn_in_batches_are_shaped_once() {
  let gpu = device();
  let mut renderer = Renderer::new(&gpu.0, &gpu.1);
  let canvas = CanvasHandle::test_surface(512, 512, 1., false);
  let labels = labels();
  let mut draw_page = || {
    replacement(
      &canvas,
      labels.len(),
      3,
      |d, at| draw_label(d, &labels, at),
      || encode(&canvas, &mut renderer, &gpu),
    )
  };
  // No frame of this engine has ended while the first replacement is shaped,
  // so it is least recently used within the budget and the second shapes what
  // that evicted. From then on the page is one frame.
  let warm = [draw_page(), draw_page()];
  // Over the shape cache's budget, within its ceiling: one frame keeps the
  // page whole, while a frame per batch would evict its first labels.
  let shaped = canvas.shaped_text_bytes();
  assert!(
    (SHAPE_BUDGET + 1..=2 * SHAPE_BUDGET).contains(&shaped),
    "the page is charged {shaped} bytes"
  );
  let steady = [draw_page(), draw_page()];
  assert_eq!(canvas.status().error, None);
  assert!(warm[0] > 0);
  assert_eq!(steady, [0, 0], "labels of the previous replacement were shaped again");
}

#[test]
#[ignore = "requires a GPU adapter; run explicitly and separately from context-creation tests"]
fn gpu_retained_front_outlives_every_texture_it_was_drawn_from() {
  let gpu = device();
  // Two textures of budget and four of ceiling.
  let mut renderer = Renderer::new(&gpu.0, &gpu.1).with_asset_budget(CanvasAssetBudget::new(2 * CHARGE));
  let canvas = CanvasHandle::test_surface(64, 64, 1., false);
  let d = canvas.context_2d();
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
  assert_eq!(encode(&canvas, &mut renderer, &gpu), 2 * IMAGE_BYTES);
  let shown = take(&canvas, &mut renderer, &gpu);
  assert_eq!(pixel(&shown, 4, 4), [0, 0, 255, 255]);
  assert_eq!(pixel(&shown, 40, 4), [255, 0, 0, 255]);
  // Replacements that supersede one another end a frame each, so red and blue
  // age, are evicted and released, while the completed front stays shown.
  for salt in 1..5 {
    d.begin_presentation().unwrap();
    d.reset();
    for (at, image) in images(4, salt).iter().enumerate() {
      draw_cell(&d, image, at);
    }
    encode(&canvas, &mut renderer, &gpu);
  }
  assert!(renderer.assets.bytes() <= 4 * CHARGE);
  let pending = take(&canvas, &mut renderer, &gpu);
  assert_eq!(
    (&pending.rgba, pending.revision),
    (&shown.rgba, shown.revision),
    "the pending replacement must not change the completed front"
  );
  // The artwork captured with the front reprojects without either texture.
  let token = d.begin_presentation().unwrap();
  d.reset();
  d.draw_retained_artwork(Transform2D::translate(0., 0.)).unwrap();
  d.commit_presentation(token).unwrap();
  assert_eq!(encode(&canvas, &mut renderer, &gpu), 0);
  let reprojected = take(&canvas, &mut renderer, &gpu);
  for (x, y) in [(4, 4), (8, 60), (40, 4), (60, 60)] {
    assert_eq!(pixel(&reprojected, x, y), pixel(&shown, x, y));
  }
  // Red and blue had left the cache: drawing them again uploads them again.
  let token = d.begin_presentation().unwrap();
  draw_front();
  d.commit_presentation(token).unwrap();
  assert_eq!(encode(&canvas, &mut renderer, &gpu), 2 * IMAGE_BYTES);
  assert_eq!(canvas.status().error, None);
}
