use std::{collections::BTreeMap, mem::size_of};

use cosmic_text::LayoutLine;

use super::{
  rows::{padding::Padding, visual_bounds},
  *,
};
use crate::layout::text_style::TextAlign;

impl DecoratedInputLayout {
  pub(super) fn append_empty_row(&mut self, start: usize, end: usize) {
    let height = (self.config.base.font_size * self.config.base.line_height).max(1.0);
    let x = self.alignment_offset(0.0);
    let row = self.rows.len();
    self.carets.push(InputCaret {
      byte: start,
      x,
      y: self.size.height,
      height,
      font_size: self.config.base.font_size,
      row,
    });
    if start != end {
      self.carets.push(InputCaret {
        byte: end,
        x,
        y: self.size.height,
        height,
        font_size: self.config.base.font_size,
        row,
      });
    }
    self.rows.push(InputRow {
      range: start..end,
      ending: None,
      y: self.size.height,
      height,
      band_top: 0.0,
      band_height: self.config.base.font_size,
      left: x,
      width: 0.0,
      rtl: false,
    });
    self.size.height += height;
    self.line_height = self.line_height.max(height);
  }

  pub(super) fn append_row(
    &mut self,
    row: LayoutLine,
    styles: &[usize],
    metrics: &[(f32, f32)],
    padding: &Padding,
    offset: usize,
    coverage: Range<usize>,
    rtl: bool,
  ) {
    let height = row
      .line_height_opt
      .unwrap_or(self.config.base.font_size * self.config.base.line_height)
      .max(row.max_ascent + row.max_descent)
      .max(1.0);
    let top = self.size.height;
    let band_top = (height - row.max_ascent - row.max_descent) * 0.5;
    let baseline = top + band_top + row.max_ascent;
    let bounds = visual_bounds(&row.glyphs);
    let right = bounds.values().map(|bound| bound.1).fold(0.0_f32, f32::max);
    let left = bounds.values().map(|bound| bound.0).fold(right, f32::min);
    let width = (right - left).max(0.0);
    let shift = 0.0;
    self.append_clusters(&row, styles, metrics, padding, offset, top, height, baseline, shift);
    self.ensure_row_boundaries(offset, coverage.clone(), top, height, shift);
    for mut glyph in row.glyphs {
      let style = styles[glyph.metadata];
      let text = if style == 0 {
        &self.config.base
      } else {
        &self.config.styles[style - 1].text
      };
      let source = &self.config.source[offset + glyph.start..offset + glyph.end];
      // Whitespace/control glyphs retain source geometry but paint no ink.
      if source.chars().all(|ch| {
        ch.is_whitespace()
          || ch.is_control()
          || matches!(ch, '\u{200b}' | '\u{200c}' | '\u{200d}' | '\u{2066}'..='\u{2069}')
      }) {
        continue;
      }
      glyph.x += shift;
      glyph.start += offset;
      glyph.end += offset;
      self.glyphs.push(InputGlyph {
        glyph,
        baseline,
        row: self.rows.len(),
        color: text.color,
        shadow: text.shadow,
      });
    }
    self.rows.push(InputRow {
      range: offset + coverage.start..offset + coverage.end,
      ending: None,
      y: top,
      height,
      band_top,
      band_height: row.max_ascent + row.max_descent,
      left,
      width,
      rtl,
    });
    self.size.width = self.size.width.max(width);
    self.size.height += height;
    self.line_height = self.line_height.max(height);
  }

  fn append_clusters(
    &mut self,
    row: &LayoutLine,
    styles: &[usize],
    metrics: &[(f32, f32)],
    padding: &Padding,
    offset: usize,
    top: f32,
    height: f32,
    baseline: f32,
    shift: f32,
  ) {
    let mut clusters = BTreeMap::new();
    let mut bands: BTreeMap<_, (f32, f32)> = BTreeMap::new();
    for glyph in &row.glyphs {
      let key = (glyph.start, glyph.end);
      clusters.entry(key).or_insert(glyph);
      let band = bands.entry(key).or_insert((0.0, 0.0));
      band.0 = band.0.max(metrics[glyph.metadata].0);
      band.1 = band.1.max(metrics[glyph.metadata].1);
    }
    let bounds = visual_bounds(&row.glyphs);
    let mut visual: Vec<_> = clusters.into_iter().collect();
    visual.sort_by(|a, b| bounds[&a.0].0.total_cmp(&bounds[&b.0].0).then(a.0.cmp(&b.0)));
    let mut fragment: Option<(usize, f32, f32, f32, f32)> = None;
    for (range, glyph) in visual {
      let (left, right) = bounds[&range];
      let (left_pad, right_pad) = padding.edges.get(&range).copied().unwrap_or((0.0, 0.0));
      let body_left = left + left_pad + shift;
      let body_right = right - right_pad + shift;
      let (start, end) = if glyph.level.is_rtl() {
        (body_right, body_left)
      } else {
        (body_left, body_right)
      };
      for (byte, x) in [(range.0 + offset, start), (range.1 + offset, end)] {
        self.carets.push(InputCaret {
          byte,
          x,
          y: top,
          height,
          font_size: glyph.font_size,
          row: self.rows.len(),
        });
      }
      self.intervals.push(ClusterInterval {
        range: range.0 + offset..range.1 + offset,
        x: left + shift,
        right: right + shift,
        y: top,
      });
      let token = styles[glyph.metadata].checked_sub(1);
      if fragment.as_ref().map(|part| part.0) != token
        && let Some(part) = fragment.take()
      {
        self.push_fragment(part, baseline);
      }
      if let Some(token) = token {
        let (ascent, descent) = bands[&range];
        fragment = Some(
          fragment.map_or((token, left + shift, right + shift, ascent, descent), |old| {
            (
              token,
              old.1.min(left + shift),
              old.2.max(right + shift),
              old.3.max(ascent),
              old.4.max(descent),
            )
          }),
        );
      }
    }
    if let Some(part) = fragment {
      self.push_fragment(part, baseline);
    }
  }

  pub(super) fn append_ending(&mut self, ending: Range<usize>) {
    let row = self.rows.len() - 1;
    let edge = self
      .carets
      .iter()
      .find(|caret| caret.row == row && caret.byte == ending.start)
      .copied()
      .expect("every completed row has its raw content-end caret");
    // Ending interiors have no independent ink. They retain their raw byte
    // endpoint at the last shaped row's logical edge, even after soft wraps.
    for byte in ending.start + 1..ending.end {
      self.carets.push(InputCaret { byte, ..edge });
    }
    self.intervals.push(ClusterInterval {
      range: ending.clone(),
      x: edge.x,
      right: edge.x,
      y: edge.y,
    });
    self.rows[row].ending = Some(ending);
  }

  fn push_fragment(&mut self, (token, left, right, ascent, descent): (usize, f32, f32, f32, f32), baseline: f32) {
    if let Some((color, radius)) = self.config.styles[token].fill {
      let style = &self.config.styles[token].text;
      let height = (style.font_size * style.line_height).max(ascent + descent);
      let y = baseline - ascent - (height - ascent - descent) * 0.5;
      self.fragments.push(InputFragment {
        x: left,
        y,
        width: (right - left).max(0.0),
        height,
        color,
        radius,
        row: self.rows.len(),
      });
    }
  }

  fn ensure_row_boundaries(&mut self, offset: usize, coverage: Range<usize>, y: f32, height: f32, shift: f32) {
    for byte in [offset + coverage.start, offset + coverage.end] {
      if self
        .carets
        .iter()
        .rev()
        .take_while(|caret| caret.row == self.rows.len())
        .any(|caret| caret.byte == byte)
      {
        continue;
      }
      let nearest = self
        .carets
        .iter()
        .rev()
        .take_while(|caret| caret.row == self.rows.len())
        .min_by_key(|caret| caret.byte.abs_diff(byte))
        .copied();
      self.carets.push(nearest.map_or(
        InputCaret {
          byte,
          x: shift,
          y,
          height,
          font_size: self.config.base.font_size,
          row: self.rows.len(),
        },
        |caret| InputCaret { byte, ..caret },
      ));
    }
  }

  fn alignment_offset(&self, used: f32) -> f32 {
    if !self.config.width.is_finite() || self.config.width >= f32::MAX {
      return 0.0;
    }
    let available = (self.config.width - used).max(0.0);
    match self.config.base.text_align {
      TextAlign::Left | TextAlign::Justified => 0.0,
      TextAlign::Center => available * 0.5,
      TextAlign::Right | TextAlign::End => available,
    }
  }

  pub(super) fn finish_carets(&mut self) {
    if self.carets.is_empty() {
      self.append_empty_row(0, self.config.source.len());
    }
  }

  pub(crate) fn positions(&self) -> CaretPositions {
    self
      .carets
      .iter()
      .map(|caret| CaretPosition {
        index: caret.byte,
        x: caret.x,
        y: caret.y,
      })
      .collect::<Vec<_>>()
      .into()
  }

  pub(crate) fn same_geometry(&self, other: &Self) -> bool {
    self.revision == other.revision && self.config == other.config
  }
  pub(crate) fn default_caret(&self, byte: usize) -> usize {
    let row = self
      .rows
      .iter()
      .position(|row| row.ending.as_ref().is_some_and(|ending| ending.contains(&byte)))
      .or_else(|| {
        self
          .rows
          .iter()
          .position(|row| byte >= row.range.start && byte <= row.range.end)
      })
      .unwrap_or(self.rows.len() - 1);
    self
      .carets
      .iter()
      .enumerate()
      .filter(|(_, caret)| caret.row == row)
      .min_by_key(|(_, caret)| caret.byte.abs_diff(byte))
      .map_or(0, |(index, _)| index)
  }
  pub(crate) fn hit(&self, x: f32, y: f32) -> usize {
    let bottom = self.rows.last().map_or(0.0, |row| row.y + row.height);
    let row = self
      .rows
      .partition_point(|row| row.y <= y.min(bottom))
      .saturating_sub(1);
    self
      .carets
      .iter()
      .enumerate()
      .filter(|(_, caret)| caret.row == row)
      .min_by(|a, b| (a.1.x - x).abs().total_cmp(&(b.1.x - x).abs()))
      .map_or(0, |(index, _)| index)
  }
  pub(crate) fn vertical(&self, caret: usize, down: bool) -> usize {
    let position = self.carets[caret];
    let row = if down {
      (position.row + 1).min(self.rows.len() - 1)
    } else {
      position.row.saturating_sub(1)
    };
    self.hit(position.x, self.rows[row].y)
  }
  pub(crate) fn selection(&self, range: Range<usize>, scroll: (f32, f32)) -> Vec<TextSelectionRange> {
    let mut selected: Vec<_> = self
      .intervals
      .iter()
      .filter(|part| part.range.start < range.end && part.range.end > range.start)
      .collect();
    selected.sort_by(|a, b| a.y.total_cmp(&b.y).then(a.x.total_cmp(&b.x)));
    let mut result: Vec<TextSelectionRange> = Vec::new();
    for part in selected {
      let x = part.x - scroll.0;
      let y = part.y - scroll.1;
      if let Some(previous) = result
        .last_mut()
        .filter(|previous| previous.y == y && x <= previous.x + previous.width + 0.01)
      {
        previous.width = previous.width.max(part.right - scroll.0 - previous.x);
      } else {
        result.push(TextSelectionRange {
          x,
          y,
          width: (part.right - part.x).max(1.0),
        });
      }
    }
    result
  }
  pub(crate) fn selection_band(&self, y: f32) -> (f32, f32) {
    let index = self.rows.partition_point(|row| row.y <= y + 0.01).saturating_sub(1);
    (self.rows[index].band_top, self.rows[index].band_height)
  }
  pub(crate) fn allocated_bytes(&self) -> usize {
    size_of::<Self>()
      + self.config.source.len()
      // The source allocation and the retained layout each have an Arc header.
      + 4 * size_of::<usize>()
      + style_heap(&self.config.base)
      + self
        .config
        .styles
        .iter()
        .map(|style| style_heap(&style.text))
        .sum::<usize>()
      + self.glyphs.capacity() * size_of::<InputGlyph>()
      + self.fragments.capacity() * size_of::<InputFragment>()
      + self.carets.capacity() * size_of::<InputCaret>()
      + self.intervals.capacity() * size_of::<ClusterInterval>()
      + self.rows.capacity() * size_of::<InputRow>()
      + self.config.styles.capacity() * size_of::<TokenStyle>()
  }
  pub(crate) fn estimated_shared_memory_bytes(layout: &Arc<Self>) -> usize {
    layout.allocated_bytes() / Arc::strong_count(layout)
  }
}

fn style_heap(style: &crate::layout::text_style::TextStyle) -> usize {
  // Charge each retained shared allocation in full: sharing may save memory,
  // but cannot permit the cache to exceed its allocation budget.
  style.font_family.len()
    + 2 * size_of::<usize>()
    + style.font_features.iter().count() * size_of::<crate::layout::text_style::FontFeature>()
    + if style.font_features.is_empty() {
      0
    } else {
      2 * size_of::<usize>()
    }
}
