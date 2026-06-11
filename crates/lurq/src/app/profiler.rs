use std::time::Duration;
#[cfg(feature = "perf_profile")]
use std::time::Instant;

#[derive(Clone, Default)]
pub struct FrameProfile {
  pub layout: Duration,
  pub layout_recalculated: bool,
  pub quad_resolve: Duration,
  pub glyph_rasterize: Duration,
  pub gpu_submit: Duration,
  pub render: RenderProfile,
  pub total: Duration,
  pub quad_count: usize,
  pub rect_count: usize,
  pub glyph_count: usize,
  pub glyph_cache_hits: usize,
  pub glyph_cache_misses: usize,
  pub text_measure_cache_hits: usize,
  pub text_measure_cache_misses: usize,
  pub memory: RuntimeMemoryProfile,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PerfMeterStats {
  pub fps: u32,
  pub total_ms: f32,
  pub layout_ms: f32,
  pub quad_resolve_ms: f32,
  pub glyph_ms: f32,
  pub render_acquire_ms: f32,
  pub render_upload_ms: f32,
  pub render_encode_ms: f32,
  pub render_submit_ms: f32,
  pub render_present_ms: f32,
  pub quad_count: usize,
  pub glyph_count: usize,
}

#[derive(Clone, Copy, Default)]
pub struct RenderProfile {
  pub init: Duration,
  pub acquire: Duration,
  pub globals_upload: Duration,
  pub atlas_upload: Duration,
  pub buffer_upload: Duration,
  pub image_upload: Duration,
  pub encode: Duration,
  pub submit: Duration,
  pub present: Duration,
  pub total: Duration,
}

impl FrameProfile {
  pub fn glyph_cache_hit_rate(&self) -> f32 {
    let total = self.glyph_cache_hits + self.glyph_cache_misses;
    if total == 0 {
      1.0
    } else {
      self.glyph_cache_hits as f32 / total as f32
    }
  }

  pub fn text_measure_hit_rate(&self) -> f32 {
    let total = self.text_measure_cache_hits + self.text_measure_cache_misses;
    if total == 0 {
      1.0
    } else {
      self.text_measure_cache_hits as f32 / total as f32
    }
  }
}

impl std::fmt::Display for FrameProfile {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(
      f,
      "total={:.2}ms layout={:.2}ms quads={:.2}ms glyphs={:.2}ms render={:.2}ms acquire={:.2}ms upload={:.2}ms encode={:.2}ms submit={:.2}ms present={:.2}ms | {} rects {} glyphs {} quads | measure hit={:.0}% glyph hit={:.0}% | {}",
      self.total.as_secs_f64() * 1000.0,
      self.layout.as_secs_f64() * 1000.0,
      self.quad_resolve.as_secs_f64() * 1000.0,
      self.glyph_rasterize.as_secs_f64() * 1000.0,
      self.gpu_submit.as_secs_f64() * 1000.0,
      self.render.acquire.as_secs_f64() * 1000.0,
      (self.render.globals_upload + self.render.atlas_upload + self.render.buffer_upload + self.render.image_upload)
        .as_secs_f64()
        * 1000.0,
      self.render.encode.as_secs_f64() * 1000.0,
      self.render.submit.as_secs_f64() * 1000.0,
      self.render.present.as_secs_f64() * 1000.0,
      self.rect_count,
      self.glyph_count,
      self.quad_count,
      self.text_measure_hit_rate() * 100.0,
      self.glyph_cache_hit_rate() * 100.0,
      self.memory,
    )
  }
}

#[derive(Clone, Copy, Default)]
pub struct RuntimeMemoryProfile {
  pub total_bytes: usize,
  pub runtime_struct_bytes: usize,
  pub root_tree_bytes: usize,
  pub root_context_bytes: usize,
  pub root_component_bytes: usize,
  pub last_layout_bytes: usize,
  pub glyph_engine_bytes: usize,
  pub render_engine_bytes: usize,
  pub hover_path_bytes: usize,
  pub active_path_bytes: usize,
  pub dragging_scroll_bytes: usize,
}

impl RuntimeMemoryProfile {
  pub fn total_kib(&self) -> f32 {
    self.total_bytes as f32 / 1024.0
  }
}

impl std::fmt::Display for RuntimeMemoryProfile {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(
      f,
      "runtime_mem={:.1}KiB root={:.1}KiB ctx={:.1}KiB layout={:.1}KiB glyph_engine={:.1}KiB",
      self.total_kib(),
      self.root_tree_bytes as f32 / 1024.0,
      self.root_context_bytes as f32 / 1024.0,
      self.last_layout_bytes as f32 / 1024.0,
      self.glyph_engine_bytes as f32 / 1024.0,
    )
  }
}

#[cfg_attr(not(feature = "perf_profile"), allow(dead_code))]
pub(crate) struct ProfileScope {
  #[cfg(feature = "perf_profile")]
  start: Instant,
}

impl ProfileScope {
  #[cfg_attr(not(feature = "perf_profile"), allow(dead_code))]
  pub(crate) fn start() -> Self {
    Self {
      #[cfg(feature = "perf_profile")]
      start: Instant::now(),
    }
  }

  #[cfg_attr(not(feature = "perf_profile"), allow(dead_code))]
  pub(crate) fn elapsed(&self) -> Duration {
    #[cfg(feature = "perf_profile")]
    {
      self.start.elapsed()
    }
    #[cfg(not(feature = "perf_profile"))]
    {
      Duration::default()
    }
  }
}

macro_rules! profile_scope {
  () => {{
    #[cfg(feature = "perf_profile")]
    {
      $crate::app::profiler::ProfileScope::start()
    }
    #[cfg(not(feature = "perf_profile"))]
    {
      ()
    }
  }};
}

macro_rules! profile_elapsed {
  ($scope:expr) => {{
    #[cfg(feature = "perf_profile")]
    {
      $scope.elapsed()
    }
    #[cfg(not(feature = "perf_profile"))]
    {
      let _ = &$scope;
      std::time::Duration::default()
    }
  }};
}

macro_rules! profile_if {
  ($($body:tt)*) => {
    #[cfg(feature = "perf_profile")]
    {
      $($body)*
    }
  };
}

macro_rules! profile_value {
  ($value:expr) => {{
    #[cfg(feature = "perf_profile")]
    {
      $value
    }
    #[cfg(not(feature = "perf_profile"))]
    {
      Default::default()
    }
  }};
}

pub(crate) use profile_elapsed;
pub(crate) use profile_if;
pub(crate) use profile_scope;
pub(crate) use profile_value;
