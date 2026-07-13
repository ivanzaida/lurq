#[cfg(test)]
use std::borrow::Cow;
#[cfg(feature = "perf_profile")]
use std::time::Instant;
use std::{
  collections::{HashMap, hash_map::DefaultHasher},
  hash::{Hash, Hasher},
  path::Path,
};

use cosmic_text::{
  Attrs, Buffer, CacheKey as GlyphCacheKey, Color as CosmicColor, Family, Font, FontSystem, LayoutGlyph, Metrics,
  Shaping, SwashContent, SwashImage, Wrap,
};
use swash::{
  scale::{Render, ScaleContext, Source, StrikeWith},
  zeno::{Angle, Format, Transform as SwashTransform, Vector},
};

use crate::{
  app::profile_types::GlyphEngineProfile,
  layout::{
    Size,
    quad::{ClipRect, RichTextSpan},
    render_list::{GlyphAtlas, GlyphAtlasDirtyRect, GlyphCmd},
    text_style::{FontStyle, FontWeight, TextAlign, TextStyle},
  },
  node::{color::Color, text_selection::CaretPosition, transform::Transform2D},
};

const GLYPH_LAYOUT_CACHE_LIMIT: usize = 1024;
const GLYPH_ATLAS_PADDING: u32 = 2;
const GLYPH_ATLAS_BYTES_PER_PIXEL: usize = 4;
const DIRTY_RECT_MAX_HORIZONTAL_GAP: u32 = GLYPH_ATLAS_PADDING * 4;
const DIRTY_RECT_MERGE_WASTE_NUMERATOR: u64 = 3;
const DIRTY_RECT_MERGE_WASTE_DENOMINATOR: u64 = 2;

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

  fn matches_measure(&self, text: &str, style: &TextStyle, max_width: f32, wrap: bool) -> bool {
    self.raster_mode == 0
      && self.text == text
      && self.font_family == style.font_family
      && self.font_size_bits == style.font_size.to_bits()
      && self.line_height_bits == style.line_height.to_bits()
      && self.max_width_bits == max_width.to_bits()
      && self.weight == weight_to_u8(style.weight)
      && self.style == style_to_u8(style.style)
      && self.text_align == text_align_to_u8(style.text_align)
      && self.wrap == wrap
  }
}

fn text_measure_fingerprint(text: &str, style: &TextStyle, max_width: f32, wrap: bool) -> u64 {
  let mut hasher = DefaultHasher::new();
  hash_sampled_text(text, &mut hasher);
  style.font_family.hash(&mut hasher);
  style.font_size.to_bits().hash(&mut hasher);
  style.line_height.to_bits().hash(&mut hasher);
  max_width.to_bits().hash(&mut hasher);
  weight_to_u8(style.weight).hash(&mut hasher);
  style_to_u8(style.style).hash(&mut hasher);
  text_align_to_u8(style.text_align).hash(&mut hasher);
  wrap.hash(&mut hasher);
  hasher.finish()
}

fn hash_sampled_text<H: Hasher>(text: &str, hasher: &mut H) {
  const SAMPLE: usize = 128;
  text.len().hash(hasher);
  if text.len() <= SAMPLE * 3 {
    text.hash(hasher);
    return;
  }

  hasher.write(&text.as_bytes()[..SAMPLE]);
  let middle = text.len() / 2;
  let middle_start = text_floor_char_boundary(text, middle.saturating_sub(SAMPLE / 2));
  let middle_end = text_floor_char_boundary(text, (middle_start + SAMPLE).min(text.len()));
  hasher.write(&text.as_bytes()[middle_start..middle_end]);
  let tail_start = text_floor_char_boundary(text, text.len() - SAMPLE);
  hasher.write(&text.as_bytes()[tail_start..]);
}

fn text_floor_char_boundary(text: &str, mut index: usize) -> usize {
  while index > 0 && !text.is_char_boundary(index) {
    index -= 1;
  }
  index
}

#[derive(Clone, PartialEq, Eq, Hash)]
struct ClippedCacheKey {
  base: CacheKey,
  clip_x_bits: u32,
  clip_y_bits: u32,
  clip_width_bits: u32,
  clip_height_bits: u32,
}

impl ClippedCacheKey {
  fn new(base: CacheKey, origin_x: f32, origin_y: f32, clip: ClipRect) -> Option<Self> {
    clip.active.then_some(Self {
      base,
      clip_x_bits: (clip.x - origin_x).to_bits(),
      clip_y_bits: (clip.y - origin_y).to_bits(),
      clip_width_bits: clip.width.to_bits(),
      clip_height_bits: clip.height.to_bits(),
    })
  }
}

#[derive(Clone, PartialEq, Eq, Hash)]
struct RichTextCacheKey {
  spans: Vec<RichTextSpanCacheKey>,
  max_width_bits: u32,
  wrap: bool,
  raster_mode: u8,
}

impl RichTextCacheKey {
  fn new_for_raster(spans: &[RichTextSpan], max_width: f32, wrap: bool, snap_to_pixel: bool) -> Self {
    Self {
      spans: spans.iter().map(RichTextSpanCacheKey::new).collect(),
      max_width_bits: max_width.to_bits(),
      wrap,
      raster_mode: if snap_to_pixel { 0 } else { 1 },
    }
  }

  fn matches(&self, spans: &[RichTextSpan], max_width: f32, wrap: bool, snap_to_pixel: bool) -> bool {
    self.max_width_bits == max_width.to_bits()
      && self.wrap == wrap
      && self.raster_mode == if snap_to_pixel { 0 } else { 1 }
      && spans_match(&self.spans, spans)
  }
}

#[derive(Clone, PartialEq, Eq, Hash)]
struct RichTextShapeKey {
  spans: Vec<RichTextSpanCacheKey>,
  max_width_bits: u32,
  wrap: bool,
}

impl RichTextShapeKey {
  fn new(spans: &[RichTextSpan], max_width: f32, wrap: bool) -> Self {
    Self {
      spans: spans.iter().map(RichTextSpanCacheKey::new).collect(),
      max_width_bits: max_width.to_bits(),
      wrap,
    }
  }

  fn matches(&self, spans: &[RichTextSpan], max_width: f32, wrap: bool) -> bool {
    self.max_width_bits == max_width.to_bits() && self.wrap == wrap && spans_match(&self.spans, spans)
  }
}

#[derive(Clone, PartialEq, Eq, Hash)]
struct RichTextSpanCacheKey {
  text: String,
  font_family: std::sync::Arc<str>,
  font_size_bits: u32,
  line_height_bits: u32,
  weight: u8,
  style: u8,
  text_align: u8,
  color: [u8; 4],
}

impl RichTextSpanCacheKey {
  fn new(span: &RichTextSpan) -> Self {
    let style = &span.style;
    Self {
      text: span.text.clone(),
      font_family: style.font_family.clone(),
      font_size_bits: style.font_size.to_bits(),
      line_height_bits: style.line_height.to_bits(),
      weight: weight_to_u8(style.weight),
      style: style_to_u8(style.style),
      text_align: text_align_to_u8(style.text_align),
      color: [style.color.r(), style.color.g(), style.color.b(), style.color.a()],
    }
  }

  fn matches(&self, span: &RichTextSpan) -> bool {
    let style = &span.style;
    self.text == span.text
      && self.font_family == style.font_family
      && self.font_size_bits == style.font_size.to_bits()
      && self.line_height_bits == style.line_height.to_bits()
      && self.weight == weight_to_u8(style.weight)
      && self.style == style_to_u8(style.style)
      && self.text_align == text_align_to_u8(style.text_align)
      && self.color == [style.color.r(), style.color.g(), style.color.b(), style.color.a()]
  }
}

fn spans_match(cached: &[RichTextSpanCacheKey], spans: &[RichTextSpan]) -> bool {
  cached.len() == spans.len() && cached.iter().zip(spans).all(|(cached, span)| cached.matches(span))
}

fn rich_text_shape_fingerprint(spans: &[RichTextSpan], max_width: f32, wrap: bool) -> u64 {
  let mut hasher = DefaultHasher::new();
  max_width.to_bits().hash(&mut hasher);
  wrap.hash(&mut hasher);
  hash_rich_text_spans(spans, &mut hasher);
  hasher.finish()
}

fn rich_text_raster_fingerprint(spans: &[RichTextSpan], max_width: f32, wrap: bool, snap_to_pixel: bool) -> u64 {
  let mut hasher = DefaultHasher::new();
  max_width.to_bits().hash(&mut hasher);
  wrap.hash(&mut hasher);
  (if snap_to_pixel { 0u8 } else { 1u8 }).hash(&mut hasher);
  hash_rich_text_spans(spans, &mut hasher);
  hasher.finish()
}

fn hash_rich_text_spans(spans: &[RichTextSpan], hasher: &mut DefaultHasher) {
  spans.len().hash(hasher);
  for span in spans {
    let style = &span.style;
    span.text.hash(hasher);
    style.font_family.hash(hasher);
    style.font_size.to_bits().hash(hasher);
    style.line_height.to_bits().hash(hasher);
    weight_to_u8(style.weight).hash(hasher);
    style_to_u8(style.style).hash(hasher);
    text_align_to_u8(style.text_align).hash(hasher);
    [style.color.r(), style.color.g(), style.color.b(), style.color.a()].hash(hasher);
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

/// Vertical extents of a shaped single line, relative to the render origin
/// (a glyph placed at origin_y `Y` puts these at `Y + value`). `ink_*` is the
/// tight bound of the visible glyph ink; `optical_*` is the descender-agnostic
/// box used for centering — the font's cap-height box (`[baseline - cap_height,
/// baseline]`) for real text, falling back to the ink bounds when the font
/// exposes no usable cap height (e.g. icon fonts). Centering the optical box
/// keeps text with and without descenders — and icons next to it — visually
/// aligned, instead of letting descenders drag the visible mass upward.
#[derive(Clone, Copy)]
pub(crate) struct TextVerticalExtents {
  pub(crate) ink_top: f32,
  pub(crate) ink_bottom: f32,
  pub(crate) optical_top: f32,
  pub(crate) optical_bottom: f32,
}

pub(crate) struct GlyphEngine {
  font_system: FontSystem,
  swash_context: ScaleContext,
  transformed_scale_context: ScaleContext,
  font_aliases: HashMap<String, String>,
  measure_cache: HashMap<u64, Vec<(CacheKey, Size)>>,
  vertical_extents_cache: HashMap<u64, Vec<(CacheKey, Option<TextVerticalExtents>)>>,
  caret_cache: HashMap<u64, Vec<(CacheKey, Vec<CaretPosition>)>>,
  rich_shaped_layout_cache: HashMap<u64, Vec<(RichTextShapeKey, CachedRichShapedLayout)>>,
  glyph_layout_cache: HashMap<CacheKey, Vec<CachedGlyph>>,
  clipped_glyph_layout_cache: HashMap<ClippedCacheKey, Vec<CachedGlyph>>,
  rich_glyph_layout_cache: HashMap<u64, Vec<(RichTextCacheKey, Vec<CachedRichGlyph>)>>,
  transformed_glyph_layout_cache: HashMap<CacheKey, Vec<CachedTransformedGlyph>>,
  atlas_packer: AtlasPacker,
  atlas_entries: HashMap<GlyphCacheKey, PackedGlyph>,
  transformed_atlas_entries: HashMap<TransformedGlyphKey, PackedGlyph>,
  buffer_pool: Vec<Buffer>,
  pub(crate) measure_hits: usize,
  pub(crate) measure_misses: usize,
  pub(crate) glyph_hits: usize,
  pub(crate) glyph_misses: usize,
  #[cfg(feature = "perf_profile")]
  profile: GlyphEngineProfile,
}

impl GlyphEngine {
  pub(crate) fn new() -> Self {
    let mut font_system = FontSystem::new();
    load_platform_fonts(&mut font_system);

    Self {
      font_system,
      swash_context: ScaleContext::new(),
      transformed_scale_context: ScaleContext::new(),
      font_aliases: HashMap::new(),
      measure_cache: HashMap::new(),
      vertical_extents_cache: HashMap::new(),
      caret_cache: HashMap::new(),
      rich_shaped_layout_cache: HashMap::new(),
      glyph_layout_cache: HashMap::new(),
      clipped_glyph_layout_cache: HashMap::new(),
      rich_glyph_layout_cache: HashMap::new(),
      transformed_glyph_layout_cache: HashMap::new(),
      atlas_packer: AtlasPacker::new(),
      atlas_entries: HashMap::new(),
      transformed_atlas_entries: HashMap::new(),
      buffer_pool: Vec::new(),
      measure_hits: 0,
      measure_misses: 0,
      glyph_hits: 0,
      glyph_misses: 0,
      #[cfg(feature = "perf_profile")]
      profile: GlyphEngineProfile::default(),
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
    self.vertical_extents_cache.clear();
    self.caret_cache.clear();
    self.rich_shaped_layout_cache.clear();
    self.glyph_layout_cache.clear();
    self.clipped_glyph_layout_cache.clear();
    self.rich_glyph_layout_cache.clear();
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
    #[cfg(feature = "perf_profile")]
    {
      self.profile = GlyphEngineProfile::default();
    }
  }

  #[cfg_attr(not(feature = "perf_profile"), allow(dead_code))]
  pub(crate) fn profile(&self) -> GlyphEngineProfile {
    #[cfg(feature = "perf_profile")]
    {
      self.profile
    }
    #[cfg(not(feature = "perf_profile"))]
    {
      GlyphEngineProfile::default()
    }
  }

  pub(crate) fn measure_text(&mut self, text: &str, style: &TextStyle, max_width: f32) -> Size {
    let wrap = is_bounded_text_width(max_width);
    let fingerprint = text_measure_fingerprint(text, style, max_width, wrap);
    if let Some(cached) = self
      .measure_cache
      .get(&fingerprint)
      .and_then(|bucket| {
        bucket
          .iter()
          .find(|(key, _)| key.matches_measure(text, style, max_width, wrap))
      })
      .map(|(_, size)| *size)
    {
      self.measure_hits += 1;
      return cached;
    }
    self.measure_misses += 1;
    let size = self.shape_and_measure(text, style, max_width, wrap);
    if self.measure_cache.len() >= GLYPH_LAYOUT_CACHE_LIMIT {
      self.measure_cache.clear();
    }
    self
      .measure_cache
      .entry(fingerprint)
      .or_default()
      .push((CacheKey::new(text, style, max_width, wrap), size));
    size
  }

  /// Vertical extents of `text` relative to the render origin used by
  /// `rasterize_text_*`. Whitespace-only or empty text yields `None`. See
  /// [`TextVerticalExtents`] for how the ink vs. optical (cap-height) boxes are
  /// used by the different vertical-align modes.
  pub(crate) fn text_vertical_extents(
    &mut self,
    text: &str,
    style: &TextStyle,
    max_width: f32,
    wrap: bool,
  ) -> Option<TextVerticalExtents> {
    let fingerprint = text_measure_fingerprint(text, style, max_width, wrap);
    if let Some(cached) = self
      .vertical_extents_cache
      .get(&fingerprint)
      .and_then(|bucket| {
        bucket
          .iter()
          .find(|(key, _)| key.matches_measure(text, style, max_width, wrap))
      })
      .map(|(_, extents)| *extents)
    {
      self.measure_hits += 1;
      return cached;
    }
    self.measure_misses += 1;
    let extents = self.compute_text_vertical_extents(text, style, max_width, wrap);
    if self.vertical_extents_cache.len() >= GLYPH_LAYOUT_CACHE_LIMIT {
      self.vertical_extents_cache.clear();
    }
    self
      .vertical_extents_cache
      .entry(fingerprint)
      .or_default()
      .push((CacheKey::new(text, style, max_width, wrap), extents));
    extents
  }

  fn compute_text_vertical_extents(
    &mut self,
    text: &str,
    style: &TextStyle,
    max_width: f32,
    wrap: bool,
  ) -> Option<TextVerticalExtents> {
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

    let mut top = f32::INFINITY;
    let mut bottom = f32::NEG_INFINITY;
    let mut first: Option<(cosmic_text::fontdb::ID, f32, f32)> = None;
    for run in buffer.layout_runs() {
      for glyph in run.glyphs.iter() {
        if glyph_cluster_is_whitespace(run.text, glyph) {
          continue;
        }
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
        if packed.height == 0 {
          continue;
        }
        let glyph_top = run.line_y + glyph.y - y_offset - packed.top as f32;
        top = top.min(glyph_top);
        bottom = bottom.max(glyph_top + packed.height as f32);
        if first.is_none() {
          first = Some((glyph.font_id, run.line_y + glyph.y - y_offset, glyph.font_size));
        }
      }
    }

    self.buffer_pool.push(buffer);
    if bottom < top {
      return None;
    }

    // Optical box = the font's cap-height box on the first line's baseline,
    // which is descender- and content-independent. Icon fonts usually report no
    // usable cap height, so fall back to the ink box for them.
    let (optical_top, optical_bottom) = first
      .and_then(|(font_id, baseline, font_size)| {
        let cap_px = self.font_cap_height_px(font_id, font_size)?;
        // Only trust a plausible text cap height. Icon/symbol fonts report 0 or
        // a full-em value; those fall back to ink so the glyph shape itself is
        // centered (which keeps icons aligned with adjacent cap-centered text).
        (cap_px > font_size * 0.4 && cap_px < font_size * 0.95).then_some((baseline - cap_px, baseline))
      })
      .unwrap_or((top, bottom));

    Some(TextVerticalExtents {
      ink_top: top,
      ink_bottom: bottom,
      optical_top,
      optical_bottom,
    })
  }

  /// Cap height of `font_id` in pixels at `font_size`, if the font exposes a
  /// usable one (icon/symbol fonts often report 0).
  fn font_cap_height_px(&mut self, font_id: cosmic_text::fontdb::ID, font_size: f32) -> Option<f32> {
    let font = self.font_system.get_font(font_id)?;
    let metrics = font.as_swash().metrics(&[]);
    let upem = metrics.units_per_em as f32;
    if upem <= 0.0 || metrics.cap_height <= 0.0 {
      return None;
    }
    Some(metrics.cap_height * font_size / upem)
  }

  #[cfg_attr(not(feature = "markdown"), allow(dead_code))]
  pub(crate) fn measure_rich_text(&mut self, spans: &[RichTextSpan], max_width: f32) -> Size {
    if spans.is_empty() {
      return Size::default();
    }
    if let [span] = spans {
      return self.measure_text(&span.text, &span.style, max_width);
    }
    let wrap = is_bounded_text_width(max_width);
    let fingerprint = rich_text_shape_fingerprint(spans, max_width, wrap);
    if let Some(cached) = self.find_rich_shaped_layout(fingerprint, spans, max_width, wrap) {
      self.measure_hits += 1;
      return cached.size;
    }

    self.measure_misses += 1;
    let layout = self.shape_rich_text_layout(spans, max_width, wrap);
    let size = layout.size;
    if self.rich_shaped_layout_cache.len() >= GLYPH_LAYOUT_CACHE_LIMIT {
      self.rich_shaped_layout_cache.clear();
    }
    self
      .rich_shaped_layout_cache
      .entry(fingerprint)
      .or_default()
      .push((RichTextShapeKey::new(spans, max_width, wrap), layout));
    size
  }

  pub(crate) fn caret_positions(
    &mut self,
    text: &str,
    style: &TextStyle,
    max_width: f32,
    wrap: bool,
  ) -> Vec<CaretPosition> {
    // Selectable text recomputes caret positions on every layout; cache them
    // like measurements — a full per-character shaping walk per node per pass
    // makes lists of selectable text crawl.
    let fingerprint = text_measure_fingerprint(text, style, max_width, wrap);
    if let Some(cached) = self
      .caret_cache
      .get(&fingerprint)
      .and_then(|bucket| {
        bucket
          .iter()
          .find(|(key, _)| key.matches_measure(text, style, max_width, wrap))
      })
      .map(|(_, positions)| positions.clone())
    {
      return cached;
    }
    let positions = self.compute_caret_positions(text, style, max_width, wrap);
    if self.caret_cache.len() >= GLYPH_LAYOUT_CACHE_LIMIT {
      self.caret_cache.clear();
    }
    self
      .caret_cache
      .entry(fingerprint)
      .or_default()
      .push((CacheKey::new(text, style, max_width, wrap), positions.clone()));
    positions
  }

  fn compute_caret_positions(
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
      None,
      &mut glyphs,
    );
    glyphs
  }

  #[allow(dead_code)]
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
    self.rasterize_text_with_snap_into(text, style, max_width, wrap, origin_x, origin_y, true, None, out);
  }

  #[allow(clippy::too_many_arguments)]
  pub(crate) fn rasterize_text_with_wrap_clipped_into(
    &mut self,
    text: &str,
    style: &TextStyle,
    max_width: f32,
    wrap: bool,
    origin_x: f32,
    origin_y: f32,
    clip: ClipRect,
    out: &mut Vec<GlyphCmd>,
  ) {
    self.rasterize_text_with_snap_into(text, style, max_width, wrap, origin_x, origin_y, true, Some(clip), out);
  }

  #[allow(dead_code)]
  pub(crate) fn rasterize_rich_text_with_wrap_into(
    &mut self,
    spans: &[RichTextSpan],
    max_width: f32,
    wrap: bool,
    origin_x: f32,
    origin_y: f32,
    out: &mut Vec<GlyphCmd>,
  ) {
    self.rasterize_rich_text_with_snap_into(spans, max_width, wrap, origin_x, origin_y, true, out);
  }

  #[allow(clippy::too_many_arguments)]
  pub(crate) fn rasterize_rich_text_with_wrap_clipped_into(
    &mut self,
    spans: &[RichTextSpan],
    max_width: f32,
    wrap: bool,
    origin_x: f32,
    origin_y: f32,
    clip: ClipRect,
    out: &mut Vec<GlyphCmd>,
  ) {
    if let [span] = spans {
      self.rasterize_text_with_wrap_clipped_into(
        &span.text,
        &span.style,
        max_width,
        wrap,
        origin_x,
        origin_y,
        clip,
        out,
      );
      return;
    }
    self.rasterize_rich_text_with_snap_into(spans, max_width, wrap, origin_x, origin_y, true, out);
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
      None,
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
    self.rasterize_text_with_snap_into(text, style, max_width, wrap, origin_x, origin_y, false, None, out);
  }

  pub(crate) fn rasterize_rich_text_unsnapped_with_wrap_into(
    &mut self,
    spans: &[RichTextSpan],
    max_width: f32,
    wrap: bool,
    origin_x: f32,
    origin_y: f32,
    out: &mut Vec<GlyphCmd>,
  ) {
    self.rasterize_rich_text_with_snap_into(spans, max_width, wrap, origin_x, origin_y, false, out);
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
        if glyph_cluster_is_whitespace(run.text, glyph) {
          continue;
        }
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
    self.transformed_glyph_layout_cache.insert(key, cached);
  }

  #[allow(clippy::too_many_arguments)]
  pub(crate) fn rasterize_rich_text_with_baked_transform_into(
    &mut self,
    spans: &[RichTextSpan],
    max_width: f32,
    wrap: bool,
    origin_x: f32,
    origin_y: f32,
    transform: Transform2D,
    transform_origin: [f32; 2],
    out: &mut Vec<GlyphCmd>,
  ) {
    let Some(first) = spans.first() else {
      return;
    };
    if spans.len() == 1 {
      self.rasterize_text_with_baked_transform_into(
        &first.text,
        &first.style,
        max_width,
        wrap,
        origin_x,
        origin_y,
        transform,
        transform_origin,
        out,
      );
      return;
    }
    let wrap = effective_text_wrap(max_width, wrap);
    let swash_transform = swash_transform_from_screen(transform);
    let mut buffer = self.acquire_buffer(&first.style, max_width, wrap);
    self.set_rich_buffer_text(&mut buffer, spans);
    buffer.shape_until_scroll(&mut self.font_system, false);

    for run in buffer.layout_runs() {
      for glyph in run.glyphs.iter() {
        if glyph_cluster_is_whitespace(run.text, glyph) {
          continue;
        }
        let x_offset = glyph.font_size * glyph.x_offset;
        let y_offset = glyph.font_size * glyph.y_offset;
        let (cache_key, ..) = GlyphCacheKey::new(
          glyph.font_id,
          glyph.glyph_id,
          glyph.font_size,
          (0.0, 0.0),
          glyph.cache_key_flags,
        );
        let Some(packed) = self.get_or_pack_transformed_glyph(cache_key, swash_transform) else {
          continue;
        };
        let (transformed_origin_x, transformed_origin_y) = transformed_glyph_origin(
          origin_x,
          origin_y,
          glyph.x + x_offset,
          run.line_y + glyph.y - y_offset,
          transform,
          transform_origin,
        );
        self.push_glyph_cmd(
          out,
          transformed_origin_x + packed.left as f32,
          transformed_origin_y - packed.top as f32,
          packed,
          glyph_color(glyph.color_opt, first.style.color),
          false,
        );
      }
    }
    self.buffer_pool.push(buffer);
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
    clip: Option<ClipRect>,
    out: &mut Vec<GlyphCmd>,
  ) {
    let wrap = effective_text_wrap(max_width, wrap);
    let key = CacheKey::new_for_raster(text, style, max_width, wrap, snap_to_pixel);
    let atlas_w = self.atlas_packer.width as f32;
    let atlas_h = self.atlas_packer.height as f32;
    // `LURQ_GLYPH_DEBUG=<substring>` dumps every emission of a matching text
    // run: cache path, geometry, atlas coords, and the resulting UVs — the
    // ground truth for "this text is invisible on screen" hunts.
    let debug_marker = glyph_debug_marker().filter(|marker| text.contains(marker.as_str()));
    if let Some(marker) = &debug_marker {
      eprintln!(
        "[glyph-debug:{marker}] run text={text:?} size={} family={:?} origin=({origin_x}, {origin_y}) \
         max_width={max_width} snap={snap_to_pixel} clip={} atlas={atlas_w}x{atlas_h}",
        style.font_size,
        style.font_family,
        clip_debug_text(clip),
      );
    }
    if let Some(cached) = self.glyph_layout_cache.get(&key) {
      if let Some(marker) = &debug_marker {
        for glyph in cached.iter() {
          eprintln!(
            "[glyph-debug:{marker}] HIT glyph at ({}, {}) {}x{} atlas=({}, {})",
            origin_x + glyph.x,
            origin_y + glyph.y,
            glyph.width,
            glyph.height,
            glyph.atlas_x,
            glyph.atlas_y,
          );
        }
      }
      self.glyph_hits += cached.len();
      #[cfg(feature = "perf_profile")]
      let append_start = Instant::now();
      append_glyph_cmds_from_cached(
        cached,
        origin_x,
        origin_y,
        style,
        atlas_w,
        atlas_h,
        snap_to_pixel,
        clip,
        out,
      );
      #[cfg(feature = "perf_profile")]
      {
        self.profile.append_cached += append_start.elapsed();
      }
      return;
    }
    let clipped_key = clip.and_then(|clip| ClippedCacheKey::new(key.clone(), origin_x, origin_y, clip));
    if let Some(clipped_key) = clipped_key.as_ref() {
      if let Some(cached) = self.clipped_glyph_layout_cache.get(clipped_key) {
        self.glyph_hits += cached.len();
        #[cfg(feature = "perf_profile")]
        let append_start = Instant::now();
        append_glyph_cmds_from_cached(
          cached,
          origin_x,
          origin_y,
          style,
          atlas_w,
          atlas_h,
          snap_to_pixel,
          clip,
          out,
        );
        #[cfg(feature = "perf_profile")]
        {
          self.profile.append_cached += append_start.elapsed();
        }
        return;
      }
    }

    let mut buffer = self.acquire_buffer(style, max_width, wrap);
    if let Some(height) = clipped_raster_shape_height(origin_y, style, clip) {
      buffer.set_size(&mut self.font_system, text_buffer_width(max_width), Some(height));
    }
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
    let mut skipped_run_for_clip = false;
    for run in buffer.layout_runs() {
      if !text_run_intersects_clip(origin_y, run.line_top, run.line_height, clip) {
        if let Some(marker) = &debug_marker {
          eprintln!(
            "[glyph-debug:{marker}] MISS line SKIPPED by clip: line_top={} line_height={} origin_y={origin_y} clip={}",
            run.line_top,
            run.line_height,
            clip_debug_text(clip),
          );
        }
        skipped_run_for_clip = true;
        continue;
      }
      for glyph in run.glyphs.iter() {
        if glyph_cluster_is_whitespace(run.text, glyph) {
          continue;
        }
        let cached_glyph = if snap_to_pixel {
          let physical = glyph.physical((0.0, run.line_y), 1.0);
          let Some(packed) = self.get_or_pack_glyph(physical.cache_key) else {
            if let Some(marker) = &debug_marker {
              eprintln!(
                "[glyph-debug:{marker}] MISS glyph {} FAILED to rasterize/pack (font_id={:?})",
                glyph.glyph_id, physical.cache_key.font_id,
              );
            }
            continue;
          };

          CachedGlyph {
            x: (physical.x + packed.left) as f32,
            y: (physical.y - packed.top) as f32,
            atlas_x: packed.x,
            atlas_y: packed.y,
            width: packed.width,
            height: packed.height,
            is_color: packed.is_color,
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
            if let Some(marker) = &debug_marker {
              eprintln!(
                "[glyph-debug:{marker}] MISS glyph {} FAILED to rasterize/pack (font_id={:?})",
                glyph.glyph_id, glyph.font_id,
              );
            }
            continue;
          };

          CachedGlyph {
            x: glyph.x + x_offset + packed.left as f32,
            y: run.line_y + glyph.y - y_offset - packed.top as f32,
            atlas_x: packed.x,
            atlas_y: packed.y,
            width: packed.width,
            height: packed.height,
            is_color: packed.is_color,
          }
        };

        cached.push(cached_glyph);
      }
    }

    self.buffer_pool.push(buffer);
    let atlas_w = self.atlas_packer.width as f32;
    let atlas_h = self.atlas_packer.height as f32;
    if let Some(marker) = &debug_marker {
      eprintln!(
        "[glyph-debug:{marker}] MISS rasterized {} glyphs, skipped_for_clip={skipped_run_for_clip}, atlas now {atlas_w}x{atlas_h}",
        cached.len(),
      );
      for glyph in &cached {
        eprintln!(
          "[glyph-debug:{marker}] MISS glyph at ({}, {}) {}x{} atlas=({}, {})",
          origin_x + glyph.x,
          origin_y + glyph.y,
          glyph.width,
          glyph.height,
          glyph.atlas_x,
          glyph.atlas_y,
        );
      }
    }
    #[cfg(feature = "perf_profile")]
    let append_start = Instant::now();
    append_glyph_cmds_from_cached(
      &cached,
      origin_x,
      origin_y,
      style,
      atlas_w,
      atlas_h,
      snap_to_pixel,
      clip,
      out,
    );
    #[cfg(feature = "perf_profile")]
    {
      self.profile.append_cached += append_start.elapsed();
    }
    if !skipped_run_for_clip {
      if self.glyph_layout_cache.len() >= GLYPH_LAYOUT_CACHE_LIMIT {
        self.glyph_layout_cache.clear();
      }
      self.glyph_layout_cache.insert(key, cached);
    } else if let Some(clipped_key) = clipped_key {
      if self.clipped_glyph_layout_cache.len() >= GLYPH_LAYOUT_CACHE_LIMIT {
        self.clipped_glyph_layout_cache.clear();
      }
      self.clipped_glyph_layout_cache.insert(clipped_key, cached);
    }
  }

  #[allow(clippy::too_many_arguments)]
  fn rasterize_rich_text_with_snap_into(
    &mut self,
    spans: &[RichTextSpan],
    max_width: f32,
    wrap: bool,
    origin_x: f32,
    origin_y: f32,
    snap_to_pixel: bool,
    out: &mut Vec<GlyphCmd>,
  ) {
    let Some(first) = spans.first() else {
      return;
    };
    if spans.len() == 1 {
      self.rasterize_text_with_snap_into(
        &first.text,
        &first.style,
        max_width,
        wrap,
        origin_x,
        origin_y,
        snap_to_pixel,
        None,
        out,
      );
      return;
    }
    let wrap = effective_text_wrap(max_width, wrap);
    let fingerprint = rich_text_raster_fingerprint(spans, max_width, wrap, snap_to_pixel);
    let atlas_w = self.atlas_packer.width as f32;
    let atlas_h = self.atlas_packer.height as f32;
    if let Some(cached) = self.find_rich_glyph_layout(fingerprint, spans, max_width, wrap, snap_to_pixel) {
      let hit_count = cached.len();
      #[cfg(feature = "perf_profile")]
      let append_start = Instant::now();
      append_rich_glyph_cmds_from_cached(cached, origin_x, origin_y, atlas_w, atlas_h, snap_to_pixel, out);
      #[cfg(feature = "perf_profile")]
      {
        self.profile.append_cached += append_start.elapsed();
      }
      self.glyph_hits += hit_count;
      return;
    }

    if snap_to_pixel {
      let shape_fingerprint = rich_text_shape_fingerprint(spans, max_width, wrap);
      if let Some(shaped) = self.find_rich_shaped_layout(shape_fingerprint, spans, max_width, wrap) {
        let cached = self.pack_rich_shaped_layout(&shaped);
        if self.rich_glyph_layout_cache.len() >= GLYPH_LAYOUT_CACHE_LIMIT {
          self.rich_glyph_layout_cache.clear();
        }
        let atlas_w = self.atlas_packer.width as f32;
        let atlas_h = self.atlas_packer.height as f32;
        #[cfg(feature = "perf_profile")]
        let append_start = Instant::now();
        append_rich_glyph_cmds_from_cached(&cached, origin_x, origin_y, atlas_w, atlas_h, snap_to_pixel, out);
        #[cfg(feature = "perf_profile")]
        {
          self.profile.append_cached += append_start.elapsed();
        }
        self.rich_glyph_layout_cache.entry(fingerprint).or_default().push((
          RichTextCacheKey::new_for_raster(spans, max_width, wrap, snap_to_pixel),
          cached,
        ));
        return;
      }

      let shaped = self.shape_rich_text_layout(spans, max_width, wrap);
      let cached = self.pack_rich_shaped_layout(&shaped);
      if self.rich_shaped_layout_cache.len() >= GLYPH_LAYOUT_CACHE_LIMIT {
        self.rich_shaped_layout_cache.clear();
      }
      self
        .rich_shaped_layout_cache
        .entry(shape_fingerprint)
        .or_default()
        .push((RichTextShapeKey::new(spans, max_width, wrap), shaped));
      if self.rich_glyph_layout_cache.len() >= GLYPH_LAYOUT_CACHE_LIMIT {
        self.rich_glyph_layout_cache.clear();
      }
      let atlas_w = self.atlas_packer.width as f32;
      let atlas_h = self.atlas_packer.height as f32;
      #[cfg(feature = "perf_profile")]
      let append_start = Instant::now();
      append_rich_glyph_cmds_from_cached(&cached, origin_x, origin_y, atlas_w, atlas_h, snap_to_pixel, out);
      #[cfg(feature = "perf_profile")]
      {
        self.profile.append_cached += append_start.elapsed();
      }
      self.rich_glyph_layout_cache.entry(fingerprint).or_default().push((
        RichTextCacheKey::new_for_raster(spans, max_width, wrap, snap_to_pixel),
        cached,
      ));
      return;
    }

    let mut buffer = self.acquire_buffer(&first.style, max_width, wrap);
    self.set_rich_buffer_text(&mut buffer, spans);
    buffer.shape_until_scroll(&mut self.font_system, false);

    let mut cached = Vec::new();
    for run in buffer.layout_runs() {
      for glyph in run.glyphs.iter() {
        if glyph_cluster_is_whitespace(run.text, glyph) {
          continue;
        }
        let default_color = first.style.color;
        let color = glyph_color(glyph.color_opt, default_color);
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
            is_color: packed.is_color,
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
            is_color: packed.is_color,
          }
        };

        cached.push(CachedRichGlyph {
          glyph: cached_glyph,
          color,
        });
      }
    }
    self.buffer_pool.push(buffer);
    if self.rich_glyph_layout_cache.len() >= GLYPH_LAYOUT_CACHE_LIMIT {
      self.rich_glyph_layout_cache.clear();
    }
    let atlas_w = self.atlas_packer.width as f32;
    let atlas_h = self.atlas_packer.height as f32;
    #[cfg(feature = "perf_profile")]
    let append_start = Instant::now();
    append_rich_glyph_cmds_from_cached(&cached, origin_x, origin_y, atlas_w, atlas_h, snap_to_pixel, out);
    #[cfg(feature = "perf_profile")]
    {
      self.profile.append_cached += append_start.elapsed();
    }
    self.rich_glyph_layout_cache.entry(fingerprint).or_default().push((
      RichTextCacheKey::new_for_raster(spans, max_width, wrap, snap_to_pixel),
      cached,
    ));
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
        color_glyph: packed.is_color,
        shadow_sigma: 0.0,
        clip: crate::layout::quad::ClipRect::default(),
      });
    }
  }

  pub(crate) fn atlas(&mut self) -> GlyphAtlas {
    self.atlas_packer.to_atlas()
  }

  fn get_or_pack_glyph(&mut self, cache_key: GlyphCacheKey) -> Option<PackedGlyph> {
    if let Some(&packed) = self.atlas_entries.get(&cache_key) {
      self.glyph_hits += 1;
      return Some(packed);
    }

    let font = self.font_system.get_font(cache_key.font_id)?;
    #[cfg(feature = "perf_profile")]
    {
      self.profile.swash_requests += 1;
    }
    #[cfg(feature = "perf_profile")]
    let swash_start = Instant::now();
    let image = render_glyph_image(&mut self.swash_context, &font, cache_key);
    #[cfg(feature = "perf_profile")]
    {
      self.profile.swash_lookup += swash_start.elapsed();
    }
    let Some(image) = image else {
      self.glyph_misses += 1;
      return None;
    };
    if image.placement.width == 0 || image.placement.height == 0 {
      return None;
    }

    #[cfg(feature = "perf_profile")]
    let atlas_pack_start = Instant::now();
    let (x, y, width, height, is_color) = self.atlas_packer.pack_image(&image);
    #[cfg(feature = "perf_profile")]
    {
      self.profile.atlas_pack += atlas_pack_start.elapsed();
      self.profile.atlas_packs += 1;
    }
    let packed = PackedGlyph {
      x,
      y,
      width,
      height,
      left: image.placement.left - GLYPH_ATLAS_PADDING as i32,
      top: image.placement.top + GLYPH_ATLAS_PADDING as i32,
      is_color,
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
    #[cfg(feature = "perf_profile")]
    {
      self.profile.swash_requests += 1;
    }
    #[cfg(feature = "perf_profile")]
    let swash_start = Instant::now();
    let image = Render::new(&[
      Source::ColorOutline(0),
      Source::ColorBitmap(StrikeWith::BestFit),
      Source::Outline,
    ])
    .format(Format::Alpha)
    .offset(offset)
    .transform(Some(transform))
    .render(&mut scaler, cache_key.glyph_id);
    #[cfg(feature = "perf_profile")]
    {
      self.profile.swash_lookup += swash_start.elapsed();
    }
    let image = image?;
    if image.placement.width == 0 || image.placement.height == 0 {
      return None;
    }

    #[cfg(feature = "perf_profile")]
    let atlas_pack_start = Instant::now();
    let (x, y, width, height, is_color) = self.atlas_packer.pack_image(&image);
    #[cfg(feature = "perf_profile")]
    {
      self.profile.atlas_pack += atlas_pack_start.elapsed();
      self.profile.atlas_packs += 1;
    }
    let packed = PackedGlyph {
      x,
      y,
      width,
      height,
      left: image.placement.left - GLYPH_ATLAS_PADDING as i32,
      top: image.placement.top + GLYPH_ATLAS_PADDING as i32,
      is_color,
    };
    self.transformed_atlas_entries.insert(key, packed);
    self.glyph_misses += 1;
    Some(packed)
  }

  fn shape_and_measure(&mut self, text: &str, style: &TextStyle, max_width: f32, wrap: bool) -> Size {
    #[cfg(feature = "perf_profile")]
    let profile_start = Instant::now();
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
    #[cfg(feature = "perf_profile")]
    {
      self.profile.shape_text += profile_start.elapsed();
    }
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

  fn shape_rich_text_layout(&mut self, spans: &[RichTextSpan], max_width: f32, wrap: bool) -> CachedRichShapedLayout {
    let Some(first) = spans.first() else {
      return CachedRichShapedLayout {
        size: Size::default(),
        glyphs: Vec::<CachedRichShapedGlyph>::new().into(),
      };
    };
    #[cfg(feature = "perf_profile")]
    let profile_start = Instant::now();

    #[cfg(feature = "perf_profile")]
    let phase_start = Instant::now();
    let mut buffer = self.acquire_buffer(&first.style, max_width, wrap);
    #[cfg(feature = "perf_profile")]
    {
      self.profile.rich_acquire_buffer += phase_start.elapsed();
    }

    #[cfg(feature = "perf_profile")]
    let phase_start = Instant::now();
    self.set_rich_buffer_text(&mut buffer, spans);
    #[cfg(feature = "perf_profile")]
    {
      self.profile.rich_set_text += phase_start.elapsed();
    }

    #[cfg(feature = "perf_profile")]
    let phase_start = Instant::now();
    buffer.shape_until_scroll(&mut self.font_system, false);
    #[cfg(feature = "perf_profile")]
    {
      self.profile.rich_cosmic_shape += phase_start.elapsed();
    }

    #[cfg(feature = "perf_profile")]
    let phase_start = Instant::now();
    let size = measure_buffer(&buffer, first.style.font_size * first.style.line_height);
    #[cfg(feature = "perf_profile")]
    {
      self.profile.rich_measure += phase_start.elapsed();
    }

    #[cfg(feature = "perf_profile")]
    let phase_start = Instant::now();
    let mut glyphs = Vec::new();
    for run in buffer.layout_runs() {
      for glyph in run.glyphs.iter() {
        if glyph_cluster_is_whitespace(run.text, glyph) {
          continue;
        }
        let physical = glyph.physical((0.0, run.line_y), 1.0);
        glyphs.push(CachedRichShapedGlyph {
          x: physical.x as f32,
          y: physical.y as f32,
          cache_key: physical.cache_key,
          color: glyph_color(glyph.color_opt, first.style.color),
        });
      }
    }
    #[cfg(feature = "perf_profile")]
    {
      self.profile.rich_extract += phase_start.elapsed();
    }

    self.buffer_pool.push(buffer);
    #[cfg(feature = "perf_profile")]
    {
      self.profile.shape_rich_text += profile_start.elapsed();
    }
    CachedRichShapedLayout {
      size,
      glyphs: glyphs.into(),
    }
  }

  fn pack_rich_shaped_layout(&mut self, layout: &CachedRichShapedLayout) -> Vec<CachedRichGlyph> {
    #[cfg(feature = "perf_profile")]
    let profile_start = Instant::now();
    let mut cached = Vec::with_capacity(layout.glyphs.len());
    for glyph in layout.glyphs.iter() {
      let Some(packed) = self.get_or_pack_glyph(glyph.cache_key) else {
        continue;
      };
      cached.push(CachedRichGlyph {
        glyph: CachedGlyph {
          x: glyph.x + packed.left as f32,
          y: glyph.y - packed.top as f32,
          atlas_x: packed.x,
          atlas_y: packed.y,
          width: packed.width,
          height: packed.height,
          is_color: packed.is_color,
        },
        color: glyph.color,
      });
    }
    #[cfg(feature = "perf_profile")]
    {
      self.profile.pack_rich_shaped += profile_start.elapsed();
    }
    cached
  }

  fn find_rich_shaped_layout(
    &self,
    fingerprint: u64,
    spans: &[RichTextSpan],
    max_width: f32,
    wrap: bool,
  ) -> Option<CachedRichShapedLayout> {
    self
      .rich_shaped_layout_cache
      .get(&fingerprint)?
      .iter()
      .find(|(key, _)| key.matches(spans, max_width, wrap))
      .map(|(_, layout)| layout.clone())
  }

  fn find_rich_glyph_layout(
    &self,
    fingerprint: u64,
    spans: &[RichTextSpan],
    max_width: f32,
    wrap: bool,
    snap_to_pixel: bool,
  ) -> Option<&[CachedRichGlyph]> {
    self
      .rich_glyph_layout_cache
      .get(&fingerprint)?
      .iter()
      .find(|(key, _)| key.matches(spans, max_width, wrap, snap_to_pixel))
      .map(|(_, glyphs)| glyphs.as_slice())
  }

  fn set_rich_buffer_text(&mut self, buffer: &mut Buffer, spans: &[RichTextSpan]) {
    let Some(first) = spans.first() else {
      return;
    };
    #[cfg(feature = "perf_profile")]
    {
      self.profile.rich_text_loads += 1;
      self.profile.rich_loaded_spans += spans.len();
      self.profile.rich_loaded_bytes += spans.iter().map(|span| span.text.len()).sum::<usize>();
      if spans.len() == 1 {
        self.profile.rich_single_span_loads += 1;
      } else {
        self.profile.rich_multi_span_loads += 1;
      }
    }

    if spans.len() == 1 {
      #[cfg(feature = "perf_profile")]
      let phase_start = Instant::now();
      if self.font_aliases.is_empty() {
        let attrs = attrs_for_style(&first.style, first.style.font_family.as_ref());
        #[cfg(feature = "perf_profile")]
        {
          self.profile.rich_prepare_spans += phase_start.elapsed();
        }

        #[cfg(feature = "perf_profile")]
        let phase_start = Instant::now();
        set_buffer_text(
          buffer,
          &mut self.font_system,
          &first.text,
          attrs,
          first.style.text_align,
        );
        #[cfg(feature = "perf_profile")]
        {
          self.profile.rich_buffer_set_text += phase_start.elapsed();
        }
      } else {
        let family = self.resolve_family(&first.style);
        let attrs = attrs_for_style(&first.style, &family);
        #[cfg(feature = "perf_profile")]
        {
          self.profile.rich_prepare_spans += phase_start.elapsed();
        }

        #[cfg(feature = "perf_profile")]
        let phase_start = Instant::now();
        set_buffer_text(
          buffer,
          &mut self.font_system,
          &first.text,
          attrs,
          first.style.text_align,
        );
        #[cfg(feature = "perf_profile")]
        {
          self.profile.rich_buffer_set_text += phase_start.elapsed();
        }
      }
      return;
    }

    if self.font_aliases.is_empty() {
      #[cfg(feature = "perf_profile")]
      let phase_start = Instant::now();
      let rich_spans: Vec<_> = spans
        .iter()
        .map(|span| {
          (
            span.text.as_str(),
            attrs_for_style(&span.style, span.style.font_family.as_ref()),
          )
        })
        .collect();
      let default_attrs = attrs_for_style(&first.style, first.style.font_family.as_ref());
      #[cfg(feature = "perf_profile")]
      {
        self.profile.rich_prepare_spans += phase_start.elapsed();
      }

      #[cfg(feature = "perf_profile")]
      let phase_start = Instant::now();
      buffer.set_rich_text(&mut self.font_system, rich_spans, default_attrs, Shaping::Advanced);
      #[cfg(feature = "perf_profile")]
      {
        self.profile.rich_buffer_set_text += phase_start.elapsed();
      }
    } else {
      #[cfg(feature = "perf_profile")]
      let phase_start = Instant::now();
      let families: Vec<_> = spans.iter().map(|span| self.resolve_family(&span.style)).collect();
      let rich_spans: Vec<_> = spans
        .iter()
        .zip(families.iter())
        .map(|(span, family)| (span.text.as_str(), attrs_for_style(&span.style, family)))
        .collect();
      let default_family = self.resolve_family(&first.style);
      let default_attrs = attrs_for_style(&first.style, &default_family);
      #[cfg(feature = "perf_profile")]
      {
        self.profile.rich_prepare_spans += phase_start.elapsed();
      }

      #[cfg(feature = "perf_profile")]
      let phase_start = Instant::now();
      buffer.set_rich_text(&mut self.font_system, rich_spans, default_attrs, Shaping::Advanced);
      #[cfg(feature = "perf_profile")]
      {
        self.profile.rich_buffer_set_text += phase_start.elapsed();
      }
    }
    #[cfg(feature = "perf_profile")]
    let phase_start = Instant::now();
    for line in &mut buffer.lines {
      line.set_align(Some(first.style.text_align.to_cosmic()));
    }
    #[cfg(feature = "perf_profile")]
    {
      self.profile.rich_align_lines += phase_start.elapsed();
    }
  }

  fn push_glyph_cmd(
    &self,
    out: &mut Vec<GlyphCmd>,
    x: f32,
    y: f32,
    packed: PackedGlyph,
    color: [f32; 4],
    snap_to_pixel: bool,
  ) {
    let atlas_w = self.atlas_packer.width as f32;
    let atlas_h = self.atlas_packer.height as f32;
    out.push(GlyphCmd {
      order: 0,
      x: if snap_to_pixel { x.round() } else { x },
      y: if snap_to_pixel { y.round() } else { y },
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
      color_glyph: packed.is_color,
      shadow_sigma: 0.0,
      clip: crate::layout::quad::ClipRect::default(),
    });
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
      .values()
      .flat_map(|bucket| bucket.iter().map(|(key, _)| key))
      .map(|key| key.text.capacity() + key.font_family.len())
      .sum::<usize>();
    let measure_cache_bucket_bytes = self
      .measure_cache
      .values()
      .map(|bucket| bucket.capacity() * std::mem::size_of::<(CacheKey, Size)>())
      .sum::<usize>();
    let rich_shaped_key_heap = self
      .rich_shaped_layout_cache
      .values()
      .flat_map(|bucket| bucket.iter().flat_map(|(key, _)| key.spans.iter()))
      .map(|span| span.text.capacity() + span.font_family.len())
      .sum::<usize>();
    let rich_shaped_layout_cache_bytes = self
      .rich_shaped_layout_cache
      .values()
      .flat_map(|bucket| bucket.iter().map(|(_, layout)| layout))
      .map(|layout| layout.glyphs.len() * std::mem::size_of::<CachedRichShapedGlyph>())
      .sum::<usize>();
    let glyph_layout_cache_bytes = self
      .glyph_layout_cache
      .values()
      .map(|glyphs| glyphs.capacity() * std::mem::size_of::<CachedGlyph>())
      .sum::<usize>();
    let clipped_key_heap = self
      .clipped_glyph_layout_cache
      .keys()
      .map(|key| key.base.text.capacity() + key.base.font_family.len())
      .sum::<usize>();
    let clipped_glyph_layout_cache_bytes = self
      .clipped_glyph_layout_cache
      .values()
      .map(|glyphs| glyphs.capacity() * std::mem::size_of::<CachedGlyph>())
      .sum::<usize>();
    let rich_key_heap = self
      .rich_glyph_layout_cache
      .values()
      .flat_map(|bucket| bucket.iter().flat_map(|(key, _)| key.spans.iter()))
      .map(|span| span.text.capacity() + span.font_family.len())
      .sum::<usize>();
    let rich_glyph_layout_cache_bytes = self
      .rich_glyph_layout_cache
      .values()
      .flat_map(|bucket| bucket.iter().map(|(_, glyphs)| glyphs))
      .map(|glyphs| glyphs.capacity() * std::mem::size_of::<CachedRichGlyph>())
      .sum::<usize>();
    let transformed_glyph_layout_cache_bytes = self
      .transformed_glyph_layout_cache
      .values()
      .map(|glyphs| glyphs.capacity() * std::mem::size_of::<CachedTransformedGlyph>())
      .sum::<usize>();

    std::mem::size_of::<Self>()
      + self.font_aliases.capacity() * std::mem::size_of::<(String, String)>()
      + alias_heap
      + self.measure_cache.capacity() * std::mem::size_of::<(u64, Vec<(CacheKey, Size)>)>()
      + measure_cache_bucket_bytes
      + measure_key_heap
      + self.rich_shaped_layout_cache.capacity()
        * std::mem::size_of::<(u64, Vec<(RichTextShapeKey, CachedRichShapedLayout)>)>()
      + self
        .rich_shaped_layout_cache
        .values()
        .map(|bucket| bucket.capacity() * std::mem::size_of::<(RichTextShapeKey, CachedRichShapedLayout)>())
        .sum::<usize>()
      + rich_shaped_key_heap
      + rich_shaped_layout_cache_bytes
      + self.glyph_layout_cache.capacity() * std::mem::size_of::<(CacheKey, Vec<CachedGlyph>)>()
      + glyph_layout_cache_bytes
      + self.clipped_glyph_layout_cache.capacity() * std::mem::size_of::<(ClippedCacheKey, Vec<CachedGlyph>)>()
      + clipped_key_heap
      + clipped_glyph_layout_cache_bytes
      + self.rich_glyph_layout_cache.capacity()
        * std::mem::size_of::<(u64, Vec<(RichTextCacheKey, Vec<CachedRichGlyph>)>)>()
      + self
        .rich_glyph_layout_cache
        .values()
        .map(|bucket| bucket.capacity() * std::mem::size_of::<(RichTextCacheKey, Vec<CachedRichGlyph>)>())
        .sum::<usize>()
      + rich_key_heap
      + rich_glyph_layout_cache_bytes
      + self.transformed_glyph_layout_cache.capacity() * std::mem::size_of::<(CacheKey, Vec<CachedTransformedGlyph>)>()
      + transformed_glyph_layout_cache_bytes
      + self.atlas_entries.capacity() * std::mem::size_of::<(GlyphCacheKey, PackedGlyph)>()
      + self.transformed_atlas_entries.capacity() * std::mem::size_of::<(TransformedGlyphKey, PackedGlyph)>()
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
  is_color: bool,
}

#[derive(Clone, Copy)]
struct CachedGlyph {
  x: f32,
  y: f32,
  atlas_x: u32,
  atlas_y: u32,
  width: u32,
  height: u32,
  is_color: bool,
}

#[derive(Clone)]
struct CachedRichShapedLayout {
  size: Size,
  glyphs: std::sync::Arc<[CachedRichShapedGlyph]>,
}

#[derive(Clone, Copy)]
struct CachedRichShapedGlyph {
  x: f32,
  y: f32,
  cache_key: GlyphCacheKey,
  color: [f32; 4],
}

#[derive(Clone, Copy)]
struct CachedRichGlyph {
  glyph: CachedGlyph,
  color: [f32; 4],
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

/// The `LURQ_GLYPH_DEBUG` env marker, read once — a substring of the text runs
/// to trace through emission (see `rasterize_text_with_snap_into`).
/// Render an optional clip rect for the glyph-debug lines.
fn clip_debug_text(clip: Option<ClipRect>) -> String {
  match clip {
    Some(clip) => format!(
      "({}, {}, {}x{}, active={})",
      clip.x, clip.y, clip.width, clip.height, clip.active
    ),
    None => "none".to_owned(),
  }
}

pub(crate) fn glyph_debug_marker() -> Option<String> {
  static MARKER: std::sync::OnceLock<Option<String>> = std::sync::OnceLock::new();
  MARKER
    .get_or_init(|| {
      std::env::var("LURQ_GLYPH_DEBUG")
        .ok()
        .filter(|marker| !marker.is_empty())
    })
    .clone()
}

/// `LURQ_GLYPH_DEBUG_FORCE` — draw-command overrides applied to the marked
/// text runs, to bisect why a well-formed run is invisible: `noclip` clears
/// their clip, `order` lifts them above everything, `both` does both.
pub(crate) fn glyph_debug_force() -> Option<String> {
  static FORCE: std::sync::OnceLock<Option<String>> = std::sync::OnceLock::new();
  FORCE
    .get_or_init(|| {
      std::env::var("LURQ_GLYPH_DEBUG_FORCE")
        .ok()
        .filter(|force| !force.is_empty())
    })
    .clone()
}

fn render_glyph_image(context: &mut ScaleContext, font: &Font, cache_key: GlyphCacheKey) -> Option<SwashImage> {
  let mut scaler = context
    .builder(font.as_swash())
    .size(f32::from_bits(cache_key.font_size_bits))
    .hint(true)
    .build();
  let offset = Vector::new(cache_key.x_bin.as_float(), cache_key.y_bin.as_float());
  let transform = cache_key
    .flags
    .contains(cosmic_text::CacheKeyFlags::FAKE_ITALIC)
    .then(|| SwashTransform::skew(Angle::from_degrees(14.0), Angle::from_degrees(0.0)));

  Render::new(&[
    Source::ColorOutline(0),
    Source::ColorBitmap(StrikeWith::BestFit),
    Source::Outline,
  ])
  .format(Format::Alpha)
  .offset(offset)
  .transform(transform)
  .render(&mut scaler, cache_key.glyph_id)
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

fn attrs_for_style<'a>(style: &TextStyle, resolved_family: &'a str) -> Attrs<'a> {
  let family = if resolved_family.is_empty() {
    Family::SansSerif
  } else {
    Family::Name(resolved_family)
  };
  Attrs::new()
    .family(family)
    .weight(style.weight.to_cosmic())
    .style(style.style.to_cosmic())
    .color(CosmicColor::rgba(
      style.color.r(),
      style.color.g(),
      style.color.b(),
      style.color.a(),
    ))
}

fn glyph_color(color: Option<CosmicColor>, default: Color) -> [f32; 4] {
  color
    .map(|color| Color::new(color.r(), color.g(), color.b(), color.a()).to_linear_f32_array())
    .unwrap_or_else(|| default.to_linear_f32_array())
}

fn glyph_cluster_is_whitespace(line_text: &str, glyph: &LayoutGlyph) -> bool {
  cluster_is_whitespace(line_text, glyph.start, glyph.end)
}

/// Whether the source cluster `[start, end)` is non-empty and entirely
/// whitespace. An empty range slices to "", and `"".chars().all(..)` is
/// vacuously true — which would misclassify a glyph with no source bytes as
/// whitespace and skip drawing it (its advance stays, leaving a gap mid-word),
/// so an empty cluster must not count as whitespace.
fn cluster_is_whitespace(line_text: &str, start: usize, end: usize) -> bool {
  match line_text.get(start..end) {
    Some(cluster) => !cluster.is_empty() && cluster.chars().all(char::is_whitespace),
    None => false,
  }
}

#[cfg_attr(not(feature = "markdown"), allow(dead_code))]
fn measure_buffer(buffer: &Buffer, fallback_line_height: f32) -> Size {
  let mut width = 0.0_f32;
  let mut first_line_y = 0.0_f32;
  let mut last_line_y = 0.0_f32;
  let mut last_line_height = fallback_line_height;
  let mut has_runs = false;
  for run in buffer.layout_runs() {
    width = width.max(run.line_w);
    if !has_runs {
      first_line_y = run.line_y;
      has_runs = true;
    }
    last_line_y = run.line_y;
    last_line_height = run.line_height;
  }
  let height = if has_runs {
    last_line_y - first_line_y + last_line_height
  } else {
    0.0
  };
  Size::new(width, height)
}

fn append_glyph_cmds_from_cached(
  cached: &[CachedGlyph],
  origin_x: f32,
  origin_y: f32,
  style: &TextStyle,
  atlas_w: f32,
  atlas_h: f32,
  snap_to_pixel: bool,
  clip: Option<ClipRect>,
  out: &mut Vec<GlyphCmd>,
) {
  let color = style.color.to_linear_f32_array();
  out.reserve(cached.len());
  for glyph in cached {
    let x = origin_x + glyph.x;
    let y = origin_y + glyph.y;
    if !glyph_rect_intersects_clip(x, y, glyph.width as f32, glyph.height as f32, clip) {
      continue;
    }
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
      color_glyph: glyph.is_color,
      shadow_sigma: 0.0,
      clip: crate::layout::quad::ClipRect::default(),
    });
  }
}

fn text_run_intersects_clip(origin_y: f32, line_top: f32, line_height: f32, clip: Option<ClipRect>) -> bool {
  let Some(clip) = clip.filter(|clip| clip.active) else {
    return true;
  };
  let y = origin_y + line_top;
  y < clip.y + clip.height && y + line_height > clip.y
}

fn glyph_rect_intersects_clip(x: f32, y: f32, width: f32, height: f32, clip: Option<ClipRect>) -> bool {
  let Some(clip) = clip.filter(|clip| clip.active) else {
    return true;
  };
  x < clip.x + clip.width && x + width > clip.x && y < clip.y + clip.height && y + height > clip.y
}

fn clipped_raster_shape_height(origin_y: f32, style: &TextStyle, clip: Option<ClipRect>) -> Option<f32> {
  let clip = clip.filter(|clip| clip.active)?;
  let local_bottom = clip.y + clip.height - origin_y;
  let line_slop = style.font_size * style.line_height;
  (local_bottom > 0.0).then_some(local_bottom + line_slop)
}

fn append_rich_glyph_cmds_from_cached(
  cached: &[CachedRichGlyph],
  origin_x: f32,
  origin_y: f32,
  atlas_w: f32,
  atlas_h: f32,
  snap_to_pixel: bool,
  out: &mut Vec<GlyphCmd>,
) {
  out.reserve(cached.len());
  for rich_glyph in cached {
    let glyph = rich_glyph.glyph;
    let x = origin_x + glyph.x;
    let y = origin_y + glyph.y;
    out.push(GlyphCmd {
      order: 0,
      x: if snap_to_pixel { x.round() } else { x },
      y: if snap_to_pixel { y.round() } else { y },
      width: glyph.width as f32,
      height: glyph.height as f32,
      color: rich_glyph.color,
      uv_min: [glyph.atlas_x as f32 / atlas_w, glyph.atlas_y as f32 / atlas_h],
      uv_max: [
        (glyph.atlas_x + glyph.width) as f32 / atlas_w,
        (glyph.atlas_y + glyph.height) as f32 / atlas_h,
      ],
      transform: [1.0, 0.0, 0.0, 1.0],
      transform_origin: [0.0, 0.0],
      sharpness: 1.0,
      color_glyph: glyph.is_color,
      shadow_sigma: 0.0,
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

#[cfg(test)]
fn glyph_coverage_mask(image: &SwashImage) -> Cow<'_, [u8]> {
  match image.content {
    SwashContent::Mask => Cow::Borrowed(&image.data),
    SwashContent::Color => Cow::Owned(image.data.chunks_exact(4).map(|rgba| rgba[3]).collect::<Vec<_>>()),
    SwashContent::SubpixelMask => Cow::Owned(
      image
        .data
        .chunks_exact(4)
        .map(|rgba| {
          let coverage = rgba[0] as u16 + rgba[1] as u16 + rgba[2] as u16;
          (coverage / 3) as u8
        })
        .collect::<Vec<_>>(),
    ),
  }
}

#[cfg(test)]
fn glyph_atlas_pixels(image: &SwashImage) -> (Cow<'_, [u8]>, bool) {
  match image.content {
    SwashContent::Color => (Cow::Borrowed(&image.data), true),
    SwashContent::Mask => (Cow::Owned(alpha_to_rgba(&image.data)), false),
    SwashContent::SubpixelMask => {
      let alpha = image
        .data
        .chunks_exact(3)
        .map(|rgb| {
          let coverage = rgb[0] as u16 + rgb[1] as u16 + rgb[2] as u16;
          (coverage / 3) as u8
        })
        .collect::<Vec<_>>();
      (Cow::Owned(alpha_to_rgba(&alpha)), false)
    }
  }
}

#[cfg(test)]
fn alpha_to_rgba(alpha: &[u8]) -> Vec<u8> {
  let mut rgba = Vec::with_capacity(alpha.len() * 4);
  for coverage in alpha {
    rgba.extend_from_slice(&[255, 255, 255, *coverage]);
  }
  rgba
}

fn write_alpha_row_as_rgba(data: &mut [u8], dst_start: usize, alpha: &[u8]) {
  for (index, coverage) in alpha.iter().enumerate() {
    let dst = dst_start + index * GLYPH_ATLAS_BYTES_PER_PIXEL;
    if dst + 4 > data.len() {
      break;
    }
    data[dst..dst + 4].copy_from_slice(&[255, 255, 255, *coverage]);
  }
}

fn write_subpixel_row_as_rgba(data: &mut [u8], dst_start: usize, subpixel: &[u8]) {
  for (index, rgb) in subpixel.chunks_exact(3).enumerate() {
    let dst = dst_start + index * GLYPH_ATLAS_BYTES_PER_PIXEL;
    if dst + 4 > data.len() {
      break;
    }
    let coverage = (rgb[0] as u16 + rgb[1] as u16 + rgb[2] as u16) / 3;
    data[dst..dst + 4].copy_from_slice(&[255, 255, 255, coverage as u8]);
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

/// Upper bound on retained dirty-log entries; consumers further behind than
/// the pruned tail fall back to a full atlas upload.
const GLYPH_ATLAS_DIRTY_LOG_LIMIT: usize = 512;

pub(crate) struct AtlasPacker {
  pub data: Vec<u8>,
  pub width: u32,
  pub height: u32,
  cursor_x: u32,
  cursor_y: u32,
  row_height: u32,
  version: u64,
  /// Version-tagged log of packed rects (see [`GlyphAtlas::dirty_rects`]).
  /// Retained — not drained per snapshot — because multiple windows consume
  /// the same atlas from different uploaded versions.
  dirty_rects: std::collections::VecDeque<GlyphAtlasDirtyRect>,
  /// Versions at or before this are no longer covered by the log.
  dirty_from_version: u64,
  snapshot_data: std::sync::Arc<[u8]>,
  snapshot_rects: std::sync::Arc<[GlyphAtlasDirtyRect]>,
  snapshot_width: u32,
  snapshot_height: u32,
  snapshot_version: u64,
}

impl AtlasPacker {
  pub(crate) fn new() -> Self {
    let width = 1024;
    let height = 1024;
    Self {
      data: vec![0u8; (width * height) as usize * GLYPH_ATLAS_BYTES_PER_PIXEL],
      width,
      height,
      cursor_x: 0,
      cursor_y: 0,
      row_height: 0,
      version: 0,
      dirty_rects: std::collections::VecDeque::new(),
      dirty_from_version: 0,
      snapshot_data: Vec::<u8>::new().into(),
      snapshot_rects: Vec::new().into(),
      snapshot_width: 0,
      snapshot_height: 0,
      snapshot_version: u64::MAX,
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

  #[cfg(test)]
  fn pack_pixels(&mut self, glyph_data: &[u8], gw: u32, gh: u32) -> (u32, u32, u32, u32) {
    let (x0, y0, reserved_width, reserved_height) = self.reserve_pixels(gw, gh);
    let padding = GLYPH_ATLAS_PADDING;

    for row in 0..gh {
      let src_start = (row * gw) as usize * GLYPH_ATLAS_BYTES_PER_PIXEL;
      let src_end = src_start + gw as usize * GLYPH_ATLAS_BYTES_PER_PIXEL;
      let dst_start = ((y0 + padding + row) * self.width + x0 + padding) as usize * GLYPH_ATLAS_BYTES_PER_PIXEL;
      let dst_end = dst_start + gw as usize * GLYPH_ATLAS_BYTES_PER_PIXEL;
      if src_end <= glyph_data.len() && dst_end <= self.data.len() {
        self.data[dst_start..dst_end].copy_from_slice(&glyph_data[src_start..src_end]);
      }
    }

    self.record_packed_rect(x0, y0, reserved_width, reserved_height);
    (x0, y0, reserved_width, reserved_height)
  }

  fn pack_image(&mut self, image: &SwashImage) -> (u32, u32, u32, u32, bool) {
    let gw = image.placement.width;
    let gh = image.placement.height;
    let (x0, y0, reserved_width, reserved_height) = self.reserve_pixels(gw, gh);
    let padding = GLYPH_ATLAS_PADDING;
    let is_color = matches!(image.content, SwashContent::Color);

    for row in 0..gh {
      let dst_start = ((y0 + padding + row) * self.width + x0 + padding) as usize * GLYPH_ATLAS_BYTES_PER_PIXEL;
      match image.content {
        SwashContent::Color => {
          let src_start = (row * gw) as usize * GLYPH_ATLAS_BYTES_PER_PIXEL;
          let src_end = src_start + gw as usize * GLYPH_ATLAS_BYTES_PER_PIXEL;
          let dst_end = dst_start + gw as usize * GLYPH_ATLAS_BYTES_PER_PIXEL;
          if src_end <= image.data.len() && dst_end <= self.data.len() {
            self.data[dst_start..dst_end].copy_from_slice(&image.data[src_start..src_end]);
          }
        }
        SwashContent::Mask => {
          let src_start = (row * gw) as usize;
          let src_end = src_start + gw as usize;
          if src_end <= image.data.len() {
            write_alpha_row_as_rgba(&mut self.data, dst_start, &image.data[src_start..src_end]);
          }
        }
        SwashContent::SubpixelMask => {
          let src_start = (row * gw) as usize * 3;
          let src_end = src_start + gw as usize * 3;
          if src_end <= image.data.len() {
            write_subpixel_row_as_rgba(&mut self.data, dst_start, &image.data[src_start..src_end]);
          }
        }
      }
    }

    self.record_packed_rect(x0, y0, reserved_width, reserved_height);
    (x0, y0, reserved_width, reserved_height, is_color)
  }

  fn reserve_pixels(&mut self, gw: u32, gh: u32) -> (u32, u32, u32, u32) {
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
      self
        .data
        .resize((self.width * new_height) as usize * GLYPH_ATLAS_BYTES_PER_PIXEL, 0);
      self.height = new_height;
    }

    let padded_x = self.cursor_x;
    let padded_y = self.cursor_y;
    let x0 = padded_x;
    let y0 = padded_y;

    self.cursor_x += reserved_width;
    self.row_height = self.row_height.max(reserved_height);
    (x0, y0, reserved_width, reserved_height)
  }

  fn record_packed_rect(&mut self, x0: u32, y0: u32, reserved_width: u32, reserved_height: u32) {
    self.version += 1;
    let rect = GlyphAtlasDirtyRect {
      x: x0,
      y: y0,
      width: reserved_width,
      height: reserved_height,
      version: self.version,
    };
    // Merge adjacent same-row packs as they land so glyph bursts don't blow
    // up the log (the merged rect carries the newest version).
    match self.dirty_rects.back_mut() {
      Some(last) if should_merge_dirty_rects(*last, rect) => *last = union_dirty_rect(*last, rect),
      _ => self.dirty_rects.push_back(rect),
    }
    while self.dirty_rects.len() > GLYPH_ATLAS_DIRTY_LOG_LIMIT {
      if let Some(removed) = self.dirty_rects.pop_front() {
        self.dirty_from_version = removed.version;
      }
    }
  }

  pub(crate) fn to_atlas(&mut self) -> GlyphAtlas {
    if self.snapshot_version != self.version || self.snapshot_width != self.width || self.snapshot_height != self.height
    {
      self.snapshot_data = std::sync::Arc::from(self.data.clone());
      self.snapshot_rects = self.dirty_rects.iter().copied().collect();
      self.snapshot_width = self.width;
      self.snapshot_height = self.height;
      self.snapshot_version = self.version;
    }

    GlyphAtlas {
      data: self.snapshot_data.clone(),
      width: self.width,
      height: self.height,
      version: self.version,
      dirty_rects: self.snapshot_rects.clone(),
      dirty_from_version: self.dirty_from_version,
    }
  }
}

/// The dirty rects a consumer whose GPU texture is at `uploaded_version` must
/// apply to reach `atlas.version` — `None` when the log no longer covers the
/// gap and the full atlas has to be re-uploaded instead.
pub(crate) fn atlas_rects_to_apply(atlas: &GlyphAtlas, uploaded_version: u64) -> Option<Vec<GlyphAtlasDirtyRect>> {
  if uploaded_version < atlas.dirty_from_version {
    return None;
  }
  let pending: Vec<GlyphAtlasDirtyRect> = atlas
    .dirty_rects
    .iter()
    .filter(|rect| rect.version > uploaded_version)
    .copied()
    .collect();
  if pending.is_empty() && uploaded_version < atlas.version {
    // The version moved but the log doesn't show how — stay safe.
    return None;
  }
  Some(coalesce_dirty_rects(pending))
}

fn coalesce_dirty_rects(mut rects: Vec<GlyphAtlasDirtyRect>) -> Vec<GlyphAtlasDirtyRect> {
  if rects.len() <= 1 {
    return rects;
  }

  rects.sort_by_key(|rect| (rect.y, rect.x));
  let mut merged: Vec<GlyphAtlasDirtyRect> = Vec::with_capacity(rects.len());
  for rect in rects {
    if rect.width == 0 || rect.height == 0 {
      continue;
    }

    if let Some(last) = merged.last_mut() {
      if should_merge_dirty_rects(*last, rect) {
        *last = union_dirty_rect(*last, rect);
        continue;
      }
    }

    merged.push(rect);
  }

  merged
}

fn should_merge_dirty_rects(a: GlyphAtlasDirtyRect, b: GlyphAtlasDirtyRect) -> bool {
  if a.y != b.y {
    return false;
  }

  let gap = horizontal_gap(a, b);
  if gap > DIRTY_RECT_MAX_HORIZONTAL_GAP {
    return false;
  }

  let area_sum = dirty_rect_area(a) + dirty_rect_area(b);
  let merged_area = dirty_rect_area(union_dirty_rect(a, b));
  merged_area * DIRTY_RECT_MERGE_WASTE_DENOMINATOR <= area_sum * DIRTY_RECT_MERGE_WASTE_NUMERATOR
}

fn horizontal_gap(a: GlyphAtlasDirtyRect, b: GlyphAtlasDirtyRect) -> u32 {
  let a_right = a.x.saturating_add(a.width);
  let b_right = b.x.saturating_add(b.width);
  if a_right < b.x {
    b.x - a_right
  } else if b_right < a.x {
    a.x - b_right
  } else {
    0
  }
}

fn union_dirty_rect(a: GlyphAtlasDirtyRect, b: GlyphAtlasDirtyRect) -> GlyphAtlasDirtyRect {
  let x0 = a.x.min(b.x);
  let y0 = a.y.min(b.y);
  let x1 = a.x.saturating_add(a.width).max(b.x.saturating_add(b.width));
  let y1 = a.y.saturating_add(a.height).max(b.y.saturating_add(b.height));
  GlyphAtlasDirtyRect {
    x: x0,
    y: y0,
    width: x1.saturating_sub(x0),
    height: y1.saturating_sub(y0),
    version: a.version.max(b.version),
  }
}

fn dirty_rect_area(rect: GlyphAtlasDirtyRect) -> u64 {
  u64::from(rect.width) * u64::from(rect.height)
}

#[cfg(test)]
mod tests {
  use cosmic_text::{Attrs, Family, Placement, Shaping, SwashContent, SwashImage};
  use swash::scale::ScaleContext;

  use super::{
    AtlasPacker, GLYPH_ATLAS_BYTES_PER_PIXEL, GLYPH_ATLAS_PADDING, GlyphAtlasDirtyRect, GlyphEngine,
    cluster_is_whitespace, coalesce_dirty_rects, glyph_atlas_pixels, glyph_coverage_mask, is_bounded_text_width,
    render_glyph_image, swash_transform_from_screen,
  };
  use crate::{layout::quad::ClipRect, node::transform::Transform2D};

  #[test]
  fn empty_cluster_is_not_treated_as_whitespace() {
    // A glyph whose source-cluster range is empty (start == end) must not be
    // skipped as whitespace, otherwise it drops out of the word while keeping
    // its advance, leaving a visible gap mid-word.
    assert!(!cluster_is_whitespace("items", 3, 3));
    assert!(!cluster_is_whitespace("items", 3, 4)); // "m" — a real glyph
    assert!(cluster_is_whitespace("a b", 1, 2)); // the space
    assert!(!cluster_is_whitespace("items", 10, 12)); // out of range
  }

  #[test]
  fn atlas_packer_leaves_padding_between_glyph_regions() {
    let mut packer = AtlasPacker::new();
    let (_, _, first_u1, _) = packer.pack(&[255; 16], 2, 2);
    let (second_u0, ..) = packer.pack(&[255; 16], 2, 2);

    let first_x1 = (first_u1 * packer.width as f32).round() as u32;
    let second_x0 = (second_u0 * packer.width as f32).round() as u32;

    assert!(second_x0 >= first_x1);
  }

  #[test]
  fn atlas_packer_leaves_transparent_glyph_padding() {
    let mut packer = AtlasPacker::new();
    let pixels = [
      10, 11, 12, 13, //
      20, 21, 22, 23, //
      30, 31, 32, 33, //
      40, 41, 42, 43,
    ];
    let (x, y, width, height) = packer.pack_pixels(&pixels, 2, 2);
    let p = GLYPH_ATLAS_PADDING as usize;
    let stride = packer.width as usize;
    let pixel = |x: usize, y: usize| (y * stride + x) * GLYPH_ATLAS_BYTES_PER_PIXEL;

    assert_eq!(
      (x, y, width, height),
      (0, 0, 2 + GLYPH_ATLAS_PADDING * 2, 2 + GLYPH_ATLAS_PADDING * 2)
    );
    assert_eq!(packer.data[0], 0);
    assert_eq!(&packer.data[pixel(p, p)..pixel(p, p) + 4], &[10, 11, 12, 13]);
    assert_eq!(&packer.data[pixel(p + 1, p)..pixel(p + 1, p) + 4], &[20, 21, 22, 23]);
    assert_eq!(&packer.data[pixel(p, p + 1)..pixel(p, p + 1) + 4], &[30, 31, 32, 33]);
    assert_eq!(
      &packer.data[pixel(p + 1, p + 1)..pixel(p + 1, p + 1) + 4],
      &[40, 41, 42, 43]
    );
    assert_eq!(
      &packer.data[pixel(p + 2, p + 2)..pixel(p + 2, p + 2) + 4],
      &[0, 0, 0, 0]
    );
  }

  #[test]
  fn color_glyph_atlas_pixels_preserve_rgba() {
    let image = SwashImage {
      content: SwashContent::Color,
      placement: Placement {
        left: 0,
        top: 0,
        width: 1,
        height: 1,
      },
      data: vec![12, 34, 56, 78],
      ..SwashImage::default()
    };

    let (pixels, is_color) = glyph_atlas_pixels(&image);

    assert!(is_color);
    assert_eq!(pixels.as_ref(), &[12, 34, 56, 78]);
  }

  #[test]
  fn mask_glyph_atlas_pixels_store_alpha_coverage() {
    let image = SwashImage {
      content: SwashContent::Mask,
      placement: Placement {
        left: 0,
        top: 0,
        width: 2,
        height: 1,
      },
      data: vec![10, 240],
      ..SwashImage::default()
    };

    let (pixels, is_color) = glyph_atlas_pixels(&image);

    assert!(!is_color);
    assert_eq!(pixels.as_ref(), &[255, 255, 255, 10, 255, 255, 255, 240]);
  }

  #[test]
  fn atlas_snapshot_reuses_data_when_version_is_unchanged() {
    let mut packer = AtlasPacker::new();
    let first = packer.to_atlas();
    let second = packer.to_atlas();

    assert!(std::sync::Arc::ptr_eq(&first.data, &second.data));

    packer.pack_pixels(&[255; 4], 2, 2);
    let third = packer.to_atlas();

    assert!(!std::sync::Arc::ptr_eq(&second.data, &third.data));
    assert_eq!(third.dirty_rects.len(), 1);

    // The dirty log is retained (not drained) so every window's render
    // engine can catch up from its own uploaded version.
    let fourth = packer.to_atlas();
    assert_eq!(fourth.dirty_rects.len(), 1);
    assert!(std::sync::Arc::ptr_eq(&third.dirty_rects, &fourth.dirty_rects));
    assert!(std::sync::Arc::ptr_eq(&third.data, &fourth.data));
  }

  #[test]
  fn atlas_dirty_log_serves_consumers_at_different_versions() {
    let mut packer = AtlasPacker::new();
    packer.pack_pixels(&[255; 4], 2, 2);
    let snap1 = packer.to_atlas();
    // Consumer A uploads snap1 fully; its texture is now at snap1.version.
    let a_version = snap1.version;

    packer.pack_pixels(&[255; 4], 2, 2);
    packer.pack_pixels(&[255; 4], 2, 2);
    let snap2 = packer.to_atlas();

    // Consumer A only needs the packs after its version.
    let a_rects = super::atlas_rects_to_apply(&snap2, a_version).expect("log covers consumer A");
    assert!(a_rects.iter().all(|rect| rect.version > a_version));
    assert!(!a_rects.is_empty());

    // Consumer B never uploaded any rects (texture at version 0) — the log
    // must still cover it, including the packs consumer A already applied.
    let b_rects = super::atlas_rects_to_apply(&snap2, 0).expect("log covers consumer B");
    let b_area: u64 = b_rects.iter().map(|rect| super::dirty_rect_area(*rect)).sum();
    let a_area: u64 = a_rects.iter().map(|rect| super::dirty_rect_area(*rect)).sum();
    assert!(
      b_area >= a_area,
      "stale consumer must receive at least as much as a fresh one"
    );

    // Up to date: nothing to apply.
    let none = super::atlas_rects_to_apply(&snap2, snap2.version).expect("up-to-date consumer");
    assert!(none.is_empty());
  }

  #[test]
  fn atlas_dirty_log_prunes_to_full_upload_for_stale_consumers() {
    let mut packer = AtlasPacker::new();
    packer.pack_pixels(&[255; 4], 2, 2);
    let stale_version = packer.to_atlas().version;

    // Overflow the log with widely-spaced packs (each on its own atlas row so
    // they never merge away).
    for _ in 0..(super::GLYPH_ATLAS_DIRTY_LOG_LIMIT + 8) {
      packer.pack_pixels(&[255; 4096], 1000, 1);
    }
    let snap = packer.to_atlas();

    assert!(snap.dirty_from_version > stale_version);
    // A consumer behind the pruned tail must full-upload...
    assert!(super::atlas_rects_to_apply(&snap, stale_version).is_none());
    // ...while one at the tail can still use the log.
    assert!(super::atlas_rects_to_apply(&snap, snap.dirty_from_version).is_some());
  }

  #[test]
  fn dirty_rect_coalescing_merges_same_atlas_row() {
    let rects = coalesce_dirty_rects(vec![
      GlyphAtlasDirtyRect {
        x: 0,
        y: 12,
        width: 8,
        height: 10,
        version: 1,
      },
      GlyphAtlasDirtyRect {
        x: 8,
        y: 12,
        width: 7,
        height: 10,
        version: 2,
      },
      GlyphAtlasDirtyRect {
        x: 15,
        y: 12,
        width: 9,
        height: 10,
        version: 3,
      },
    ]);

    assert_eq!(rects.len(), 1);
    assert_eq!(
      rects[0],
      GlyphAtlasDirtyRect {
        x: 0,
        y: 12,
        width: 24,
        height: 10,
        version: 3,
      }
    );
  }

  #[test]
  fn dirty_rect_coalescing_keeps_separate_atlas_rows() {
    let rects = coalesce_dirty_rects(vec![
      GlyphAtlasDirtyRect {
        x: 0,
        y: 12,
        width: 20,
        height: 10,
        version: 1,
      },
      GlyphAtlasDirtyRect {
        x: 0,
        y: 22,
        width: 20,
        height: 10,
        version: 2,
      },
    ]);

    assert_eq!(rects.len(), 2);
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
    let font = engine
      .font_system
      .get_font(physical.cache_key.font_id)
      .expect("the y glyph font should be available");
    let mut context = ScaleContext::new();
    let image = render_glyph_image(&mut context, &font, physical.cache_key).expect("the y glyph should rasterize");
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

  /// Tiny UI text (chips/pills render around 7-8px) must still rasterize
  /// visible coverage — a correctly-sized but all-zero mask packs into the
  /// atlas and draws as invisible text, which no glyph-command-level check
  /// catches.
  #[test]
  fn tiny_font_sizes_rasterize_nonzero_coverage() {
    let mut engine = GlyphEngine::new();
    for weight in [
      crate::layout::text_style::FontWeight::Normal,
      crate::layout::text_style::FontWeight::Bold,
    ] {
      for font_size in [7.7_f32, 8.4, 9.1, 11.0] {
        let style = crate::layout::text_style::TextStyle {
          weight,
          font_size,
          ..crate::layout::text_style::TextStyle::default()
        };
        let mut buffer = engine.acquire_buffer(&style, 100.0, true);
        let resolved = engine.resolve_family(&style);
        let attrs = Attrs::new()
          .family(Family::Name(&resolved))
          .weight(style.weight.to_cosmic())
          .style(style.style.to_cosmic());
        buffer.set_text(&mut engine.font_system, "2363", attrs, Shaping::Advanced);
        buffer.shape_until_scroll(&mut engine.font_system, false);

        let mut glyph_count = 0;
        for run in buffer.layout_runs() {
          for glyph in run.glyphs.iter() {
            glyph_count += 1;
            // A glyph must rasterize visibly at every subpixel offset it can
            // land on, not just the whole-pixel bin.
            for x_offset in [0.0_f32, 0.25, 0.5, 0.75] {
              for y_offset in [0.0_f32, 0.25, 0.5, 0.75] {
                let physical = glyph.physical((x_offset, run.line_y + y_offset), 1.0);
                let font = engine
                  .font_system
                  .get_font(physical.cache_key.font_id)
                  .expect("glyph font should be available");
                let mut context = ScaleContext::new();
                let image = render_glyph_image(&mut context, &font, physical.cache_key);
                let visible = image
                  .as_ref()
                  .is_some_and(|image| glyph_coverage_mask(image).iter().any(|coverage| *coverage > 0));
                assert!(
                  visible,
                  "{font_size}px glyph {} at subpixel ({x_offset}, {y_offset}) rasterized invisible",
                  glyph.glyph_id
                );
              }
            }
          }
        }
        engine.buffer_pool.push(buffer);

        assert!(glyph_count > 0, "digits should shape at {font_size}px");
      }
    }
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
  fn whitespace_only_text_does_not_pack_glyphs() {
    let mut engine = GlyphEngine::new();
    let style = crate::layout::text_style::TextStyle {
      font_size: 16.0,
      ..crate::layout::text_style::TextStyle::default()
    };

    let glyphs = engine.rasterize_text(" \t  ", &style, 400.0, 0.0, 0.0);

    assert!(glyphs.is_empty());
    assert_eq!(engine.glyph_hits, 0);
    assert_eq!(engine.glyph_misses, 0);
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
  fn clipped_wrapped_text_only_appends_intersecting_glyphs() {
    let mut engine = GlyphEngine::new();
    let style = crate::layout::text_style::TextStyle {
      font_size: 16.0,
      ..crate::layout::text_style::TextStyle::default()
    };
    let text = "alpha beta gamma delta epsilon zeta eta theta iota kappa lambda mu nu xi omicron pi rho sigma tau";
    let max_width = 96.0;

    let full = engine.rasterize_text(text, &style, max_width, 0.0, 0.0);
    let clip = ClipRect {
      x: 0.0,
      y: 0.0,
      width: max_width,
      height: 36.0,
      active: true,
      border_radius: None,
    };
    let mut clipped = Vec::new();
    engine.rasterize_text_with_wrap_clipped_into(text, &style, max_width, true, 0.0, 0.0, clip, &mut clipped);

    assert!(!clipped.is_empty());
    assert!(clipped.len() < full.len());
    let clipped_len = clipped.len();
    for glyph in clipped {
      assert!(
        glyph.x < clip.x + clip.width
          && glyph.x + glyph.width > clip.x
          && glyph.y < clip.y + clip.height
          && glyph.y + glyph.height > clip.y,
        "clipped rasterization appended a glyph outside the clip: x={} y={} w={} h={}",
        glyph.x,
        glyph.y,
        glyph.width,
        glyph.height
      );
    }

    engine.reset_stats();
    let mut cached = Vec::new();
    engine.rasterize_text_with_wrap_clipped_into(text, &style, max_width, true, 0.0, 0.0, clip, &mut cached);

    assert_eq!(cached.len(), clipped_len);
    assert!(engine.glyph_hits >= cached.len());
    assert_eq!(engine.glyph_misses, 0);
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

  #[test]
  fn rich_text_rasterization_reuses_layout_cache() {
    let mut engine = GlyphEngine::new();
    let base = crate::layout::text_style::TextStyle {
      font_size: 16.0,
      ..crate::layout::text_style::TextStyle::default()
    };
    let mut accent = base.clone();
    accent.color = crate::node::color::Color::from_hex("#2563eb");
    let spans = vec![
      crate::layout::quad::RichTextSpan {
        text: "Hello ".to_owned(),
        style: base,
      },
      crate::layout::quad::RichTextSpan {
        text: "rich text cache".to_owned(),
        style: accent,
      },
    ];

    let mut first = Vec::new();
    engine.rasterize_rich_text_with_wrap_into(&spans, 320.0, true, 0.0, 0.0, &mut first);
    assert!(!first.is_empty());

    engine.reset_stats();
    let mut second = Vec::new();
    engine.rasterize_rich_text_with_wrap_into(&spans, 320.0, true, 10.0, 20.0, &mut second);

    assert_eq!(first.len(), second.len());
    assert!(
      engine.glyph_hits >= second.len(),
      "second rich-text rasterization should hit the rich glyph layout cache"
    );
    assert_eq!(first[0].color, second[0].color);
    assert_eq!(first.last().unwrap().color, second.last().unwrap().color);
  }

  #[test]
  fn measured_rich_text_layout_feeds_rasterization() {
    let mut engine = GlyphEngine::new();
    let style = crate::layout::text_style::TextStyle {
      font_size: 16.0,
      ..crate::layout::text_style::TextStyle::default()
    };
    let spans = vec![
      crate::layout::quad::RichTextSpan {
        text: "Measured rich text ".to_owned(),
        style: style.clone(),
      },
      crate::layout::quad::RichTextSpan {
        text: "can be rasterized without shaping it again.".to_owned(),
        style,
      },
    ];

    let measured = engine.measure_rich_text(&spans, 260.0);
    assert!(measured.width > 0.0);
    assert_eq!(engine.rich_shaped_layout_cache.len(), 1);

    engine.reset_stats();
    let mut glyphs = Vec::new();
    engine.rasterize_rich_text_with_wrap_into(&spans, 260.0, true, 0.0, 0.0, &mut glyphs);

    assert!(!glyphs.is_empty());
    assert_eq!(engine.rich_shaped_layout_cache.len(), 1);
    assert_eq!(engine.rich_glyph_layout_cache.len(), 1);
  }
}
