//! The columns of a data grid as a person arranges them: their order, which are hidden, their widths and where
//! they are pinned, a value of the grid the app holds and stores; and the menu of a column's header that
//! arranges them.

use std::sync::Arc;

use super::{ColumnPin, HostState, SortDirection, TableColumn, TrackWidth};

/// Where one column of a data grid is, as a person arranged it.
#[derive(Clone, Debug, PartialEq)]
pub struct ColumnPlace {
  /// The column's id.
  pub id: Arc<str>,
  /// Whether it is hidden: shown again from a column's menu.
  pub hidden: bool,
  /// The width a person made it: `None` for the column's own.
  pub width: Option<f32>,
  /// Whether it is as wide as its content: a column sized to fit from its menu.
  pub fit: bool,
  /// Where it stays while the rows scroll sideways.
  pub pin: ColumnPin,
}

/// The columns of a data grid as a person arranged them: in the order they are shown, each hidden or not, its
/// width and its pin. The grid tells the whole of it, every column in it, at each change, so the app that stores
/// it gives the grid back what a person left; a column it does not name stays as the app gave it, after the
/// others.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ColumnLayout {
  pub columns: Vec<ColumnPlace>,
}

#[cfg(feature = "devtools")]
impl crate::app::component::DevtoolsInspectable for ColumnLayout {}

impl ColumnLayout {
  /// The place of the column `id`, where the layout names it.
  pub fn place(&self, id: &str) -> Option<&ColumnPlace> {
    self.columns.iter().find(|place| &*place.id == id)
  }

  /// The ids of the columns that are hidden.
  pub fn hidden(&self) -> impl Iterator<Item = &str> {
    self.columns.iter().filter(|place| place.hidden).map(|place| &*place.id)
  }

  /// Every column of `columns` in the layout's order, as the layout places it or as the app gave it.
  pub(super) fn full<R>(&self, columns: &[TableColumn<R>]) -> ColumnLayout {
    let mut places: Vec<ColumnPlace> = self
      .columns
      .iter()
      .filter(|place| columns.iter().any(|column| column.id == place.id))
      .cloned()
      .collect();
    for column in columns {
      if !places.iter().any(|place| place.id == column.id) {
        places.push(ColumnPlace {
          id: column.id.clone(),
          hidden: false,
          width: None,
          fit: false,
          pin: column.pin,
        });
      }
    }
    ColumnLayout { columns: places }
  }

  /// `columns` as the layout arranges them: in its order, the hidden ones out, each with its width and its pin;
  /// the pinned at the start first and the pinned at the end last, each in the order they are in.
  pub(super) fn arrange<R>(&self, columns: Vec<TableColumn<R>>) -> Vec<TableColumn<R>> {
    let full = self.full(&columns);
    let mut by_id: Vec<Option<TableColumn<R>>> = columns.into_iter().map(Some).collect();
    let mut arranged = Vec::with_capacity(by_id.len());
    for place in &full.columns {
      let Some(slot) = by_id
        .iter_mut()
        .find(|slot| slot.as_ref().is_some_and(|column| column.id == place.id))
      else {
        continue;
      };
      let mut column = slot.take().expect("found");
      if place.hidden {
        continue;
      }
      if place.fit {
        column.track.width = TrackWidth::Content;
      } else if let Some(width) = place.width {
        column.track.width = TrackWidth::Px(width.max(0.0));
      }
      column.pin = place.pin;
      arranged.push(column);
    }
    let rank = |pin: ColumnPin| match pin {
      ColumnPin::Start => 0,
      ColumnPin::None => 1,
      ColumnPin::End => 2,
    };
    arranged.sort_by_key(|column| rank(column.pin));
    arranged
  }

  fn place_mut(&mut self, id: &str) -> Option<&mut ColumnPlace> {
    self.columns.iter_mut().find(|place| &*place.id == id)
  }

  /// The layout with the column `id` hidden or shown.
  pub(super) fn with_hidden(mut self, id: &str, hidden: bool) -> Self {
    if let Some(place) = self.place_mut(id) {
      place.hidden = hidden;
    }
    self
  }

  /// The layout with every column shown.
  pub(super) fn with_all_shown(mut self) -> Self {
    for place in &mut self.columns {
      place.hidden = false;
    }
    self
  }

  /// The layout with the column `id` pinned at `pin`.
  pub(super) fn with_pin(mut self, id: &str, pin: ColumnPin) -> Self {
    if let Some(place) = self.place_mut(id) {
      place.pin = pin;
    }
    self
  }

  /// The layout with the column `id` as wide as its content.
  pub(super) fn with_fit(mut self, id: &str) -> Self {
    if let Some(place) = self.place_mut(id) {
      place.fit = true;
      place.width = None;
    }
    self
  }

  /// The layout with the column `id` `width` wide.
  pub(super) fn with_width(mut self, id: &str, width: f32) -> Self {
    if let Some(place) = self.place_mut(id) {
      place.fit = false;
      place.width = Some(width);
    }
    self
  }

  /// The layout with the column `id` moved to before the shown column `before` (to the end of the shown ones
  /// where it is `None`), among the columns pinned as it is.
  pub(super) fn with_moved(mut self, id: &str, before: Option<&str>) -> Self {
    let Some(from) = self.columns.iter().position(|place| &*place.id == id) else {
      return self;
    };
    if before == Some(id) {
      return self;
    }
    let place = self.columns.remove(from);
    let at = before
      .and_then(|before| self.columns.iter().position(|place| &*place.id == before))
      .unwrap_or_else(|| {
        // After the last shown column pinned as it is.
        self
          .columns
          .iter()
          .rposition(|other| !other.hidden && other.pin == place.pin)
          .map_or(self.columns.len(), |last| last + 1)
      });
    self.columns.insert(at, place);
    self
  }
}

/// What a column's menu does.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum MenuChoice {
  Sort(Option<SortDirection>),
  Pin(ColumnPin),
  Fit,
  Hide,
  /// Shows a hidden column, or hides a shown one: the menu's list of columns.
  Toggle(Arc<str>),
  ShowAll,
}

/// Where a column dragged by its header goes, drawn on a header: before it, or after the last.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum DropSide {
  Before,
  After,
}

type Choose = Arc<dyn Fn(MenuChoice) + Send + Sync>;
type Told<T> = Arc<dyn Fn(T) + Send + Sync>;

/// A column's menu and its grid: what the menu shows of the column and the others, and what its choices, the
/// keys of the header and a drag of it do.
pub(super) struct ColumnMenuLink {
  pub id: Arc<str>,
  pub title: Arc<str>,
  /// Whether the menu sorts the rows by the column, and which way they are sorted by it.
  pub sortable: bool,
  pub sorted: Option<SortDirection>,
  pub pin: ColumnPin,
  /// Whether the column may be hidden: not the last one shown.
  pub can_hide: bool,
  /// Every column, shown or not: its id, its title and whether it is shown.
  pub columns: Vec<(Arc<str>, Arc<str>, bool)>,
  pub open: bool,
  pub host: Arc<HostState>,
  pub choose: Choose,
  pub set_open: Told<bool>,
  /// Moves the column by places among those shown: the keys of its header.
  pub move_by: Told<isize>,
  /// Moves the column to before the shown column at a place, or to the end of them: a drag of its header.
  pub move_to: Told<Option<usize>>,
  /// The column's place among those shown.
  pub place: usize,
}

impl HostState {
  /// Where a column dragged by its header goes at `x`: before the shown column whose middle is after `x`, or
  /// after the last where none is.
  pub(super) fn drop_place(&self, x: f32) -> Option<usize> {
    let refs = self.header_refs.lock();
    let mut places: Vec<(usize, f32)> = refs
      .iter()
      .filter(|(_, element)| element.is_attached())
      .map(|(column, element)| {
        let (left, _, width, _) = element.rect();
        (*column, left + width / 2.0)
      })
      .collect();
    places.sort_by_key(|(column, _)| *column);
    places
      .into_iter()
      .find(|(_, middle)| x < *middle)
      .map(|(column, _)| column)
  }
}
