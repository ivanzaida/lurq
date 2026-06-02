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
