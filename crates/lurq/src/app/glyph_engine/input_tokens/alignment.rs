//! Final input width changes only placement; it never reshapes cached rows.
use super::*;
use crate::layout::text_style::TextAlign;

impl DecoratedInputLayout {
  pub(super) fn align_viewport(&mut self) {
    self.viewport_width = self.size.width.max(self.config.min_width).min(self.config.width);
    let shifts: Vec<_> = self
      .rows
      .iter()
      .map(|row| {
        let width = if self.viewport_width.is_finite() {
          self.viewport_width
        } else {
          row.width
        };
        let extra = (width - row.width).max(0.0);
        // These are cosmic's public alignment semantics, including directional End.
        let left = if !self.config.wrap && row.width > width {
          // Scroll offsets are nonnegative. Canonicalize an overflowing row
          // to a nonnegative content domain; viewport/anchors reveal either edge.
          0.0
        } else {
          match (self.config.base.text_align, row.rtl) {
            (TextAlign::Left, false) | (TextAlign::Justified, false) => 0.0,
            (TextAlign::Left, true) | (TextAlign::End, true) => width - row.width - extra,
            (TextAlign::Right, false) | (TextAlign::End, false) => extra,
            (TextAlign::Right, true) | (TextAlign::Justified, true) => width - row.width,
            (TextAlign::Center, false) => extra * 0.5,
            (TextAlign::Center, true) => width - row.width - extra * 0.5,
          }
        };
        left - row.left
      })
      .collect();
    for glyph in &mut self.glyphs {
      glyph.glyph.x += shifts[glyph.row];
    }
    for fragment in &mut self.fragments {
      fragment.x += shifts[fragment.row];
    }
    for caret in &mut self.carets {
      caret.x += shifts[caret.row];
    }
    for interval in &mut self.intervals {
      let row = self.rows.partition_point(|row| row.y <= interval.y).saturating_sub(1);
      interval.x += shifts[row];
      interval.right += shifts[row];
    }
    for (row, shift) in self.rows.iter_mut().zip(shifts) {
      row.left += shift;
    }
  }
}
