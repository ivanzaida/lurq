#![cfg_attr(not(feature = "perf_profile"), allow(dead_code))]

use std::time::Duration;

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
  pub glyph_engine: GlyphEngineProfile,
  pub memory: RuntimeMemoryProfile,
}

#[derive(Clone, Copy, Default)]
pub struct GlyphEngineProfile {
  pub shape_text: Duration,
  pub shape_rich_text: Duration,
  pub pack_rich_shaped: Duration,
  pub swash_lookup: Duration,
  pub atlas_pack: Duration,
  pub append_cached: Duration,
  pub swash_requests: usize,
  pub atlas_packs: usize,
}

#[derive(Clone, Copy, Default)]
pub struct RenderProfile {
  pub init: Duration,
  pub acquire: Duration,
  pub globals_upload: Duration,
  pub atlas_upload: Duration,
  pub glyph_atlas_upload_bytes: usize,
  pub glyph_atlas_upload_rects: usize,
  pub glyph_atlas_full_uploads: usize,
  pub buffer_upload: Duration,
  pub image_upload: Duration,
  pub encode: Duration,
  pub submit: Duration,
  pub present: Duration,
  pub total: Duration,
}

impl RenderProfile {
  pub fn upload_total(self) -> Duration {
    self.globals_upload + self.atlas_upload + self.buffer_upload + self.image_upload
  }

  pub fn active_total(self) -> Duration {
    self.total.saturating_sub(self.acquire)
  }
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
      "total={:.2}ms layout={:.2}ms quads={:.2}ms glyphs={:.2}ms render_cpu={:.2}ms wait={:.2}ms upload={:.2}ms encode={:.2}ms submit={:.2}ms present={:.2}ms | atlas={}B {} rects {} full | {} rects {} glyphs {} quads | text shape={:.2}ms rich_shape={:.2}ms rich_pack={:.2}ms swash={:.2}ms/{} atlas_pack={:.2}ms/{} append={:.2}ms | measure hit={:.0}% glyph hit={:.0}% | {}",
      self.total.as_secs_f64() * 1000.0,
      self.layout.as_secs_f64() * 1000.0,
      self.quad_resolve.as_secs_f64() * 1000.0,
      self.glyph_rasterize.as_secs_f64() * 1000.0,
      self.render.active_total().as_secs_f64() * 1000.0,
      self.render.acquire.as_secs_f64() * 1000.0,
      self.render.upload_total().as_secs_f64() * 1000.0,
      self.render.encode.as_secs_f64() * 1000.0,
      self.render.submit.as_secs_f64() * 1000.0,
      self.render.present.as_secs_f64() * 1000.0,
      self.render.glyph_atlas_upload_bytes,
      self.render.glyph_atlas_upload_rects,
      self.render.glyph_atlas_full_uploads,
      self.rect_count,
      self.glyph_count,
      self.quad_count,
      self.glyph_engine.shape_text.as_secs_f64() * 1000.0,
      self.glyph_engine.shape_rich_text.as_secs_f64() * 1000.0,
      self.glyph_engine.pack_rich_shaped.as_secs_f64() * 1000.0,
      self.glyph_engine.swash_lookup.as_secs_f64() * 1000.0,
      self.glyph_engine.swash_requests,
      self.glyph_engine.atlas_pack.as_secs_f64() * 1000.0,
      self.glyph_engine.atlas_packs,
      self.glyph_engine.append_cached.as_secs_f64() * 1000.0,
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
