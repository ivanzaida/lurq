//! A search of what a data grid's cells show: every match of a query in the text of the cells of the rows shown,
//! the current one among them, and moving from one to the next across rows not built yet.

use std::{ops::Range, sync::Arc};

use super::{HostState, groups::Lines};
use crate::{
  node::FocusRows,
  ui::highlight::{FindOptions, find_matches},
};

/// How many matches a grid's search has, and which is the current one: what the app is told
/// ([`DataGrid::on_matches_change`](crate::ui::DataGrid::on_matches_change)) and what a find bar shows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SearchMatches {
  pub count: usize,
  /// The current match, from 0: `None` where there is none.
  pub current: Option<usize>,
}

impl crate::app::component::DevtoolsInspectable for SearchMatches {}

/// A match: in the cell of a row, by its rank among those shown, and a column, by its place among those shown;
/// the bytes of the cell's text it covers.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Found {
  pub rank: usize,
  pub column: usize,
  pub range: Range<usize>,
}

/// What the matches were found for: the rows (held, so that they are the same while this is kept), the lines, the
/// columns, the query and how it is matched.
#[derive(Clone, PartialEq)]
struct SearchKey {
  rows: usize,
  lines: usize,
  columns: Vec<Arc<str>>,
  query: String,
  match_case: bool,
  whole_word: bool,
  regex: bool,
}

/// What a grid's host keeps of its search.
#[derive(Default)]
pub(super) struct SearchState {
  key: Option<SearchKey>,
  /// Holds the rows the matches were found in, so that their place is not another list's.
  rows: Option<Arc<dyn std::any::Any + Send + Sync>>,
  pub found: Arc<Vec<Found>>,
  pub current: usize,
  /// What the app was last told, and who is told.
  told: Option<SearchMatches>,
  pub heard: Vec<Arc<dyn Fn(SearchMatches) + Send + Sync>>,
}

impl SearchState {
  /// The matches of `query` in the rows `lines` shows of `rows`, each column's text by `texts` (`None` for a
  /// column whose cell is an element): those kept where they were found for the same, or found now.
  #[allow(clippy::too_many_arguments)]
  pub fn find<R: Send + Sync + 'static>(
    &mut self,
    rows: &Arc<Vec<R>>,
    lines: &Arc<Lines>,
    ids: Vec<Arc<str>>,
    texts: &[Option<Arc<dyn Fn(&R) -> String + Send + Sync>>],
    query: &str,
    options: FindOptions,
  ) -> Arc<Vec<Found>> {
    let key = SearchKey {
      rows: Arc::as_ptr(rows) as *const () as usize,
      lines: Arc::as_ptr(lines) as usize,
      columns: ids,
      query: query.to_owned(),
      match_case: options.match_case,
      whole_word: options.whole_word,
      regex: options.regex,
    };
    if self.key.as_ref() == Some(&key) {
      return self.found.clone();
    }
    let mut found = Vec::new();
    if !query.is_empty() {
      for rank in 0..lines.ranks() {
        let Some(index) = lines.row_of_rank(rank) else {
          continue;
        };
        let row = &rows[index];
        for (column, text) in texts.iter().enumerate() {
          let Some(text) = text else {
            continue;
          };
          for range in find_matches(&text(row), query, options).unwrap_or_default() {
            found.push(Found { rank, column, range });
          }
        }
      }
    }
    // A new search starts at its first match; one of the same query found again keeps its place where it can.
    let same_query = self.key.as_ref().is_some_and(|held| held.query == key.query);
    if !same_query || self.current >= found.len() {
      self.current = 0;
    }
    self.found = Arc::new(found);
    self.key = Some(key);
    self.rows = Some(Arc::new(rows.clone()));
    self.found.clone()
  }

  pub fn matches(&self) -> SearchMatches {
    SearchMatches {
      count: self.found.len(),
      current: (!self.found.is_empty()).then_some(self.current),
    }
  }

  /// The matches in the cell of `rank` and `column`, and which of them is the current one.
  pub fn in_cell(found: &[Found], current: usize, rank: usize, column: usize) -> (Vec<Range<usize>>, Option<usize>) {
    let start = found.partition_point(|at| (at.rank, at.column) < (rank, column));
    let mut ranges = Vec::new();
    let mut now = None;
    for (at, hit) in found[start..].iter().enumerate() {
      if (hit.rank, hit.column) != (rank, column) {
        break;
      }
      if start + at == current {
        now = Some(ranges.len());
      }
      ranges.push(hit.range.clone());
    }
    (ranges, now)
  }

  /// What the app is to be told, where it is not what it was told last.
  pub fn untold(&mut self) -> Option<(SearchMatches, Vec<Arc<dyn Fn(SearchMatches) + Send + Sync>>)> {
    let matches = self.matches();
    if self.told == Some(matches) {
      return None;
    }
    self.told = Some(matches);
    Some((matches, self.heard.clone()))
  }
}

impl HostState {
  /// Goes `by` matches on from the current one, round from the last to the first, and brings it into view.
  pub(super) fn step_match(&self, by: isize) {
    let (rank, column) = {
      let mut search = self.search.lock();
      let count = search.found.len();
      if count == 0 {
        return;
      }
      search.current = (search.current as isize + by).rem_euclid(count as isize) as usize;
      let hit = &search.found[search.current];
      (hit.rank, hit.column)
    };
    let position = self.lines.lock().position_of(rank).unwrap_or(rank);
    self.reveal_column(column);
    self.reveal(position);
  }

  /// Scrolls the column at `column` among those shown into view sideways, where it is not.
  fn reveal_column(&self, column: usize) {
    let metrics = *self.metrics.lock();
    let Some(tracks) = self.tracks.lock().as_ref().map(|(_, tracks)| tracks.clone()) else {
      return;
    };
    let viewport = self.scroll.viewport_width();
    if viewport <= 0.0 {
      return;
    }
    let available = (viewport - metrics.padding_start - metrics.padding_end).max(0.0);
    let widths = tracks.widths(Some(available), 0.0);
    let track = metrics.lead_tracks + column;
    let Some(width) = widths.get(track) else {
      return;
    };
    let x = metrics.padding_start + widths[..track].iter().sum::<f32>();
    // The columns pinned at the start cover what scrolls under them.
    let covered = metrics.padding_start
      + widths[..metrics.lead_tracks + metrics.pins.0.min(column)]
        .iter()
        .sum::<f32>();
    let scroll_x = self.scroll.scroll_x();
    let next = if x - covered < scroll_x {
      (x - covered).max(0.0)
    } else if x + width > scroll_x + viewport {
      x + width - viewport
    } else {
      return;
    };
    self.scroll.set_scroll(next, self.scroll.scroll_y());
  }
}
