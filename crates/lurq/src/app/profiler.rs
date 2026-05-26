use std::time::{Duration, Instant};

#[derive(Clone, Default)]
pub struct FrameProfile {
  pub layout: Duration,
  pub quad_resolve: Duration,
  pub glyph_rasterize: Duration,
  pub gpu_submit: Duration,
  pub total: Duration,
  pub quad_count: usize,
  pub rect_count: usize,
  pub glyph_count: usize,
  pub glyph_cache_hits: usize,
  pub glyph_cache_misses: usize,
  pub text_measure_cache_hits: usize,
  pub text_measure_cache_misses: usize,
}

impl FrameProfile {
  pub fn glyph_cache_hit_rate(&self) -> f32 {
    let total = self.glyph_cache_hits + self.glyph_cache_misses;
    if total == 0 { 1.0 } else { self.glyph_cache_hits as f32 / total as f32 }
  }

  pub fn text_measure_hit_rate(&self) -> f32 {
    let total = self.text_measure_cache_hits + self.text_measure_cache_misses;
    if total == 0 { 1.0 } else { self.text_measure_cache_hits as f32 / total as f32 }
  }
}

impl std::fmt::Display for FrameProfile {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(
      f,
      "total={:.2}ms layout={:.2}ms quads={:.2}ms glyphs={:.2}ms gpu={:.2}ms | {} rects {} glyphs {} quads | measure hit={:.0}% glyph hit={:.0}%",
      self.total.as_secs_f64() * 1000.0,
      self.layout.as_secs_f64() * 1000.0,
      self.quad_resolve.as_secs_f64() * 1000.0,
      self.glyph_rasterize.as_secs_f64() * 1000.0,
      self.gpu_submit.as_secs_f64() * 1000.0,
      self.rect_count,
      self.glyph_count,
      self.quad_count,
      self.text_measure_hit_rate() * 100.0,
      self.glyph_cache_hit_rate() * 100.0,
    )
  }
}

pub(crate) struct ProfileScope {
  start: Instant,
}

impl ProfileScope {
  pub(crate) fn start() -> Self {
    Self { start: Instant::now() }
  }

  pub(crate) fn elapsed(&self) -> Duration {
    self.start.elapsed()
  }
}
