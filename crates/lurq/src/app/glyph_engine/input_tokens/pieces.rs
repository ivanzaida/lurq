//! Wrapped input text laid out a paragraph at a time. Each hard paragraph is
//! laid out on its own, keyed by its text, its styles and the width, and kept;
//! an edit lays out again only the paragraph it changed and places the others
//! from the cache below each other.
use std::{
  collections::{HashMap, VecDeque},
  hash::{DefaultHasher, Hash, Hasher},
  sync::Arc,
};

use cosmic_text::LineIter;

use super::{DecoratedInputLayout, GlyphEngine, InputConfig, ParagraphFormat, ParagraphMark, TokenStyle};

/// At most this many paragraphs, and this many bytes of them, are kept.
const MAX_PARAGRAPHS: usize = 2048;
const MAX_BYTES: usize = 16 * 1024 * 1024;

#[derive(Default)]
pub(super) struct ParagraphCache {
  entries: HashMap<u64, Vec<Arc<DecoratedInputLayout>>>,
  /// Keys in insertion order, oldest first, for eviction.
  order: VecDeque<u64>,
  bytes: usize,
}

impl ParagraphCache {
  pub(super) fn clear(&mut self) {
    self.entries.clear();
    self.order.clear();
    self.bytes = 0;
  }

  pub(super) fn estimated_memory_bytes(&self) -> usize {
    self.bytes + self.order.capacity() * std::mem::size_of::<u64>()
  }

  fn find(&self, key: u64, config: &InputConfig) -> Option<Arc<DecoratedInputLayout>> {
    self
      .entries
      .get(&key)?
      .iter()
      .find(|piece| &piece.config == config)
      .cloned()
  }

  fn insert(&mut self, key: u64, piece: Arc<DecoratedInputLayout>) {
    let bytes = piece.allocated_bytes();
    if bytes > MAX_BYTES {
      return;
    }
    while self.order.len() >= MAX_PARAGRAPHS || self.bytes + bytes > MAX_BYTES {
      let Some(oldest) = self.order.pop_front() else {
        break;
      };
      if let Some(pieces) = self.entries.get_mut(&oldest) {
        if !pieces.is_empty() {
          self.bytes -= pieces.remove(0).allocated_bytes();
        }
        if pieces.is_empty() {
          self.entries.remove(&oldest);
        }
      }
    }
    self.bytes += bytes;
    self.order.push_back(key);
    self.entries.entry(key).or_default().push(piece);
  }
}

impl GlyphEngine {
  /// Lays out wrapped text paragraph by paragraph, unaligned, reusing every
  /// paragraph laid out before with the same text, styles and width.
  pub(super) fn shape_paragraphs(&mut self, output: &mut DecoratedInputLayout) {
    let source = output.config.source.clone();
    let mut paragraphs: Vec<_> = LineIter::new(&source).collect();
    // Like `Buffer::set_text`: one final empty paragraph after a last line
    // break, or for empty text.
    if paragraphs
      .last()
      .is_none_or(|(_, ending)| *ending != cosmic_text::LineEnding::None)
    {
      paragraphs.push((source.len()..source.len(), cosmic_text::LineEnding::None));
    }
    let mut first_style = 0;
    let formatted = !output.config.paragraphs.is_empty();
    // The fill and the bar of the run of paragraphs being laid out, which a paragraph that continues it grows,
    // and every fill, to be made as wide as the input once that is known.
    let (mut run_fill, mut run_bar) = (None::<usize>, None::<usize>);
    let mut fills = Vec::new();
    for (index, (range, ending)) in paragraphs.into_iter().enumerate() {
      let styles = &output.config.styles;
      while first_style < styles.len() && styles[first_style].range.end <= range.start {
        first_style += 1;
      }
      let local = local_styles(&styles[first_style..], range.clone());
      let format = output.config.paragraphs.get(index).cloned();
      let indent = format.as_ref().map_or(0.0, |format| format.indent.max(0.0));
      let config = InputConfig {
        source: source[range.clone()].into(),
        base: format
          .as_ref()
          .map_or_else(|| output.config.base.clone(), |format| format.base.clone()),
        styles: local,
        width: (output.config.width - indent).max(0.0),
        min_width: 0.0,
        wrap: true,
        paragraphs: Vec::new(),
        exact_rows: format.is_some(),
      };
      let piece = self.paragraph_piece(config);
      let gap = format
        .as_ref()
        .filter(|_| index > 0)
        .map_or(0.0, |format| format.gap_before.max(0.0));
      output.size.height += gap;
      let (top, first_row) = (output.size.height, output.rows_len());
      if let Some(format) = &format {
        if !format.continues {
          (run_fill, run_bar) = (None, None);
        }
        // Under what the paragraph itself draws: its marks' fills and lines come after these.
        let part = |x: f32, width: f32, color, radius| super::InputFragment {
          x,
          y: top,
          width,
          height: 0.0,
          color,
          radius,
          token: None,
          row: first_row,
        };
        match format.fill {
          Some((color, radius)) if run_fill.is_none() => {
            run_fill = Some(output.fragments.len());
            fills.push(output.fragments.len());
            output.fragments.push(part(-indent, 0.0, color, radius));
          }
          Some(_) => {}
          None => run_fill = None,
        }
        match format.marker {
          ParagraphMark::Bar { width, color } if run_bar.is_none() => {
            run_bar = Some(output.fragments.len());
            output.fragments.push(part(-indent, width, color, 0.0));
          }
          ParagraphMark::Bar { .. } => {}
          _ => run_bar = None,
        }
      }
      output.append_piece(&piece, range.start, indent);
      for grown in [run_fill, run_bar].into_iter().flatten() {
        output.fragments[grown].height = output.size.height - output.fragments[grown].y;
      }
      if let Some(format) = &format {
        self.append_marker(output, format, first_row);
      }
      let ending = ending.as_str().len();
      if ending > 0 {
        output.append_ending(range.end..range.end + ending);
      }
    }
    if formatted {
      // The rows an input counts its height in are of its own type, whatever its headings are.
      let base = &output.config.base;
      output.line_height = (base.font_size * base.line_height).max(1.0);
      let width = output.size.width.max(output.config.min_width).min(output.config.width);
      for fill in fills {
        output.fragments[fill].width = width;
      }
    }
  }

  /// Draws what stands before the text of a paragraph whose first row is the row `first_row`: a dot as a fill, a
  /// number as glyphs. They are placed from the start of the paragraph's text, which the alignment then moves
  /// with the row.
  fn append_marker(&mut self, output: &mut DecoratedInputLayout, format: &ParagraphFormat, first_row: usize) {
    let Some((row_y, row_height)) = output.row_box(first_row) else {
      return;
    };
    match &format.marker {
      ParagraphMark::None | ParagraphMark::Bar { .. } => {}
      ParagraphMark::Dot { size, gap, color } => {
        output.fragments.push(super::InputFragment {
          x: -(gap + size),
          y: row_y + (row_height - size) * 0.5,
          width: *size,
          height: *size,
          color: *color,
          radius: size * 0.5,
          token: None,
          row: first_row,
        });
      }
      ParagraphMark::Text { text, gap, color } => {
        let mut base = format.base.clone();
        base.color = *color;
        let piece = self.paragraph_piece(InputConfig {
          source: text.clone(),
          base,
          styles: Vec::new(),
          width: f32::MAX,
          min_width: 0.0,
          wrap: true,
          paragraphs: Vec::new(),
          exact_rows: true,
        });
        let start = -(gap + piece.size.width);
        output.glyphs.extend(piece.glyphs.iter().map(|glyph| {
          let mut layout = glyph.glyph.clone();
          layout.x += start;
          super::InputGlyph {
            glyph: layout,
            baseline: glyph.baseline + row_y,
            row: first_row,
            color: glyph.color,
            shadow: glyph.shadow,
          }
        }));
      }
    }
  }

  fn paragraph_piece(&mut self, config: InputConfig) -> Arc<DecoratedInputLayout> {
    let key = piece_key(&config);
    if let Some(piece) = self.input_cache.paragraphs.find(key, &config) {
      return piece;
    }
    let mut piece = DecoratedInputLayout::empty(config, self.input_cache.revision);
    self.shape_rows(&mut piece);
    let piece = Arc::new(piece);
    self.input_cache.paragraphs.insert(key, piece.clone());
    piece
  }
}

impl DecoratedInputLayout {
  fn rows_len(&self) -> usize {
    self.rows.len()
  }

  /// The top and the height of a row.
  fn row_box(&self, row: usize) -> Option<(f32, f32)> {
    self.rows.get(row).map(|row| (row.y, row.height))
  }

  /// Places a paragraph laid out on its own below the rows so far, its
  /// source starting at byte `offset`, its rows kept `indent` from the start.
  fn append_piece(&mut self, piece: &Self, offset: usize, indent: f32) {
    let (top, first_row) = (self.size.height, self.rows.len());
    self.glyphs.extend(piece.glyphs.iter().map(|glyph| {
      let mut layout = glyph.glyph.clone();
      layout.start += offset;
      layout.end += offset;
      super::InputGlyph {
        glyph: layout,
        baseline: glyph.baseline + top,
        row: glyph.row + first_row,
        color: glyph.color,
        shadow: glyph.shadow,
      }
    }));
    self
      .fragments
      .extend(piece.fragments.iter().map(|part| super::InputFragment {
        y: part.y + top,
        row: part.row + first_row,
        ..*part
      }));
    self.carets.extend(piece.carets.iter().map(|caret| super::InputCaret {
      byte: caret.byte + offset,
      y: caret.y + top,
      row: caret.row + first_row,
      ..*caret
    }));
    self
      .intervals
      .extend(piece.intervals.iter().map(|part| super::ClusterInterval {
        range: part.range.start + offset..part.range.end + offset,
        y: part.y + top,
        ..*part
      }));
    self.rows.extend(piece.rows.iter().map(|row| super::InputRow {
      range: row.range.start + offset..row.range.end + offset,
      ending: None,
      y: row.y + top,
      indent,
      ..*row
    }));
    self.size.width = self.size.width.max(piece.size.width + indent);
    self.size.height += piece.size.height;
    self.line_height = self.line_height.max(piece.line_height);
  }
}

/// The styles over `range`, clipped to it and counted from its start.
fn local_styles(styles: &[TokenStyle], range: std::ops::Range<usize>) -> Vec<TokenStyle> {
  styles
    .iter()
    .take_while(|style| style.range.start < range.end)
    .filter_map(|style| {
      let start = style.range.start.max(range.start);
      let end = style.range.end.min(range.end);
      (start < end).then(|| TokenStyle {
        range: start - range.start..end - range.start,
        ..style.clone()
      })
    })
    .collect()
}

/// A hash of what a paragraph's layout depends on besides its styles' looks,
/// which the cache compares in full.
fn piece_key(config: &InputConfig) -> u64 {
  let mut hasher = DefaultHasher::new();
  config.source.hash(&mut hasher);
  config.width.to_bits().hash(&mut hasher);
  for style in &config.styles {
    style.range.hash(&mut hasher);
  }
  hasher.finish()
}
