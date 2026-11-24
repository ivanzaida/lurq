//! Whole-paragraph input shaping; cached geometry is also the paint payload.
use std::{borrow::Cow, collections::VecDeque, ops::Range, sync::Arc};

use cosmic_text::LayoutGlyph;

use super::GlyphEngine;
#[cfg(test)]
use super::attrs_for_style;
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
mod chunks;
#[cfg(test)]
#[path = "../../../../tests/input/text_input/crlf_geometry.rs"]
mod crlf_tests;
mod geometry;
mod justification;
mod paint;
#[cfg(test)]
#[path = "../../../../tests/input/text_input/paint_oracle.rs"]
pub(crate) mod paint_oracle;
mod paragraphs;
mod pieces;
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
  /// Lines under and through the range's text, in its color.
  pub(crate) lines: TextLines,
  /// What a token has besides its fill: its own room at its end, its height, its edge and its icon.
  pub(crate) chip: TokenChip,
}

impl TokenStyle {
  /// The room after the range's text: its padding, or the chip's own.
  pub(crate) fn padding_end(&self) -> f32 {
    self.chip.padding_end.unwrap_or(self.padding)
  }
}

/// What an input token has besides the fill of a styled range. A range with the default has none of it.
#[derive(Clone, Default, PartialEq)]
pub(crate) struct TokenChip {
  /// The room after the text, in place of the padding: the end's own, and the icon's with its gap.
  pub(crate) padding_end: Option<f32>,
  /// The height of the fill, in place of its line's.
  pub(crate) height: Option<f32>,
  /// A line inside the fill: its width and color.
  pub(crate) border: Option<(f32, Color)>,
  pub(crate) icon: Option<TokenIcon>,
  /// The token is one thing to the caret: at its ends the caret stands outside its fill, before its padding and
  /// after the room at its end, and never between its text and its icon.
  pub(crate) atomic: bool,
  /// The selection covers the token and it has a look of its own for that: its fill is drawn over the selection's
  /// highlight, which would otherwise lie over it.
  pub(crate) over_selection: bool,
  /// The range is no text but a box of this width and height that stands on the baseline of its row: its glyph
  /// draws nothing and takes the box's width, and the row is as high as its tallest box asks. What an object of
  /// a styled input is laid out as ([`InputObjectBox`]).
  pub(crate) object: Option<(f32, f32)>,
}

/// An icon at the end of a token, drawn in the room its padding keeps for it.
#[derive(Clone, PartialEq)]
pub(crate) struct TokenIcon {
  /// The glyph's text, family, size and color.
  pub(crate) style: TextStyle,
  pub(crate) glyph: char,
  pub(crate) size: f32,
  /// The room from the icon to the end of the fill.
  pub(crate) inset: f32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct TextLines {
  pub(crate) underline: bool,
  pub(crate) strikethrough: bool,
}

/// The room a row keeps after a box that is as wide as it can be, for the caret that stands there.
pub(crate) const OBJECT_CARET_ROOM: f32 = 2.0;

/// Where the box of a range that is an object stands ([`TokenChip::object`]): the byte its range starts at, and
/// its place in the coordinates the glyphs and the carets have.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct InputObjectBox {
  pub(crate) byte: usize,
  pub(crate) x: f32,
  pub(crate) y: f32,
  pub(crate) width: f32,
  pub(crate) height: f32,
  row: usize,
}

#[derive(Clone, PartialEq)]
pub(crate) struct InputConfig {
  pub(crate) source: Arc<str>,
  pub(crate) base: TextStyle,
  pub(crate) styles: Vec<TokenStyle>,
  pub(crate) width: f32,
  pub(crate) min_width: f32,
  pub(crate) wrap: bool,
  /// How each hard paragraph of a wrapped input is laid out, in order, where the input has kinds of paragraphs;
  /// a paragraph past the end of it, and every one of an input without kinds, is laid out in `base` from the
  /// start of the input.
  pub(crate) paragraphs: Vec<ParagraphFormat>,
  /// Rows are as high as the line height of the type says, whatever the font's own line is: a font whose ascent
  /// and descent are more than the line draws past it, as in CSS. A paragraph with a kind is laid out so, for a
  /// kind's rows to be the same in every font; any other row is never lower than its font's line.
  pub(crate) exact_rows: bool,
}

impl InputConfig {
  /// The size of the box the style `token` is, where it is one: its own, or with a width that is more than the
  /// row's, as wide as the row and as high as keeps its shape. The row keeps the room of a caret after its
  /// widest box, so that the caret after a box as wide as the row is inside the input and seen.
  pub(crate) fn object_size(&self, token: usize) -> Option<(f32, f32)> {
    let (width, height) = self.styles.get(token)?.chip.object?;
    let (width, height) = (width.max(0.0), height.max(0.0));
    let room = (self.width - OBJECT_CARET_ROOM).max(0.0);
    if width > room && room.is_finite() && width > 0.0 {
      return Some((room, height * (room / width)));
    }
    Some((width, height))
  }

  fn has_objects(&self) -> bool {
    self.styles.iter().any(|style| style.chip.object.is_some())
  }
}

/// How one hard paragraph is laid out and what is drawn before it.
#[derive(Clone, PartialEq)]
pub(crate) struct ParagraphFormat {
  /// The paragraph's type, in place of the input's.
  pub(crate) base: TextStyle,
  /// The room before its text.
  pub(crate) indent: f32,
  /// The room between it and the paragraph before.
  pub(crate) gap_before: f32,
  /// The room above it where it starts a run of its kind, and under it where it ends one, inside its fill and
  /// its bar.
  pub(crate) padding: (f32, f32),
  pub(crate) marker: ParagraphMark,
  /// The type of the marker's text, in place of the paragraph's: its baseline is the first row's.
  pub(crate) marker_text: Option<TextStyle>,
  /// A fill behind the paragraph, across the input: its color and the radius of its corners.
  pub(crate) fill: Option<(Color, f32)>,
  /// Whether the paragraph before is of the same kind: a bar and a fill then go on from it as one.
  pub(crate) continues: bool,
  /// The band over the run it starts, which names it: a block of code's language.
  pub(crate) header: Option<ParagraphBand>,
}

/// The band over a run of paragraphs that names it, resolved: its name in its type, its height, the room before
/// the name, and its fill and the line under it.
#[derive(Clone, PartialEq)]
#[cfg_attr(not(feature = "markdown"), allow(dead_code))]
pub(crate) struct ParagraphBand {
  pub(crate) text: Arc<str>,
  pub(crate) style: TextStyle,
  pub(crate) height: f32,
  pub(crate) padding_x: f32,
  pub(crate) fill: Option<Color>,
  pub(crate) line: Option<(f32, Color)>,
}

/// What stands in the room before a paragraph's text. Only a styled input has kinds of paragraphs, and those
/// come with the `markdown` feature.
#[derive(Clone, PartialEq)]
#[cfg_attr(not(feature = "markdown"), allow(dead_code))]
pub(crate) enum ParagraphMark {
  None,
  /// A dot `size` across, centred on the first row, ending `gap` before the text.
  Dot {
    size: f32,
    gap: f32,
    color: Color,
  },
  /// A text in the paragraph's type, ending `gap` before the text.
  Text {
    text: Arc<str>,
    gap: f32,
    color: Color,
  },
  /// A bar `width` wide from the start of the input, as high as the paragraph.
  Bar {
    width: f32,
    color: Color,
  },
  /// A box `size` square, centred on the first row, ending `gap` before the text, with an edge; checked, filled,
  /// with an icon in it: the glyph's type and character.
  Check {
    size: f32,
    gap: f32,
    radius: f32,
    edge: (f32, Color),
    checked: Option<(Color, Option<(TextStyle, char)>)>,
  },
  /// A line `thickness` high across the input, in the middle of the first row.
  Rule {
    thickness: f32,
    color: Color,
  },
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
  /// The token's fill the rectangle is, with its edge; `None` for a line under or through text.
  pub(crate) token: Option<(usize, Option<(f32, Color)>)>,
  /// An edge inside a rectangle that is no token's: the box of an item of a checklist.
  pub(crate) edge: Option<(f32, Color)>,
  /// Whether the rectangle is the box of an item of a checklist, which a press checks.
  #[cfg_attr(not(feature = "markdown"), allow(dead_code))]
  pub(crate) check: bool,
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
  /// The boxes of the ranges that are objects, in the order they were laid out.
  pub(crate) objects: Vec<InputObjectBox>,
  rows: Vec<InputRow>,
  intervals: Vec<ClusterInterval>,
  pub(crate) size: Size,
  pub(crate) line_height: f32,
  pub(crate) viewport_width: f32,
}

struct InputRow {
  range: Range<usize>,
  ending: Option<Range<usize>>,
  y: f32,
  height: f32,
  band_top: f32,
  band_height: f32,
  left: f32,
  width: f32,
  rtl: bool,
  /// The room before the row's text that its paragraph keeps.
  indent: f32,
}

impl DecoratedInputLayout {
  /// The byte the paragraph starts at whose box of a checklist's item is at `(x, y)`, in the coordinates of the
  /// carets, with a little room round it for a finger.
  #[cfg(feature = "markdown")]
  pub(crate) fn check_at(&self, x: f32, y: f32) -> Option<usize> {
    const ROOM: f32 = 3.0;
    let part = self.fragments.iter().find(|part| {
      part.check
        && x >= part.x - ROOM
        && x < part.x + part.width + ROOM
        && y >= part.y - ROOM
        && y < part.y + part.height + ROOM
    })?;
    self.rows.get(part.row).map(|row| row.range.start)
  }

  fn empty(config: InputConfig, revision: u64) -> Self {
    Self {
      config,
      revision,
      glyphs: Vec::new(),
      fragments: Vec::new(),
      carets: Vec::new(),
      objects: Vec::new(),
      rows: Vec::new(),
      intervals: Vec::new(),
      size: Size::default(),
      line_height: 1.0,
      viewport_width: 0.0,
    }
  }
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
  /// Laid-out paragraphs, so an edit lays out only the paragraph it changed.
  paragraphs: pieces::ParagraphCache,
  /// Glyph commands of the layouts drawn last.
  raster: paint::RasterCache,
  /// Shaped chunks of long paragraphs, so an edit shapes only its chunk.
  chunks: chunks::ChunkCache,
  /// Shapes every paragraph whole, for tests that compare the two.
  whole_paragraphs: bool,
}

impl InputCache {
  pub(super) fn clear(&mut self) {
    self.revision = self.revision.wrapping_add(1);
    self.entries.clear();
    self.paragraphs.clear();
    self.raster.clear();
    self.chunks.clear();
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
      + self.paragraphs.estimated_memory_bytes()
      + self.raster.estimated_memory_bytes()
      + self.chunks.estimated_memory_bytes()
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
    let mut output = DecoratedInputLayout::empty(config, self.input_cache.revision);
    self.shape_input(&mut output);
    let output = Arc::new(output);
    self.input_cache.insert(output.clone());
    output
  }

  fn shape_input(&mut self, output: &mut DecoratedInputLayout) {
    // Wrapped text is laid out a paragraph at a time from the paragraph
    // cache. Justified rows stretch to the widest paragraph, and a single
    // scrolling line has no paragraphs, so those lay out whole.
    if output.config.wrap && output.config.base.text_align != crate::layout::text_style::TextAlign::Justified {
      self.shape_paragraphs(output);
    } else {
      self.shape_rows(output);
    }
    output.finish_carets();
    output.align_viewport();
  }

  /// Lays out the rows of all of `output`'s text, unaligned.
  fn shape_rows(&mut self, output: &mut DecoratedInputLayout) {
    let config = &output.config;
    // CR and LF each become one ASCII space in Scroll, preserving every raw
    // UTF-8 byte offset. Only presentation is flattened; config.source stays raw.
    let presentation = if config.wrap {
      Cow::Borrowed(config.source.as_ref())
    } else {
      Cow::Owned(config.source.replace(['\r', '\n'], " "))
    };
    // A box has no type of its own: where the text has one, it is given the ascent and the descent of the
    // input's, whatever font would have drawn the character it stands for.
    let object_band = if config.has_objects() {
      let base = config.base.clone();
      self.line_ascent_descent(&base)
    } else {
      None
    };
    let mut buffer = self.input_paragraphs(config, &presentation);
    let mut offset = 0;
    let mut prepared = Vec::new();
    for line in &mut buffer.lines {
      let bytes = line.text().len();
      let chunked = self.shape_in_chunks(line.text(), line.attrs_list());
      let shape = match &chunked {
        Some(shape) => shape,
        None => line.shape(&mut self.font_system, 8),
      };
      rows::paragraph(&mut prepared, shape, offset, bytes, &output.config, object_band);
      let ending = offset + bytes..offset + bytes + line.ending().as_str().len();
      rows::set_ending(&mut prepared, ending.clone());
      offset = ending.end;
    }
    rows::finish(output, prepared);
    self.buffer_pool.push(buffer);
  }
}
