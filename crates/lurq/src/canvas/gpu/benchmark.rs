//! CPU-only benchmark of the real recording and preparation paths.
use std::{hint::black_box, time::Instant};

use super::*;

#[test]
#[ignore = "release CPU benchmark; run with --ignored --nocapture"]
fn canvas_camera_prepare_benchmark() {
  let canvas = CanvasHandle::new();
  {
    let mut s = canvas.inner.lock();
    s.attached = true;
    s.metrics.pixel_width = 1395;
    s.metrics.pixel_height = 1253;
  }
  let d = canvas.context_2d();
  let paths: Vec<_> = (0..5000)
    .map(|i| {
      let mut p = Path2D::new();
      p.round_rect((i % 100) as f32 * 12., (i / 100) as f32 * 20., 10., 16., 3.)
        .unwrap();
      p
    })
    .collect();
  for mode in ["pan", "zoom", "zoom-cross-buckets"] {
    let mut cache = MeshCache::default();
    let mut recording = Vec::new();
    let mut preparing = Vec::new();
    let mut vertex_count = 0;
    for frame in 0..88 {
      let zoom = match mode {
        "pan" => 1.,
        "zoom" => 1.1 + (frame % 40) as f32 * 0.015,
        _ => 0.5 * 2_f32.powf((frame % 40) as f32 / 10.),
      };
      let start = Instant::now();
      d.clear();
      d.set_transform(
        Transform2D::translate(frame as f32 * 0.25, -(frame as f32) * 0.125).then(&Transform2D::scale_uniform(zoom)),
      );
      for path in &paths {
        d.fill_path(black_box(path), FillRule::NonZero);
      }
      let recorded = start.elapsed();
      let batch = canvas.take_batch().unwrap();
      let start = Instant::now();
      let prepared = Prepared::new(black_box(&batch.commands), &mut cache).unwrap();
      let elapsed = start.elapsed();
      vertex_count = black_box(prepared.vertices.len());
      batch.submit();
      if frame >= 8 {
        recording.push(recorded.as_secs_f64() * 1000.);
        preparing.push(elapsed.as_secs_f64() * 1000.);
      }
    }
    recording.sort_by(f64::total_cmp);
    preparing.sort_by(f64::total_cmp);
    eprintln!(
      "{mode}: paths={} record_median_ms={:.4} record_p95_ms={:.4} prepare_median_ms={:.4} prepare_p95_ms={:.4} last_vertices={vertex_count}",
      paths.len(),
      recording[40],
      recording[75],
      preparing[40],
      preparing[75]
    );
    eprintln!("{mode}: tessellations_including_warmup={}", cache.misses);
    assert!(canvas.status().error.is_none(), "{:?}", canvas.status());
  }
}
