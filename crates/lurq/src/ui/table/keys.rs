//! The keys of a table's rows, reckoned as they are asked for: the key of a row built, of a row a key goes to, of
//! the corners of a range. Where a key is looked up among the rows (the selected ones, for the header's box), the
//! table makes an index of every key once for the rows it is given, and keeps it while they are the same.

use std::{
  any::Any,
  collections::HashMap,
  hash::Hash,
  sync::{Arc, OnceLock},
};

use super::groups::Lines;

/// Where each key is among all the rows: made once it is asked for.
pub(super) type KeyIndex<K> = Arc<OnceLock<HashMap<K, usize>>>;

/// The keys of the rows of a table, by their place among all the rows or their rank among those shown.
pub(super) struct RowKeys<K> {
  key: Arc<dyn Fn(usize) -> K + Send + Sync>,
  count: usize,
  lines: Arc<Lines>,
  index: KeyIndex<K>,
}

impl<K> Clone for RowKeys<K> {
  fn clone(&self) -> Self {
    Self {
      key: self.key.clone(),
      count: self.count,
      lines: self.lines.clone(),
      index: self.index.clone(),
    }
  }
}

impl<K: Clone + Eq + Hash> RowKeys<K> {
  pub fn new(key: Arc<dyn Fn(usize) -> K + Send + Sync>, count: usize, lines: Arc<Lines>, index: KeyIndex<K>) -> Self {
    Self {
      key,
      count,
      lines,
      index,
    }
  }

  /// How many rows are shown: the ranks.
  pub fn len(&self) -> usize {
    self.lines.ranks()
  }

  /// How many rows there are, shown or not.
  pub fn count(&self) -> usize {
    self.count
  }

  /// The key of the row of `rank` among those shown.
  pub fn get(&self, rank: usize) -> Option<K> {
    self.lines.row_of_rank(rank).map(|index| (self.key)(index))
  }

  /// The key of the row at `index` among all the rows.
  pub fn at(&self, index: usize) -> K {
    (self.key)(index)
  }

  /// The place among all the rows of the row `key` names.
  pub fn index_of(&self, key: &K) -> Option<usize> {
    self
      .index
      .get_or_init(|| (0..self.count).map(|index| ((self.key)(index), index)).collect())
      .get(key)
      .copied()
  }

  /// The rank among those shown of the row `key` names.
  pub fn rank_of(&self, key: &K) -> Option<usize> {
    self.lines.rank_of_index(self.index_of(key)?)
  }

  /// The keys of the rows of the ranks from `first` to `last`.
  pub fn ranked(&self, first: usize, last: usize) -> Vec<K> {
    (first..=last).filter_map(|rank| self.get(rank)).collect()
  }

  /// How many of `keys` name rows that are shown, each counted once.
  pub fn shown_among<'a>(&self, keys: impl IntoIterator<Item = &'a K>) -> usize
  where
    K: 'a,
  {
    let mut seen = std::collections::HashSet::new();
    keys
      .into_iter()
      .filter(|key| seen.insert(*key))
      .filter(|key| self.rank_of(key).is_some())
      .count()
  }
}

/// The index of the keys a mounted table made for the rows it was last given, held with those rows so that they are
/// the same rows while it is kept.
#[derive(Default)]
pub(super) struct KeptIndex(Option<(Arc<dyn Any + Send + Sync>, Arc<dyn Any + Send + Sync>)>);

impl KeptIndex {
  /// The index for `rows`: the one kept, where it was made for them, or a new one, kept from now on.
  pub fn for_rows<R: Send + Sync + 'static, K: Send + Sync + 'static>(&mut self, rows: &Arc<Vec<R>>) -> KeyIndex<K> {
    if let Some((kept_rows, index)) = &self.0
      && let Some(kept_rows) = kept_rows.downcast_ref::<Arc<Vec<R>>>()
      && Arc::ptr_eq(kept_rows, rows)
      && let Some(index) = index.downcast_ref::<KeyIndex<K>>()
    {
      return index.clone();
    }
    let index: KeyIndex<K> = Arc::new(OnceLock::new());
    self.0 = Some((Arc::new(rows.clone()), Arc::new(index.clone())));
    index
  }
}
