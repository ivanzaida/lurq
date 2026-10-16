//! The caret positions of a shaped buffer: one before and one after every
//! laid-out glyph, in visual order, with the top of its row. Plain text and
//! rich text both read them from the buffer they are drawn from, so a
//! selection's hit-testing, highlight and copied text index the glyphs on
//! screen, whatever fonts the spans use and wherever they wrap.

use cosmic_text::Buffer;

use crate::node::text_selection::{CaretIndex, CaretPosition, CaretPositions, glyph_caret_edges};

/// The caret positions of `buffer`, whose lines hold `text_len` bytes of
/// text with their line endings. A row without glyphs gets one caret at its
/// start, and the end of the text gets one after the last glyph when no
/// glyph ends there.
pub(super) fn buffer_caret_positions(buffer: &Buffer, text_len: usize) -> CaretPositions {
  let mut line_offsets = Vec::with_capacity(buffer.lines.len());
  let mut offset = 0usize;
  for line in &buffer.lines {
    line_offsets.push(offset);
    offset += line.text().len() + line.ending().as_str().len();
  }

  let mut positions = Vec::with_capacity(text_len + buffer.lines.len().max(1));
  let mut index = CaretIndex::default();
  for run in buffer.layout_runs() {
    let line_offset = line_offsets.get(run.line_i).copied().unwrap_or(0);
    let y = run.line_top.max(0.0);
    let start = positions.len();
    if run.glyphs.is_empty() {
      positions.push(CaretPosition {
        index: line_offset,
        x: 0.0,
        y,
      });
      index.push_line(start, positions.len(), line_offset, line_offset, y);
      continue;
    }

    for glyph in run.glyphs {
      let (start_x, end_x) = glyph_caret_edges(glyph);
      positions.push(CaretPosition {
        index: line_offset + glyph.start,
        x: start_x,
        y,
      });
      positions.push(CaretPosition {
        index: line_offset + glyph.end,
        x: end_x,
        y,
      });
    }
    index.push_line(
      start,
      positions.len(),
      line_offset,
      line_offset + buffer.lines[run.line_i].text().len(),
      y,
    );
  }

  if positions.is_empty() {
    positions.push(CaretPosition {
      index: 0,
      x: 0.0,
      y: 0.0,
    });
    index.push_line(0, 1, 0, 0, 0.0);
  }
  if !positions.iter().any(|position| position.index == text_len) {
    let last_y = positions.last().map(|position| position.y).unwrap_or(0.0);
    positions.push(CaretPosition {
      index: text_len,
      x: positions.last().map(|position| position.x).unwrap_or(0.0),
      y: last_y,
    });
    index.push_line(positions.len() - 1, positions.len(), text_len, text_len, last_y);
  }
  CaretPositions::with_index(positions, index)
}
