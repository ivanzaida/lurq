//! A stream of frames written into one image as fast as a producer thread makes them, to see how many the window
//! shows: what was the demo's `--continuous-video-probe`. A bar crosses the picture each way, a step a frame, and
//! every second the producer and the window each print what they did: frames made, paints, frames painted, versions
//! seen once, twice or skipped, and the longest gap between two paints.
//!
//! cargo run -p lurq --example video_probe -- --fps=120 --width=1280 --height=720

// `components::Image` is deprecated for apps in favour of `ui::Picture`; this is the engine's image node, which
// `ui::Picture` draws with, fed a streaming image.
#![allow(deprecated)]

use std::{
  sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicU64, Ordering},
  },
  thread,
  time::{Duration, Instant},
};

use lurq::{
  app::{App, Tree, wgpu_render::WgpuRenderEngine, winit_shell::WinitWindow},
  components::{Image, Stack},
  images::ImageData,
  node::dimension::Dimension,
};

#[derive(Clone, Copy)]
struct ContinuousVideoProbeOptions {
  width: u32,
  height: u32,
  fps: u32,
}

struct ContinuousVideoProbeStats {
  started_at: Instant,
  paints: u32,
  rendered: u32,
  version_changes: u32,
  repeated_versions: u32,
  skipped_versions: u64,
  max_paint_delta: Duration,
  last_version: Option<u64>,
}

impl ContinuousVideoProbeStats {
  fn new(now: Instant) -> Self {
    Self {
      started_at: now,
      paints: 0,
      rendered: 0,
      version_changes: 0,
      repeated_versions: 0,
      skipped_versions: 0,
      max_paint_delta: Duration::ZERO,
      last_version: None,
    }
  }
}

fn run_continuous_video_probe(probe: ContinuousVideoProbeOptions) {
  let app = App::new();
  let mut tree = Tree::new();
  tree.set_render_engine_factory(|| Box::new(WgpuRenderEngine::new()));
  let renderer = "wgpu";
  let image = ImageData::streaming_rgba(
    vec![0; (probe.width * probe.height * 4) as usize],
    probe.width,
    probe.height,
  );
  let produced = Arc::new(AtomicU64::new(0));
  let stop = Arc::new(AtomicBool::new(false));
  start_continuous_video_probe_producer(image.clone(), probe, produced.clone(), stop.clone());

  tree.set_root(
    Stack::new()
      .child(
        Image::new(image.clone())
          .width(Dimension::Pct(100.0))
          .height(Dimension::Pct(100.0)),
      )
      .width(Dimension::Pct(100.0))
      .height(Dimension::Pct(100.0))
      .background("#0f172a"),
  );

  let stats = Arc::new(Mutex::new(ContinuousVideoProbeStats::new(Instant::now())));
  let stats_for_paint = stats.clone();
  let image_for_paint = image.clone();
  let produced_for_paint = produced.clone();
  let title = format!(
    "lurq continuous video probe ({renderer}) {}x{} @ {}fps",
    probe.width, probe.height, probe.fps
  );
  eprintln!(
    "continuous video probe start renderer={} size={}x{} target_fps={}",
    renderer, probe.width, probe.height, probe.fps
  );
  WinitWindow::new(app, tree)
    .with_title(&title)
    .with_size(probe.width.min(1280), probe.height.min(720))
    .on_paint(move |_tree, delta, report| {
      let now = Instant::now();
      let version = image_for_paint.version();
      let mut stats = stats_for_paint.lock().expect("continuous video probe stats lock");
      stats.paints += 1;
      stats.rendered += u32::from(report.rendered);
      stats.max_paint_delta = stats.max_paint_delta.max(delta);
      match stats.last_version {
        Some(last_version) if version == last_version => {
          stats.repeated_versions += 1;
        }
        Some(last_version) => {
          stats.version_changes += 1;
          stats.skipped_versions += version.saturating_sub(last_version).saturating_sub(1);
        }
        None => {
          stats.version_changes += 1;
        }
      }
      stats.last_version = Some(version);
      if now.duration_since(stats.started_at) < Duration::from_secs(1) {
        return;
      }
      eprintln!(
        "continuous video probe render produced={} paints={} rendered={} version_changes={} repeated_versions={} skipped_versions={} max_paint_delta_ms={:.2} last_version={}",
        produced_for_paint.load(Ordering::Relaxed),
        stats.paints,
        stats.rendered,
        stats.version_changes,
        stats.repeated_versions,
        stats.skipped_versions,
        stats.max_paint_delta.as_secs_f64() * 1000.0,
        version
      );
      *stats = ContinuousVideoProbeStats::new(now);
      stats.last_version = Some(version);
    })
    .run();
  stop.store(true, Ordering::Relaxed);
}

fn start_continuous_video_probe_producer(
  image: ImageData,
  probe: ContinuousVideoProbeOptions,
  produced: Arc<AtomicU64>,
  stop: Arc<AtomicBool>,
) {
  thread::spawn(move || {
    let interval = Duration::from_secs_f64(1.0 / probe.fps.max(1) as f64);
    let mut next_frame_at = Instant::now();
    let mut last_frame_at = None;
    let mut stats_started_at = Instant::now();
    let mut frames = 0_u32;
    let mut max_frame_delta = Duration::ZERO;
    while !stop.load(Ordering::Relaxed) {
      let now = Instant::now();
      if now < next_frame_at {
        thread::sleep((next_frame_at - now).min(Duration::from_millis(1)));
        continue;
      }
      if let Some(last) = last_frame_at {
        max_frame_delta = max_frame_delta.max(now.duration_since(last));
      }
      last_frame_at = Some(now);
      let frame = produced.fetch_add(1, Ordering::Relaxed) + 1;
      image.update_streaming_rgba(|pixels| {
        fill_continuous_video_probe_frame(pixels, probe.width, probe.height, frame);
      });
      frames += 1;
      next_frame_at += interval;
      if now.duration_since(stats_started_at) >= Duration::from_secs(1) {
        eprintln!(
          "continuous video probe producer target_fps={} produced={} max_frame_delta_ms={:.2}",
          probe.fps,
          frames,
          max_frame_delta.as_secs_f64() * 1000.0
        );
        frames = 0;
        max_frame_delta = Duration::ZERO;
        stats_started_at = now;
      }
    }
  });
}

fn fill_continuous_video_probe_frame(pixels: &mut [u8], width: u32, height: u32, frame: u64) {
  let width = width as usize;
  let height = height as usize;
  if width == 0 || height == 0 {
    return;
  }
  let expected_len = width.saturating_mul(height).saturating_mul(4);
  if pixels.len() < expected_len {
    return;
  }

  let marker_width = width.min(72);
  let marker_height = height.min(72);
  let background = [8, 12, 24, 255];
  let previous_x = ((frame.saturating_sub(1) as usize * 11) % width).min(width - 1);
  let current_x = ((frame as usize * 11) % width).min(width - 1);
  let previous_y = ((frame.saturating_sub(1) as usize * 7) % height).min(height - 1);
  let current_y = ((frame as usize * 7) % height).min(height - 1);

  fill_wrapped_rect(pixels, width, height, previous_x, 0, marker_width, height, background);
  fill_wrapped_rect(pixels, width, height, 0, previous_y, width, marker_height, background);

  let vertical_color = [
    frame.wrapping_mul(17) as u8,
    frame.wrapping_mul(31).wrapping_add(80) as u8,
    245,
    255,
  ];
  let horizontal_color = [
    245,
    frame.wrapping_mul(23).wrapping_add(40) as u8,
    frame.wrapping_mul(13) as u8,
    255,
  ];
  fill_wrapped_rect(
    pixels,
    width,
    height,
    current_x,
    0,
    marker_width,
    height,
    vertical_color,
  );
  fill_wrapped_rect(
    pixels,
    width,
    height,
    0,
    current_y,
    width,
    marker_height,
    horizontal_color,
  );
}

fn fill_wrapped_rect(
  pixels: &mut [u8],
  width: usize,
  height: usize,
  x: usize,
  y: usize,
  rect_width: usize,
  rect_height: usize,
  color: [u8; 4],
) {
  for row in 0..rect_height {
    let py = (y + row) % height;
    for col in 0..rect_width {
      let px = (x + col) % width;
      let i = (py * width + px) * 4;
      pixels[i..i + 4].copy_from_slice(&color);
    }
  }
}

fn main() {
  let mut probe = ContinuousVideoProbeOptions {
    width: 1280,
    height: 720,
    fps: 120,
  };
  for arg in std::env::args().skip(1) {
    let number =
      |value: &str, name: &str| -> u32 { value.parse().unwrap_or_else(|_| panic!("{name} must be a number")) };
    if let Some(fps) = arg.strip_prefix("--fps=") {
      probe.fps = number(fps, "--fps");
    } else if let Some(width) = arg.strip_prefix("--width=") {
      probe.width = number(width, "--width");
    } else if let Some(height) = arg.strip_prefix("--height=") {
      probe.height = number(height, "--height");
    }
  }
  run_continuous_video_probe(probe);
}
