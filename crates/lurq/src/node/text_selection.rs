use std::{
  ops::Range,
  sync::{Arc, OnceLock},
};

// Short fields use a direct scan without retaining a line table.
const CARET_INDEX_MIN_POSITIONS: usize = 256;

/// Immutable geometry and line ranges shared by the shaping cache and node states.
#[derive(Clone)]
pub(crate) struct CaretPositions(Arc<CaretData>);

struct CaretData {
  positions: OnceLock<Vec<CaretPosition>>,
  segments: Vec<CaretSegment>,
  len: usize,
  index: OnceLock<CaretIndex>,
}

struct CaretLine {
  start: usize,
  end: usize,
  min_index: usize,
  max_index: usize,
  y: f32,
}

#[derive(Default)]
pub(crate) struct CaretIndex {
  lines: Vec<CaretLine>,
  ordered_indices: bool,
  ordered_y: bool,
}

impl From<Vec<CaretPosition>> for CaretPositions {
  fn from(positions: Vec<CaretPosition>) -> Self {
    Self(Arc::new(CaretData {
      len: positions.len(),
      positions: OnceLock::from(positions),
      segments: Vec::new(),
      index: OnceLock::new(),
    }))
  }
}

// The legacy test oracle can request a flat slice. Runtime callers cannot
// accidentally materialize a document through slice coercion.
#[cfg(test)]
impl std::ops::Deref for CaretPositions {
  type Target = [CaretPosition];

  fn deref(&self) -> &Self::Target {
    self.0.positions.get_or_init(|| self.iter().collect())
  }
}

impl CaretPositions {
  pub(crate) fn with_index(positions: Vec<CaretPosition>, mut index: CaretIndex) -> Self {
    if positions.len() < CARET_INDEX_MIN_POSITIONS {
      return positions.into();
    }
    index.finish();
    Self(Arc::new(CaretData {
      len: positions.len(),
      positions: OnceLock::from(positions),
      segments: Vec::new(),
      index: OnceLock::from(index),
    }))
  }

  pub(crate) fn len(&self) -> usize {
    self.0.len
  }

  fn is_empty(&self) -> bool {
    self.len() == 0
  }

  pub(crate) fn iter(&self) -> impl Iterator<Item = CaretPosition> + '_ {
    self.slice(0..self.len()).iter()
  }

  fn get(&self, index: usize) -> Option<CaretPosition> {
    if index >= self.len() {
      return None;
    }
    self.slice(index..index + 1).iter().next()
  }

  fn last(&self) -> Option<CaretPosition> {
    self.len().checked_sub(1).and_then(|i| self.get(i))
  }

  /// Mask remapping detaches a flat copy and cannot keep a stale line index.
  pub(crate) fn make_mut(&mut self) -> &mut [CaretPosition] {
    if !self.0.segments.is_empty() || Arc::get_mut(&mut self.0).is_none() {
      let positions = if self.0.segments.is_empty() {
        self.0.positions.get().unwrap().clone()
      } else {
        self.iter().collect()
      };
      *self = positions.into();
    }
    let data = Arc::get_mut(&mut self.0).unwrap();
    data.index.take();
    data.positions.get_mut().unwrap()
  }

  fn index(&self) -> Option<&CaretIndex> {
    if self.len() < CARET_INDEX_MIN_POSITIONS && self.0.segments.is_empty() {
      return None;
    }
    Some(
      self
        .0
        .index
        .get_or_init(|| CaretIndex::new(self.0.positions.get().expect("flat caret geometry"))),
    )
  }

  fn slice(&self, range: Range<usize>) -> CaretSlice<'_> {
    CaretSlice { positions: self, range }
  }

  fn matching_positions(&self, start: usize, end: usize) -> CaretSlice<'_> {
    let Some(index) = self.index() else {
      return self.slice(0..self.len());
    };
    let lines = index.matching_lines(start, end);
    match (lines.first(), lines.last()) {
      (Some(first), Some(last)) => self.slice(first.start..last.end),
      _ => self.slice(0..0),
    }
  }
}

/// Paragraph-local positions stay unchanged when preceding text grows or shrinks.
/// Each visual row supplies its exact document y separately, avoiding float drift.
pub(crate) struct ParagraphCarets {
  positions: Vec<CaretPosition>,
  runs: Vec<Range<usize>>,
  bytes: usize,
}

impl ParagraphCarets {
  pub(crate) fn new(layouts: &[cosmic_text::LayoutLine]) -> Self {
    let count = layouts.iter().map(|line| (line.glyphs.len() * 2).max(1)).sum();
    let mut positions = Vec::with_capacity(count);
    let mut runs = Vec::with_capacity(layouts.len());
    for line in layouts {
      let start = positions.len();
      if line.glyphs.is_empty() {
        positions.push(CaretPosition {
          index: 0,
          x: 0.0,
          y: 0.0,
        });
      }
      for glyph in &line.glyphs {
        positions.push(CaretPosition {
          index: glyph.start,
          x: glyph.x,
          y: 0.0,
        });
        positions.push(CaretPosition {
          index: glyph.end,
          x: glyph.x + glyph.w,
          y: 0.0,
        });
      }
      runs.push(start..positions.len());
    }
    let bytes = std::mem::size_of::<Self>()
      + 2 * std::mem::size_of::<usize>()
      + positions.capacity() * std::mem::size_of::<CaretPosition>()
      + runs.capacity() * std::mem::size_of::<Range<usize>>();
    Self { positions, runs, bytes }
  }

  #[cfg(feature = "perf_profile")]
  pub(crate) fn len(&self) -> usize {
    self.positions.len()
  }

  pub(crate) fn bytes(&self) -> usize {
    self.bytes
  }
}

struct CaretSegment {
  paragraph: Arc<ParagraphCarets>,
  range: Range<usize>,
  text_offset: usize,
  y: f32,
  start: usize,
}

impl CaretSegment {
  fn position(&self, index: usize) -> CaretPosition {
    let p = self.paragraph.positions[self.range.start + (index - self.start)];
    CaretPosition {
      index: self.text_offset + p.index,
      x: p.x,
      y: self.y,
    }
  }
}

#[derive(Default)]
pub(crate) struct CaretPositionsBuilder {
  segments: Vec<CaretSegment>,
  index: CaretIndex,
  len: usize,
  last: Option<CaretPosition>,
  text_len: usize,
  has_text_end: bool,
}

impl CaretPositionsBuilder {
  pub(crate) fn new(text_len: usize, runs: usize) -> Self {
    Self {
      text_len,
      segments: Vec::with_capacity(runs + 1),
      index: CaretIndex {
        lines: Vec::with_capacity(runs),
        ..CaretIndex::default()
      },
      ..Self::default()
    }
  }

  pub(crate) fn push_run(
    &mut self,
    paragraph: Arc<ParagraphCarets>,
    run: usize,
    text_offset: usize,
    text_len: usize,
    y: f32,
  ) {
    let range = paragraph.runs[run].clone();
    if text_offset + text_len == self.text_len {
      self.has_text_end |= paragraph.positions[range.clone()].iter().any(|p| p.index == text_len);
    }
    let end = self.len + range.len();
    self
      .index
      .push_line(self.len, end, text_offset, text_offset + text_len, y);
    let segment = CaretSegment {
      paragraph,
      range,
      text_offset,
      y,
      start: self.len,
    };
    self.last = Some(segment.position(end - 1));
    self.segments.push(segment);
    self.len = end;
  }

  pub(crate) fn finish(mut self) -> CaretPositions {
    if self.len == 0 || !self.has_text_end {
      let last = self.last.unwrap_or(CaretPosition {
        index: 0,
        x: 0.0,
        y: 0.0,
      });
      // Preserve the legacy empty-document origin and final missing endpoint.
      if self.len == 0 {
        self.push_point(CaretPosition {
          index: 0,
          x: 0.0,
          y: 0.0,
        });
      }
      if !self.has_text_end {
        self.push_point(CaretPosition {
          index: self.text_len,
          x: last.x,
          y: last.y,
        });
      }
    }
    self.index.finish();
    CaretPositions(Arc::new(CaretData {
      positions: OnceLock::new(),
      segments: self.segments,
      len: self.len,
      index: OnceLock::from(self.index),
    }))
  }

  fn push_point(&mut self, point: CaretPosition) {
    let paragraph = Arc::new(ParagraphCarets {
      positions: vec![CaretPosition {
        index: 0,
        x: point.x,
        y: 0.0,
      }],
      runs: vec![0..1],
      bytes: std::mem::size_of::<ParagraphCarets>()
        + 2 * std::mem::size_of::<usize>()
        + std::mem::size_of::<CaretPosition>()
        + std::mem::size_of::<Range<usize>>(),
    });
    self.push_run(paragraph, 0, point.index, 0, point.y);
  }
}

struct CaretSlice<'a> {
  positions: &'a CaretPositions,
  range: Range<usize>,
}

impl<'a> CaretSlice<'a> {
  fn first(&self) -> Option<CaretPosition> {
    if self.range.is_empty() {
      None
    } else {
      self.positions.get(self.range.start)
    }
  }

  fn last(&self) -> Option<CaretPosition> {
    if self.range.is_empty() {
      None
    } else {
      self.positions.get(self.range.end - 1)
    }
  }

  fn iter(&self) -> CaretIter<'a> {
    let data = &self.positions.0;
    if data.segments.is_empty() {
      CaretIter::Flat(data.positions.get().unwrap()[self.range.clone()].iter().copied())
    } else {
      let segment = data
        .segments
        .partition_point(|s| s.start <= self.range.start)
        .saturating_sub(1);
      CaretIter::Segments {
        segments: &data.segments,
        range: self.range.clone(),
        segment,
      }
    }
  }
}

enum CaretIter<'a> {
  Flat(std::iter::Copied<std::slice::Iter<'a, CaretPosition>>),
  Segments {
    segments: &'a [CaretSegment],
    range: Range<usize>,
    segment: usize,
  },
}

impl Iterator for CaretIter<'_> {
  type Item = CaretPosition;

  fn next(&mut self) -> Option<Self::Item> {
    match self {
      Self::Flat(iter) => iter.next(),
      Self::Segments {
        segments,
        range,
        segment,
      } => {
        if range.start >= range.end {
          return None;
        }
        while *segment + 1 < segments.len() && segments[*segment + 1].start <= range.start {
          *segment += 1;
        }
        let position = segments[*segment].position(range.start);
        range.start += 1;
        Some(position)
      }
    }
  }
  fn size_hint(&self) -> (usize, Option<usize>) {
    match self {
      Self::Flat(iter) => iter.size_hint(),
      Self::Segments { range, .. } => (range.len(), Some(range.len())),
    }
  }
}

impl CaretIndex {
  fn new(positions: &[CaretPosition]) -> Self {
    let mut lines = Vec::new();
    let mut start = 0;
    while start < positions.len() {
      let first = positions[start];
      let mut end = start + 1;
      let mut min_index = first.index;
      let mut max_index = first.index;
      while end < positions.len() && (positions[end].y - first.y).abs() <= f32::EPSILON {
        min_index = min_index.min(positions[end].index);
        max_index = max_index.max(positions[end].index);
        end += 1;
      }
      lines.push(CaretLine {
        start,
        end,
        min_index,
        max_index,
        y: first.y,
      });
      start = end;
    }
    let mut index = Self {
      lines,
      ..Self::default()
    };
    index.finish();
    index
  }

  /// Shaping already knows each run's paragraph range. These conservative byte
  /// bounds avoid a second glyph walk and include every wrapped or bidi caret.
  pub(crate) fn push_line(&mut self, start: usize, end: usize, min_index: usize, max_index: usize, y: f32) {
    if let Some(last) = self.lines.last_mut()
      && (last.y - y).abs() <= f32::EPSILON
    {
      last.end = end;
      last.min_index = last.min_index.min(min_index);
      last.max_index = last.max_index.max(max_index);
      return;
    }
    self.lines.push(CaretLine {
      start,
      end,
      min_index,
      max_index,
      y,
    });
  }

  fn finish(&mut self) {
    // Bidi glyph order need not follow byte order. Only use binary search when
    // the line ranges are monotonic, preserving visual order and tie behavior.
    self.ordered_indices = self
      .lines
      .windows(2)
      .all(|pair| pair[0].min_index <= pair[1].min_index && pair[0].max_index <= pair[1].max_index);
    self.ordered_y =
      self.lines.iter().all(|line| line.y.is_finite()) && self.lines.windows(2).all(|pair| pair[0].y < pair[1].y);
  }

  fn matching_lines(&self, start: usize, end: usize) -> &[CaretLine] {
    if !self.ordered_indices {
      return &self.lines;
    }
    let first = self.lines.partition_point(|line| line.max_index < start);
    let last = self.lines.partition_point(|line| line.min_index <= end);
    &self.lines[first.min(last)..last]
  }
}

#[derive(Clone, Copy)]
pub(crate) struct CaretPosition {
  pub(crate) index: usize,
  pub(crate) x: f32,
  pub(crate) y: f32,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct TextSelectionRange {
  pub(crate) x: f32,
  pub(crate) y: f32,
  pub(crate) width: f32,
}

pub(crate) fn selection_range_indices(value: &str, anchor: Option<usize>, caret: usize) -> Option<(usize, usize)> {
  let anchor = clamp_to_char_boundary(value, anchor?);
  let caret = clamp_to_char_boundary(value, caret);
  if anchor == caret {
    return None;
  }
  Some((anchor.min(caret), anchor.max(caret)))
}

pub(crate) fn caret_x_for_index(positions: &CaretPositions, index: usize) -> f32 {
  positions
    .matching_positions(index, index)
    .iter()
    .find(|position| position.index == index)
    .map(|position| position.x)
    .unwrap_or_else(|| positions.last().map(|position| position.x).unwrap_or(0.0))
}

pub(crate) fn caret_y_for_index(positions: &CaretPositions, index: usize) -> Option<f32> {
  positions
    .matching_positions(index, index)
    .iter()
    .find(|position| position.index == index)
    .map(|position| position.y)
}

pub(crate) fn selection_ranges_for_positions(
  positions: &CaretPositions,
  start: usize,
  end: usize,
  scroll_x: f32,
  scroll_y: f32,
) -> Vec<TextSelectionRange> {
  let mut ranges = Vec::new();
  if let Some(index) = positions.index() {
    for line in index.matching_lines(start, end) {
      append_selection_range(
        &mut ranges,
        positions.slice(line.start..line.end),
        start,
        end,
        scroll_x,
        scroll_y,
      );
    }
  } else {
    let mut line_start = 0;
    while line_start < positions.len() {
      let y = positions.get(line_start).unwrap().y;
      let mut line_end = line_start + 1;
      while line_end < positions.len() && (positions.get(line_end).unwrap().y - y).abs() <= f32::EPSILON {
        line_end += 1;
      }
      append_selection_range(
        &mut ranges,
        positions.slice(line_start..line_end),
        start,
        end,
        scroll_x,
        scroll_y,
      );
      line_start = line_end;
    }
  }
  ranges
}

fn append_selection_range(
  ranges: &mut Vec<TextSelectionRange>,
  positions: CaretSlice<'_>,
  start: usize,
  end: usize,
  scroll_x: f32,
  scroll_y: f32,
) {
  let first = positions.first().unwrap();
  let last = positions.last().unwrap();
  if start <= last.index && end >= first.index {
    let selection_start = start.max(first.index).min(last.index);
    let selection_end = end.min(last.index).max(first.index);
    if selection_start != selection_end {
      let x_for_index = |index| {
        positions
          .iter()
          .find(|position| position.index == index)
          .map(|position| position.x)
          .unwrap_or(last.x)
      };
      let start_x = x_for_index(selection_start);
      let end_x = x_for_index(selection_end);
      ranges.push(TextSelectionRange {
        x: start_x.min(end_x) - scroll_x,
        y: first.y - scroll_y,
        width: (start_x - end_x).abs().max(1.0),
      });
    }
  }
}

pub(crate) fn line_bounds(value: &str, index: usize) -> (usize, usize) {
  let index = clamp_to_char_boundary(value, index);
  let line_start = value[..index].rfind('\n').map(|position| position + 1).unwrap_or(0);
  let line_end = value[index..]
    .find('\n')
    .map(|position| index + position)
    .unwrap_or(value.len());
  (line_start, line_end)
}

pub(crate) fn word_selection_bounds(value: &str, index: usize) -> (usize, usize) {
  if value.is_empty() {
    return (0, 0);
  }

  let index = clamp_to_char_boundary(value, index);
  let (seed_index, seed_class) = if let Some((idx, ch)) = char_at_or_after(value, index)
    && !ch.is_whitespace()
  {
    (idx, word_selection_class(ch))
  } else if let Some((idx, ch)) = char_before(value, index) {
    (idx, word_selection_class(ch))
  } else if let Some((idx, ch)) = char_at_or_after(value, index) {
    (idx, word_selection_class(ch))
  } else {
    return (0, 0);
  };

  let mut start = seed_index;
  while let Some((previous, ch)) = char_before(value, start) {
    if word_selection_class(ch) != seed_class {
      break;
    }
    start = previous;
  }

  let mut end = seed_index + value[seed_index..].chars().next().unwrap().len_utf8();
  while end < value.len() {
    let ch = value[end..].chars().next().unwrap();
    if word_selection_class(ch) != seed_class {
      break;
    }
    end += ch.len_utf8();
  }

  (start, end)
}

pub(crate) fn closest_caret_in_range(positions: &CaretPositions, start: usize, end: usize, x: f32) -> usize {
  positions
    .matching_positions(start, end)
    .iter()
    .filter(|position| position.index >= start && position.index <= end)
    .min_by(|a, b| {
      (a.x - x)
        .abs()
        .partial_cmp(&(b.x - x).abs())
        .unwrap_or(std::cmp::Ordering::Equal)
    })
    .map(|position| position.index)
    .unwrap_or(start)
}

pub(crate) fn closest_caret_to_point(positions: &CaretPositions, x: f32, y: f32) -> usize {
  let line = if let Some(index) = positions.index().filter(|index| index.ordered_y && y.is_finite()) {
    let line = &index.lines[index.lines.partition_point(|line| line.y <= y).saturating_sub(1)];
    Some((line.start, line.end))
  } else {
    line_range_for_y(positions, y)
  };
  let Some((line_start, line_end)) = line else {
    return 0;
  };

  positions
    .slice(line_start..line_end)
    .iter()
    .min_by(|a, b| {
      (a.x - x)
        .abs()
        .partial_cmp(&(b.x - x).abs())
        .unwrap_or(std::cmp::Ordering::Equal)
    })
    .map(|position| position.index)
    .unwrap_or(0)
}

fn line_range_for_y(positions: &CaretPositions, y: f32) -> Option<(usize, usize)> {
  if positions.is_empty() {
    return None;
  }

  let mut line_start = 0;
  let mut previous_line = None;
  while line_start < positions.len() {
    let line_y = positions.get(line_start).unwrap().y;
    let mut line_end = line_start + 1;
    while line_end < positions.len() && (positions.get(line_end).unwrap().y - line_y).abs() <= f32::EPSILON {
      line_end += 1;
    }

    if y < line_y {
      return Some(previous_line.unwrap_or((line_start, line_end)));
    }

    let next_line_y = positions.get(line_end).map(|position| position.y);
    if next_line_y.is_none_or(|next_y| y < next_y) {
      return Some((line_start, line_end));
    }

    previous_line = Some((line_start, line_end));
    line_start = line_end;
  }

  previous_line
}

pub(crate) fn previous_word_boundary(value: &str, index: usize) -> usize {
  let mut index = clamp_to_char_boundary(value, index);
  while let Some((previous, ch)) = char_before(value, index) {
    if !ch.is_whitespace() {
      break;
    }
    index = previous;
  }
  while let Some((previous, ch)) = char_before(value, index) {
    if ch.is_whitespace() {
      break;
    }
    index = previous;
  }
  index
}

pub(crate) fn next_word_boundary(value: &str, index: usize) -> usize {
  let mut index = clamp_to_char_boundary(value, index);
  while index < value.len() {
    let ch = value[index..].chars().next().unwrap();
    if ch.is_whitespace() {
      break;
    }
    index += ch.len_utf8();
  }
  while index < value.len() {
    let ch = value[index..].chars().next().unwrap();
    if !ch.is_whitespace() {
      break;
    }
    index += ch.len_utf8();
  }
  index
}

pub(crate) fn previous_char_boundary(value: &str, index: usize) -> usize {
  let index = clamp_to_char_boundary(value, index);
  value[..index].char_indices().last().map(|(idx, _)| idx).unwrap_or(0)
}

pub(crate) fn next_char_boundary(value: &str, index: usize) -> usize {
  let index = clamp_to_char_boundary(value, index);
  value[index..]
    .char_indices()
    .nth(1)
    .map(|(offset, _)| index + offset)
    .unwrap_or(value.len())
}

pub(crate) fn clamp_to_char_boundary(value: &str, index: usize) -> usize {
  let mut index = index.min(value.len());
  while index > 0 && !value.is_char_boundary(index) {
    index -= 1;
  }
  index
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum WordSelectionClass {
  Word,
  Whitespace,
  Other,
}

fn word_selection_class(ch: char) -> WordSelectionClass {
  if ch.is_alphanumeric() || ch == '_' {
    WordSelectionClass::Word
  } else if ch.is_whitespace() {
    WordSelectionClass::Whitespace
  } else {
    WordSelectionClass::Other
  }
}

fn char_before(value: &str, index: usize) -> Option<(usize, char)> {
  let index = clamp_to_char_boundary(value, index);
  value[..index].char_indices().last()
}

fn char_at_or_after(value: &str, index: usize) -> Option<(usize, char)> {
  let index = clamp_to_char_boundary(value, index);
  value[index..].chars().next().map(|ch| (index, ch))
}

#[cfg(test)]
mod tests;
