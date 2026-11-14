//! A range of cells of a data grid: from the cell it was started from to the cell at its other corner, named by
//! the rows' keys and the columns' ids, which the grid draws as one area over its cells, copies as tab-separated
//! text, pastes into and clears.

use std::{collections::BTreeMap, hash::Hash, sync::Arc};

use super::{CellValue, EditorKind, GridModel, HostState};

/// One cell of a data grid: its row's key and its column's id.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct CellKey<K> {
  pub row: K,
  pub column: Arc<str>,
}

impl<K> CellKey<K> {
  pub fn new(row: K, column: &str) -> Self {
    Self {
      row,
      column: Arc::from(column),
    }
  }
}

/// A range of cells of a data grid: the cell it was started from, where the focus is, and the cell at its other
/// corner, which a drag, Shift with the arrows or Shift with a press moves. The grid tells it with the rows and
/// the columns it spans in the order it shows them, so a range made before a sort names the same cells after it.
///
/// A range the app gives the grid is drawn from its corners: [`CellRange::new`] makes one, and the grid fills in
/// what it spans when it tells it.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct CellRange<K> {
  /// The cell the range was started from.
  pub anchor: CellKey<K>,
  /// The cell at its other corner.
  pub extent: CellKey<K>,
  /// The keys of the rows it spans, in the order the grid showed them when the range was made.
  pub rows: Arc<[K]>,
  /// The ids of the columns it spans, in the order the grid showed them.
  pub columns: Arc<[Arc<str>]>,
}

#[cfg(feature = "devtools")]
impl<K> crate::app::component::DevtoolsInspectable for CellRange<K> {}

impl<K> CellRange<K> {
  /// A range from `anchor` to `extent`: the cells between them as the grid shows its rows and columns.
  pub fn new(anchor: CellKey<K>, extent: CellKey<K>) -> Self {
    Self {
      anchor,
      extent,
      rows: Arc::from(Vec::new()),
      columns: Arc::from(Vec::new()),
    }
  }

  /// How many cells it spans, as the grid told it.
  pub fn len(&self) -> usize {
    self.rows.len() * self.columns.len()
  }

  pub fn is_empty(&self) -> bool {
    self.len() == 0
  }
}

/// A range as places among the rows and the columns the grid shows now.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Span {
  pub anchor: (usize, usize),
  pub extent: (usize, usize),
}

/// What a cell of a range draws: the fill, and an edge on each side that is the range's.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub(super) struct RangeMark {
  /// Every cell of the range but the one the focus is on has its fill.
  pub fill: bool,
  pub top: bool,
  pub bottom: bool,
  pub start: bool,
  pub end: bool,
}

impl Span {
  pub fn rows(&self) -> (usize, usize) {
    (self.anchor.0.min(self.extent.0), self.anchor.0.max(self.extent.0))
  }

  pub fn columns(&self) -> (usize, usize) {
    (self.anchor.1.min(self.extent.1), self.anchor.1.max(self.extent.1))
  }

  pub fn contains(&self, row: usize, column: usize) -> bool {
    let ((top, bottom), (start, end)) = (self.rows(), self.columns());
    (top..=bottom).contains(&row) && (start..=end).contains(&column)
  }

  /// What the cell `(row, column)` draws of the range: `None` outside it, or for a range of one cell, which is
  /// the focused cell alone.
  pub fn mark(&self, row: usize, column: usize) -> Option<RangeMark> {
    if self.anchor == self.extent || !self.contains(row, column) {
      return None;
    }
    let ((top, bottom), (start, end)) = (self.rows(), self.columns());
    Some(RangeMark {
      fill: (row, column) != self.anchor,
      top: row == top,
      bottom: row == bottom,
      start: column == start,
      end: column == end,
    })
  }
}

/// Where `range` is among `keys` and `ids`: `None` where a corner is no longer shown.
pub(super) fn span_of<K: PartialEq>(range: &CellRange<K>, keys: &[K], ids: &[Arc<str>]) -> Option<Span> {
  let place = |cell: &CellKey<K>| {
    Some((
      keys.iter().position(|key| *key == cell.row)?,
      ids.iter().position(|id| *id == cell.column)?,
    ))
  };
  Some(Span {
    anchor: place(&range.anchor)?,
    extent: place(&range.extent)?,
  })
}

/// Cells as tab-separated text: a line for each row, a tab between cells. A tab or a line break in a cell is a
/// space, so the text reads back into the same cells.
fn to_tsv(rows: &[Vec<String>]) -> String {
  rows
    .iter()
    .map(|row| {
      row
        .iter()
        .map(|cell| cell.replace(['\t', '\n', '\r'], " "))
        .collect::<Vec<_>>()
        .join("\t")
    })
    .collect::<Vec<_>>()
    .join("\n")
}

/// Tab-separated text as rows of cells: a spreadsheet's copy, with its line break at the end or without.
pub(super) fn from_tsv(text: &str) -> Vec<Vec<String>> {
  let text = text.strip_suffix('\n').unwrap_or(text);
  let text = text.strip_suffix('\r').unwrap_or(text);
  text
    .split('\n')
    .map(|line| {
      line
        .strip_suffix('\r')
        .unwrap_or(line)
        .split('\t')
        .map(str::to_owned)
        .collect()
    })
    .collect()
}

/// What a text pasted into a cell edited with `kind` is: `None` where it is none of its values.
fn parsed<R>(kind: &EditorKind<R>, text: &str) -> Option<CellValue> {
  let trimmed = text.trim();
  match kind {
    EditorKind::Text(_) => Some(CellValue::Text(text.to_owned())),
    EditorKind::Number(_) => trimmed.parse::<f64>().ok().map(CellValue::Number),
    EditorKind::Choice(choices, _) => choices
      .iter()
      .position(|choice| choice.eq_ignore_ascii_case(trimmed))
      .map(CellValue::Choice),
    EditorKind::Checkbox(_) => match trimmed.to_lowercase().as_str() {
      "true" | "yes" | "1" | "x" | "\u{2713}" => Some(CellValue::Checked(true)),
      "false" | "no" | "0" | "" => Some(CellValue::Checked(false)),
      _ => None,
    },
  }
}

impl<R: Send + Sync + 'static, K: Clone + Eq + Hash + Send + Sync + 'static> GridModel<R, K> {
  /// The range of `span` as the app is told it: its corners and what it spans.
  fn range_of(&self, span: Span) -> Option<CellRange<K>> {
    let id = |column: usize| self.editors.get(column).map(|(id, _)| id.clone());
    let ((top, bottom), (start, end)) = (span.rows(), span.columns());
    Some(CellRange {
      anchor: CellKey {
        row: self.keys.get(span.anchor.0)?.clone(),
        column: id(span.anchor.1)?,
      },
      extent: CellKey {
        row: self.keys.get(span.extent.0)?.clone(),
        column: id(span.extent.1)?,
      },
      rows: Arc::from(self.keys.get(top..=bottom)?.to_vec()),
      columns: Arc::from((start..=end).filter_map(id).collect::<Vec<_>>()),
    })
  }

  /// Asks for `span` to be the range and tells every handler of it: nothing where it is the range now.
  pub(super) fn write_span(&self, span: Option<Span>) {
    if span == self.span {
      return;
    }
    let next = span.and_then(|span| self.range_of(span));
    if next != self.range {
      self.tell_range(next);
    }
  }

  /// Asks for `span` to be the range, whatever this render's range is: a drag's, whose handlers may be an
  /// earlier render's.
  pub(super) fn write_span_now(&self, span: Option<Span>) {
    self.tell_range(span.and_then(|span| self.range_of(span)));
  }

  fn tell_range(&self, next: Option<CellRange<K>>) {
    if let Some(write) = &self.write_range {
      write(next.clone());
    }
    for handler in &self.on_range {
      handler(next.as_ref());
    }
  }

  /// The cells a copy, a paste or a clear works on: the range's, or the one at `at` where there is none.
  fn target(&self, at: (usize, usize)) -> Span {
    self.span.unwrap_or(Span { anchor: at, extent: at })
  }

  /// What a cell says as text, as a copy takes it: the value its editor edits, which a paste reads back, or its
  /// column's text, or nothing for a cell the app builds.
  fn cell_text(&self, row: usize, column: usize) -> String {
    let Some(values) = self.rows.get(row) else {
      return String::new();
    };
    if let Some(editor) = self.editors.get(column).and_then(|(_, editor)| editor.as_ref()) {
      return match &editor.kind {
        EditorKind::Text(text) => text(values),
        EditorKind::Number(number) => number(values).to_string(),
        EditorKind::Choice(choices, chosen) => choices
          .get(chosen(values))
          .map(|choice| choice.to_string())
          .unwrap_or_default(),
        EditorKind::Checkbox(checked) => checked(values).to_string(),
      };
    }
    self
      .texts
      .get(column)
      .and_then(Option::as_ref)
      .map(|text| text(values))
      .unwrap_or_default()
  }

  /// The value a cell has now, as its editor edits it.
  fn value_now(&self, row: usize, column: usize) -> Option<CellValue> {
    let (values, editor) = (self.rows.get(row)?, self.editor(column)?);
    Some(match &editor.kind {
      EditorKind::Text(text) => CellValue::Text(text(values)),
      EditorKind::Number(number) => CellValue::Number(number(values)),
      EditorKind::Choice(_, chosen) => CellValue::Choice(chosen(values)),
      EditorKind::Checkbox(checked) => CellValue::Checked(checked(values)),
    })
  }

  /// Tells the app `value` for the cell where it is another than the cell's: whether it was told.
  fn change(&self, row: usize, column: usize, value: CellValue) -> bool {
    // A cleared number or choice is never the value it had: it is always told.
    if self.value_now(row, column).is_none_or(|now| now == value) {
      return false;
    }
    self.tell(row, column, value);
    true
  }

  /// The cells of the range, or the focused one, as tab-separated text; a cut clears those that are edited.
  pub(super) fn copy_cells(&self, at: (usize, usize), cut: bool) -> String {
    let span = self.target(at);
    let ((top, bottom), (start, end)) = (span.rows(), span.columns());
    let rows: Vec<Vec<String>> = (top..=bottom.min(self.rows.len().saturating_sub(1)))
      .map(|row| (start..=end).map(|column| self.cell_text(row, column)).collect())
      .collect();
    if cut {
      self.clear_cells(at);
    }
    to_tsv(&rows)
  }

  /// Pastes tab-separated `text` from the range's first cell, or the focused one: each cell that is edited and
  /// would change is told to the app, which may refuse it. One value pasted into a range fills it. The range is
  /// then what was pasted into.
  pub(super) fn paste_cells(&self, at: (usize, usize), text: &str) {
    let block = from_tsv(text);
    let span = self.target(at);
    let (top, start) = (span.rows().0, span.columns().0);
    let (rows, columns) = (self.rows.len(), self.editors.len());
    let one = block.len() == 1 && block[0].len() == 1;
    let (height, width) = if one {
      (span.rows().1 - top + 1, span.columns().1 - start + 1)
    } else {
      (block.len(), block.iter().map(Vec::len).max().unwrap_or(0))
    };
    for (down, row) in (top..rows).take(height).enumerate() {
      for (across, column) in (start..columns).take(width).enumerate() {
        let text = if one {
          &block[0][0]
        } else {
          match block[down].get(across) {
            Some(text) => text,
            None => continue,
          }
        };
        if let Some(editor) = self.editor(column)
          && let Some(value) = parsed(&editor.kind, text)
        {
          self.change(row, column, value);
        }
      }
    }
    if !one && height * width > 1 {
      self.write_span(Some(Span {
        anchor: (top, start),
        extent: (
          (top + height - 1).min(rows.saturating_sub(1)),
          (start + width - 1).min(columns.saturating_sub(1)),
        ),
      }));
    }
  }

  /// Clears every cell of the range, or the focused one, that is edited: a text to nothing, a box unticked, a
  /// number and a choice told [`CellValue::Empty`].
  pub(super) fn clear_cells(&self, at: (usize, usize)) {
    let span = self.target(at);
    let ((top, bottom), (start, end)) = (span.rows(), span.columns());
    for row in top..=bottom.min(self.rows.len().saturating_sub(1)) {
      for column in start..=end {
        let Some(editor) = self.editor(column) else {
          continue;
        };
        let empty = match editor.kind {
          EditorKind::Text(_) => CellValue::Text(String::new()),
          EditorKind::Checkbox(_) => CellValue::Checked(false),
          EditorKind::Number(_) | EditorKind::Choice(..) => CellValue::Empty,
        };
        self.change(row, column, empty);
      }
    }
  }
}

impl HostState {
  /// The cell drawn at `(x, y)` among those built, or the nearest of them to it: the columns that stay first,
  /// which are drawn over the others.
  pub(super) fn cell_at(&self, x: f32, y: f32) -> Option<(usize, usize)> {
    let refs = self.cell_refs.lock();
    let metrics = *self.metrics.lock();
    let mut rows: BTreeMap<usize, (f32, f32)> = BTreeMap::new();
    let mut columns: BTreeMap<usize, (f32, f32)> = BTreeMap::new();
    for ((row, column), element) in refs.iter() {
      if !element.is_attached() {
        continue;
      }
      let (left, top, width, height) = element.rect();
      rows.entry(*row).or_insert((top, top + height));
      columns.entry(*column).or_insert((left, left + width));
    }
    let nearest = |places: &BTreeMap<usize, (f32, f32)>, at: f32, first: &dyn Fn(usize) -> bool| {
      places
        .iter()
        .filter(|(place, (from, to))| first(**place) && at >= *from && at < *to)
        .map(|(place, _)| *place)
        .next()
        .or_else(|| {
          places
            .iter()
            .filter(|(_, (from, to))| at >= *from && at < *to)
            .map(|(place, _)| *place)
            .next()
        })
        .or_else(|| {
          places
            .iter()
            .min_by(|(_, a), (_, b)| {
              let away = |(from, to): (f32, f32)| (from - at).abs().min((to - at).abs());
              away(**a).total_cmp(&away(**b))
            })
            .map(|(place, _)| *place)
        })
    };
    let count = self.columns_count();
    let (start, end) = metrics.pins;
    let pinned = move |column: usize| column < start || column + end >= count;
    Some((nearest(&rows, y, &|_| true)?, nearest(&columns, x, &pinned)?))
  }

  /// How many columns the grid has, as its tracks say.
  pub(super) fn columns_count(&self) -> usize {
    let lead = self.metrics.lock().lead_tracks;
    self
      .tracks
      .lock()
      .as_ref()
      .map_or(0, |(specs, _)| specs.len().saturating_sub(lead))
  }

  /// Scrolls the cell `(row, column)` into view, under the header and between the columns that stay, without
  /// moving the focus, and builds it.
  pub(super) fn reveal_cell(&self, row: usize, column: usize) {
    let metrics = *self.metrics.lock();
    let mut scroll_y = self.scroll.scroll_y();
    if row < metrics.count && metrics.row_height > 0.0 {
      let top = row as f32 * metrics.row_height;
      let room = (self.scroll.viewport_height() - metrics.header_height).max(metrics.row_height);
      if top < scroll_y {
        scroll_y = top;
      } else if top + metrics.row_height > scroll_y + room {
        scroll_y = top + metrics.row_height - room;
      }
    }
    let mut scroll_x = self.scroll.scroll_x();
    let tracks = self.tracks.lock().as_ref().map(|(_, tracks)| tracks.clone());
    let viewport = self.scroll.viewport_width();
    let (start, end) = metrics.pins;
    let count = self.columns_count();
    if let Some(tracks) = tracks
      && viewport > 0.0
      && column >= start
      && column + end < count
    {
      let available = (viewport - metrics.padding_start - metrics.padding_end).max(0.0);
      let widths = tracks.widths(Some(available), 0.0);
      let track = column + metrics.lead_tracks;
      let x = metrics.padding_start + widths[..track.min(widths.len())].iter().sum::<f32>();
      let width = widths.get(track).copied().unwrap_or(0.0);
      // What stays at each side covers the columns that scroll under it.
      let lead = if start > 0 {
        widths[..(metrics.lead_tracks + start).min(widths.len())]
          .iter()
          .sum::<f32>()
      } else {
        0.0
      };
      let trail = widths[widths.len().saturating_sub(end)..].iter().sum::<f32>();
      if x < scroll_x + lead {
        scroll_x = x - lead;
      } else if x + width > scroll_x + viewport - trail {
        scroll_x = x + width - viewport + trail;
      }
    }
    self.scroll.set_scroll(scroll_x.max(0.0), scroll_y.max(0.0));
    self.rebuild();
  }

  /// A drag of cells is past the edge of the view at `(x, y)`: the rows or the columns scroll a step that way.
  pub(super) fn scroll_toward(&self, x: f32, y: f32) {
    if !self.scroll_ref.is_attached() {
      return;
    }
    let metrics = *self.metrics.lock();
    let (left, top, width, height) = self.scroll_ref.rect();
    let step = metrics.row_height.max(24.0);
    let dy = if y > top + height {
      (y - top - height).min(step)
    } else if y < top + metrics.header_height {
      -(top + metrics.header_height - y).min(step)
    } else {
      0.0
    };
    let dx = if x > left + width {
      (x - left - width).min(step)
    } else if x < left {
      -(left - x).min(step)
    } else {
      0.0
    };
    if dx == 0.0 && dy == 0.0 {
      return;
    }
    self.scroll.set_scroll(
      (self.scroll.scroll_x() + dx).max(0.0),
      (self.scroll.scroll_y() + dy).max(0.0),
    );
    self.rebuild();
  }
}
