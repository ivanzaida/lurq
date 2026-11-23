//! Manual check of the graphics card a renderer draws on, and of the memory its device takes (PR #206).
//!
//! Lists the system's cards with their LUIDs, then draws to a hidden window for a few seconds with the
//! renderer and the card given, printing its process id so the GPU memory of the process can be read by card
//! (`\GPU Process Memory(pid_<pid>_luid_*)\Dedicated Usage`). What lurq says through `tracing` is printed.
//!
//! ```text
//! cargo run -p lurq --release --example graphics_card_probe --features dx12 -- <wgpu|dx12> <any|low|high|luid:LOW:HIGH> [usage|perf][-immediate] [seconds]
//! ```
//!
//! `graphics_card_probe.ps1` beside it runs it and reads the memory while it draws.

#[cfg(windows)]
use std::num::NonZeroIsize;

#[cfg(windows)]
use lurq::{
  app::{GraphicsCard, dx12_render::Dx12RenderEngine, render_engine::RenderEngine, wgpu_render::*},
  layout::{
    quad::ClipRect,
    render_list::{GlyphAtlas, RectCmd, RenderList},
  },
  node::color::Color,
};
#[cfg(windows)]
use raw_window_handle::{DisplayHandle, RawWindowHandle, Win32WindowHandle, WindowHandle};
#[cfg(windows)]
use windows::{
  Win32::{
    Foundation::{HWND, LPARAM, LRESULT, WPARAM},
    Graphics::Dxgi::{CreateDXGIFactory1, IDXGIFactory1},
    UI::WindowsAndMessaging::{CreateWindowExW, DefWindowProcW, RegisterClassW, WINDOW_EX_STYLE, WNDCLASSW, WS_POPUP},
  },
  core::w,
};

#[cfg(windows)]
extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
  unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
}

#[cfg(windows)]
struct Printer;

#[cfg(windows)]
impl tracing::Subscriber for Printer {
  fn enabled(&self, meta: &tracing::Metadata<'_>) -> bool {
    *meta.level() <= tracing::Level::INFO && !meta.target().starts_with("wgpu_hal")
  }
  fn new_span(&self, _: &tracing::span::Attributes<'_>) -> tracing::span::Id {
    tracing::span::Id::from_u64(1)
  }
  fn record(&self, _: &tracing::span::Id, _: &tracing::span::Record<'_>) {}
  fn record_follows_from(&self, _: &tracing::span::Id, _: &tracing::span::Id) {}
  fn event(&self, event: &tracing::Event<'_>) {
    struct Fields(String);
    impl tracing::field::Visit for Fields {
      fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        self.0.push_str(&format!(" {}={:?}", field.name(), value));
      }
    }
    let mut fields = Fields(String::new());
    event.record(&mut fields);
    println!(
      "[{} {}]{}",
      event.metadata().level(),
      event.metadata().target(),
      fields.0
    );
  }
  fn enter(&self, _: &tracing::span::Id) {}
  fn exit(&self, _: &tracing::span::Id) {}
}

#[cfg(windows)]
fn list_cards() {
  unsafe {
    let factory: IDXGIFactory1 = CreateDXGIFactory1().expect("DXGI factory");
    let mut index = 0;
    while let Ok(adapter) = factory.EnumAdapters1(index) {
      let desc = adapter.GetDesc1().expect("adapter description");
      let name = String::from_utf16_lossy(&desc.Description)
        .trim_end_matches('\0')
        .to_string();
      println!(
        "card {index}: {name} vendor=0x{:04x} device=0x{:04x} luid={}:{} (0x{:08x}_0x{:08x}) dedicated={} MB",
        desc.VendorId,
        desc.DeviceId,
        desc.AdapterLuid.LowPart,
        desc.AdapterLuid.HighPart,
        desc.AdapterLuid.HighPart as u32,
        desc.AdapterLuid.LowPart,
        desc.DedicatedVideoMemory / (1024 * 1024),
      );
      index += 1;
    }
  }
}

#[cfg(windows)]
fn parse_card(text: &str) -> GraphicsCard {
  match text {
    "any" => GraphicsCard::Any,
    "low" => GraphicsCard::LowPower,
    "high" => GraphicsCard::HighPerformance,
    other => {
      let mut parts = other
        .strip_prefix("luid:")
        .expect("card: any, low, high or luid:LOW:HIGH")
        .split(':');
      let low = parts.next().unwrap().parse().expect("LUID low part");
      let high = parts.next().unwrap().parse().expect("LUID high part");
      GraphicsCard::Luid(low, high)
    }
  }
}

#[cfg(windows)]
fn scene(frame: u32) -> RenderList {
  let x = (frame % 600) as f32;
  RenderList {
    clear_color: Color::new(30, 30, 40, 255),
    rects: vec![RectCmd {
      order: 0,
      x: 64.0 + x,
      y: 64.0,
      width: 256.0,
      height: 128.0,
      color: Color::new(40, 120, 200, 255),
      radii: [12.0; 4],
      stroke: [0.0; 4],
      stroke_color: Color::new(0, 0, 0, 0),
      stroke_style: lurq::layout::render_list::StrokeStyle::default(),
      transform: [1.0, 0.0, 0.0, 1.0],
      transform_origin: [0.0, 0.0],
      clip: ClipRect::default(),
      gradient: None,
      shadow: None,
    }],
    glyphs: Vec::new(),
    #[cfg(feature = "raster")]
    images: Vec::new(),
    layers: Vec::new(),
    backdrops: Vec::new(),
    glasses: Vec::new(),
    atlas: GlyphAtlas {
      data: std::sync::Arc::from([].as_slice()),
      width: 0,
      height: 0,
      version: 0,
      dirty_rects: std::sync::Arc::from([].as_slice()),
      dirty_from_version: 0,
    },
  }
}

#[cfg(windows)]
fn main() {
  tracing::subscriber::set_global_default(Printer).expect("subscriber");
  let args: Vec<String> = std::env::args().skip(1).collect();
  let renderer = args.first().map(String::as_str).unwrap_or("wgpu");
  let card = parse_card(args.get(1).map(String::as_str).unwrap_or("any"));
  let hints = args.get(2).map(String::as_str).unwrap_or("usage");
  let seconds: u64 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(8);

  println!("pid {}", std::process::id());
  list_cards();
  println!("renderer={renderer} card={card:?} hints={hints}");

  const WIDTH: u32 = 2160;
  const HEIGHT: u32 = 1350;
  let hwnd = unsafe {
    let class_name = w!("lurq_graphics_card_probe");
    let class = WNDCLASSW {
      lpfnWndProc: Some(wndproc),
      lpszClassName: class_name,
      ..Default::default()
    };
    assert_ne!(RegisterClassW(&class), 0, "failed to register window class");
    CreateWindowExW(
      WINDOW_EX_STYLE::default(),
      class_name,
      w!("lurq graphics card probe"),
      WS_POPUP,
      0,
      0,
      WIDTH as i32,
      HEIGHT as i32,
      None,
      None,
      None,
      None,
    )
    .expect("failed to create window")
  };
  let raw_handle = Win32WindowHandle::new(NonZeroIsize::new(hwnd.0 as isize).expect("null hwnd"));
  let window = unsafe { WindowHandle::borrow_raw(RawWindowHandle::Win32(raw_handle)) };
  let display = DisplayHandle::windows();

  let mut engine: Box<dyn RenderEngine> = match renderer {
    "dx12" => Box::new(Dx12RenderEngine::new().with_graphics_card(card)),
    _ => {
      let mut engine = WgpuRenderEngine::new().with_graphics_card(card);
      if hints.starts_with("perf") {
        engine = engine.with_memory_hints(WgpuMemoryHints::Performance);
      }
      if hints.ends_with("-immediate") {
        engine = engine.with_present_mode(WgpuPresentMode::Immediate);
      }
      Box::new(engine)
    }
  };
  engine.resize(WIDTH, HEIGHT);

  let start = std::time::Instant::now();
  let mut frame = 0u32;
  let mut slowest = std::time::Duration::ZERO;
  let mut total = std::time::Duration::ZERO;
  while start.elapsed().as_secs() < seconds {
    let began = std::time::Instant::now();
    assert!(engine.render(&scene(frame), window, display), "frame {frame} failed");
    let took = began.elapsed();
    if frame > 10 {
      slowest = slowest.max(took);
      total += took;
    }
    frame += 1;
  }
  let measured = frame.saturating_sub(11).max(1);
  println!(
    "frames={frame} mean={:.3} ms slowest={:.3} ms",
    total.as_secs_f64() * 1000.0 / measured as f64,
    slowest.as_secs_f64() * 1000.0,
  );
}

#[cfg(not(windows))]
fn main() {
  eprintln!("This check requires Windows");
}
