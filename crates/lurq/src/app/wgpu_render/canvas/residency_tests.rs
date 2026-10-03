//! A page drawn the same way every frame uploads its assets once, even when
//! the frame's own assets are charged more than the renderer's asset budget.
use super::{tests::device, *};
use crate::{
  canvas::{CanvasFont, Context2D},
  images::ImageData,
};

/// Redraws the whole canvas and returns the asset bytes that frame uploaded.
fn frame(canvas: &CanvasHandle, renderer: &mut Renderer, gpu: &(Device, Queue), draw: impl Fn(&Context2D)) -> u64 {
  let before = canvas.status().gpu.uploaded_bytes;
  let d = canvas.context_2d();
  d.clear();
  draw(&d);
  renderer.process(&gpu.0, &gpu.1, &[canvas.clone()]);
  gpu.0.poll(PollType::wait_indefinitely()).unwrap();
  canvas.status().gpu.uploaded_bytes - before
}

#[test]
#[ignore = "requires a GPU adapter; run explicitly and separately from context-creation tests"]
fn gpu_canvas_keeps_every_asset_of_a_frame_larger_than_the_budget() {
  let gpu = device();
  let mut renderer = Renderer::new(&gpu.0, &gpu.1);
  let canvas = CanvasHandle::test_surface(256, 256, 1., false);
  // Every texture is charged at least 64 KiB, so 1,100 small images are charged
  // 68.75 MiB, over the 64 MiB default budget, while the frame queues 1.1 MB.
  let images: Vec<_> = (0..1100u32)
    .map(|index| ImageData::from_rgba(vec![(index % 251) as u8; 16 * 16 * 4], 16, 16))
    .collect();
  let uploads: Vec<_> = (0..4)
    .map(|_| {
      frame(&canvas, &mut renderer, &gpu, |d| {
        for (index, image) in images.iter().enumerate() {
          let (x, y) = ((index % 32) as f32 * 8., (index / 32) as f32 * 7.);
          d.draw_image_scaled(image, x, y, 8., 7.).unwrap();
        }
      })
    })
    .collect();
  assert_eq!(canvas.status().error, None);
  assert_eq!(uploads[0], 1100 * 16 * 16 * 4);
  assert_eq!(uploads[1..], [0, 0, 0], "images of the previous frame were uploaded again");
}

#[test]
#[ignore = "requires a GPU adapter; run explicitly and separately from context-creation tests"]
fn gpu_canvas_uploads_a_page_of_labels_once() {
  let gpu = device();
  let mut renderer = Renderer::new(&gpu.0, &gpu.1);
  let canvas = CanvasHandle::test_surface(512, 512, 1., false);
  // More labels than the shape cache once held entries: each label shaped again
  // was a new asset, so every frame uploaded every label.
  let labels: Vec<_> = (0..400).map(|index| format!("Label {index}")).collect();
  let uploads: Vec<_> = (0..4)
    .map(|_| {
      frame(&canvas, &mut renderer, &gpu, |d| {
        d.set_font(CanvasFont::new("sans-serif", 12.));
        d.set_fill_style("#e5e7eb");
        for (index, label) in labels.iter().enumerate() {
          let (x, y) = ((index % 8) as f32 * 64., 12. + (index / 8) as f32 * 10.);
          d.fill_text(label, x, y).unwrap();
        }
      })
    })
    .collect();
  assert_eq!(canvas.status().error, None);
  assert!(uploads[0] > 0);
  assert_eq!(uploads[1..], [0, 0, 0], "labels of the previous frame were uploaded again");
}
