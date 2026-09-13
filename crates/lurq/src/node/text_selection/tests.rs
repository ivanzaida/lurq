use super::*;
use crate::{
  app::glyph_engine::GlyphEngine,
  layout::text_style::{TextAlign, TextStyle},
};

fn signature(ranges: Vec<TextSelectionRange>) -> Vec<(u32, u32, u32)> {
  ranges
    .into_iter()
    .map(|r| (r.x.to_bits(), r.y.to_bits(), r.width.to_bits()))
    .collect()
}

fn assert_matches_linear(positions: &CaretPositions) {
  let reference = positions.iter().collect::<Vec<_>>();
  let was_materialized = positions.0.positions.get().is_some();
  let stride = (positions.len() / 23).max(1);
  let indices: Vec<_> = positions
    .iter()
    .step_by(stride)
    .map(|p| p.index)
    .chain([0, 1, usize::MAX])
    .collect();
  for &i in &indices {
    assert_eq!(
      caret_x_for_index(positions, i).to_bits(),
      linear::caret_x_for_index(&reference, i).to_bits()
    );
    assert_eq!(
      caret_y_for_index(positions, i).map(f32::to_bits),
      linear::caret_y_for_index(&reference, i).map(f32::to_bits)
    );
    for &j in &indices {
      let (start, end) = (i.min(j), i.max(j));
      assert_eq!(
        signature(selection_ranges_for_positions(positions, start, end, 7.25, 19.5)),
        signature(linear::selection_ranges_for_positions(
          &reference, start, end, 7.25, 19.5
        ))
      );
      for x in [-10.0, 40.0, 500.0] {
        assert_eq!(
          closest_caret_in_range(positions, start, end, x),
          linear::closest_caret_in_range(&reference, start, end, x)
        );
      }
    }
  }
  for p in positions.iter().step_by(stride) {
    for y in [p.y - 0.01, p.y, p.y + 0.01, -10.0, f32::MAX, f32::NAN] {
      for x in [-10.0, p.x, p.x + 0.5, 500.0, f32::NAN] {
        assert_eq!(
          closest_caret_to_point(positions, x, y),
          linear::closest_caret_to_point(&reference, x, y)
        );
      }
    }
  }
  assert_eq!(
    positions.0.positions.get().is_some(),
    was_materialized,
    "selection lookups must not materialize segmented geometry"
  );
}

#[test]
fn indexed_shaped_carets_match_linear_queries() {
  let mut engine = GlyphEngine::new();
  // Long enough to exercise the index, including blank lines, soft wrapping,
  // ligatures, combining marks, bidi runs, and multibyte source boundaries.
  let source = "office cafe\u{301} مرحبا שלום 漢字 👨‍👩‍👧‍👦\r\n\nsecond paragraph with words that wrap\n".repeat(8);
  for align in [TextAlign::Left, TextAlign::Center, TextAlign::Right] {
    for font_size in [13.0, 19.5] {
      for (width, wrap) in [(90.0, true), (320.0, true), (320.0, false)] {
        let style = TextStyle {
          font_size,
          text_align: align,
          ..TextStyle::default()
        };
        let positions = engine.caret_positions(&source, &style, width, wrap);
        assert!(positions.len() >= 256);
        assert!(
          positions.0.index.get().is_some(),
          "shaping must supply the index without a lazy full scan"
        );
        let cached = engine.caret_positions(&source, &style, width, wrap);
        assert!(
          Arc::ptr_eq(&positions.0, &cached.0),
          "cache hits must share positions and the index"
        );
        assert_matches_linear(&positions);
      }
    }
  }
}

#[test]
fn segmented_edits_and_remapping_keep_queries_lazy_and_correct() {
  let mut engine = GlyphEngine::new();
  let style = TextStyle {
    font_size: 19.5,
    line_height: 1.23,
    ..TextStyle::default()
  };
  let mut lines = (0..24)
    .map(|i| format!("{i}: office cafe\u{301} مرحبا שלום 漢字 👨‍👩‍👧‍👦 wrapping paragraph"))
    .collect::<Vec<_>>();
  for action in 0..5 {
    match action {
      1 => lines[0].push_str(" more lines of wrapping text are inserted before all other coordinates"),
      2 => lines.insert(12, "inserted paragraph 😀".to_owned()),
      3 => {
        lines.remove(1);
      }
      4 => lines[2..8].reverse(),
      _ => {}
    }
    let source = lines.join("\r\n") + if action % 2 == 0 { "\n" } else { "" };
    let positions = engine.caret_positions(&source, &style, 140.25, true);
    assert!(!positions.0.segments.is_empty());
    assert!(positions.0.positions.get().is_none());
    assert_matches_linear(&positions);
    assert!(positions.0.positions.get().is_none());
    let mut remapped = positions.clone();
    for p in remapped.make_mut() {
      p.index *= 2;
    }
    assert!(remapped.0.segments.is_empty());
    assert_matches_linear(&remapped);
    assert!(positions.0.positions.get().is_none());
  }
}

#[test]
fn empty_short_and_nonmonotonic_carets_preserve_fallbacks_and_ties() {
  assert_matches_linear(&Vec::new().into());
  assert_matches_linear(
    &vec![CaretPosition {
      index: 0,
      x: 0.0,
      y: 0.0,
    }]
    .into(),
  );
  for unordered_y in [false, true] {
    let positions = (0..400)
      .map(|i| CaretPosition {
        index: if i % 2 == 0 { 399 - i } else { i },
        x: (i % 10 / 2) as f32 * 8.0,
        y: if unordered_y {
          ((i / 10) % 3) as f32
        } else {
          (i / 10) as f32
        },
      })
      .collect::<Vec<_>>()
      .into();
    assert_matches_linear(&positions);
  }
  let positions = (0..300)
    .map(|i| CaretPosition {
      index: i,
      x: i as f32,
      y: if i < 150 { f32::EPSILON } else { 2.0 * f32::EPSILON },
    })
    .collect::<Vec<_>>()
    .into();
  assert_matches_linear(&positions);
}

#[test]
fn remapping_detaches_shared_geometry_and_invalidates_indices() {
  let original: CaretPositions = (0..400)
    .map(|i| CaretPosition {
      index: i * 3,
      x: (i % 10) as f32,
      y: (i / 10) as f32,
    })
    .collect::<Vec<_>>()
    .into();
  assert_matches_linear(&original);
  let mut remapped = original.clone();
  assert!(Arc::ptr_eq(&original.0, &remapped.0));
  for p in remapped.make_mut() {
    p.index = p.index / 3 * 4;
  }
  assert!(!Arc::ptr_eq(&original.0, &remapped.0));
  assert!(remapped.0.index.get().is_none());
  assert_eq!(original[1].index, 3);
  assert_eq!(remapped[1].index, 4);
  assert_matches_linear(&original);
  assert_matches_linear(&remapped);
  // The unique-owner path must also discard an already-built index.
  remapped.make_mut()[0].index = 5000;
  assert!(remapped.0.index.get().is_none());
  assert_matches_linear(&remapped);
}

mod linear {
  use super::{CaretPosition, TextSelectionRange};
  pub(crate) fn caret_x_for_index(positions: &[CaretPosition], index: usize) -> f32 {
    positions
      .iter()
      .find(|position| position.index == index)
      .map(|position| position.x)
      .unwrap_or_else(|| positions.last().map(|position| position.x).unwrap_or(0.0))
  }

  pub(crate) fn caret_y_for_index(positions: &[CaretPosition], index: usize) -> Option<f32> {
    positions
      .iter()
      .find(|position| position.index == index)
      .map(|position| position.y)
  }

  pub(crate) fn selection_ranges_for_positions(
    positions: &[CaretPosition],
    start: usize,
    end: usize,
    scroll_x: f32,
    scroll_y: f32,
  ) -> Vec<TextSelectionRange> {
    let mut ranges = Vec::new();
    let mut line_start = 0;
    while line_start < positions.len() {
      let y = positions[line_start].y;
      let mut line_end = line_start + 1;
      while line_end < positions.len() && (positions[line_end].y - y).abs() <= f32::EPSILON {
        line_end += 1;
      }

      let line_positions = &positions[line_start..line_end];
      let first = line_positions.first().unwrap();
      let last = line_positions.last().unwrap();
      if start <= last.index && end >= first.index {
        let selection_start = start.max(first.index).min(last.index);
        let selection_end = end.min(last.index).max(first.index);
        if selection_start != selection_end {
          let start_x = caret_x_for_index(line_positions, selection_start);
          let end_x = caret_x_for_index(line_positions, selection_end);
          ranges.push(TextSelectionRange {
            x: start_x.min(end_x) - scroll_x,
            y: y - scroll_y,
            width: (start_x - end_x).abs().max(1.0),
          });
        }
      }

      line_start = line_end;
    }
    ranges
  }

  pub(crate) fn closest_caret_in_range(positions: &[CaretPosition], start: usize, end: usize, x: f32) -> usize {
    positions
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

  pub(crate) fn closest_caret_to_point(positions: &[CaretPosition], x: f32, y: f32) -> usize {
    let Some((line_start, line_end)) = line_range_for_y(positions, y) else {
      return 0;
    };

    positions[line_start..line_end]
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

  fn line_range_for_y(positions: &[CaretPosition], y: f32) -> Option<(usize, usize)> {
    if positions.is_empty() {
      return None;
    }

    let mut line_start = 0;
    let mut previous_line = None;
    while line_start < positions.len() {
      let line_y = positions[line_start].y;
      let mut line_end = line_start + 1;
      while line_end < positions.len() && (positions[line_end].y - line_y).abs() <= f32::EPSILON {
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
}
