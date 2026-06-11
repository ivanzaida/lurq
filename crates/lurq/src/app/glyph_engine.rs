use std::{
  collections::HashMap,
  hash::{Hash, Hasher},
  path::Path,
};

use cosmic_text::{
  Attrs, Buffer, CacheKey as GlyphCacheKey, Family, FontSystem, Metrics, Shaping, SwashCache, SwashContent, SwashImage,
  Wrap,
};
use swash::{
  scale::{Render, ScaleContext, Source, StrikeWith},
  zeno::{Angle, Format, Transform as SwashTransform, Vector},
};

use crate::{
  layout::{
    Size,
    render_list::{GlyphAtlas, GlyphCmd},
    text_style::{FontStyle, FontWeight, TextAlign, TextStyle},
  },
  node::{text_selection::CaretPosition, transform::Transform2D},
};

const GLYPH_LAYOUT_CACHE_LIMIT: usize = 1024;
const GLYPH_ATLAS_PADDING: u32 = 2;

#[derive(Clone, PartialEq)]
struct CacheKey {
  text: String,
  font_family: std::sync::Arc<str>,
  font_size_bits: u32,
  line_height_bits: u32,
  max_width_bits: u32,
  weight: u8,
  style: u8,
  text_align: u8,
  wrap: bool,
  raster_mode: u8,
}

impl Eq for CacheKey {}

impl Hash for CacheKey {
  fn hash<H: Hasher>(&self, state: &mut H) {
    self.text.hash(state);
    self.font_family.hash(state);
    self.font_size_bits.hash(state);
    self.line_height_bits.hash(state);
    self.max_width_bits.hash(state);
    self.weight.hash(state);
    self.style.hash(state);
    self.text_align.hash(state);
    self.wrap.hash(state);
    self.raster_mode.hash(state);
  }
}

impl CacheKey {
  fn new(text: &str, style: &TextStyle, max_width: f32, wrap: bool) -> Self {
    Self {
      text: text.to_owned(),
      font_family: style.font_family.clone(),
      font_size_bits: style.font_size.to_bits(),
      line_height_bits: style.line_height.to_bits(),
      max_width_bits: max_width.to_bits(),
      weight: weight_to_u8(style.weight),
      style: style_to_u8(style.style),
      text_align: text_align_to_u8(style.text_align),
      wrap,
      raster_mode: 0,
    }
  }

  fn new_for_raster(text: &str, style: &TextStyle, max_width: f32, wrap: bool, snap_to_pixel: bool) -> Self {
    let mut key = Self::new(text, style, max_width, wrap);
    key.raster_mode = if snap_to_pixel { 0 } else { 1 };
    key
  }

  fn new_for_baked_transform(text: &str, style: &TextStyle, max_width: f32, wrap: bool) -> Self {
    let mut key = Self::new(text, style, max_width, wrap);
    key.raster_mode = 2;
    key
  }
}

fn weight_to_u8(w: FontWeight) -> u8 {
  match w {
    FontWeight::Thin => 0,
    FontWeight::Light => 1,
    FontWeight::Normal => 2,
    FontWeight::Medium => 3,
    FontWeight::Bold => 4,
    FontWeight::Black => 5,
  }
}

fn style_to_u8(s: FontStyle) -> u8 {
  match s {
    FontStyle::Normal => 0,
    FontStyle::Italic => 1,
  }
}

fn text_align_to_u8(align: TextAlign) -> u8 {
  match align {
    TextAlign::Left => 0,
    TextAlign::Center => 1,
    TextAlign::Right => 2,
    TextAlign::Justified => 3,
    TextAlign::End => 4,
  }
}

fn set_buffer_text(buffer: &mut Buffer, font_system: &mut FontSystem, text: &str, attrs: Attrs, text_align: TextAlign) {
  buffer.set_text(font_system, text, attrs, Shaping::Advanced);
  for line in &mut buffer.lines {
    line.set_align(Some(text_align.to_cosmic()));
  }
}

pub(crate) struct GlyphEngine {
  font_system: FontSystem,
  swash_cache: SwashCache,
  transformed_scale_context: ScaleContext,
  font_aliases: HashMap<String, String>,
  measure_cache: HashMap<CacheKey, Size>,
  glyph_layout_cache: HashMap<CacheKey, Vec<CachedGlyph>>,
  transformed_glyph_layout_cache: HashMap<CacheKey, Vec<CachedTransformedGlyph>>,
  atlas_packer: AtlasPacker,
  atlas_entries: HashMap<GlyphCacheKey, PackedGlyph>,
  transformed_atlas_entries: HashMap<TransformedGlyphKey, PackedGlyph>,
  buffer_pool: Vec<Buffer>,
  pub(crate) measure_hits: usize,
  pub(crate) measure_misses: usize,
  pub(crate) glyph_hits: usize,
  pub(crate) glyph_misses: usize,
}

impl GlyphEngine {
  pub(crate) fn new() -> Self {
    let mut font_system = FontSystem::new();
    load_platform_fonts(&mut font_system);

    Self {
      font_system,
      swash_cache: SwashCache::new(),
      transformed_scale_context: ScaleContext::new(),
      font_aliases: HashMap::new(),
      measure_cache: HashMap::new(),
      glyph_layout_cache: HashMap::new(),
      transformed_glyph_layout_cache: HashMap::new(),
      atlas_packer: AtlasPacker::new(),
      atlas_entries: HashMap::new(),
      transformed_atlas_entries: HashMap::new(),
      buffer_pool: Vec::new(),
      measure_hits: 0,
      measure_misses: 0,
      glyph_hits: 0,
      glyph_misses: 0,
    }
  }

  pub(crate) fn load_font(&mut self, data: Vec<u8>) {
    self.font_system.db_mut().load_font_data(data);
    self.clear_text_caches();
    self.clear_atlas();
  }

  pub(crate) fn load_font_file(&mut self, path: &Path) {
    self.font_system.db_mut().load_font_file(path).ok();
    self.clear_text_caches();
    self.clear_atlas();
  }

  pub(crate) fn load_fonts_dir(&mut self, path: &Path) {
    self.font_system.db_mut().load_fonts_dir(path);
    self.clear_text_caches();
    self.clear_atlas();
  }

  pub(crate) fn register_font(&mut self, name: &str, family: &str) {
    self.font_aliases.insert(name.to_owned(), family.to_owned());
    self.clear_text_caches();
    self.clear_atlas();
  }

  pub(crate) fn clear_cache(&mut self) {
    self.clear_text_caches();
  }

  fn clear_text_caches(&mut self) {
    self.measure_cache.clear();
    self.glyph_layout_cache.clear();
    self.transformed_glyph_layout_cache.clear();
  }

  fn clear_atlas(&mut self) {
    self.atlas_packer = AtlasPacker::new();
    self.atlas_entries.clear();
    self.transformed_atlas_entries.clear();
  }

  #[cfg_attr(not(feature = "perf_profile"), allow(dead_code))]
  pub(crate) fn reset_stats(&mut self) {
    self.measure_hits = 0;
    self.measure_misses = 0;
    self.glyph_hits = 0;
    self.glyph_misses = 0;
  }

  pub(crate) fn measure_text(&mut self, text: &str, style: &TextStyle, max_width: f32) -> Size {
    let wrap = is_bounded_text_width(max_width);
    let key = CacheKey::new(text, style, max_width, wrap);
    if let Some(&cached) = self.measure_cache.get(&key) {
      self.measure_hits += 1;
      return cached;
    }
    self.measure_misses += 1;
    let size = self.shape_and_measure(text, style, max_width, wrap);
    self.measure_cache.insert(key, size);
    size
  }

  pub(crate) fn caret_positions(
    &mut self,
    text: &str,
    style: &TextStyle,
    max_width: f32,
    wrap: bool,
  ) -> Vec<CaretPosition> {
    let mut buffer = self.acquire_buffer(style, max_width, effective_text_wrap(max_width, wrap));
    let resolved = self.resolve_family(style);
    let family = if resolved.is_empty() {
      Family::SansSerif
    } else {
      Family::Name(&resolved)
    };
    let attrs = Attrs::new()
      .family(family)
      .weight(style.weight.to_cosmic())
      .style(style.style.to_cosmic());
    set_buffer_text(&mut buffer, &mut self.font_system, text, attrs, style.text_align);
    buffer.shape_until_scroll(&mut self.font_system, false);

    let mut line_offsets = Vec::with_capacity(buffer.lines.len());
    let mut offset = 0usize;
    for line in &buffer.lines {
      line_offsets.push(offset);
      offset += line.text().len() + line.ending().as_str().len();
    }

    let mut positions = Vec::with_capacity(text.chars().count() + buffer.lines.len().max(1));
    for run in buffer.layout_runs() {
      let line_offset = line_offsets.get(run.line_i).copied().unwrap_or(0);
      let y = run.line_top.max(0.0);
      if run.glyphs.is_empty() {
        positions.push(CaretPosition {
          index: line_offset,
          x: 0.0,
          y,
        });
        continue;
      }

      for glyph in run.glyphs {
        positions.push(CaretPosition {
          index: line_offset + glyph.start,
          x: glyph.x,
          y,
        });
        positions.push(CaretPosition {
          index: line_offset + glyph.end,
          x: glyph.x + glyph.w,
          y,
        });
      }
    }

    if positions.is_empty() {
      positions.push(CaretPosition {
        index: 0,
        x: 0.0,
        y: 0.0,
      });
    }
    if !positions.iter().any(|position| position.index == text.len()) {
      let last_y = positions.last().map(|position| position.y).unwrap_or(0.0);
      positions.push(CaretPosition {
        index: text.len(),
        x: positions.last().map(|position| position.x).unwrap_or(0.0),
        y: last_y,
      });
    }

    self.buffer_pool.push(buffer);
    positions
  }

  pub(crate) fn rasterize_text(
    &mut self,
    text: &str,
    style: &TextStyle,
    max_width: f32,
    origin_x: f32,
    origin_y: f32,
  ) -> Vec<GlyphCmd> {
    let mut glyphs = Vec::new();
    self.rasterize_text_with_snap_into(
      text,
      style,
      max_width,
      max_width.is_finite(),
      origin_x,
      origin_y,
      true,
      &mut glyphs,
    );
    glyphs
  }

  pub(crate) fn rasterize_text_with_wrap_into(
    &mut self,
    text: &str,
    style: &TextStyle,
    max_width: f32,
    wrap: bool,
    origin_x: f32,
    origin_y: f32,
    out: &mut Vec<GlyphCmd>,
  ) {
    self.rasterize_text_with_snap_into(text, style, max_width, wrap, origin_x, origin_y, true, out);
  }

  #[cfg(test)]
  pub(crate) fn rasterize_text_unsnapped(
    &mut self,
    text: &str,
    style: &TextStyle,
    max_width: f32,
    origin_x: f32,
    origin_y: f32,
  ) -> Vec<GlyphCmd> {
    let mut glyphs = Vec::new();
    self.rasterize_text_with_snap_into(
      text,
      style,
      max_width,
      max_width.is_finite(),
      origin_x,
      origin_y,
      false,
      &mut glyphs,
    );
    glyphs
  }

  pub(crate) fn rasterize_text_unsnapped_with_wrap_into(
    &mut self,
    text: &str,
    style: &TextStyle,
    max_width: f32,
    wrap: bool,
    origin_x: f32,
    origin_y: f32,
    out: &mut Vec<GlyphCmd>,
  ) {
    self.rasterize_text_with_snap_into(text, style, max_width, wrap, origin_x, origin_y, false, out);
  }

  #[allow(clippy::too_many_arguments)]
  pub(crate) fn rasterize_text_with_baked_transform_into(
    &mut self,
    text: &str,
    style: &TextStyle,
    max_width: f32,
    wrap: bool,
    origin_x: f32,
    origin_y: f32,
    transform: Transform2D,
    transform_origin: [f32; 2],
    out: &mut Vec<GlyphCmd>,
  ) {
    let wrap = effective_text_wrap(max_width, wrap);
    let key = CacheKey::new_for_baked_transform(text, style, max_width, wrap);
    let swash_transform = swash_transform_from_screen(transform);
    if let Some(cached) = self.transformed_glyph_layout_cache.get(&key).cloned() {
      self.append_baked_transformed_glyph_cmds_from_cached(
        &cached,
        origin_x,
        origin_y,
        style,
        transform,
        transform_origin,
        swash_transform,
        out,
      );
      return;
    }

    let mut buffer = self.acquire_buffer(style, max_width, wrap);
    let resolved = self.resolve_family(style);
    let family = if resolved.is_empty() {
      Family::SansSerif
    } else {
      Family::Name(&resolved)
    };
    let attrs = Attrs::new()
      .family(family)
      .weight(style.weight.to_cosmic())
      .style(style.style.to_cosmic());
    set_buffer_text(&mut buffer, &mut self.font_system, text, attrs, style.text_align);
    buffer.shape_until_scroll(&mut self.font_system, false);

    let mut cached = Vec::new();
    for run in buffer.layout_runs() {
      for glyph in run.glyphs.iter() {
        let x_offset = glyph.font_size * glyph.x_offset;
        let y_offset = glyph.font_size * glyph.y_offset;
        let (cache_key, ..) = GlyphCacheKey::new(
          glyph.font_id,
          glyph.glyph_id,
          glyph.font_size,
          (0.0, 0.0),
          glyph.cache_key_flags,
        );
        cached.push(CachedTransformedGlyph {
          x: glyph.x + x_offset,
          y: run.line_y + glyph.y - y_offset,
          cache_key,
        });
      }
    }

    self.buffer_pool.push(buffer);
    if self.transformed_glyph_layout_cache.len() >= GLYPH_LAYOUT_CACHE_LIMIT {
      self.transformed_glyph_layout_cache.clear();
    }
    self.transformed_glyph_layout_cache.insert(key, cached.clone());
    self.append_baked_transformed_glyph_cmds_from_cached(
      &cached,
      origin_x,
      origin_y,
      style,
      transform,
      transform_origin,
      swash_transform,
      out,
    );
  }

  #[allow(clippy::too_many_arguments)]
  fn rasterize_text_with_snap_into(
    &mut self,
    text: &str,
    style: &TextStyle,
    max_width: f32,
    wrap: bool,
    origin_x: f32,
    origin_y: f32,
    snap_to_pixel: bool,
    out: &mut Vec<GlyphCmd>,
  ) {
    let wrap = effective_text_wrap(max_width, wrap);
    let key = CacheKey::new_for_raster(text, style, max_width, wrap, snap_to_pixel);
    let atlas_w = self.atlas_packer.width as f32;
    let atlas_h = self.atlas_packer.height as f32;
    if let Some(cached) = self.glyph_layout_cache.get(&key) {
      self.glyph_hits += cached.len();
      append_glyph_cmds_from_cached(cached, origin_x, origin_y, style, atlas_w, atlas_h, snap_to_pixel, out);
      return;
    }

    let mut buffer = self.acquire_buffer(style, max_width, wrap);
    let resolved = self.resolve_family(style);
    let family = if resolved.is_empty() {
      Family::SansSerif
    } else {
      Family::Name(&resolved)
    };
    let attrs = Attrs::new()
      .family(family)
      .weight(style.weight.to_cosmic())
      .style(style.style.to_cosmic());
    set_buffer_text(&mut buffer, &mut self.font_system, text, attrs, style.text_align);
    buffer.shape_until_scroll(&mut self.font_system, false);

    let mut cached = Vec::new();
    for run in buffer.layout_runs() {
      for glyph in run.glyphs.iter() {
        let cached_glyph = if snap_to_pixel {
          let physical = glyph.physical((0.0, run.line_y), 1.0);
          let Some(packed) = self.get_or_pack_glyph(physical.cache_key) else {
            continue;
          };

          CachedGlyph {
            x: (physical.x + packed.left) as f32,
            y: (physical.y - packed.top) as f32,
            atlas_x: packed.x,
            atlas_y: packed.y,
            width: packed.width,
            height: packed.height,
          }
        } else {
          let x_offset = glyph.font_size * glyph.x_offset;
          let y_offset = glyph.font_size * glyph.y_offset;
          let (cache_key, ..) = GlyphCacheKey::new(
            glyph.font_id,
            glyph.glyph_id,
            glyph.font_size,
            (0.0, 0.0),
            glyph.cache_key_flags,
          );
          let Some(packed) = self.get_or_pack_glyph(cache_key) else {
            continue;
          };

          CachedGlyph {
            x: glyph.x + x_offset + packed.left as f32,
            y: run.line_y + glyph.y - y_offset - packed.top as f32,
            atlas_x: packed.x,
            atlas_y: packed.y,
            width: packed.width,
            height: packed.height,
          }
        };

        cached.push(cached_glyph);
      }
    }

    self.buffer_pool.push(buffer);
    if self.glyph_layout_cache.len() >= GLYPH_LAYOUT_CACHE_LIMIT {
      self.glyph_layout_cache.clear();
    }
    self.glyph_layout_cache.insert(key, cached.clone());
    let atlas_w = self.atlas_packer.width as f32;
    let atlas_h = self.atlas_packer.height as f32;
    append_glyph_cmds_from_cached(&cached, origin_x, origin_y, style, atlas_w, atlas_h, snap_to_pixel, out);
  }

  #[allow(clippy::too_many_arguments)]
  fn append_baked_transformed_glyph_cmds_from_cached(
    &mut self,
    cached: &[CachedTransformedGlyph],
    origin_x: f32,
    origin_y: f32,
    style: &TextStyle,
    transform: Transform2D,
    transform_origin: [f32; 2],
    swash_transform: SwashTransform,
    out: &mut Vec<GlyphCmd>,
  ) {
    let atlas_w = self.atlas_packer.width as f32;
    let atlas_h = self.atlas_packer.height as f32;
    let color = style.color.to_linear_f32_array();
    out.reserve(cached.len());

    for glyph in cached {
      let Some(packed) = self.get_or_pack_transformed_glyph(glyph.cache_key, swash_transform) else {
        continue;
      };
      let (transformed_origin_x, transformed_origin_y) =
        transformed_glyph_origin(origin_x, origin_y, glyph.x, glyph.y, transform, transform_origin);

      out.push(GlyphCmd {
        order: 0,
        x: transformed_origin_x + packed.left as f32,
        y: transformed_origin_y - packed.top as f32,
        width: packed.width as f32,
        height: packed.height as f32,
        color,
        uv_min: [packed.x as f32 / atlas_w, packed.y as f32 / atlas_h],
        uv_max: [
          (packed.x + packed.width) as f32 / atlas_w,
          (packed.y + packed.height) as f32 / atlas_h,
        ],
        transform: [1.0, 0.0, 0.0, 1.0],
        transform_origin: [0.0, 0.0],
        sharpness: 1.0,
        clip: crate::layout::quad::ClipRect::default(),
      });
    }
  }

  pub(crate) fn atlas(&self) -> GlyphAtlas {
    self.atlas_packer.to_atlas()
  }

  fn get_or_pack_glyph(&mut self, cache_key: GlyphCacheKey) -> Option<PackedGlyph> {
    if let Some(&packed) = self.atlas_entries.get(&cache_key) {
      self.glyph_hits += 1;
      return Some(packed);
    }

    let Some(image) = self.swash_cache.get_image(&mut self.font_system, cache_key) else {
      self.glyph_misses += 1;
      return None;
    };
    if image.placement.width == 0 || image.placement.height == 0 {
      return None;
    }

    let width = image.placement.width;
    let height = image.placement.height;
    let mask = glyph_coverage_mask(&image);
    let (x, y, width, height) = self.atlas_packer.pack_pixels(&mask, width, height);
    let packed = PackedGlyph {
      x,
      y,
      width,
      height,
      left: image.placement.left - GLYPH_ATLAS_PADDING as i32,
      top: image.placement.top + GLYPH_ATLAS_PADDING as i32,
    };
    self.atlas_entries.insert(cache_key, packed);
    self.glyph_misses += 1;
    Some(packed)
  }

  fn get_or_pack_transformed_glyph(
    &mut self,
    cache_key: GlyphCacheKey,
    transform: SwashTransform,
  ) -> Option<PackedGlyph> {
    let key = TransformedGlyphKey::new(cache_key, transform);
    if let Some(&packed) = self.transformed_atlas_entries.get(&key) {
      self.glyph_hits += 1;
      return Some(packed);
    }

    let font = self.font_system.get_font(cache_key.font_id)?;
    let mut scaler = self
      .transformed_scale_context
      .builder(font.as_swash())
      .size(f32::from_bits(cache_key.font_size_bits))
      .hint(false)
      .build();
    let offset = Vector::new(cache_key.x_bin.as_float(), cache_key.y_bin.as_float());
    let transform = if cache_key.flags.contains(cosmic_text::CacheKeyFlags::FAKE_ITALIC) {
      SwashTransform::skew(Angle::from_degrees(14.0), Angle::from_degrees(0.0)).then(&transform)
    } else {
      transform
    };
    let image = Render::new(&[
      Source::ColorOutline(0),
      Source::ColorBitmap(StrikeWith::BestFit),
      Source::Outline,
    ])
    .format(Format::Alpha)
    .offset(offset)
    .transform(Some(transform))
    .render(&mut scaler, cache_key.glyph_id)?;
    if image.placement.width == 0 || image.placement.height == 0 {
      return None;
    }

    let mask = glyph_coverage_mask(&image);
    let (x, y, width, height) = self
      .atlas_packer
      .pack_pixels(&mask, image.placement.width, image.placement.height);
    let packed = PackedGlyph {
      x,
      y,
      width,
      height,
      left: image.placement.left - GLYPH_ATLAS_PADDING as i32,
      top: image.placement.top + GLYPH_ATLAS_PADDING as i32,
    };
    self.transformed_atlas_entries.insert(key, packed);
    self.glyph_misses += 1;
    Some(packed)
  }

  fn shape_and_measure(&mut self, text: &str, style: &TextStyle, max_width: f32, wrap: bool) -> Size {
    let metrics = Metrics::new(style.font_size, style.font_size * style.line_height);
    let mut buffer = self
      .buffer_pool
      .pop()
      .unwrap_or_else(|| Buffer::new(&mut self.font_system, metrics));
    buffer.set_metrics(&mut self.font_system, metrics);
    buffer.set_size(&mut self.font_system, text_buffer_width(max_width), None);
    buffer.set_wrap(&mut self.font_system, if wrap { Wrap::WordOrGlyph } else { Wrap::None });

    let resolved = self.resolve_family(style);
    let family = if resolved.is_empty() {
      Family::SansSerif
    } else {
      Family::Name(&resolved)
    };
    let attrs = Attrs::new()
      .family(family)
      .weight(style.weight.to_cosmic())
      .style(style.style.to_cosmic());
    set_buffer_text(&mut buffer, &mut self.font_system, text, attrs, style.text_align);
    buffer.shape_until_scroll(&mut self.font_system, false);

    let mut width = 0.0_f32;
    let mut first_line_y = 0.0_f32;
    let mut last_line_y = 0.0_f32;
    let mut has_runs = false;
    for run in buffer.layout_runs() {
      width = width.max(run.line_w);
      if !has_runs {
        first_line_y = run.line_y;
        has_runs = true;
      }
      last_line_y = run.line_y;
    }
    let height = if has_runs {
      last_line_y - first_line_y + metrics.line_height
    } else {
      0.0
    };

    self.buffer_pool.push(buffer);
    Size::new(width, height)
  }

  fn acquire_buffer(&mut self, style: &TextStyle, max_width: f32, wrap: bool) -> Buffer {
    let metrics = Metrics::new(style.font_size, style.font_size * style.line_height);
    let wrap = effective_text_wrap(max_width, wrap);
    let mut buffer = self
      .buffer_pool
      .pop()
      .unwrap_or_else(|| Buffer::new(&mut self.font_system, metrics));
    buffer.set_metrics(&mut self.font_system, metrics);
    buffer.set_size(&mut self.font_system, text_buffer_width(max_width), None);
    buffer.set_wrap(&mut self.font_system, if wrap { Wrap::WordOrGlyph } else { Wrap::None });
    buffer
  }

  fn resolve_family(&self, style: &TextStyle) -> std::sync::Arc<str> {
    self
      .font_aliases
      .get(&*style.font_family)
      .map(|s| std::sync::Arc::from(s.as_str()))
      .unwrap_or_else(|| style.font_family.clone())
  }

  #[cfg_attr(not(feature = "perf_profile"), allow(dead_code))]
  pub(crate) fn estimated_memory_bytes(&self) -> usize {
    let alias_heap = self
      .font_aliases
      .iter()
      .map(|(name, family)| name.capacity() + family.capacity())
      .sum::<usize>();
    let measure_key_heap = self
      .measure_cache
      .keys()
      .map(|key| key.text.capacity() + key.font_family.len())
      .sum::<usize>();
    let glyph_layout_cache_bytes = self
      .glyph_layout_cache
      .values()
      .map(|glyphs| glyphs.capacity() * std::mem::size_of::<CachedGlyph>())
      .sum::<usize>();
    let transformed_glyph_layout_cache_bytes = self
      .transformed_glyph_layout_cache
      .values()
      .map(|glyphs| glyphs.capacity() * std::mem::size_of::<CachedTransformedGlyph>())
      .sum::<usize>();

    std::mem::size_of::<Self>()
      + self.font_aliases.capacity() * std::mem::size_of::<(String, String)>()
      + alias_heap
      + self.measure_cache.capacity() * std::mem::size_of::<(CacheKey, Size)>()
      + measure_key_heap
      + self.glyph_layout_cache.capacity() * std::mem::size_of::<(CacheKey, Vec<CachedGlyph>)>()
      + glyph_layout_cache_bytes
      + self.transformed_glyph_layout_cache.capacity() * std::mem::size_of::<(CacheKey, Vec<CachedTransformedGlyph>)>()
      + transformed_glyph_layout_cache_bytes
      + self.buffer_pool.capacity() * std::mem::size_of::<Buffer>()
  }
}

#[derive(Clone, Copy)]
struct PackedGlyph {
  x: u32,
  y: u32,
  width: u32,
  height: u32,
  left: i32,
  top: i32,
}

#[derive(Clone, Copy)]
struct CachedGlyph {
  x: f32,
  y: f32,
  atlas_x: u32,
  atlas_y: u32,
  width: u32,
  height: u32,
}

#[derive(Clone, Copy)]
struct CachedTransformedGlyph {
  x: f32,
  y: f32,
  cache_key: GlyphCacheKey,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct TransformedGlyphKey {
  cache_key: GlyphCacheKey,
  transform: [u32; 4],
}

impl TransformedGlyphKey {
  fn new(cache_key: GlyphCacheKey, transform: SwashTransform) -> Self {
    Self {
      cache_key,
      transform: [
        transform.xx.to_bits(),
        transform.xy.to_bits(),
        transform.yx.to_bits(),
        transform.yy.to_bits(),
      ],
    }
  }
}

fn swash_transform_from_screen(transform: Transform2D) -> SwashTransform {
  SwashTransform::new(transform.a, -transform.b, -transform.c, transform.d, 0.0, 0.0)
}

fn is_bounded_text_width(max_width: f32) -> bool {
  max_width.is_finite() && max_width < f32::MAX
}

fn effective_text_wrap(max_width: f32, wrap: bool) -> bool {
  wrap && is_bounded_text_width(max_width)
}

fn text_buffer_width(max_width: f32) -> Option<f32> {
  is_bounded_text_width(max_width).then_some(max_width)
}

fn append_glyph_cmds_from_cached(
  cached: &[CachedGlyph],
  origin_x: f32,
  origin_y: f32,
  style: &TextStyle,
  atlas_w: f32,
  atlas_h: f32,
  snap_to_pixel: bool,
  out: &mut Vec<GlyphCmd>,
) {
  let color = style.color.to_linear_f32_array();
  out.reserve(cached.len());
  for glyph in cached {
    let x = origin_x + glyph.x;
    let y = origin_y + glyph.y;
    out.push(GlyphCmd {
      order: 0,
      x: if snap_to_pixel { x.round() } else { x },
      y: if snap_to_pixel { y.round() } else { y },
      width: glyph.width as f32,
      height: glyph.height as f32,
      color,
      uv_min: [glyph.atlas_x as f32 / atlas_w, glyph.atlas_y as f32 / atlas_h],
      uv_max: [
        (glyph.atlas_x + glyph.width) as f32 / atlas_w,
        (glyph.atlas_y + glyph.height) as f32 / atlas_h,
      ],
      transform: [1.0, 0.0, 0.0, 1.0],
      transform_origin: [0.0, 0.0],
      sharpness: 1.0,
      clip: crate::layout::quad::ClipRect::default(),
    });
  }
}

#[allow(clippy::too_many_arguments)]
fn transformed_glyph_origin(
  origin_x: f32,
  origin_y: f32,
  glyph_x: f32,
  glyph_y: f32,
  transform: Transform2D,
  transform_origin: [f32; 2],
) -> (f32, f32) {
  let glyph_origin_x = origin_x + glyph_x;
  let glyph_origin_y = origin_y + glyph_y;
  (
    transform_origin[0]
      + transform.a * (glyph_origin_x - transform_origin[0])
      + transform.c * (glyph_origin_y - transform_origin[1]),
    transform_origin[1]
      + transform.b * (glyph_origin_x - transform_origin[0])
      + transform.d * (glyph_origin_y - transform_origin[1]),
  )
}

fn glyph_coverage_mask(image: &SwashImage) -> Vec<u8> {
  match image.content {
    SwashContent::Mask => image.data.clone(),
    SwashContent::Color => image.data.chunks_exact(4).map(|rgba| rgba[3]).collect::<Vec<_>>(),
    SwashContent::SubpixelMask => image
      .data
      .chunks_exact(4)
      .map(|rgba| {
        let coverage = rgba[0] as u16 + rgba[1] as u16 + rgba[2] as u16;
        (coverage / 3) as u8
      })
      .collect::<Vec<_>>(),
  }
}

fn load_platform_fonts(font_system: &mut FontSystem) {
  #[cfg(target_os = "windows")]
  {
    for file in [
      "C:\\Windows\\Fonts\\segoeui.ttf",
      "C:\\Windows\\Fonts\\segoeuib.ttf",
      "C:\\Windows\\Fonts\\arial.ttf",
      "C:\\Windows\\Fonts\\arialbd.ttf",
    ] {
      let _ = font_system.db_mut().load_font_file(file);
    }
  }

  #[cfg(not(target_os = "windows"))]
  {
    let _ = font_system;
  }
}

pub(crate) struct AtlasPacker {
  pub data: Vec<u8>,
  pub width: u32,
  pub height: u32,
  cursor_x: u32,
  cursor_y: u32,
  row_height: u32,
  version: u64,
}

impl AtlasPacker {
  pub(crate) fn new() -> Self {
    let width = 1024;
    let height = 1024;
    Self {
      data: vec![0u8; (width * height) as usize],
      width,
      height,
      cursor_x: 0,
      cursor_y: 0,
      row_height: 0,
      version: 0,
    }
  }

  #[cfg(test)]
  pub(crate) fn pack(&mut self, glyph_data: &[u8], gw: u32, gh: u32) -> (f32, f32, f32, f32) {
    let (x0, y0, gw, gh) = self.pack_pixels(glyph_data, gw, gh);
    let u0 = x0 as f32 / self.width as f32;
    let v0 = y0 as f32 / self.height as f32;
    let u1 = (x0 + gw) as f32 / self.width as f32;
    let v1 = (y0 + gh) as f32 / self.height as f32;
    (u0, v0, u1, v1)
  }

  fn pack_pixels(&mut self, glyph_data: &[u8], gw: u32, gh: u32) -> (u32, u32, u32, u32) {
    let padding = GLYPH_ATLAS_PADDING;
    let reserved_width = gw + padding * 2;
    let reserved_height = gh + padding * 2;

    if self.cursor_x + reserved_width > self.width {
      self.cursor_x = 0;
      self.cursor_y += self.row_height;
      self.row_height = 0;
    }

    if self.cursor_y + reserved_height > self.height {
      let new_height = self.height * 2;
      self.data.resize((self.width * new_height) as usize, 0);
      self.height = new_height;
    }

    let padded_x = self.cursor_x;
    let padded_y = self.cursor_y;
    let x0 = padded_x;
    let y0 = padded_y;

    for row in 0..gh {
      for col in 0..gw {
        let src = (row * gw + col) as usize;
        let dst = ((padded_y + padding + row) * self.width + padded_x + padding + col) as usize;
        if src < glyph_data.len() && dst < self.data.len() {
          self.data[dst] = glyph_data[src];
        }
      }
    }

    self.cursor_x += reserved_width;
    self.row_height = self.row_height.max(reserved_height);
    self.version += 1;

    (x0, y0, reserved_width, reserved_height)
  }

  pub(crate) fn to_atlas(&self) -> GlyphAtlas {
    GlyphAtlas {
      data: self.data.clone(),
      width: self.width,
      height: self.height,
      version: self.version,
    }
  }
}

#[cfg(test)]
mod tests {
  use cosmic_text::{Attrs, Family, Shaping};

  use super::{
    AtlasPacker, GLYPH_ATLAS_PADDING, GlyphEngine, glyph_coverage_mask, is_bounded_text_width,
    swash_transform_from_screen,
  };
  use crate::node::transform::Transform2D;

  #[test]
  fn atlas_packer_leaves_padding_between_glyph_regions() {
    let mut packer = AtlasPacker::new();
    let (_, _, first_u1, _) = packer.pack(&[255; 4], 2, 2);
    let (second_u0, ..) = packer.pack(&[255; 4], 2, 2);

    let first_x1 = (first_u1 * packer.width as f32).round() as u32;
    let second_x0 = (second_u0 * packer.width as f32).round() as u32;

    assert!(second_x0 >= first_x1);
  }

  #[test]
  fn atlas_packer_leaves_transparent_glyph_padding() {
    let mut packer = AtlasPacker::new();
    let (x, y, width, height) = packer.pack_pixels(&[10, 20, 30, 40], 2, 2);
    let p = GLYPH_ATLAS_PADDING as usize;
    let stride = packer.width as usize;

    assert_eq!(
      (x, y, width, height),
      (0, 0, 2 + GLYPH_ATLAS_PADDING * 2, 2 + GLYPH_ATLAS_PADDING * 2)
    );
    assert_eq!(packer.data[0], 0);
    assert_eq!(packer.data[p * stride + p], 10);
    assert_eq!(packer.data[p * stride + p + 1], 20);
    assert_eq!(packer.data[(p + 1) * stride + p], 30);
    assert_eq!(packer.data[(p + 1) * stride + p + 1], 40);
    assert_eq!(packer.data[(p + 2) * stride + p + 2], 0);
  }

  #[test]
  fn small_key_label_descender_mask_contains_bottom_coverage() {
    let mut engine = GlyphEngine::new();
    let style = crate::layout::text_style::TextStyle {
      font_size: 11.0,
      ..crate::layout::text_style::TextStyle::default()
    };
    let mut buffer = engine.acquire_buffer(&style, 42.0, true);
    let resolved = engine.resolve_family(&style);
    let attrs = Attrs::new()
      .family(Family::Name(&resolved))
      .weight(style.weight.to_cosmic())
      .style(style.style.to_cosmic());
    buffer.set_text(&mut engine.font_system, "key=\"a\"", attrs, Shaping::Advanced);
    buffer.shape_until_scroll(&mut engine.font_system, false);

    let y_glyph = buffer
      .layout_runs()
      .flat_map(|run| run.glyphs.iter().map(move |glyph| (run.line_y, glyph)))
      .find(|(_, glyph)| glyph.start <= 2 && glyph.end >= 3)
      .expect("the y glyph should be present");
    let physical = y_glyph.1.physical((0.0, y_glyph.0), 1.0);
    let image = engine
      .swash_cache
      .get_image(&mut engine.font_system, physical.cache_key)
      .as_ref()
      .expect("the y glyph should rasterize");
    let mask = glyph_coverage_mask(&image);
    let width = image.placement.width as usize;
    let height = image.placement.height as usize;
    let bottom_row = &mask[(height - 1) * width..height * width];

    assert!(
      bottom_row.iter().any(|coverage| *coverage > 0),
      "the rasterized y glyph should contain nonzero bottom-row coverage"
    );

    engine.buffer_pool.push(buffer);
  }

  #[test]
  fn rasterized_glyph_positions_snap_after_fractional_origin() {
    let mut engine = GlyphEngine::new();
    let style = crate::layout::text_style::TextStyle {
      font_size: 11.0,
      ..crate::layout::text_style::TextStyle::default()
    };

    let glyphs = engine.rasterize_text("key=\"a\"", &style, 42.0, 0.0, 13.5);

    assert!(!glyphs.is_empty());
    for glyph in glyphs {
      assert!(
        glyph.y.fract().abs() < f32::EPSILON,
        "glyph y should be snapped after adding the final origin: {}",
        glyph.y
      );
    }
  }

  #[test]
  fn unsnapped_rasterized_glyph_positions_preserve_float_layout() {
    let mut engine = GlyphEngine::new();
    let style = crate::layout::text_style::TextStyle {
      font_size: 16.0,
      ..crate::layout::text_style::TextStyle::default()
    };

    let glyphs = engine.rasterize_text_unsnapped("This selectable text", &style, 400.0, 0.0, 0.0);

    assert!(!glyphs.is_empty());
    assert!(
      glyphs.iter().any(|glyph| glyph.x.fract().abs() > 0.001),
      "transformed text should keep float layout glyph positions"
    );
  }

  #[test]
  fn screen_transform_is_flipped_for_glyph_outline_space() {
    let screen = Transform2D::rotate_deg(-10.0);
    let glyph = swash_transform_from_screen(screen);

    assert_eq!(glyph.xx, screen.a);
    assert_eq!(glyph.xy, -screen.b);
    assert_eq!(glyph.yx, -screen.c);
    assert_eq!(glyph.yy, screen.d);
  }

  #[test]
  #[cfg(target_os = "windows")]
  fn medium_weight_text_does_not_fallback_to_symbol_font() {
    let mut engine = GlyphEngine::new();
    let style = crate::layout::text_style::TextStyle {
      font_size: 14.0,
      weight: crate::layout::text_style::FontWeight::Medium,
      ..crate::layout::text_style::TextStyle::default()
    };
    let mut buffer = engine.acquire_buffer(&style, 72.0, true);
    let resolved = engine.resolve_family(&style);
    let attrs = Attrs::new()
      .family(Family::Name(&resolved))
      .weight(style.weight.to_cosmic())
      .style(style.style.to_cosmic());
    buffer.set_text(&mut engine.font_system, "Name", attrs, Shaping::Advanced);
    buffer.shape_until_scroll(&mut engine.font_system, false);

    for run in buffer.layout_runs() {
      for glyph in run.glyphs {
        let family = engine
          .font_system
          .db()
          .face(glyph.font_id)
          .and_then(|face| face.families.first())
          .map(|family| family.0.as_str())
          .unwrap_or("<unknown>");
        assert_ne!(family, "Marlett");
      }
    }
    engine.buffer_pool.push(buffer);
  }

  #[test]
  fn wrapped_glyph_y_positions_are_pixel_snapped() {
    let mut engine = GlyphEngine::new();
    let style = crate::layout::text_style::TextStyle {
      font_size: 13.0,
      line_height: 1.17,
      ..crate::layout::text_style::TextStyle::default()
    };

    let glyphs = engine.rasterize_text("alpha beta gamma delta", &style, 48.0, 0.0, 0.0);

    assert!(
      glyphs.iter().any(|glyph| glyph.y > style.font_size * style.line_height),
      "test text should wrap to at least two lines"
    );
    for glyph in glyphs {
      assert!(
        glyph.y.fract().abs() < f32::EPSILON,
        "glyph y should be pixel-snapped after baseline offset is applied: {}",
        glyph.y
      );
    }
  }

  #[test]
  fn wrapped_text_measurement_contains_painted_glyph_bounds() {
    let mut engine = GlyphEngine::new();
    let style = crate::layout::text_style::TextStyle {
      font_size: 16.0,
      ..crate::layout::text_style::TextStyle::default()
    };
    let text = "Wrapping keeps long typography inside its assigned layout width.";
    let max_width = 260.0;
    let measured = engine.measure_text(text, &style, max_width);
    let glyphs = engine.rasterize_text(text, &style, max_width, 0.0, 0.0);
    let glyph_bottom = glyphs
      .iter()
      .map(|glyph| glyph.y + glyph.height)
      .fold(0.0_f32, f32::max);

    let transparent_padding = GLYPH_ATLAS_PADDING as f32;
    assert!(
      glyph_bottom <= measured.height.ceil() + transparent_padding,
      "painted glyph bottom should fit measured height plus transparent atlas padding: bottom={}, measured={}, padding={}",
      glyph_bottom,
      measured.height,
      transparent_padding
    );
  }

  #[test]
  fn wrapped_text_measurement_contains_scaled_painted_glyph_bounds() {
    let mut engine = GlyphEngine::new();
    let style = crate::layout::text_style::TextStyle {
      font_size: 16.0,
      ..crate::layout::text_style::TextStyle::default()
    };
    let text = "Wrapping keeps long typography inside its assigned layout width.";
    let max_width = 260.0;
    let measured = engine.measure_text(text, &style, max_width);
    for scale in [1.1, 1.25, 1.5, 1.75, 2.0] {
      let mut scaled_style = style.clone();
      scaled_style.font_size *= scale;
      let glyphs = engine.rasterize_text(text, &scaled_style, max_width * scale, 0.0, 0.0);
      let glyph_bottom = glyphs
        .iter()
        .map(|glyph| glyph.y + glyph.height)
        .fold(0.0_f32, f32::max);

      let clip_bottom = (measured.height * scale).ceil();
      let transparent_padding = GLYPH_ATLAS_PADDING as f32;
      assert!(
        glyph_bottom <= clip_bottom + transparent_padding + 1.0,
        "painted scaled glyph bottom should fit scaled measured height plus rasterization slop and transparent atlas padding: bottom={}, clip={}, measured={}, scale={}, padding={}",
        glyph_bottom,
        clip_bottom,
        measured.height,
        scale,
        transparent_padding
      );
    }
  }

  #[test]
  fn max_float_text_width_is_unbounded() {
    assert!(!is_bounded_text_width(f32::MAX));
    assert!(!is_bounded_text_width(f32::INFINITY));
    assert!(is_bounded_text_width(320.0));
  }

  #[test]
  fn max_float_raster_width_does_not_wrap_text() {
    let mut engine = GlyphEngine::new();
    let style = crate::layout::text_style::TextStyle {
      font_size: 13.0,
      line_height: 1.17,
      ..crate::layout::text_style::TextStyle::default()
    };
    let glyphs = engine.rasterize_text("0.1.8 -> 0.10.10", &style, f32::MAX, 0.0, 0.0);
    let max_y = glyphs.iter().map(|glyph| glyph.y).fold(0.0_f32, f32::max);

    assert!(
      max_y < style.font_size * style.line_height,
      "unbounded rasterization should stay on one line, max glyph y was {max_y}"
    );
  }
}
