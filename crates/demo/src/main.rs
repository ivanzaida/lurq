mod animation_demo;
mod components_demo;
mod context_demo;
mod dnd_demo;
mod events_demo;
mod layout_demo;
mod positioning_demo;
mod reactivity_demo;
mod scroll_demo;
mod sidebar;
mod sizing_demo;
mod style;
mod text_demo;
mod transform_demo;
mod visual_demo;

use std::time::{Duration, Instant};

#[cfg(not(any(feature = "wgpu", all(feature = "dx12", target_os = "windows"))))]
compile_error!("demo requires feature `wgpu` or feature `dx12` on Windows");

#[cfg(all(feature = "dx12", target_os = "windows"))]
use lurq::app::dx12_render::Dx12RenderEngine;
#[cfg(feature = "wgpu")]
use lurq::app::wgpu_render::WgpuRenderEngine;
use lurq::{
  app::{Runtime, component::Component, ctx::Ctx, winit_shell::WinitWindow},
  components::Row,
  core::Signal,
  layout::{
    Alignment,
    layout_kind::Justify,
    scrollbar::{ScrollBarStyle, ScrollBarVisibility},
    text_style::FontWeight,
  },
  node::{Element, color::Color, dimension::Dimension},
};

use crate::{
  animation_demo::animation_content,
  dnd_demo::DndDemo,
  layout_demo::layout_content,
  positioning_demo::PositioningDemo,
  scroll_demo::scroll_content,
  sidebar::{DemoTab, sidebar},
  sizing_demo::sizing_content,
  style::{BORDER, DemoTheme, SURFACE_DARK, TEXT, text},
};

const SIDEBAR_WIDTH: f32 = 200.0;
const PERF_SAMPLE_INTERVAL: Duration = Duration::from_secs(1);

#[cfg(feature = "wgpu")]
const DEFAULT_RENDERER: &str = "wgpu";
#[cfg(all(not(feature = "wgpu"), feature = "dx12", target_os = "windows"))]
const DEFAULT_RENDERER: &str = "dx12";

#[derive(Clone, Copy, Default, PartialEq)]
struct PerfStats {
  fps: u32,
  total_ms: f32,
  layout_ms: f32,
  quad_resolve_ms: f32,
  glyph_ms: f32,
  render_acquire_ms: f32,
  render_upload_ms: f32,
  render_encode_ms: f32,
  render_submit_ms: f32,
  render_present_ms: f32,
  quad_count: usize,
  glyph_count: usize,
}

#[derive(Clone)]
struct DemoProps {
  perf: Signal<PerfStats>,
}

impl PartialEq for DemoProps {
  fn eq(&self, _other: &Self) -> bool {
    true
  }
}

struct DemoApp {
  selected_tab: Signal<DemoTab>,
  perf: Signal<PerfStats>,
  theme: Signal<DemoTheme>,
}

impl Component for DemoApp {
  type Props = DemoProps;

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      selected_tab: ctx.signal(DemoTab::Layout),
      perf: ctx.props::<DemoProps>().perf.clone(),
      theme: ctx.signal(DemoTheme::Dark),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let selected_tab = self.selected_tab.get();
    let theme = self.theme.get();
    let palette = theme.palette();
    let content = match selected_tab {
      DemoTab::Layout => layout_content(),
      DemoTab::Sizing => sizing_content(),
      DemoTab::Position => ctx.mount::<PositioningDemo>(()),
      DemoTab::Dnd => ctx.mount::<DndDemo>(()),
      DemoTab::Animation => animation_content(),
      DemoTab::Transform => transform_demo::transform_content(),
      DemoTab::Scroll => scroll_content(),
      DemoTab::Visual => visual_demo::visual_content(),
      DemoTab::Text => text_demo::text_content(),
      DemoTab::Events => ctx.mount::<events_demo::EventsDemo>(()),
      DemoTab::Reactivity => ctx.mount::<reactivity_demo::ReactivityDemo>(()),
      DemoTab::Components => ctx.mount::<components_demo::ComponentsDemo>(()),
      DemoTab::Context => ctx.mount::<context_demo::ContextDemo>(context_demo::ContextDemoProps {
        theme: self.theme.clone(),
      }),
    };

    let content = Row::new()
      .align_items(Alignment::Stretch)
      .child(
        lurq::components::ScrollVertical::new(sidebar(selected_tab, self.selected_tab.clone(), theme))
          .scrollbar(ScrollBarStyle {
            visible: ScrollBarVisibility::Auto,
            width: 6.0,
            thumb_color: Color::from_hex(palette.primary),
            thumb_radius: 4.0,
            ..ScrollBarStyle::default()
          })
          .scrollbar_hovered(move |style| style.with_thumb_color(Color::from_hex(palette.accent)))
          .width(SIDEBAR_WIDTH)
          .fill(palette.surface_dark),
      )
      .child(
        lurq::components::ScrollVertical::new(content)
          .scrollbar(ScrollBarStyle {
            visible: ScrollBarVisibility::Auto,
            width: 7.0,
            thumb_color: Color::from_hex(palette.primary),
            thumb_radius: 4.0,
            ..ScrollBarStyle::default()
          })
          .scrollbar_hovered(move |style| style.with_thumb_color(Color::from_hex(palette.accent)))
          .fill(palette.bg)
          .flex(1.0),
      )
      .fill(palette.bg);

    lurq::components::Stack::new()
      .child(content)
      .child(ctx.mount::<PerfOverlay>(PerfOverlayProps {
        perf: self.perf.clone(),
      }))
      .fill(palette.bg)
  }
}

#[derive(Clone)]
struct PerfOverlayProps {
  perf: Signal<PerfStats>,
}

impl PartialEq for PerfOverlayProps {
  fn eq(&self, _other: &Self) -> bool {
    true
  }
}

struct PerfOverlay {
  perf: Signal<PerfStats>,
}

impl Component for PerfOverlay {
  type Props = PerfOverlayProps;

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      perf: ctx.props::<PerfOverlayProps>().perf.clone(),
    }
  }

  fn render(&self, _ctx: &mut Ctx) -> impl Into<Element> {
    perf_overlay(self.perf.get())
  }
}

struct PerfMeter {
  perf: Signal<PerfStats>,
  last_sample: Instant,
  last_seen_frame: u64,
  frames_since_sample: u64,
}

impl PerfMeter {
  fn new(perf: Signal<PerfStats>) -> Self {
    Self {
      perf,
      last_sample: Instant::now(),
      last_seen_frame: 0,
      frames_since_sample: 0,
    }
  }

  fn tick(&mut self, runtime: &mut Runtime) {
    let frame_count = runtime.frame_count();
    if frame_count > self.last_seen_frame {
      self.frames_since_sample += frame_count - self.last_seen_frame;
      self.last_seen_frame = frame_count;
    }

    let now = Instant::now();
    let elapsed = now.duration_since(self.last_sample);
    if elapsed < PERF_SAMPLE_INTERVAL {
      return;
    }

    let profile = runtime.last_profile();
    let fps = (self.frames_since_sample as f32 / elapsed.as_secs_f32()).round() as u32;
    let stats = PerfStats {
      fps,
      total_ms: ms(profile.total),
      layout_ms: ms(profile.layout),
      quad_resolve_ms: ms(profile.quad_resolve),
      glyph_ms: ms(profile.glyph_rasterize),
      render_acquire_ms: ms(profile.render.acquire),
      render_upload_ms: ms(profile.render.globals_upload + profile.render.atlas_upload),
      render_encode_ms: ms(profile.render.encode),
      render_submit_ms: ms(profile.render.submit),
      render_present_ms: ms(profile.render.present),
      quad_count: profile.quad_count,
      glyph_count: profile.glyph_count,
    };
    if self.perf.get_untracked() != stats {
      self.perf.set(stats);
    }
    self.frames_since_sample = 0;
    self.last_sample = now;
  }
}

fn ms(duration: Duration) -> f32 {
  duration.as_secs_f32() * 1000.0
}

fn perf_overlay(stats: PerfStats) -> impl Into<Element> {
  lurq::components::Column::new()
    .child(lurq::components::Spacer::new().height(12.0))
    .child(
      Row::new()
        .justify(Justify::End)
        .align_items(Alignment::Start)
        .child(perf_widget(stats))
        .child(lurq::components::Spacer::new().width(16.0))
        .width(Dimension::Pct(100.0)),
    )
    .width(Dimension::Pct(100.0))
    .height(225.0)
    .overflow_visible()
}

fn perf_widget(stats: PerfStats) -> lurq::components::Column {
  lurq::components::Column::new()
    .spacing(2.0)
    .child(perf_row("FPS", stats.fps.to_string(), FontWeight::Bold))
    .child(perf_row(
      "total",
      format!("{:.2} ms", stats.total_ms),
      FontWeight::Normal,
    ))
    .child(perf_row(
      "layout",
      format!("{:.2} ms", stats.layout_ms),
      FontWeight::Normal,
    ))
    .child(perf_row(
      "resolve",
      format!("{:.2} ms", stats.quad_resolve_ms),
      FontWeight::Normal,
    ))
    .child(perf_row(
      "glyph",
      format!("{:.2} ms", stats.glyph_ms),
      FontWeight::Normal,
    ))
    .child(perf_row(
      "acquire",
      format!("{:.2} ms", stats.render_acquire_ms),
      FontWeight::Normal,
    ))
    .child(perf_row(
      "upload",
      format!("{:.2} ms", stats.render_upload_ms),
      FontWeight::Normal,
    ))
    .child(perf_row(
      "encode",
      format!("{:.2} ms", stats.render_encode_ms),
      FontWeight::Normal,
    ))
    .child(perf_row(
      "submit",
      format!("{:.2} ms", stats.render_submit_ms),
      FontWeight::Normal,
    ))
    .child(perf_row(
      "present",
      format!("{:.2} ms", stats.render_present_ms),
      FontWeight::Normal,
    ))
    .child(perf_row("quads", stats.quad_count.to_string(), FontWeight::Normal))
    .child(perf_row("glyphs", stats.glyph_count.to_string(), FontWeight::Normal))
    .pad_xy(8.0, 8.0)
    .size(160.0, 198.0)
    .fill(SURFACE_DARK)
    .border_inside(1.0, Color::from_hex(BORDER))
    .rounded(6.0)
}

fn perf_row(label: &str, value: String, value_weight: FontWeight) -> lurq::components::Row {
  Row::new()
    .align_items(Alignment::Center)
    .child(text(label, 10.0, FontWeight::Normal, TEXT).nowrap())
    .child(lurq::components::Spacer::new().flex(1.0))
    .child(text(&value, 10.0, value_weight, TEXT).nowrap())
    .width(Dimension::Pct(100.0))
}

fn set_selected_render_engine(runtime: &mut Runtime) -> &'static str {
  match selected_renderer_arg().as_str() {
    "wgpu" => {
      set_wgpu_render_engine(runtime);
      "wgpu"
    }
    "dx12" | "d3d12" => {
      set_dx12_render_engine(runtime);
      "dx12"
    }
    other => panic!("unknown renderer `{other}`; expected `wgpu` or `dx12`"),
  }
}

fn selected_renderer_arg() -> String {
  let mut args = std::env::args().skip(1);
  while let Some(arg) = args.next() {
    if arg == "--renderer" {
      return args
        .next()
        .unwrap_or_else(|| panic!("--renderer requires `wgpu` or `dx12`"))
        .to_ascii_lowercase();
    }

    if let Some(renderer) = arg.strip_prefix("--renderer=") {
      return renderer.to_ascii_lowercase();
    }
  }

  DEFAULT_RENDERER.to_owned()
}

#[cfg(feature = "wgpu")]
fn set_wgpu_render_engine(runtime: &mut Runtime) {
  runtime.set_render_engine(Box::new(WgpuRenderEngine::new()));
}

#[cfg(not(feature = "wgpu"))]
fn set_wgpu_render_engine(_runtime: &mut Runtime) {
  panic!("--renderer wgpu requires the demo `wgpu` feature");
}

#[cfg(all(feature = "dx12", target_os = "windows"))]
fn set_dx12_render_engine(runtime: &mut Runtime) {
  runtime.set_render_engine(Box::new(Dx12RenderEngine::new()));
}

#[cfg(not(all(feature = "dx12", target_os = "windows")))]
fn set_dx12_render_engine(_runtime: &mut Runtime) {
  panic!("--renderer dx12 requires the demo `dx12` feature on Windows");
}

fn main() {
  let mut runtime = Runtime::new();
  #[cfg(feature = "resources")]
  runtime.set_resource_root(std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets"));
  runtime.set_profiling_enabled(true);
  let renderer = set_selected_render_engine(&mut runtime);
  animation_demo::register_keyframes(&mut runtime);
  let perf = Signal::new(PerfStats::default());
  let mut perf_meter = PerfMeter::new(perf.clone());
  runtime.mount_root::<DemoApp>(DemoProps { perf });
  let title = format!("lurq demo ({renderer})");
  WinitWindow::new(runtime)
    .with_title(&title)
    .on_tick(move |rt: &mut Runtime| perf_meter.tick(rt))
    .run();
}
