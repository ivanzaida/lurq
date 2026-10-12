//! Whole-paragraph input shaping; cached geometry is also the paint payload.
use std::{borrow::Cow, collections::VecDeque, ops::Range, sync::Arc};

use cosmic_text::{LayoutGlyph, Metrics, Shaping};

use super::{GlyphEngine, attrs_for_style};
use crate::{
  layout::{
    Size,
    text_style::{TextShadow, TextStyle},
  },
  node::{
    color::Color,
    text_selection::{CaretPosition, CaretPositions, TextSelectionRange},
  },
};

mod alignment;
mod geometry;
mod justification;
mod paint;
mod rows;
#[cfg(test)]
#[path = "../../../../tests/input/text_input/scroll_geometry.rs"]
mod scroll_tests;
#[cfg(test)]
#[path = "../../../../tests/input/text_input/rtl_geometry.rs"]
mod tests;

#[derive(Clone, PartialEq)]
pub(crate) struct TokenStyle {
  pub(crate) range: Range<usize>,
  pub(crate) text: TextStyle,
  pub(crate) padding: f32,
  pub(crate) fill: Option<(Color, f32)>,
}

#[derive(Clone, PartialEq)]
pub(crate) struct InputConfig {
  pub(crate) source: Arc<str>,
  pub(crate) base: TextStyle,
  pub(crate) styles: Vec<TokenStyle>,
  pub(crate) width: f32,
  pub(crate) min_width: f32,
  pub(crate) wrap: bool,
}

#[derive(Clone, Copy)]
pub(crate) struct InputCaret {
  pub(crate) byte: usize,
  pub(crate) x: f32,
  pub(crate) y: f32,
  pub(crate) height: f32,
  pub(crate) font_size: f32,
  pub(crate) row: usize,
}

pub(crate) struct InputGlyph {
  pub(crate) glyph: LayoutGlyph,
  pub(crate) baseline: f32,
  row: usize,
  pub(crate) color: Color,
  pub(crate) shadow: Option<TextShadow>,
}

pub(crate) struct InputFragment {
  pub(crate) x: f32,
  pub(crate) y: f32,
  pub(crate) width: f32,
  pub(crate) height: f32,
  pub(crate) color: Color,
  pub(crate) radius: f32,
  row: usize,
}

pub(crate) struct ClusterInterval {
  range: Range<usize>,
  x: f32,
  right: f32,
  y: f32,
}

/// Opaque validated input paint geometry. Only the toolkit can construct or change it.
#[doc(hidden)]
pub struct DecoratedInputLayout {
  pub(crate) config: InputConfig,
  pub(crate) revision: u64,
  pub(crate) glyphs: Vec<InputGlyph>,
  pub(crate) fragments: Vec<InputFragment>,
  pub(crate) carets: Vec<InputCaret>,
  rows: Vec<InputRow>,
  intervals: Vec<ClusterInterval>,
  pub(crate) size: Size,
  pub(crate) line_height: f32,
  pub(crate) viewport_width: f32,
}

struct InputRow {
  range: Range<usize>,
  y: f32,
  height: f32,
  band_top: f32,
  band_height: f32,
  left: f32,
  width: f32,
  rtl: bool,
}

impl std::fmt::Debug for DecoratedInputLayout {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("DecoratedInputLayout").finish_non_exhaustive()
  }
}

#[derive(Default)]
pub(crate) struct InputCache {
  revision: u64,
  entries: VecDeque<Arc<DecoratedInputLayout>>,
}

impl InputCache {
  pub(super) fn clear(&mut self) {
    self.revision = self.revision.wrapping_add(1);
    self.entries.clear();
  }
  fn find(&self, key: &InputConfig) -> Option<Arc<DecoratedInputLayout>> {
    self.entries.iter().find(|layout| &layout.config == key).cloned()
  }
  pub(super) fn estimated_memory_bytes(&self) -> usize {
    self.entries.capacity() * std::mem::size_of::<Arc<DecoratedInputLayout>>()
      + self
        .entries
        .iter()
        .map(|layout| layout.allocated_bytes())
        .sum::<usize>()
  }
  fn insert(&mut self, layout: Arc<DecoratedInputLayout>) {
    // A fixed entry count alone is not a text-memory bound.
    while self.entries.len() >= 16
      || self.entries.iter().map(|entry| entry.allocated_bytes()).sum::<usize>() + layout.allocated_bytes()
        > 8 * 1024 * 1024
    {
      if self.entries.pop_front().is_none() {
        break;
      }
    }
    if layout.allocated_bytes() <= 8 * 1024 * 1024 {
      self.entries.push_back(layout);
    }
  }
}

impl GlyphEngine {
  pub(crate) fn decorated_input(&mut self, config: InputConfig) -> Arc<DecoratedInputLayout> {
    if let Some(layout) = self.input_cache.find(&config) {
      return layout;
    }
    let mut output = DecoratedInputLayout {
      config,
      revision: self.input_cache.revision,
      glyphs: Vec::new(),
      fragments: Vec::new(),
      carets: Vec::new(),
      rows: Vec::new(),
      intervals: Vec::new(),
      size: Size::default(),
      line_height: 1.0,
      viewport_width: 0.0,
    };
    self.shape_input(&mut output);
    let output = Arc::new(output);
    self.input_cache.insert(output.clone());
    output
  }

  fn shape_input(&mut self, output: &mut DecoratedInputLayout) {
    let config = &output.config;
    // CR and LF each become one ASCII space in Scroll, preserving every raw
    // UTF-8 byte offset. Only presentation is flattened; config.source stays raw.
    let presentation = if config.wrap {
      Cow::Borrowed(config.source.as_ref())
    } else {
      Cow::Owned(config.source.replace(['\r', '\n'], " "))
    };
    let spans = input_spans(config, &presentation);
    let families: Vec<_> = spans.iter().map(|(_, style, _)| self.resolve_family(style)).collect();
    let attrs: Vec<_> = spans
      .iter()
      .zip(&families)
      .map(|((text, style, metadata), family)| {
        let weight = self.face_weight(family, style);
        // Unlike generic rich spans, these attrs carry explicit metric overrides;
        // tracking must therefore use each glyph's effective em size.
        let attrs = attrs_for_style(style, family, weight, style.font_size)
          .metrics(Metrics::new(style.font_size, style.font_size * style.line_height))
          .metadata(*metadata);
        (*text, attrs)
      })
      .collect();
    let family = self.resolve_family(&config.base);
    let weight = self.face_weight(&family, &config.base);
    let default_attrs = attrs_for_style(&config.base, &family, weight, config.base.font_size);
    let mut buffer = self.acquire_buffer(&config.base, f32::MAX, false);
    buffer.set_rich_text(attrs, &default_attrs, Shaping::Advanced, None);
    let mut offset = 0;
    let mut prepared = Vec::new();
    for line in &mut buffer.lines {
      let bytes = line.text().len();
      let shape = line.shape(&mut self.font_system, 8);
      rows::paragraph(&mut prepared, shape, offset, bytes, &output.config);
      offset += line.text().len() + line.ending().as_str().len();
    }
    rows::finish(output, prepared);
    // set_rich_text does not retain a final empty paragraph. The editor must
    // still expose the raw boundary after its hard line ending.
    if output.config.wrap
      && output.config.source.ends_with(['\r', '\n'])
      && output
        .carets
        .last()
        .is_none_or(|caret| caret.byte < output.config.source.len())
    {
      let end = output.config.source.len();
      output.append_empty_row(end, end);
    }
    self.buffer_pool.push(buffer);
    output.finish_carets();
    output.align_viewport();
  }
}

fn input_spans<'a>(config: &'a InputConfig, presentation: &'a str) -> Vec<(&'a str, &'a TextStyle, usize)> {
  let mut spans = Vec::new();
  let mut start = 0;
  for (index, token) in config.styles.iter().enumerate() {
    if start < token.range.start {
      spans.push((&presentation[start..token.range.start], &config.base, 0));
    }
    spans.push((&presentation[token.range.clone()], &token.text, index + 1));
    start = token.range.end;
  }
  if start < config.source.len() {
    spans.push((&presentation[start..], &config.base, 0));
  }
  if spans.is_empty() {
    spans.push(("", &config.base, 0));
  }
  spans
}
