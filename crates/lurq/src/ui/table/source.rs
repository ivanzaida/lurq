//! Rows the app gives on demand, as a server's table gives them: the grid is told how many there are, asks for
//! the ranges that come into view through the engine's futures, shows skeleton rows where a range has not come
//! and says a failed one with a retry. The sort, the filters and the search are the app's to apply: each request
//! carries them, as one [`GridQuery`].

use std::{any::Any, collections::HashMap, future::Future, ops::Range, pin::Pin, sync::Arc};

use parking_lot::Mutex;

use super::{GridQuery, TableColumn, filters::FilterBy};
use crate::app::ctx::{Ctx, FutureAction};

/// A row of a grid over rows the app gives on demand: the row, where it has come.
pub struct SourceRow<R>(Option<Arc<R>>);

impl<R> SourceRow<R> {
  /// The row, where it has come: `None` while its range is asked for, or where it failed.
  pub fn get(&self) -> Option<&R> {
    self.0.as_deref()
  }
}

impl<R> Clone for SourceRow<R> {
  fn clone(&self) -> Self {
    Self(self.0.clone())
  }
}

/// What a grid over rows on demand asks the app for: the rows of `range`, as `query` sorts, filters and searches
/// them.
#[derive(Clone, Debug, PartialEq)]
pub struct GridRequest {
  pub query: GridQuery,
  pub range: Range<usize>,
}

/// What the app answers a request with: the rows asked for, in their order (fewer at the end), and how many there
/// are in all as the query has them.
#[derive(Clone, Debug, PartialEq)]
pub struct GridPage<R> {
  pub rows: Vec<R>,
  pub total: usize,
}

pub(super) type FetchFn<R> =
  Arc<dyn Fn(GridRequest) -> Pin<Box<dyn Future<Output = Result<GridPage<R>, String>> + Send>> + Send + Sync>;

/// How many rows a request asks for.
pub(super) const BLOCK: usize = 100;
/// How many requests are on their way at once.
const AT_ONCE: usize = 4;
/// How many ranges are kept: those far from the view go first.
const KEPT: usize = 200;

/// Where a row of the grid is: come, on its way, or failed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Slot {
  Ready,
  Pending,
  Failed,
}

/// A range that came, by the query it was asked by.
struct Arrived<R> {
  generation: u64,
  block: usize,
  page: Arc<GridPage<Arc<R>>>,
}

impl<R> Clone for Arrived<R> {
  fn clone(&self) -> Self {
    Self {
      generation: self.generation,
      block: self.block,
      page: self.page.clone(),
    }
  }
}

impl<R> PartialEq for Arrived<R> {
  fn eq(&self, other: &Self) -> bool {
    self.generation == other.generation && self.block == other.block && Arc::ptr_eq(&self.page, &other.page)
  }
}

impl<R> crate::app::component::DevtoolsInspectable for Arrived<R> {}

enum Block<R> {
  Loaded(Vec<Arc<R>>, u64),
  Pending,
  Failed,
}

/// What a grid's host keeps of its source, from render to render.
struct Held<R> {
  fetch: Arc<Mutex<Option<FetchFn<R>>>>,
  actions: Vec<FutureAction<(u64, usize, GridRequest), Arrived<R>, String>>,
  /// The range each request is on its way for, by the query.
  running: Vec<Option<(u64, usize)>>,
  query: Option<GridQuery>,
  generation: u64,
  total: usize,
  blocks: HashMap<usize, Block<R>>,
  clock: u64,
  rows: Arc<Vec<SourceRow<R>>>,
  /// Whether the query changed since the app was told.
  untold: bool,
}

/// A source of rows, with the type of the app's rows out of sight: what a table over `SourceRow`s asks of it.
pub(super) trait RowSource<R>: Send + Sync {
  /// Makes the requests' futures, as the grid is mounted.
  fn start(&self, ctx: &mut Ctx, held: &mut Option<Box<dyn Any + Send>>);
  /// The rows as they have come for `query`, and how many there are: a query of another sort, filters or search
  /// starts again.
  fn rows(&self, held: &mut Option<Box<dyn Any + Send>>, query: &GridQuery) -> (Arc<Vec<R>>, usize);
  /// Asks for the ranges of the rows from `first` to `last` that have not come.
  fn want(&self, held: &mut Option<Box<dyn Any + Send>>, first: usize, last: usize);
  /// Where each range is: a row's by its place.
  fn slots(&self, held: &mut Option<Box<dyn Any + Send>>) -> Arc<dyn Fn(usize) -> Slot + Send + Sync>;
  /// Asks again for the range of the row at `index`, where it failed.
  fn retry(&self, held: &mut Option<Box<dyn Any + Send>>, index: usize);
  /// The query, where it changed since the app was last told.
  fn untold(&self, held: &mut Option<Box<dyn Any + Send>>) -> Option<GridQuery>;
}

/// The app's source: how many rows there are at first, and how a range is asked for.
pub(super) struct Source<R> {
  pub total: usize,
  pub fetch: FetchFn<R>,
}

impl<R: Send + Sync + 'static> Source<R> {
  fn held<'a>(&self, held: &'a mut Option<Box<dyn Any + Send>>) -> Option<&'a mut Held<R>> {
    held.as_mut()?.downcast_mut::<Held<R>>()
  }
}

impl<R: Send + Sync + 'static> RowSource<SourceRow<R>> for Source<R> {
  fn start(&self, ctx: &mut Ctx, held: &mut Option<Box<dyn Any + Send>>) {
    let fetch: Arc<Mutex<Option<FetchFn<R>>>> = Arc::new(Mutex::new(Some(self.fetch.clone())));
    let actions = (0..AT_ONCE)
      .map(|_| {
        let fetch = fetch.clone();
        ctx.future_action(move |(generation, block, request): (u64, usize, GridRequest)| {
          let asked = fetch.lock().clone();
          async move {
            let Some(asked) = asked else {
              return Err(String::new());
            };
            let page = asked(request).await?;
            Ok(Arrived {
              generation,
              block,
              page: Arc::new(GridPage {
                rows: page.rows.into_iter().map(Arc::new).collect(),
                total: page.total,
              }),
            })
          }
        })
      })
      .collect();
    *held = Some(Box::new(Held::<R> {
      fetch,
      actions,
      running: vec![None; AT_ONCE],
      query: None,
      generation: 0,
      total: self.total,
      blocks: HashMap::new(),
      clock: 0,
      rows: Arc::new(Vec::new()),
      untold: false,
    }));
  }

  fn rows(&self, held: &mut Option<Box<dyn Any + Send>>, query: &GridQuery) -> (Arc<Vec<SourceRow<R>>>, usize) {
    let total = self.total;
    let Some(held) = self.held(held) else {
      return (Arc::new(Vec::new()), 0);
    };
    *held.fetch.lock() = Some(self.fetch.clone());
    let mut changed = false;
    if held.query.as_ref() != Some(query) {
      // Another sort, filters or search: every range is asked for again, and a request on its way is let go.
      held.untold = held.query.is_some();
      held.query = Some(query.clone());
      held.generation += 1;
      held.blocks.clear();
      held.total = total;
      changed = true;
    }
    for (at, action) in held.actions.iter().enumerate() {
      let state = action.state().get();
      let Some((generation, block)) = held.running[at] else {
        continue;
      };
      if state.is_pending() || state.is_idle() {
        continue;
      }
      held.running[at] = None;
      if generation != held.generation {
        continue;
      }
      match state.outcome() {
        Some(Ok(arrived)) if arrived.generation == generation && arrived.block == block => {
          held.clock += 1;
          held.total = arrived.page.total;
          held
            .blocks
            .insert(block, Block::Loaded(arrived.page.rows.clone(), held.clock));
        }
        _ => {
          held.blocks.insert(block, Block::Failed);
        }
      }
      changed = true;
    }
    // Far ranges go first where too many are kept.
    let loaded = held
      .blocks
      .values()
      .filter(|block| matches!(block, Block::Loaded(..)))
      .count();
    if loaded > KEPT {
      let mut ages: Vec<(u64, usize)> = held
        .blocks
        .iter()
        .filter_map(|(at, block)| match block {
          Block::Loaded(_, used) => Some((*used, *at)),
          _ => None,
        })
        .collect();
      ages.sort_unstable();
      for (_, at) in ages.into_iter().take(loaded - KEPT) {
        held.blocks.remove(&at);
      }
      changed = true;
    }
    if changed || held.rows.len() != held.total {
      let mut rows = vec![SourceRow(None); held.total];
      for (block, state) in &held.blocks {
        if let Block::Loaded(loaded, _) = state {
          for (offset, row) in loaded.iter().enumerate() {
            if let Some(slot) = rows.get_mut(block * BLOCK + offset) {
              *slot = SourceRow(Some(row.clone()));
            }
          }
        }
      }
      held.rows = Arc::new(rows);
    }
    (held.rows.clone(), held.total)
  }

  fn want(&self, held: &mut Option<Box<dyn Any + Send>>, first: usize, last: usize) {
    let Some(held) = self.held(held) else {
      return;
    };
    let Some(query) = held.query.clone() else {
      return;
    };
    if held.total == 0 {
      // Nothing is known yet: the first range tells how many there are.
      if held.blocks.is_empty() {
        held.total = 1;
      } else {
        return;
      }
    }
    held.clock += 1;
    let clock = held.clock;
    for block in first / BLOCK..=last.min(held.total.saturating_sub(1)) / BLOCK {
      match held.blocks.get_mut(&block) {
        Some(Block::Loaded(_, used)) => {
          *used = clock;
          continue;
        }
        Some(Block::Pending | Block::Failed) => continue,
        None => {}
      }
      let Some(at) = (0..held.actions.len())
        .find(|at| held.running[*at].is_none() && !held.actions[*at].state().get_untracked().is_pending())
      else {
        // Every request is on its way: the next comes as one of them does.
        break;
      };
      held.blocks.insert(block, Block::Pending);
      held.running[at] = Some((held.generation, block));
      let range = block * BLOCK..((block + 1) * BLOCK).min(held.total.max(block * BLOCK + BLOCK));
      held.actions[at].run((
        held.generation,
        block,
        GridRequest {
          query: query.clone(),
          range,
        },
      ));
    }
  }

  fn slots(&self, held: &mut Option<Box<dyn Any + Send>>) -> Arc<dyn Fn(usize) -> Slot + Send + Sync> {
    let Some(held) = self.held(held) else {
      return Arc::new(|_| Slot::Pending);
    };
    let states: HashMap<usize, Slot> = held
      .blocks
      .iter()
      .map(|(at, block)| {
        (
          *at,
          match block {
            Block::Loaded(..) => Slot::Ready,
            Block::Pending => Slot::Pending,
            Block::Failed => Slot::Failed,
          },
        )
      })
      .collect();
    Arc::new(move |index| states.get(&(index / BLOCK)).copied().unwrap_or(Slot::Pending))
  }

  fn retry(&self, held: &mut Option<Box<dyn Any + Send>>, index: usize) {
    if let Some(held) = self.held(held)
      && matches!(held.blocks.get(&(index / BLOCK)), Some(Block::Failed))
    {
      held.blocks.remove(&(index / BLOCK));
    }
  }

  fn untold(&self, held: &mut Option<Box<dyn Any + Send>>) -> Option<GridQuery> {
    let held = self.held(held)?;
    if !held.untold {
      return None;
    }
    held.untold = false;
    held.query.clone()
  }
}

impl<R: Send + Sync + 'static> TableColumn<R> {
  /// The column over the rows of a source: what it shows of a row that has come, nothing of one that has not. It
  /// sorts and filters as it says to the person (its header, its menu); the app applies them.
  pub(super) fn over_source(self) -> TableColumn<SourceRow<R>> {
    use super::Content;
    let content = match self.content {
      Content::Text(text) => Content::Text(Arc::new(move |row: &SourceRow<R>| {
        row.get().map(|row| text(row)).unwrap_or_default()
      })),
      Content::Element(cell) => Content::Element(Arc::new(move |row: &SourceRow<R>| match row.get() {
        Some(row) => cell(row),
        None => crate::components::Row::new().into(),
      })),
      Content::Nothing => Content::Nothing,
    };
    let filter = self.filter.map(|by| match by {
      FilterBy::Text => FilterBy::Text,
      FilterBy::Set => FilterBy::Set,
      FilterBy::Values(values) => FilterBy::Values(Arc::new(move |row: &SourceRow<R>| {
        row.get().map(|row| values(row)).unwrap_or_default()
      })),
      FilterBy::Number(number) => FilterBy::Number(Arc::new(move |row: &SourceRow<R>| {
        row.get().map_or(f64::NAN, |row| number(row))
      })),
      FilterBy::Date(date) => FilterBy::Date(Arc::new(move |row: &SourceRow<R>| row.get().and_then(|row| date(row)))),
      FilterBy::Bool(value) => FilterBy::Bool(Arc::new(move |row: &SourceRow<R>| {
        row.get().is_some_and(|row| value(row))
      })),
    });
    TableColumn {
      id: self.id,
      title: self.title,
      title_hidden: self.title_hidden,
      track: self.track,
      align: self.align,
      // A column that sorts says so: the app sorts the rows it gives.
      sortable: self.sortable || self.compare.is_some(),
      pin: self.pin,
      resizable: self.resizable,
      controls: self.controls,
      kinds: self.kinds,
      content,
      compare: None,
      editor: None,
      summary: None,
      wrap: self.wrap,
      filter,
    }
  }
}

impl<R: Send + Sync + 'static> super::Table<SourceRow<R>, usize> {
  /// A table over the rows `fetch` gives on demand, `total` of them at first.
  pub(crate) fn over_source<Fut>(
    columns: impl IntoIterator<Item = TableColumn<R>>,
    total: usize,
    fetch: impl Fn(GridRequest) -> Fut + Send + Sync + 'static,
  ) -> Self
  where
    Fut: Future<Output = Result<GridPage<R>, String>> + Send + 'static,
  {
    let fetch: FetchFn<R> = Arc::new(move |request| Box::pin(fetch(request)));
    let mut table = super::Table::new_shared(columns.into_iter().map(TableColumn::over_source), Arc::new(Vec::new()));
    table.source = Some(Arc::new(Source { total, fetch }));
    table
  }
}
