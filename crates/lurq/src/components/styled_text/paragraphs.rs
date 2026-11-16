//! What each hard paragraph of a styled input is: body text, a heading, an
//! item of a list, a quote, a line of a code block. The kinds live in a signal
//! beside the spans ([`crate::components::TextInput::paragraphs`]), one for
//! every paragraph of the text; the input keeps them in step with its edits
//! and in its undo history, and draws each kind as a [`ParagraphStyles`] says.
use std::ops::Range;

use cosmic_text::{LineEnding, LineIter};

use crate::{
  app::component::{DevtoolsFormatter, DevtoolsInspectable},
  layout::text_style::{FontWeight, TextStyle},
  node::TextColor,
};

/// What a paragraph of a styled input is. A paragraph is the text between two
/// line breaks; a list is a run of items one after another.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ParagraphKind {
  /// Running text.
  #[default]
  Body,
  /// A heading of a level from 1 to 6.
  Heading(u8),
  /// An item of a bulleted list.
  Bullet,
  /// An item of a numbered list: its number is its place in the run of items it stands in.
  Ordered,
  /// A paragraph of a quote.
  Quote,
  /// A line of a block of code.
  Code,
}

impl ParagraphKind {
  /// The kind as an input keeps it: a heading's level within 1 to 6.
  pub(crate) fn canonical(self) -> Self {
    match self {
      Self::Heading(level) => Self::Heading(level.clamp(1, 6)),
      kind => kind,
    }
  }
}

impl DevtoolsInspectable for ParagraphKind {
  fn inspect(&self, formatter: &mut DevtoolsFormatter<'_>) {
    formatter.value_debug(self);
  }
}

/// What is drawn before a paragraph, in the room its indent leaves.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum ParagraphMarker {
  #[default]
  None,
  /// A dot `size` across, centred on the first row's text, `gap` before the text.
  Dot { size: f32, gap: f32, color: TextColor },
  /// The item's number and a full stop, in the paragraph's type, ending `gap` before the text.
  Number { gap: f32, color: TextColor },
  /// A bar `width` wide down the start of the paragraph, as high as all its rows: one bar down a run of
  /// paragraphs of the kind.
  Bar { width: f32, color: TextColor },
}

/// How the paragraphs of one kind are drawn: their type where it is not the
/// input's own, the room before their text, and what stands in that room.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ParagraphStyle {
  /// Multiplies the input's font size.
  pub font_size_scale: Option<f32>,
  /// The size of the type in logical pixels, in place of a scale.
  pub font_size: Option<f32>,
  pub weight: Option<FontWeight>,
  pub line_height: Option<f32>,
  pub font_family: Option<std::sync::Arc<str>>,
  pub color: Option<TextColor>,
  /// The room before the text, in logical pixels.
  pub indent: f32,
  pub marker: ParagraphMarker,
  /// A fill behind the paragraph, across the input: one fill behind a run of paragraphs of the kind.
  pub background: Option<TextColor>,
  /// The radius of the fill's corners, in logical pixels.
  pub background_radius: f32,
  /// Whether paragraphs of the kind that follow one another stand with no room between them, as the lines of a
  /// block of code do.
  pub joined: bool,
}

impl ParagraphStyle {
  pub fn new() -> Self {
    Self::default()
  }

  /// The size of the type in logical pixels.
  pub fn font_size(mut self, size: f32) -> Self {
    self.font_size = Some(size);
    self
  }

  /// The size of the type as a multiple of the input's.
  pub fn font_size_scale(mut self, scale: f32) -> Self {
    self.font_size_scale = Some(scale);
    self
  }

  pub fn weight(mut self, weight: FontWeight) -> Self {
    self.weight = Some(weight);
    self
  }

  pub fn line_height(mut self, line_height: f32) -> Self {
    self.line_height = Some(line_height);
    self
  }

  pub fn font_family(mut self, family: impl Into<std::sync::Arc<str>>) -> Self {
    self.font_family = Some(family.into());
    self
  }

  pub fn color(mut self, color: impl Into<TextColor>) -> Self {
    self.color = Some(color.into());
    self
  }

  /// The room before the text.
  pub fn indent(mut self, indent: f32) -> Self {
    self.indent = indent;
    self
  }

  pub fn marker(mut self, marker: ParagraphMarker) -> Self {
    self.marker = marker;
    self
  }

  /// A fill behind the paragraph, across the input, with corners of `radius`.
  pub fn background(mut self, color: impl Into<TextColor>, radius: f32) -> Self {
    self.background = Some(color.into());
    self.background_radius = radius;
    self
  }

  /// Whether paragraphs of the kind that follow one another stand with no room between them.
  pub fn joined(mut self, joined: bool) -> Self {
    self.joined = joined;
    self
  }

  /// The paragraph's type over the input's own.
  pub(crate) fn text(&self, base: &TextStyle) -> TextStyle {
    let mut text = base.clone();
    if let Some(size) = self.font_size.filter(|size| size.is_finite() && *size > 0.0) {
      text.font_size = size;
    } else if let Some(scale) = self.font_size_scale.filter(|scale| scale.is_finite() && *scale > 0.0) {
      text.font_size *= scale;
    }
    if let Some(weight) = self.weight {
      text.weight = weight;
    }
    if let Some(line_height) = self.line_height.filter(|height| height.is_finite() && *height > 0.0) {
      text.line_height = line_height;
    }
    if let Some(family) = &self.font_family {
      text.font_family = family.clone();
    }
    text
  }
}

/// How a styled input draws each kind of paragraph, and the room it leaves
/// between one paragraph and the next. The default draws every kind as body
/// text with no room between.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ParagraphStyles {
  /// The room between two paragraphs, in logical pixels.
  pub gap: f32,
  pub body: ParagraphStyle,
  /// Headings of the levels 1 to 6.
  pub headings: [ParagraphStyle; 6],
  pub bullet: ParagraphStyle,
  pub ordered: ParagraphStyle,
  pub quote: ParagraphStyle,
  pub code: ParagraphStyle,
}

impl ParagraphStyles {
  pub fn new() -> Self {
    Self::default()
  }

  /// The room between two paragraphs.
  pub fn gap(mut self, gap: f32) -> Self {
    self.gap = gap;
    self
  }

  /// How paragraphs of `kind` are drawn.
  pub fn with(mut self, kind: ParagraphKind, style: ParagraphStyle) -> Self {
    match kind.canonical() {
      ParagraphKind::Body => self.body = style,
      ParagraphKind::Heading(level) => self.headings[usize::from(level) - 1] = style,
      ParagraphKind::Bullet => self.bullet = style,
      ParagraphKind::Ordered => self.ordered = style,
      ParagraphKind::Quote => self.quote = style,
      ParagraphKind::Code => self.code = style,
    }
    self
  }

  /// The style of paragraphs of `kind`.
  pub fn of(&self, kind: ParagraphKind) -> &ParagraphStyle {
    match kind.canonical() {
      ParagraphKind::Body => &self.body,
      ParagraphKind::Heading(level) => &self.headings[usize::from(level) - 1],
      ParagraphKind::Bullet => &self.bullet,
      ParagraphKind::Ordered => &self.ordered,
      ParagraphKind::Quote => &self.quote,
      ParagraphKind::Code => &self.code,
    }
  }
}

/// The text of each hard paragraph of `text`, without its line break, as the
/// input lays them out: one more after a last line break, and one for no text.
pub(crate) fn paragraph_ranges(text: &str) -> Vec<Range<usize>> {
  let mut ranges = Vec::new();
  let mut open = true;
  for (range, ending) in LineIter::new(text) {
    ranges.push(range);
    open = ending != LineEnding::None;
  }
  if open {
    ranges.push(text.len()..text.len());
  }
  ranges
}

/// The paragraph the caret at `byte` stands in.
pub(crate) fn paragraph_of(ranges: &[Range<usize>], byte: usize) -> usize {
  ranges.partition_point(|range| range.start <= byte).saturating_sub(1)
}

/// `kinds` as an input keeps them over a text of `count` paragraphs: one for each, body where none was given.
pub(crate) fn normalize_kinds(mut kinds: Vec<ParagraphKind>, count: usize) -> Vec<ParagraphKind> {
  kinds.resize(count, ParagraphKind::Body);
  for kind in &mut kinds {
    *kind = kind.canonical();
  }
  kinds
}

/// The kinds after an edit that replaced the bytes `replaced` of the text
/// whose paragraphs were `before` with text that left the paragraphs `after`,
/// `inserted` bytes long: a paragraph the edit split gives its kind to both
/// halves, and paragraphs it joined take the kind of the first.
pub(crate) fn splice_kinds(
  kinds: &[ParagraphKind],
  before: &[Range<usize>],
  after: &[Range<usize>],
  replaced: Range<usize>,
  inserted: usize,
) -> Vec<ParagraphKind> {
  let kinds = normalize_kinds(kinds.to_vec(), before.len());
  let first = paragraph_of(before, replaced.start);
  let last = paragraph_of(before, replaced.end);
  let added = paragraph_of(after, replaced.start + inserted) - paragraph_of(after, replaced.start);
  let mut out = Vec::with_capacity(after.len());
  out.extend_from_slice(&kinds[..=first]);
  out.extend(std::iter::repeat_n(kinds[first], added));
  out.extend_from_slice(&kinds[(last + 1).min(kinds.len())..]);
  normalize_kinds(out, after.len())
}

#[cfg(test)]
#[path = "../../../tests/input/text_input/paragraph_ops.rs"]
mod tests;
