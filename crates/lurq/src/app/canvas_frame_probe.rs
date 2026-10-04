// A bounded page drawn frame after frame through the real DX12 and WGPU engines
// in a hidden window of the probe's own, reporting the per-frame Canvas counters
// a frame-time harness reads: text shape misses, textures created and evicted,
// and asset-upload time. It measures; it asserts only that nothing failed.
//
// cargo test -p lurq --release --features canvas,perf_profile,dx12,wgpu,screenshot --lib canvas_frame_probe --
// --ignored --nocapture --test-threads=1

use std::{sync::Arc, time::Duration};

use raw_window_handle::DisplayHandle;

use super::{
  PassReport,
  profile_types::FrameProfile,
  profiler::{PassSample, SampleData, producer::WindowProfiler},
  readback_window::HiddenWindow,
  render_engine::RenderEngine,
};
use crate::{
  canvas::{CanvasFont, CanvasHandle, Context2D},
  images::ImageData,
  layout::render_list::{GlyphAtlas, RenderList},
  node::color::Color,
};

const WIDTH: u32 = 1600;
const HEIGHT: u32 = 1000;
const FRAMES: usize = 24;
/// Frames before the steady state is summarized.
const WARMUP: usize = 4;

#[derive(Clone, Copy, Debug)]
enum Page {
  /// 344 labels, each measured three times and filled once: 688 distinct
  /// shape keys per frame, like the 684 of the real document's page at Fit.
  Labels,
  /// 1,100 distinct small images, charged 68.75 MiB: over the 64 MiB budget.
  Images,
}

fn draw(d: &Context2D, page: Page, images: &[ImageData]) {
  d.clear();
  match page {
    Page::Labels => {
      d.set_font(CanvasFont::new("sans-serif", 11.));
      d.set_fill_style("#d4d4d8");
      for index in 0..344 {
        let label = format!("Layer {index} / Frame");
        for _ in 0..3 {
          d.measure_text(&label).unwrap();
        }
        let (x, y) = ((index % 8) as f32 * 130., 14. + (index / 8) as f32 * 15.);
        d.fill_text(&label, x, y).unwrap();
      }
    }
    Page::Images => {
      for (index, image) in images.iter().enumerate() {
        let (x, y) = ((index % 40) as f32 * 20., (index / 40) as f32 * 20.);
        d.draw_image_scaled(image, x, y, 16., 16.).unwrap();
      }
    }
  }
}

fn empty_list() -> RenderList {
  RenderList {
    clear_color: Color::new(24, 24, 27, 255),
    rects: Vec::new(),
    glyphs: Vec::new(),
    images: Vec::new(),
    #[cfg(feature = "svg")]
    svgs: Vec::new(),
    layers: Vec::new(),
    atlas: GlyphAtlas {
      data: Arc::from([0_u8; 4].as_slice()),
      width: 1,
      height: 1,
      version: 1,
      dirty_rects: Arc::from(Vec::new()),
      dirty_from_version: 0,
    },
  }
}

fn ms(duration: Duration) -> f64 {
  duration.as_secs_f64() * 1000.
}

fn median(mut values: Vec<f64>) -> f64 {
  values.sort_by(f64::total_cmp);
  values[values.len() / 2]
}

fn run(engine: &mut dyn RenderEngine, backend: &'static str, page: Page) {
  let window = HiddenWindow::new(WIDTH, HEIGHT);
  engine.resize(WIDTH, HEIGHT);
  let canvas = CanvasHandle::test_surface(WIDTH, HEIGHT, 1.5, false);
  let images: Vec<_> = (0..1100u32)
    .map(|index| ImageData::from_rgba(vec![(index % 251) as u8; 16 * 16 * 4], 16, 16))
    .collect();
  let mut producer = WindowProfiler::new();
  let handle = producer.handle();
  let id = handle.start(Default::default()).unwrap().id;
  engine.set_profile_context(producer.context.clone());
  let list = empty_list();
  for frame in 0..FRAMES {
    let (started, phase) = producer.begin_pass(frame as u64 + 1);
    let scope = producer.context.canvas_text_scope();
    draw(&canvas.context_2d(), page, &images);
    engine.prepare_canvases(std::slice::from_ref(&canvas));
    assert!(engine.render(&list, window.window_handle(), DisplayHandle::windows()));
    let profile = FrameProfile {
      render: engine.last_profile().expect("an instrumented backend"),
      render_profile_available: true,
      ..Default::default()
    };
    let report = PassReport {
      required: true,
      rendered: true,
      ..Default::default()
    };
    producer.finish_pass(started, &report, backend, Some(&profile));
    drop(scope);
    drop(phase);
  }
  let report = handle.end(id).unwrap();
  assert_eq!(canvas.status().error, None);
  let passes: Vec<&PassSample> = report
    .samples
    .iter()
    .filter_map(|sample| match &sample.data {
      SampleData::Pass(pass) => Some(pass),
      _ => None,
    })
    .collect();
  assert_eq!(passes.len(), FRAMES);
  let row = |pass: &PassSample| {
    let text = pass.canvas_text.unwrap_or_default();
    let canvas = pass.frame.as_ref().unwrap().render.canvas;
    let details = canvas.asset_upload_details.unwrap_or_default();
    [
      text.shape_cache_misses as f64,
      text.shape_cache_evictions as f64,
      ms(text.total),
      details.texture_creations as f64,
      details.cache_evictions as f64,
      ms(canvas.asset_upload),
      canvas.uploaded_asset_bytes as f64 / 1024.,
      ms(canvas.total),
      canvas.asset_cache_stretch_bytes as f64 / 1024.,
      canvas.asset_cache_uncached as f64,
      text.shape_cache_stretch_bytes as f64 / 1024.,
      text.shape_cache_uncached as f64,
    ]
  };
  let first = row(passes[0]);
  let steady: Vec<_> = (0..12)
    .map(|column| median(passes[WARMUP..].iter().map(|pass| row(pass)[column]).collect()))
    .collect();
  let names = [
    "shape_misses",
    "shape_evictions",
    "text_ms",
    "textures_created",
    "texture_evictions",
    "asset_upload_ms",
    "uploaded_kib",
    "canvas_total_ms",
    "asset_stretch_kib",
    "assets_uncached",
    "shape_stretch_kib",
    "shapes_uncached",
  ];
  for ((name, first), steady) in names.iter().zip(first).zip(steady) {
    eprintln!("probe backend={backend} page={page:?} {name}: first={first:.2} steady_median={steady:.2}");
  }
}

#[test]
#[ignore = "release probe; requires a Windows desktop and a DX12 adapter; creates a hidden window"]
fn canvas_frame_probe_dx12() {
  for page in [Page::Labels, Page::Images] {
    run(&mut crate::app::dx12_render::Dx12RenderEngine::new(), "dx12", page);
  }
}

#[test]
#[ignore = "release probe; requires a Windows desktop and a GPU adapter; creates a hidden window"]
fn canvas_frame_probe_wgpu() {
  for page in [Page::Labels, Page::Images] {
    run(&mut crate::app::wgpu_render::WgpuRenderEngine::new(), "wgpu", page);
  }
}
