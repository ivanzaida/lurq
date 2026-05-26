use std::{
  collections::HashMap,
  hash::{Hash, Hasher},
  path::Path,
};

use cosmic_text::{Attrs, Buffer, Family, FontSystem, Metrics, Shaping, SwashCache};

use crate::layout::{
  Size,
  render_list::{GlyphAtlas, GlyphCmd},
  text_style::{FontStyle, FontWeight, TextStyle},
};

#[derive(Clone, PartialEq)]
struct CacheKey {
  text: String,
  font_family: String,
  font_size_bits: u32,
  line_height_bits: u32,
  max_width_bits: u32,
  weight: u8,
  style: u8,
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
  }
}

impl CacheKey {
  fn new(text: &str, style: &TextStyle, max_width: f32) -> Self {
    Self {
      text: text.to_owned(),
      font_family: style.font_family.clone(),
      font_size_bits: style.font_size.to_bits(),
      line_height_bits: style.line_height.to_bits(),
      max_width_bits: max_width.to_bits(),
      weight: weight_to_u8(style.weight),
      style: style_to_u8(style.style),
    }
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

pub(crate) struct GlyphEngine {
  font_system: FontSystem,
  swash_cache: SwashCache,
  font_aliases: HashMap<String, String>,
  measure_cache: HashMap<CacheKey, Size>,
  buffer_pool: Vec<Buffer>,
  pub(crate) measure_hits: usize,
  pub(crate) measure_misses: usize,
  pub(crate) glyph_hits: usize,
  pub(crate) glyph_misses: usize,
}

impl GlyphEngine {
  pub(crate) fn new() -> Self {
    Self {
      font_system: FontSystem::new(),
      swash_cache: SwashCache::new(),
      font_aliases: HashMap::new(),
      measure_cache: HashMap::new(),
      buffer_pool: Vec::new(),
      measure_hits: 0,
      measure_misses: 0,
      glyph_hits: 0,
      glyph_misses: 0,
    }
  }

  pub(crate) fn load_font(&mut self, data: Vec<u8>) {
    self.font_system.db_mut().load_font_data(data);
    self.measure_cache.clear();
  }

  pub(crate) fn load_font_file(&mut self, path: &Path) {
    self.font_system.db_mut().load_font_file(path).ok();
    self.measure_cache.clear();
  }

  pub(crate) fn load_fonts_dir(&mut self, path: &Path) {
    self.font_system.db_mut().load_fonts_dir(path);
    self.measure_cache.clear();
  }

  pub(crate) fn register_font(&mut self, name: &str, family: &str) {
    self.font_aliases.insert(name.to_owned(), family.to_owned());
    self.measure_cache.clear();
  }

  pub(crate) fn clear_cache(&mut self) {
    self.measure_cache.clear();
  }

  pub(crate) fn reset_stats(&mut self) {
    self.measure_hits = 0;
    self.measure_misses = 0;
    self.glyph_hits = 0;
    self.glyph_misses = 0;
  }

  pub(crate) fn measure_text(&mut self, text: &str, style: &TextStyle, max_width: f32) -> Size {
    let key = CacheKey::new(text, style, max_width);
    if let Some(&cached) = self.measure_cache.get(&key) {
      self.measure_hits += 1;
      return cached;
    }
    self.measure_misses += 1;
    let size = self.shape_and_measure(text, style, max_width);
    self.measure_cache.insert(key, size);
    size
  }

  pub(crate) fn rasterize_text(
    &mut self,
    text: &str,
    style: &TextStyle,
    max_width: f32,
    origin_x: f32,
    origin_y: f32,
    atlas_packer: &mut AtlasPacker,
  ) -> Vec<GlyphCmd> {
    let mut buffer = self.acquire_buffer(style, max_width);
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
    buffer.set_text(&mut self.font_system, text, attrs, Shaping::Advanced);
    buffer.shape_until_scroll(&mut self.font_system, false);

    let mut cmds = Vec::new();
    for run in buffer.layout_runs() {
      for glyph in run.glyphs.iter() {
        let physical = glyph.physical((0.0, 0.0), 1.0);
        let Some(image) = self.swash_cache.get_image(&mut self.font_system, physical.cache_key) else {
          continue;
        };
        if image.placement.width == 0 || image.placement.height == 0 {
          continue;
        }

        let gw = image.placement.width;
        let gh = image.placement.height;
        let (u0, v0, u1, v1) = atlas_packer.pack(&image.data, gw, gh);

        let gx = origin_x + physical.x as f32 + image.placement.left as f32;
        let gy = origin_y + run.line_y + physical.y as f32 - image.placement.top as f32;

        cmds.push(GlyphCmd {
          x: gx,
          y: gy,
          width: gw as f32,
          height: gh as f32,
          color: style.color.to_f32_array(),
          uv_min: [u0, v0],
          uv_max: [u1, v1],
          clip: crate::layout::quad::ClipRect::default(),
        });
      }
    }

    self.buffer_pool.push(buffer);
    cmds
  }

  fn shape_and_measure(&mut self, text: &str, style: &TextStyle, max_width: f32) -> Size {
    let metrics = Metrics::new(style.font_size, style.font_size * style.line_height);
    let mut buffer = self
      .buffer_pool
      .pop()
      .unwrap_or_else(|| Buffer::new(&mut self.font_system, metrics));
    buffer.set_metrics(&mut self.font_system, metrics);
    buffer.set_size(&mut self.font_system, Some(max_width), None);

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
    buffer.set_text(&mut self.font_system, text, attrs, Shaping::Advanced);
    buffer.shape_until_scroll(&mut self.font_system, false);

    let mut width = 0.0_f32;
    let mut height = 0.0_f32;
    for run in buffer.layout_runs() {
      width = width.max(run.line_w);
      height = run.line_y + metrics.line_height;
    }

    self.buffer_pool.push(buffer);
    Size::new(width, height)
  }

  fn acquire_buffer(&mut self, style: &TextStyle, max_width: f32) -> Buffer {
    let metrics = Metrics::new(style.font_size, style.font_size * style.line_height);
    let mut buffer = self
      .buffer_pool
      .pop()
      .unwrap_or_else(|| Buffer::new(&mut self.font_system, metrics));
    buffer.set_metrics(&mut self.font_system, metrics);
    buffer.set_size(&mut self.font_system, Some(max_width), None);
    buffer
  }

  fn resolve_family(&self, style: &TextStyle) -> String {
    self
      .font_aliases
      .get(&style.font_family)
      .cloned()
      .unwrap_or_else(|| style.font_family.clone())
  }
}

pub(crate) struct AtlasPacker {
  pub data: Vec<u8>,
  pub width: u32,
  pub height: u32,
  cursor_x: u32,
  cursor_y: u32,
  row_height: u32,
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
    }
  }

  pub(crate) fn pack(&mut self, glyph_data: &[u8], gw: u32, gh: u32) -> (f32, f32, f32, f32) {
    if self.cursor_x + gw > self.width {
      self.cursor_x = 0;
      self.cursor_y += self.row_height;
      self.row_height = 0;
    }

    if self.cursor_y + gh > self.height {
      let new_height = self.height * 2;
      self.data.resize((self.width * new_height) as usize, 0);
      self.height = new_height;
    }

    let x0 = self.cursor_x;
    let y0 = self.cursor_y;

    for row in 0..gh {
      let src_start = (row * gw) as usize;
      let src_end = src_start + gw as usize;
      let dst_start = ((y0 + row) * self.width + x0) as usize;
      if src_end <= glyph_data.len() {
        self.data[dst_start..dst_start + gw as usize].copy_from_slice(&glyph_data[src_start..src_end]);
      }
    }

    self.cursor_x += gw;
    self.row_height = self.row_height.max(gh);

    let u0 = x0 as f32 / self.width as f32;
    let v0 = y0 as f32 / self.height as f32;
    let u1 = (x0 + gw) as f32 / self.width as f32;
    let v1 = (y0 + gh) as f32 / self.height as f32;
    (u0, v0, u1, v1)
  }

  pub(crate) fn to_atlas(&self) -> GlyphAtlas {
    GlyphAtlas {
      data: self.data.clone(),
      width: self.width,
      height: self.height,
    }
  }
}
