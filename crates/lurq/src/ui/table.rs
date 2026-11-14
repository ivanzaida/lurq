//! A table: rows of the same things in columns, which line up whatever is in a cell.

use std::{
  cell::Cell,
  cmp::Ordering,
  collections::{HashMap, HashSet},
  hash::Hash,
  marker::PhantomData,
  sync::Arc,
  time::Instant,
};

use parking_lot::Mutex;

use super::{
  checkbox::Checkbox,
  empty_state::{EmptyState, EmptyStateSize},
  icons,
  input::{Input, InputSize},
  menu::{Menu, MenuItem},
  names,
  number_input::NumberInput,
  part_style::{FontFamily, Length},
  recipe::{Case, Interaction, Name, Recipe, ResolvedPart, ResolvedRecipe, Selector},
  select::Select,
  shared::{fading, glyph, pressable},
  skeleton::Skeleton,
  theme::{Themed, current_theme},
};
mod columns;
mod groups;
mod range;

pub use columns::{ColumnLayout, ColumnPlace};
use columns::{ColumnMenuLink, DropSide, MenuChoice};
pub use groups::{GroupKey, TableGroup};
use groups::{Lines, Shown};
pub use range::{CellKey, CellRange};
use range::{RangeMark, Span, span_of};

use crate::{
  app::{
    component::Component,
    ctx::{Ctx, Placement},
    events::{CopyEvent, DragEvent, KeyboardEvent, MouseButton, MouseButtonMask, MouseEvent, PasteEvent, ScrollEvent},
    runtime::{DOUBLE_CLICK_DISTANCE, DOUBLE_CLICK_INTERVAL},
    theme::{BorderSize, Density, IconSize, OpacityLevel, PaletteColor, RadiusSize, Theme, TypographyStyle},
  },
  components::{self, Column, Row, ScrollBoth},
  core::{ElementRef, Signal, SignalValue},
  layout::{
    Alignment,
    column_tracks::{ColumnTracks, TrackSpec, TrackWidth},
    layout_kind::{Justify, ScrollState, Sticky},
    text_style::FontWeight,
  },
  node::{
    BackgroundColor, CursorIcon, Element, FocusGroup, FocusRows, border::Border as Line, dimension::Dimension,
    outline::Outline,
  },
};

/// The parts of a table a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TablePart {
  /// The box: its edge, its corners, its fill. It clips what scrolls in it.
  Root,
  /// What scrolls, both ways: the header and the rows.
  Scroll,
  /// The header row: its height, its fill, the line under it, its padding and the room between columns.
  Header,
  /// A column's header: its type and its color. A sortable one is pressed, and colored under the pointer.
  HeaderCell,
  /// The arrow of the column the rows are sorted by, and the mark of one that sorts.
  SortIcon,
  /// What is dragged at the end of a column's header to make it wider or narrower: its width, its cursor.
  Resizer,
  /// The line in the resizer, shown under the pointer and while it is dragged.
  ResizeLine,
  /// What opens a column's menu in a data grid's header: shown under the pointer, with the focus and while the
  /// menu is open.
  MenuTrigger,
  /// Where a column dragged by its header goes: a line at the start of a header, or the end of the last.
  DropLine,
  /// A group's row, where the rows are grouped by a column's value: what is pressed to open and close it.
  GroupRow,
  /// The chevron of a group's row, which points down while it is open.
  Chevron,
  /// The value of a group's row.
  GroupLabel,
  /// How many rows a group holds.
  GroupCount,
  /// One row: its height, its padding, the room between columns, the line under it. What is pressed,
  /// selected and focused.
  Row,
  /// A cell of a row: its type and its color.
  Cell,
  /// The cell of the column of checkboxes, in the header and in each row.
  Select,
  /// What holds the control a cell of a data grid is edited with, while it is.
  Editor,
  /// What stands under the header where there are no rows.
  Empty,
  /// The footer under the rows: the count of them and the pagination.
  Footer,
  /// What the footer says of the rows: "24 rows", "1 of 24 rows selected".
  Count,
}

names!(TablePart {
  Root => "root",
  Scroll => "scroll",
  Header => "header",
  HeaderCell => "header_cell",
  SortIcon => "sort_icon",
  Resizer => "resizer",
  ResizeLine => "resize_line",
  MenuTrigger => "menu_trigger",
  DropLine => "drop_line",
  GroupRow => "group_row",
  Chevron => "chevron",
  GroupLabel => "group_label",
  GroupCount => "group_count",
  Row => "row",
  Cell => "cell",
  Select => "select",
  Editor => "editor",
  Empty => "empty",
  Footer => "footer",
  Count => "count",
});

/// The states of a table, its rows and its cells a recipe styles, besides what the pointer and the
/// keyboard do to them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TableState {
  /// Its rows are selected: it has a column of checkboxes.
  Selectable,
  /// A row is selected.
  Selected,
  /// The last row, with nothing under it in the box: it has no line under it.
  Last,
  /// A header that sorts the rows when it is pressed.
  Sortable,
  /// The header of the column the rows are sorted by.
  Sorted,
  /// A cell of a column whose text is in the muted color: a date, a note.
  Muted,
  /// A cell of a column whose text is in the mono type: a path, an id.
  Mono,
  /// A cell of a column of numbers: at its end, in the mono type, smaller and muted.
  Numeric,
  /// A cell or a header of a column that stays while the rows scroll sideways ([`ColumnPin`]).
  Pinned,
  /// It has a footer under its rows.
  Footer,
  /// It takes no input.
  Disabled,
  /// A cell of a data grid in the range of cells selected, but the one the focus is on: the range's fill.
  InRange,
  /// A cell at the top of the range of cells selected: the range's edge along its top.
  RangeTop,
  /// A cell at the bottom of the range: the edge along its bottom.
  RangeBottom,
  /// A cell at the start of the range: the edge along its start.
  RangeStart,
  /// A cell at the end of the range: the edge along its end.
  RangeEnd,
  /// A header whose column's menu is open.
  Open,
  /// A header a column dragged by its header goes before.
  DropBefore,
  /// The last header, where a column dragged by its header goes after it.
  DropAfter,
}

names!(TableState {
  Selectable => "selectable",
  Selected => "selected",
  Last => "last",
  Sortable => "sortable",
  Sorted => "sorted",
  Muted => "muted",
  Mono => "mono",
  Numeric => "numeric",
  Pinned => "pinned",
  Footer => "footer",
  Disabled => "disabled",
  InRange => "in_range",
  RangeTop => "range_top",
  RangeBottom => "range_bottom",
  RangeStart => "range_start",
  RangeEnd => "range_end",
  Open => "open",
  DropBefore => "drop_before",
  DropAfter => "drop_after",
});

/// What builds a table's nodes: a theme's in place of the default one.
pub type TableRenderer = Arc<dyn Fn(TableView) -> Element + Send + Sync>;

/// Which way the rows are sorted by a column.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SortDirection {
  /// The least first: A to Z, 1 to 9, the oldest first.
  Ascending,
  /// The most first.
  Descending,
}

/// The column the rows of a table are sorted by, and which way.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TableSort {
  /// The id of the column.
  pub column: Arc<str>,
  pub direction: SortDirection,
}

#[cfg(feature = "devtools")]
impl crate::app::component::DevtoolsInspectable for TableSort {}

impl TableSort {
  pub fn ascending(column: &str) -> Self {
    Self {
      column: Arc::from(column),
      direction: SortDirection::Ascending,
    }
  }

  pub fn descending(column: &str) -> Self {
    Self {
      column: Arc::from(column),
      direction: SortDirection::Descending,
    }
  }

  /// What a press of the header of `column` makes of `current`: a column the rows are not sorted by sorts
  /// them ascending, then descending, then not at all.
  pub fn after_press(current: Option<&TableSort>, column: &str) -> Option<TableSort> {
    match current {
      Some(sort) if &*sort.column == column => match sort.direction {
        SortDirection::Ascending => Some(Self::descending(column)),
        SortDirection::Descending => None,
      },
      _ => Some(Self::ascending(column)),
    }
  }

  /// Sorts `rows` by this column of `columns`, by what the column compares rows with
  /// ([`TableColumn::sort_by`], [`TableColumn::sort_key`]) or, for a column of text, by its text. Rows that
  /// compare equal keep their order. A column it does not find, or one with nothing to compare by, leaves
  /// them as they are.
  pub fn sort<R: 'static>(&self, columns: &[TableColumn<R>], rows: &mut [R]) {
    let Some(column) = columns.iter().find(|column| column.id == self.column) else {
      return;
    };
    let compare: CompareFn<R> = match (&column.compare, &column.content) {
      (Some(compare), _) => compare.clone(),
      (None, Content::Text(text)) => {
        let text = text.clone();
        Arc::new(move |a: &R, b: &R| text(a).cmp(&text(b)))
      }
      (None, _) => return,
    };
    match self.direction {
      SortDirection::Ascending => rows.sort_by(|a, b| compare(a, b)),
      SortDirection::Descending => rows.sort_by(|a, b| compare(b, a)),
    }
  }
}

/// Where a cell's content sits in its column.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ColumnAlign {
  #[default]
  Start,
  Center,
  /// At its end: a number, the actions of a row.
  End,
}

/// Where a column stays while the rows scroll sideways under the others.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ColumnPin {
  /// It scrolls with the rows.
  #[default]
  None,
  /// It stays at the start: a name, an id. The columns pinned there are the first ones, one after another.
  Start,
  /// It stays at the end: a row's actions, a total. The columns pinned there are the last ones.
  End,
}

type CellFn<R> = Arc<dyn Fn(&R) -> Element + Send + Sync>;
type TextFn<R> = Arc<dyn Fn(&R) -> String + Send + Sync>;
type CompareFn<R> = Arc<dyn Fn(&R, &R) -> Ordering + Send + Sync>;

/// What a column builds a row's cell from.
enum Content<R> {
  /// A text, in the type of the cell.
  Text(TextFn<R>),
  /// Whatever the app builds: a badge, an avatar and a name, buttons.
  Element(CellFn<R>),
  Nothing,
}

impl<R> Clone for Content<R> {
  fn clone(&self) -> Self {
    match self {
      Self::Text(text) => Self::Text(text.clone()),
      Self::Element(cell) => Self::Element(cell.clone()),
      Self::Nothing => Self::Nothing,
    }
  }
}

/// A column of a table: its header, its width, where its cells' content sits, and how a row's cell is
/// built from the row.
///
/// ```
/// use lurq::ui::{Badge, TableColumn};
///
/// struct Crate { name: String, status: &'static str, size_kb: u32 }
///
/// let name = TableColumn::new("name", "Name").text(|row: &Crate| row.name.clone()).mono(true).sortable(true);
/// let status = TableColumn::new("status", "Status").width(120.0).cell(|row: &Crate| Badge::new(row.status).into());
/// let size = TableColumn::new("size", "Size")
///   .width(80.0)
///   .numeric()
///   .text(|row: &Crate| format!("{} KB", row.size_kb))
///   .sort_key(|row: &Crate| row.size_kb);
/// ```
pub struct TableColumn<R> {
  id: Arc<str>,
  title: Arc<str>,
  title_hidden: bool,
  track: TrackSpec,
  align: ColumnAlign,
  sortable: bool,
  pin: ColumnPin,
  resizable: Option<bool>,
  controls: bool,
  kinds: Vec<Name>,
  content: Content<R>,
  compare: Option<CompareFn<R>>,
  editor: Option<CellEditor<R>>,
}

impl<R> Clone for TableColumn<R> {
  fn clone(&self) -> Self {
    Self {
      id: self.id.clone(),
      title: self.title.clone(),
      title_hidden: self.title_hidden,
      track: self.track,
      align: self.align,
      sortable: self.sortable,
      pin: self.pin,
      resizable: self.resizable,
      controls: self.controls,
      kinds: self.kinds.clone(),
      content: self.content.clone(),
      compare: self.compare.clone(),
      editor: self.editor.clone(),
    }
  }
}

impl<R: 'static> TableColumn<R> {
  /// A column of `id`, under the header `title`, that takes a share of the room the others leave, and whose
  /// cells are empty until it is told what to build them from.
  pub fn new(id: &str, title: &str) -> Self {
    Self {
      id: Arc::from(id),
      title: Arc::from(title),
      title_hidden: false,
      track: TrackSpec::share(1.0),
      align: ColumnAlign::Start,
      sortable: false,
      pin: ColumnPin::None,
      resizable: None,
      controls: false,
      kinds: Vec::new(),
      content: Content::Nothing,
      compare: None,
      editor: None,
    }
  }

  /// A column of what a row's end holds that is pressed: buttons, a menu. Its header says nothing (it is
  /// called "Actions" to what reads the tree), it is as wide as what is in it, its cells sit at its end, and a
  /// press in them is theirs, not the row's.
  pub fn actions(id: &str, cell: impl Fn(&R) -> Element + Send + Sync + 'static) -> Self {
    let mut column = Self::new(id, "Actions")
      .fit()
      .align(ColumnAlign::End)
      .controls(true)
      .cell(cell);
    column.title_hidden = true;
    column
  }

  /// A row's cell is this text, in the cell's type: the common case.
  pub fn text(mut self, text: impl Fn(&R) -> String + Send + Sync + 'static) -> Self {
    self.content = Content::Text(Arc::new(text));
    self
  }

  /// A row's cell is what this builds: a badge, an avatar with a name, a progress.
  pub fn cell(mut self, cell: impl Fn(&R) -> Element + Send + Sync + 'static) -> Self {
    self.content = Content::Element(Arc::new(cell));
    self
  }

  /// The column is `px` wide.
  pub fn width(mut self, px: f32) -> Self {
    self.track.width = TrackWidth::Px(px.max(0.0));
    self
  }

  /// The column takes `share` of the room the others leave: two of 1 and 2 split it one to two. Where the
  /// table is as wide as its content, it is as wide as its widest cell.
  pub fn share(mut self, share: f32) -> Self {
    self.track.width = TrackWidth::Share(share.max(0.0));
    self
  }

  /// The column is as wide as the widest of its cells the table has shown, and its header.
  pub fn fit(mut self) -> Self {
    self.track.width = TrackWidth::Content;
    self
  }

  /// The least the column is: a share does not narrow past it, and the table scrolls sideways instead.
  pub fn min_width(mut self, px: f32) -> Self {
    self.track = self.track.min(px);
    self
  }

  /// The most the column is.
  pub fn max_width(mut self, px: f32) -> Self {
    self.track = self.track.max(px);
    self
  }

  /// Where the content of its cells and its header sits.
  pub fn align(mut self, align: ColumnAlign) -> Self {
    self.align = align;
    self
  }

  /// A column of numbers: at its end, in the mono type, smaller and muted, as the board draws a size.
  pub fn numeric(mut self) -> Self {
    self.align = ColumnAlign::End;
    self.kind(TableState::Numeric, true)
  }

  /// Whether its text is in the mono type: a path, an id.
  pub fn mono(self, mono: bool) -> Self {
    self.kind(TableState::Mono, mono)
  }

  /// Whether its text is in the muted color: a date, a note.
  pub fn muted(self, muted: bool) -> Self {
    self.kind(TableState::Muted, muted)
  }

  /// Puts the column's cells in a state the theme's recipe names, or takes them out of it.
  pub fn kind(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.kinds.retain(|held| *held != state);
    if on {
      self.kinds.push(state);
    }
    self
  }

  /// Whether a press of its header sorts the rows by it: see [`Table::sort`].
  pub fn sortable(mut self, sortable: bool) -> Self {
    self.sortable = sortable;
    self
  }

  /// How two rows compare in this column, for [`TableSort::sort`]: the column is sortable.
  pub fn sort_by(mut self, compare: impl Fn(&R, &R) -> Ordering + Send + Sync + 'static) -> Self {
    self.compare = Some(Arc::new(compare));
    self.sortable = true;
    self
  }

  /// What a row is sorted by in this column, for [`TableSort::sort`]: the column is sortable.
  pub fn sort_key<O: Ord>(self, key: impl Fn(&R) -> O + Send + Sync + 'static) -> Self {
    self.sort_by(move |a, b| key(a).cmp(&key(b)))
  }

  /// Where the column stays while the rows scroll sideways under the others: at the start or at the end. The
  /// columns pinned at the start are the first ones, one after another, the column of checkboxes before them;
  /// those at the end are the last ones. A column pinned where others between it and its end are not scrolls
  /// with them. Pinned columns after the first are best given a width of their own: where each stays is reckoned
  /// from the widths of those before it.
  pub fn pin(mut self, pin: ColumnPin) -> Self {
    self.pin = pin;
    self
  }

  /// Whether a person makes the column wider or narrower, by dragging the end of its header or with Ctrl and
  /// Left or Right on its header: as the table says unless told ([`Table::resizable`]). Only a mounted table keeps
  /// the width it is made.
  pub fn resizable(mut self, resizable: bool) -> Self {
    self.resizable = Some(resizable);
    self
  }

  /// Whether its cells hold what is pressed, buttons, a menu, a switch: a press in them is theirs and does
  /// not select or press the row.
  pub fn controls(mut self, controls: bool) -> Self {
    self.controls = controls;
    self
  }

  /// How a [`DataGrid`](super::DataGrid) edits the column's cells in place: a text, a number, a choice or a box
  /// ticked. A table shows them and edits nothing. A column edited with a checkbox that builds no cell of its
  /// own shows the box.
  pub fn editor(mut self, editor: CellEditor<R>) -> Self {
    self.editor = Some(editor);
    self
  }

  /// The id of the column, which a sort names.
  pub fn id(&self) -> &str {
    &self.id
  }

  /// What its header says.
  pub fn title(&self) -> &str {
    &self.title
  }
}

type Told<K> = Arc<dyn Fn(&K) + Send + Sync>;
type SortFn = Arc<dyn Fn(Option<TableSort>) + Send + Sync>;
type SortHeard = Arc<dyn Fn(Option<&TableSort>) + Send + Sync>;
type SelectionHeard<K> = Arc<dyn Fn(&[K]) + Send + Sync>;
/// What asks for a range of cells to be the grid's, and what is told it.
type RangeWrite<K> = Arc<dyn Fn(Option<CellRange<K>>) + Send + Sync>;
type RangeHeard<K> = Arc<dyn Fn(Option<&CellRange<K>>) + Send + Sync>;
/// What asks for the columns to be arranged another way, and what is told it.
type LayoutWrite = Arc<dyn Fn(ColumnLayout) + Send + Sync>;
type LayoutHeard = Arc<dyn Fn(&ColumnLayout) + Send + Sync>;
/// What asks for other groups to be open, and what is told a group opened or closed.
type OpenWrite = Arc<dyn Fn(Vec<GroupKey>) + Send + Sync>;
type OpenHeard = Arc<dyn Fn(&GroupKey, bool) + Send + Sync>;
/// What says the count of the footer, given how many rows there are and how many are selected where they are.
type CountFn = Arc<dyn Fn(usize, Option<usize>) -> String + Send + Sync>;
/// What is told the width a column is made: its id, and the width.
type ResizeHeard = Arc<dyn Fn(&str, f32) + Send + Sync>;
type ResizeLabelFn = Arc<dyn Fn(&str) -> String + Send + Sync>;
/// The least a column is made by a person, where it says no least of its own.
const LEAST_COLUMN: f32 = 48.0;
/// How far Ctrl and Left or Right make a column narrower or wider, and with Shift too.
const RESIZE_STEP: f32 = 10.0;
const RESIZE_STEP_SHIFT: f32 = 50.0;

/// The words a table writes itself, each of which the app says otherwise.
#[derive(Clone)]
struct Words {
  /// The title of the empty state of a table with no rows.
  empty: Arc<str>,
  /// The name of the header's checkbox.
  select_all: Arc<str>,
  /// The name of a row's checkbox.
  select_row: Arc<str>,
  /// The footer's count, where the app says it.
  count: Option<CountFn>,
  /// The name of a column's resizer, given the column's title.
  resize: Option<ResizeLabelFn>,
}

impl Default for Words {
  fn default() -> Self {
    Self {
      empty: Arc::from("No rows"),
      select_all: Arc::from("Select all"),
      select_row: Arc::from("Select row"),
      count: None,
      resize: None,
    }
  }
}
type KeyFn<R, K> = Arc<dyn Fn(usize, &R) -> K + Send + Sync>;

/// Rows of the same things in columns: crates and their status, files and their sizes, people and their
/// roles. The columns of the header and of every row are the same widths whatever is in a cell.
///
/// ```
/// use lurq::{core::Signal, ui::{Table, TableColumn, TableSort}};
///
/// #[derive(Clone)]
/// struct Crate { name: String, status: String, updated: String }
///
/// # fn crates(rows: Vec<Crate>, sort: Signal<Option<TableSort>>, selected: Signal<Vec<String>>) {
/// let columns = vec![
///   TableColumn::new("name", "Name").text(|row: &Crate| row.name.clone()).mono(true).sortable(true),
///   TableColumn::new("status", "Status").width(120.0).text(|row: &Crate| row.status.clone()),
///   TableColumn::new("updated", "Updated").width(120.0).muted(true).text(|row: &Crate| row.updated.clone()),
/// ];
/// // The app gives the rows in the order it wants: here, by the sort the header was pressed to.
/// let mut rows = rows;
/// if let Some(by) = sort.get() {
///   by.sort(&columns, &mut rows);
/// }
/// let table = Table::new(columns, rows)
///   .row_key(|row: &Crate| row.name.clone())
///   .label("Crates")
///   .sort(sort)
///   .selected(selected)
///   .height(320.0)
///   .on_row_activate(|name: &String| println!("open {name}"));
/// # }
/// ```
///
/// What the rows are, their order, what is selected and what they are sorted by are the app's. A press of a
/// sortable header asks for the rows sorted by it, ascending, then descending, then not at all
/// ([`Table::sort`], [`Table::on_sort_change`]), and the app gives them in that order ([`TableSort::sort`] sorts
/// them by the column's text or what it compares by).
///
/// With a selection ([`Table::selected`]) the table has a column of checkboxes. A press of a row selects it
/// alone, with Ctrl (or Cmd) adds it or takes it out, and with Shift selects the rows from the last selected
/// one to it; a press of its checkbox adds it or takes it out, with Shift the range. The header's checkbox
/// selects every row the table shows, or none where all are, and is mixed where some are.
///
/// The header stays while the rows scroll in the table's own height ([`Table::height`]); a table wider than
/// its box scrolls sideways, its header with it. Mounted ([`Table::mount`]), a table builds the rows in view
/// and a few around them, so that thousands of rows cost about what a screen of them does; converted into an
/// element it builds them all.
///
/// The rows are one stop of Tab, at the first selected row or the first. Up and Down move through them,
/// Home and End to the first and the last, and in a mounted table PageUp and PageDown by what the table
/// shows. Space selects the focused row (adds it or takes it out), Enter activates it, as a double press
/// does ([`Table::on_row_activate`]), and Ctrl+A selects them all. Tab from a row goes on into what it holds
/// that takes the focus, its buttons, and then out of the table. The sortable headers are a stop of their
/// own before the rows, Left and Right between them.
pub struct Table<R, K = usize> {
  columns: Vec<TableColumn<R>>,
  rows: Arc<Vec<R>>,
  key_of: KeyFn<R, K>,
  sort: Option<TableSort>,
  write_sort: Option<SortFn>,
  on_sort_change: Vec<SortHeard>,
  selected: Vec<K>,
  select: Option<Arc<dyn Fn(Vec<K>) + Send + Sync>>,
  on_selection_change: Vec<SelectionHeard<K>>,
  words: Words,
  resizable: bool,
  on_column_resize: Vec<ResizeHeard>,
  on_column_resize_end: Vec<ResizeHeard>,
  /// The widths a person made the columns, by their ids: what the table's host keeps.
  resized: Arc<HashMap<Arc<str>, f32>>,
  /// Whether its cells take the focus and are edited: a data grid.
  grid: bool,
  read_only: bool,
  on_edit: Vec<EditHeard<K>>,
  /// The range of cells a grid has selected, what asks for another, and who is told.
  cell_range: Option<CellRange<K>>,
  write_cell_range: Option<RangeWrite<K>>,
  on_cell_selection_change: Vec<RangeHeard<K>>,
  /// How a grid's columns are arranged, what asks for another arrangement, and who is told.
  column_layout: Option<ColumnLayout>,
  write_column_layout: Option<LayoutWrite>,
  on_column_layout_change: Vec<LayoutHeard>,
  /// Whether a grid's headers have a menu.
  column_menu: bool,
  /// Every column of a mounted grid in its layout, shown or not, with its title: what its menus list, and what
  /// a change of the arrangement starts from.
  arranged: Option<Arc<(ColumnLayout, Vec<(Arc<str>, Arc<str>)>)>>,
  /// The columns the rows are grouped by, the groups open, what asks for others and who is told; the columns
  /// themselves, which may be hidden.
  group_by: Vec<Arc<str>>,
  open_groups: Vec<GroupKey>,
  write_open: Option<OpenWrite>,
  on_open_change: Vec<OpenHeard>,
  grouping: Vec<TableColumn<R>>,
  checkboxes: bool,
  on_row_click: Vec<Told<K>>,
  on_row_activate: Vec<Told<K>>,
  density: Option<Density>,
  height: Option<Length>,
  max_height: Option<Length>,
  empty: Option<Element>,
  loading: bool,
  loading_rows: usize,
  pagination: Option<Element>,
  count: bool,
  total: Option<usize>,
  label: Option<Arc<str>>,
  disabled: bool,
  states: Vec<Name>,
  rules: Recipe,
  forced: Option<Interaction>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl<R, K: Clone> Clone for Table<R, K> {
  fn clone(&self) -> Self {
    Self {
      columns: self.columns.clone(),
      rows: self.rows.clone(),
      key_of: self.key_of.clone(),
      sort: self.sort.clone(),
      write_sort: self.write_sort.clone(),
      on_sort_change: self.on_sort_change.clone(),
      selected: self.selected.clone(),
      select: self.select.clone(),
      on_selection_change: self.on_selection_change.clone(),
      words: self.words.clone(),
      resizable: self.resizable,
      on_column_resize: self.on_column_resize.clone(),
      on_column_resize_end: self.on_column_resize_end.clone(),
      resized: self.resized.clone(),
      grid: self.grid,
      read_only: self.read_only,
      on_edit: self.on_edit.clone(),
      cell_range: self.cell_range.clone(),
      write_cell_range: self.write_cell_range.clone(),
      on_cell_selection_change: self.on_cell_selection_change.clone(),
      column_layout: self.column_layout.clone(),
      write_column_layout: self.write_column_layout.clone(),
      on_column_layout_change: self.on_column_layout_change.clone(),
      column_menu: self.column_menu,
      arranged: self.arranged.clone(),
      group_by: self.group_by.clone(),
      open_groups: self.open_groups.clone(),
      write_open: self.write_open.clone(),
      on_open_change: self.on_open_change.clone(),
      grouping: self.grouping.clone(),
      checkboxes: self.checkboxes,
      on_row_click: self.on_row_click.clone(),
      on_row_activate: self.on_row_activate.clone(),
      density: self.density.clone(),
      height: self.height,
      max_height: self.max_height,
      empty: self.empty.clone(),
      loading: self.loading,
      loading_rows: self.loading_rows,
      pagination: self.pagination.clone(),
      count: self.count,
      total: self.total,
      label: self.label.clone(),
      disabled: self.disabled,
      states: self.states.clone(),
      rules: self.rules.clone(),
      forced: self.forced,
      id: self.id.clone(),
      key: self.key.clone(),
    }
  }
}

impl<R: Send + Sync + 'static> Table<R, usize> {
  /// A table of `rows` in `columns`, each row told apart by where it is until the app gives it a key
  /// ([`Table::row_key`]): none selected, and sorted by nothing.
  pub fn new(columns: impl IntoIterator<Item = TableColumn<R>>, rows: impl IntoIterator<Item = R>) -> Self {
    Self::new_shared(columns, Arc::new(rows.into_iter().collect()))
  }

  /// A table of rows the app keeps shared: a list of thousands given again at every render is neither copied
  /// nor built again.
  pub fn new_shared(columns: impl IntoIterator<Item = TableColumn<R>>, rows: Arc<Vec<R>>) -> Self {
    Self {
      columns: columns.into_iter().collect(),
      rows,
      key_of: Arc::new(|index, _| index),
      sort: None,
      write_sort: None,
      on_sort_change: Vec::new(),
      selected: Vec::new(),
      select: None,
      on_selection_change: Vec::new(),
      words: Words::default(),
      resizable: false,
      on_column_resize: Vec::new(),
      on_column_resize_end: Vec::new(),
      resized: Arc::default(),
      grid: false,
      read_only: false,
      on_edit: Vec::new(),
      cell_range: None,
      write_cell_range: None,
      on_cell_selection_change: Vec::new(),
      column_layout: None,
      write_column_layout: None,
      on_column_layout_change: Vec::new(),
      column_menu: false,
      arranged: None,
      group_by: Vec::new(),
      open_groups: Vec::new(),
      write_open: None,
      on_open_change: Vec::new(),
      grouping: Vec::new(),
      checkboxes: true,
      on_row_click: Vec::new(),
      on_row_activate: Vec::new(),
      density: None,
      height: None,
      max_height: None,
      empty: None,
      loading: false,
      loading_rows: 4,
      pagination: None,
      count: false,
      total: None,
      label: None,
      disabled: false,
      states: Vec::new(),
      rules: Recipe::new(),
      forced: None,
      id: None,
      key: None,
    }
  }

  /// What tells each row apart: its id, which the selection and the handlers name it by, wherever it is
  /// sorted to. Given before the selection and the handlers, which name rows by it.
  pub fn row_key<K: Clone + Eq + Hash + Send + Sync + 'static>(
    self,
    key: impl Fn(&R) -> K + Send + Sync + 'static,
  ) -> Table<R, K> {
    Table {
      columns: self.columns,
      rows: self.rows,
      key_of: Arc::new(move |_, row| key(row)),
      sort: self.sort,
      write_sort: self.write_sort,
      on_sort_change: self.on_sort_change,
      selected: Vec::new(),
      select: None,
      on_selection_change: Vec::new(),
      words: self.words,
      resizable: self.resizable,
      on_column_resize: self.on_column_resize,
      on_column_resize_end: self.on_column_resize_end,
      resized: self.resized,
      grid: self.grid,
      read_only: self.read_only,
      on_edit: Vec::new(),
      cell_range: None,
      write_cell_range: None,
      on_cell_selection_change: Vec::new(),
      column_layout: self.column_layout,
      write_column_layout: self.write_column_layout,
      on_column_layout_change: self.on_column_layout_change,
      column_menu: self.column_menu,
      arranged: None,
      group_by: self.group_by,
      open_groups: self.open_groups,
      write_open: self.write_open,
      on_open_change: self.on_open_change,
      grouping: self.grouping,
      checkboxes: self.checkboxes,
      on_row_click: Vec::new(),
      on_row_activate: Vec::new(),
      density: self.density,
      height: self.height,
      max_height: self.max_height,
      empty: self.empty,
      loading: self.loading,
      loading_rows: self.loading_rows,
      pagination: self.pagination,
      count: self.count,
      total: self.total,
      label: self.label,
      disabled: self.disabled,
      states: self.states,
      rules: self.rules,
      forced: self.forced,
      id: self.id,
      key: self.key,
    }
  }
}

impl<R: Send + Sync + 'static, K: Clone + Eq + Hash + Send + Sync + 'static> Table<R, K> {
  /// What the rows are sorted by: the signal's, which a press of a sortable header sets to the next of
  /// ascending, descending and none. The rows are given in that order by the app.
  pub fn sort(mut self, sort: Signal<Option<TableSort>>) -> Self
  where
    Option<TableSort>: SignalValue,
  {
    self.sort = sort.get();
    self.write_sort = Some(Arc::new(move |next| sort.set(next)));
    self
  }

  /// What the rows are sorted by, as the app holds it: `on_change` is told the next at each press of a
  /// sortable header, and the table shows what the app gives it at the next render.
  pub fn sort_controlled(
    mut self,
    sort: Option<TableSort>,
    on_change: impl Fn(Option<TableSort>) + Send + Sync + 'static,
  ) -> Self {
    self.sort = sort;
    self.write_sort = Some(Arc::new(on_change));
    self
  }

  /// Called with what the rows are to be sorted by at each press of a sortable header, bound to a signal or
  /// not; a second handler is called after the first. Given alone, it makes the headers sortable, and the
  /// table shows the sort the app gives it with [`sort_controlled`](Self::sort_controlled).
  pub fn on_sort_change(mut self, handler: impl Fn(Option<&TableSort>) + Send + Sync + 'static) -> Self {
    self.on_sort_change.push(Arc::new(handler));
    self
  }

  /// The rows that are selected: those whose keys the signal has. The table has a column of checkboxes.
  /// The last key is where a range selected with Shift starts.
  pub fn selected(mut self, selected: Signal<Vec<K>>) -> Self
  where
    Vec<K>: SignalValue,
  {
    self.selected = selected.get();
    self.select = Some(Arc::new(move |next| selected.set(next)));
    self
  }

  /// The rows that are selected, as the app holds them: `on_change` is told what is to be selected after each
  /// press or key that changes it, and the table shows what the app gives it at the next render.
  pub fn selected_controlled(
    mut self,
    selected: impl IntoIterator<Item = K>,
    on_change: impl Fn(Vec<K>) + Send + Sync + 'static,
  ) -> Self {
    self.selected = selected.into_iter().collect();
    self.select = Some(Arc::new(on_change));
    self
  }

  /// Called with the whole new selection after each press or key that changes it, bound to a signal or not:
  /// not for a press that leaves it as it is, nor when the app sets the signal. A second handler is called
  /// after the first. It hears a table that is selected ([`selected`](Self::selected),
  /// [`selected_controlled`](Self::selected_controlled)); alone it selects nothing.
  pub fn on_selection_change(mut self, handler: impl Fn(&[K]) + Send + Sync + 'static) -> Self {
    self.on_selection_change.push(Arc::new(handler));
    self
  }

  /// Whether a table whose rows are selected has the column of checkboxes. It has unless told otherwise;
  /// without it a row is selected by a press of it, as the board's table shows one.
  pub fn checkboxes(mut self, checkboxes: bool) -> Self {
    self.checkboxes = checkboxes;
    self
  }

  /// Called with the row a press of it is on, whatever it selects.
  pub fn on_row_click(mut self, handler: impl Fn(&K) + Send + Sync + 'static) -> Self {
    self.on_row_click.push(Arc::new(handler));
    self
  }

  /// Called with the row a double press or Enter activates: open what it is.
  pub fn on_row_activate(mut self, handler: impl Fn(&K) + Send + Sync + 'static) -> Self {
    self.on_row_activate.push(Arc::new(handler));
    self
  }

  /// How much room a row takes, whatever the theme is at: tight for much to show, loose for a finger.
  pub fn density(mut self, density: impl Into<Density>) -> Self {
    self.density = Some(density.into());
    self
  }

  /// The table's height: the header stays and the rows scroll in what is left under it.
  pub fn height(mut self, height: impl Into<Length>) -> Self {
    self.height = Some(height.into());
    self
  }

  /// The most the table is high: as high as its rows up to it, and they scroll past it.
  pub fn max_height(mut self, height: impl Into<Length>) -> Self {
    self.max_height = Some(height.into());
    self
  }

  /// The title of the empty state that stands under the header where there are no rows: "No rows" unless
  /// told. [`empty`](Self::empty) puts an element of the app's in its place.
  pub fn empty_text(mut self, text: &str) -> Self {
    self.words.empty = Arc::from(text);
    self
  }

  /// The name of the header's checkbox, which selects every row: "Select all" unless told.
  pub fn select_all_label(mut self, label: &str) -> Self {
    self.words.select_all = Arc::from(label);
    self
  }

  /// The name of a row's checkbox: "Select row" unless told.
  pub fn select_row_label(mut self, label: &str) -> Self {
    self.words.select_row = Arc::from(label);
    self
  }

  /// What the footer's count says, given how many rows there are and, where they are selected, how many
  /// are: "24 rows" and "3 of 24 rows selected" unless told.
  pub fn count_text(mut self, text: impl Fn(usize, Option<usize>) -> String + Send + Sync + 'static) -> Self {
    self.words.count = Some(Arc::new(text));
    self
  }

  /// Whether a person makes the columns wider or narrower, each that says nothing of its own
  /// ([`TableColumn::resizable`]): by dragging the end of a column's header, or with Ctrl and Left or Right on it
  /// (with Shift, by more). Off unless told. A mounted table keeps the widths it is made; one converted into an
  /// element has no resizers.
  pub fn resizable(mut self, resizable: bool) -> Self {
    self.resizable = resizable;
    self
  }

  /// Called with a column's id and its width as a person makes it wider or narrower, at each move; a second
  /// handler is called after the first.
  pub fn on_column_resize(mut self, handler: impl Fn(&str, f32) + Send + Sync + 'static) -> Self {
    self.on_column_resize.push(Arc::new(handler));
    self
  }

  /// Called with a column's id and its width when a person has made it wider or narrower: the end of a drag
  /// that changed it, or a key.
  pub fn on_column_resize_end(mut self, handler: impl Fn(&str, f32) + Send + Sync + 'static) -> Self {
    self.on_column_resize_end.push(Arc::new(handler));
    self
  }

  /// The name of a column's resizer to what reads the tree, given the column's title: "Resize Name" unless told.
  pub fn resize_label(mut self, label: impl Fn(&str) -> String + Send + Sync + 'static) -> Self {
    self.words.resize = Some(Arc::new(label));
    self
  }

  /// What stands under the header where there are no rows: by default an empty state that says so.
  pub fn empty(mut self, empty: impl Into<Element>) -> Self {
    self.empty = Some(empty.into());
    self
  }

  /// Whether the rows are on their way: the header stays, and rows of skeletons stand where they will be.
  pub fn loading(mut self, loading: bool) -> Self {
    self.loading = loading;
    self
  }

  /// How many rows of skeletons stand while the table is loading: 4 unless it is said.
  pub fn loading_rows(mut self, rows: usize) -> Self {
    self.loading_rows = rows.max(1);
    self
  }

  /// A footer under the rows with the count of them and this pagination at its end.
  pub fn pagination(mut self, pagination: impl Into<Element>) -> Self {
    self.pagination = Some(pagination.into());
    self.count = true;
    self
  }

  /// Whether the table has a footer that counts its rows: "24 rows", and how many are selected.
  pub fn count(mut self, count: bool) -> Self {
    self.count = count;
    self
  }

  /// How many rows there are in all, where the table shows some of them: a page.
  pub fn total(mut self, total: usize) -> Self {
    self.total = Some(total);
    self
  }

  /// What the table is called, to what reads the tree of the window. It draws nothing.
  pub fn label(mut self, label: &str) -> Self {
    self.label = Some(Arc::from(label));
    self
  }

  /// Whether no row and no header takes input.
  pub fn disabled(mut self, disabled: bool) -> Self {
    self.disabled = disabled;
    self
  }

  /// Puts the table in a state the theme's recipe names, or takes it out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// Styles one part of this table over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(super::PartStyle) -> super::PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for this table.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// Draws every row as it is under the pointer, held down or focused from the keyboard, with nothing
  /// pointing at it: for a gallery or a test.
  pub fn force(mut self, interaction: Option<Interaction>) -> Self {
    self.forced = interaction;
    self
  }

  /// The id of the root. A row's element has this id with `-row-` and its place after it, from 0; a header's,
  /// `-header-` and its column's id; the header's checkbox `-select-all`.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// Builds the table in `theme` with every row. Converting a table into an element does this in the theme of
  /// what is being rendered.
  pub fn build(mut self, theme: &Theme) -> Element {
    self.find_grouping();
    let tracks = ColumnTracks::new(self.track_specs(theme));
    let view = self.view(theme, tracks, None);
    match theme.renderer::<Table<R, K>>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }

  /// The table mounted where it is declared: it builds the rows in view and a few around them, keeps the
  /// widths its columns have measured from render to render, and takes PageUp and PageDown. For a table of
  /// many rows, or one whose columns are as wide as their content.
  pub fn mount(self, ctx: &mut Ctx) -> Element {
    let key = self.key.clone();
    let props = TableProps {
      table: Arc::new(Mutex::new(self)),
    };
    match key {
      Some(key) => ctx.mount_keyed::<TableHost<R, K>>(&key, props),
      None => ctx.mount::<TableHost<R, K>>(props),
    }
  }

  fn selectable(&self) -> bool {
    self.select.is_some()
  }

  /// The table as a data grid: its cells take the focus and are edited, where it is mounted.
  pub(super) fn as_grid(mut self) -> Self {
    self.grid = true;
    self.resizable = true;
    self.column_menu = true;
    self
  }

  pub(super) fn column_layout(mut self, layout: Option<ColumnLayout>, write: LayoutWrite) -> Self {
    self.column_layout = layout;
    self.write_column_layout = Some(write);
    self
  }

  pub(super) fn add_on_column_layout_change(mut self, handler: impl Fn(&ColumnLayout) + Send + Sync + 'static) -> Self {
    self.on_column_layout_change.push(Arc::new(handler));
    self
  }

  pub(super) fn set_group_by(mut self, columns: Vec<Arc<str>>) -> Self {
    self.group_by = columns;
    self
  }

  pub(super) fn set_open_groups(mut self, open: Vec<GroupKey>, write: OpenWrite) -> Self {
    self.open_groups = open;
    self.write_open = Some(write);
    self
  }

  pub(super) fn add_on_open_change(mut self, handler: impl Fn(&GroupKey, bool) + Send + Sync + 'static) -> Self {
    self.on_open_change.push(Arc::new(handler));
    self
  }

  /// The columns the rows are grouped by, found among all of them: before a grid's arrangement hides any.
  fn find_grouping(&mut self) {
    self.grouping = self
      .group_by
      .iter()
      .filter_map(|id| self.columns.iter().find(|column| &column.id == id).cloned())
      .collect();
  }

  /// The lines the table shows: its rows, or its rows grouped by the values of its grouping's columns, with the
  /// rows of the closed groups out.
  fn lines(&self) -> Lines {
    if self.grouping.is_empty() {
      return Lines::flat(self.rows.len());
    }
    let ids: Vec<Arc<str>> = self.grouping.iter().map(|column| column.id.clone()).collect();
    let open: HashSet<&GroupKey> = self.open_groups.iter().collect();
    Lines::grouped(
      self.rows.len(),
      &ids,
      &|row, level| column_text(&self.grouping[level], &self.rows[row]),
      &|key| open.contains(key),
    )
  }

  /// Opens or closes the group `key`: asks for the groups open with it in or out, and tells every handler.
  fn group_toggle(&self) -> Arc<dyn Fn(&GroupKey, bool) + Send + Sync> {
    let (now, write, heard) = (
      self.open_groups.clone(),
      self.write_open.clone(),
      self.on_open_change.clone(),
    );
    Arc::new(move |key: &GroupKey, open: bool| {
      if now.contains(key) == open {
        return;
      }
      let mut next: Vec<GroupKey> = now.iter().filter(|held| *held != key).cloned().collect();
      if open {
        next.push(key.clone());
      }
      if let Some(write) = &write {
        write(next);
      }
      for handler in &heard {
        handler(key, open);
      }
    })
  }

  pub(super) fn set_column_menu(mut self, menu: bool) -> Self {
    self.column_menu = menu;
    self
  }

  /// Asks for the columns of a grid arranged as `next`, and tells every handler: nothing where they are.
  fn layout_changer(&self) -> Option<LayoutWrite> {
    let (now, _) = &**self.arranged.as_ref()?;
    let now = now.clone();
    let (write, heard) = (self.write_column_layout.clone(), self.on_column_layout_change.clone());
    Some(Arc::new(move |next: ColumnLayout| {
      if next == now {
        return;
      }
      if let Some(write) = &write {
        write(next.clone());
      }
      for handler in &heard {
        handler(&next);
      }
    }))
  }

  pub(super) fn set_read_only(mut self, read_only: bool) -> Self {
    self.read_only = read_only;
    self
  }

  pub(super) fn add_on_edit(mut self, handler: impl Fn(&CellEdit<K>) -> bool + Send + Sync + 'static) -> Self {
    self.on_edit.push(Arc::new(handler));
    self
  }

  pub(super) fn cell_selection(mut self, range: Option<CellRange<K>>, write: RangeWrite<K>) -> Self {
    self.cell_range = range;
    self.write_cell_range = Some(write);
    self
  }

  pub(super) fn add_on_cell_selection_change(
    mut self,
    handler: impl Fn(Option<&CellRange<K>>) + Send + Sync + 'static,
  ) -> Self {
    self.on_cell_selection_change.push(Arc::new(handler));
    self
  }

  fn with_checkboxes(&self) -> bool {
    self.selectable() && self.checkboxes
  }

  /// The case of the table as a whole.
  fn case(&self) -> Case {
    let mut case = Case::new()
      .breakpoint(super::current_breakpoint())
      .state(TableState::Selectable, self.with_checkboxes())
      .state(TableState::Footer, self.count || self.pagination.is_some())
      .state(TableState::Disabled, self.disabled);
    case.states.extend(self.states.iter().copied());
    if let Some(density) = &self.density {
      case = case.density(density.clone());
    }
    case
  }

  fn resolve(&self, theme: &Theme, case: &Case) -> ResolvedRecipe {
    theme.resolve::<Table<R, K>>(case, Some(&self.rules))
  }

  /// The tracks of the columns: the checkbox's first, then a track for each column. The room between two
  /// columns is the cell's (`Geometry`), so a track holds it.
  fn track_specs(&self, theme: &Theme) -> Vec<TrackSpec> {
    let geometry = Geometry::of(&self.resolve(theme, &self.case()));
    let count = self.columns.len();
    let mut specs = Vec::with_capacity(count + 1);
    if self.with_checkboxes() {
      let lead = if self.pins().0 > 0 { geometry.padding_start } else { 0.0 };
      specs.push(TrackSpec::px(geometry.select + geometry.gap + lead));
    }
    for (index, column) in self.columns.iter().enumerate() {
      let mut extra = if index + 1 < count { geometry.gap } else { 0.0 };
      let (start, end) = self.pins();
      if index == 0 && start > 0 && !self.with_checkboxes() {
        extra += geometry.padding_start;
      }
      // A row whose last columns stay holds its padding at the end in the last cell, which covers it.
      if index + 1 == count && end > 0 {
        extra += geometry.padding_end;
      }
      let mut spec = column.track;
      if let Some(width) = self.resized.get(&column.id) {
        spec.width = TrackWidth::Px(*width);
      }
      if let TrackWidth::Px(px) = spec.width {
        spec.width = TrackWidth::Px(px + extra);
      }
      if spec.min > 0.0 {
        spec.min += extra;
      }
      if spec.max.is_finite() {
        spec.max += extra;
      }
      specs.push(spec);
    }
    specs
  }

  /// How many columns stay at the start and how many at the end: the first ones pinned at the start, one
  /// after another, and the last ones pinned at the end.
  fn pins(&self) -> (usize, usize) {
    let start = self
      .columns
      .iter()
      .take_while(|column| column.pin == ColumnPin::Start)
      .count();
    let end = self.columns[start..]
      .iter()
      .rev()
      .take_while(|column| column.pin == ColumnPin::End)
      .count();
    (start, end)
  }

  /// The table as a renderer sees it, with the rows of `window` where it builds some of them.
  fn view(&self, theme: &Theme, tracks: ColumnTracks, window: Option<Window>) -> TableView {
    let case = self.case();
    let recipe = self.resolve(theme, &case);
    let geometry = Geometry::of(&recipe);
    let selectable = self.selectable();
    let checkboxes = self.with_checkboxes();
    let (start_pins, end_pins) = self.pins();
    let sticky = start_pins > 0;
    let footer = self.count || self.pagination.is_some();
    let count = self.rows.len();

    // Each case of a row and of a cell is resolved once, however many rows there are.
    let mut row_recipes: HashMap<(bool, bool), ResolvedRecipe> = HashMap::new();
    let mut row_recipe = |selected: bool, last: bool| {
      row_recipes
        .entry((selected, last))
        .or_insert_with(|| {
          let case = case
            .clone()
            .state(TableState::Selected, selected)
            .state(TableState::Last, last)
            .state(TableState::Pinned, sticky);
          self.resolve(theme, &case)
        })
        .clone()
    };
    let mut cell_recipes: HashMap<(usize, bool, Option<RangeMark>), ResolvedRecipe> = HashMap::new();
    let mut cell_recipe = |column: usize, selected: bool, mark: Option<RangeMark>| {
      cell_recipes
        .entry((column, selected, mark))
        .or_insert_with(|| {
          let pinned = column < start_pins || column + end_pins >= self.columns.len();
          let marked = |side: fn(&RangeMark) -> bool| mark.as_ref().is_some_and(side);
          let mut case = case
            .clone()
            .state(TableState::Selected, selected)
            .state(TableState::Pinned, pinned)
            .state(TableState::InRange, marked(|mark| mark.fill))
            .state(TableState::RangeTop, marked(|mark| mark.top))
            .state(TableState::RangeBottom, marked(|mark| mark.bottom))
            .state(TableState::RangeStart, marked(|mark| mark.start))
            .state(TableState::RangeEnd, marked(|mark| mark.end));
          case.states.extend(self.columns[column].kinds.iter().copied());
          self.resolve(theme, &case)
        })
        .clone()
    };

    let keys: Arc<Vec<K>> = Arc::new(
      self
        .rows
        .iter()
        .enumerate()
        .map(|(index, row)| (self.key_of)(index, row))
        .collect(),
    );
    // The lines shown: the rows, or the groups and the rows of the open ones. A row's rank is its place among the
    // rows shown, which the cells of a grid are reckoned by.
    let lines: Arc<Lines> = match window.as_ref() {
      Some(window) => window.lines.clone(),
      None => Arc::new(self.lines()),
    };
    let grouped = !lines.groups.is_empty() || !self.grouping.is_empty();
    let ranked: Arc<Vec<K>> = if grouped {
      Arc::new(lines.order.iter().map(|index| keys[*index].clone()).collect())
    } else {
      keys.clone()
    };
    let model = Arc::new(Model {
      keys: keys.clone(),
      selected: self.selected.clone(),
      select: self.select.clone(),
      on_selection_change: self.on_selection_change.clone(),
      on_row_click: self.on_row_click.clone(),
      on_row_activate: self.on_row_activate.clone(),
    });

    // Where the cells at the start stay: the checkbox's at the edge, the first column's after it.
    let lead_inset = if checkboxes {
      geometry.select + geometry.gap + geometry.padding_start
    } else {
      0.0
    };
    // Where each pinned column stays: after the checkboxes and the pinned columns before it at the start, before
    // the pinned columns after it at the end. Each is as wide as its track is known to be.
    let track_widths = tracks.widths(None, 0.0);
    let lead_tracks = usize::from(checkboxes);
    let width_of = |column: usize| track_widths.get(column + lead_tracks).copied().unwrap_or(0.0);
    let columns_count = self.columns.len();
    let sticky_of = |column: usize| -> Option<Sticky> {
      if column < start_pins {
        Some(Sticky::left(lead_inset + (0..column).map(width_of).sum::<f32>()))
      } else if column + end_pins >= columns_count {
        Some(Sticky::right((column + 1..columns_count).map(width_of).sum::<f32>()))
      } else {
        None
      }
    };
    let trail_of = |column: usize| {
      if column + 1 == columns_count && end_pins > 0 {
        geometry.padding_end
      } else {
        0.0
      }
    };
    let table_fill = recipe.part(TablePart::Root).rest().background;
    let header_fill = recipe.part(TablePart::Header).rest().background;
    let cell_ends = |column: usize| column + 1 == self.columns.len();
    let columns_built = window.as_ref().and_then(|window| window.columns.clone());
    let built = |column: usize| {
      columns_built
        .as_ref()
        .is_none_or(|built| built.get(column).copied().unwrap_or(true))
    };

    // A mounted grid's cells take the focus and are edited.
    let grid_host = window
      .as_ref()
      .filter(|_| self.grid)
      .and_then(|window| window.host.as_ref())
      .map(|host| host.state.clone());
    let ids: Vec<Arc<str>> = self.columns.iter().map(|column| column.id.clone()).collect();
    let span = grid_host
      .as_ref()
      .and(self.cell_range.as_ref())
      .and_then(|range| span_of(range, &ranked, &ids));
    let grid_model: Option<Arc<dyn GridCells>> = grid_host.as_ref().map(|host| {
      Arc::new(GridModel {
        host: host.clone(),
        span,
        range: self.cell_range.clone(),
        write_range: self.write_cell_range.clone(),
        on_range: self.on_cell_selection_change.clone(),
        texts: Arc::new(
          self
            .columns
            .iter()
            .map(|column| match &column.content {
              Content::Text(text) => Some(text.clone()),
              _ => None,
            })
            .collect(),
        ),
        rows: self.rows.clone(),
        keys: ranked.clone(),
        lines: lines.clone(),
        editors: Arc::new(
          self
            .columns
            .iter()
            .map(|column| (column.id.clone(), column.editor.clone()))
            .collect(),
        ),
        on_edit: self.on_edit.clone(),
        model: model.clone(),
        read_only: self.read_only,
      }) as Arc<dyn GridCells>
    });
    let editing = grid_host.as_ref().and_then(|host| *host.editing.lock());
    let focused_column = grid_host
      .as_ref()
      .and_then(|host| *host.focused_column.lock())
      .unwrap_or(0);
    let held_now: HashSet<&K> = self.selected.iter().collect();
    // The lines built, by their positions.
    let shown: Vec<usize> = match &window {
      _ if self.loading => Vec::new(),
      Some(window) => window.indices.clone(),
      None => (0..lines.lines.len()).collect(),
    };
    let entry = window
      .as_ref()
      .and_then(|window| window.focused.filter(|position| shown.contains(position)))
      .or_else(|| {
        shown.iter().copied().find(|position| {
          lines
            .rank_at(*position)
            .is_some_and(|rank| held_now.contains(&keys[lines.order[rank]]))
        })
      })
      .or_else(|| shown.first().copied());
    let line_count = lines.lines.len();
    let id_of = |what: &str| self.id.as_ref().map(|id| Arc::<str>::from(format!("{id}-{what}")));

    // The groups' rows built, each opened and closed through the app's groups.
    let toggle = self.group_toggle();
    if let Some(host) = window.as_ref().and_then(|window| window.host.as_ref()) {
      let (toggle, lines) = (toggle.clone(), lines.clone());
      *host.state.group_toggle.lock() = Some(Arc::new(move |group: usize, open: bool| {
        if let Some(line) = lines.groups.get(group) {
          toggle(&line.key, open);
        }
      }));
    }
    let group_recipe = row_recipe(false, false);
    let indent = geometry.padding_start;
    let groups: Vec<TableGroup> = shown
      .iter()
      .filter_map(|&position| {
        let Shown::Group(group) = lines.lines[position] else {
          return None;
        };
        let line = &lines.groups[group];
        let title = self
          .grouping
          .iter()
          .find(|column| column.id == line.column)
          .map_or_else(|| line.column.clone(), |column| column.title.clone());
        let (toggle, key) = (toggle.clone(), line.key.clone());
        Some(TableGroup {
          position,
          key: line.key.clone(),
          title,
          count: line.count,
          open: line.open,
          entry: entry == Some(position),
          recipe: group_recipe.clone(),
          toggle: Arc::new(move |open| toggle(&key, open)),
          element: window
            .as_ref()
            .and_then(|window| window.refs.get(&position).cloned())
            .unwrap_or_default(),
          grid: grid_model.is_some(),
          indent: indent + (line.key.level() - 1) as f32 * 20.0,
          id: id_of(&format!("group-{position}")),
        })
      })
      .collect();
    let rows = shown
      .iter()
      .filter_map(|&position| lines.rank_at(position).map(|rank| (position, rank)))
      .map(|(position, rank)| {
        let index = lines.order[rank];
        let row = &self.rows[index];
        let selected = selectable && held_now.contains(&keys[index]);
        let last = position + 1 == line_count && !footer;
        let recipe = row_recipe(selected, last);
        // A cell that stays covers what scrolls under it, in the row's fill, or the table's.
        let stuck_fill: BackgroundColor = recipe
          .part(TablePart::Row)
          .rest()
          .background
          .or_else(|| table_fill.clone())
          .unwrap_or_else(|| PaletteColor::Background.into());
        let cells = self
          .columns
          .iter()
          .enumerate()
          .filter(|(column_index, _)| built(*column_index))
          .map(|(column_index, column)| {
            let mark = span.and_then(|span| span.mark(rank, column_index));
            let part = cell_recipe(column_index, selected, mark)
              .part(TablePart::Cell)
              .forced(self.forced);
            let content = match (&column.content, column.editor.as_ref().map(|editor| &editor.kind)) {
              (Content::Text(text), _) => part.text(&text(row)).nowrap().into(),
              (Content::Element(cell), _) => cell(row),
              // A column edited with a box that builds no cell of its own shows the box.
              (Content::Nothing, Some(EditorKind::Checkbox(checked))) => Row::new()
                .inert(true)
                .child(
                  Checkbox::new(&column.title)
                    .label_hidden(true)
                    .checked(checked(row))
                    .build(theme),
                )
                .into(),
              (Content::Nothing, _) => Row::new().into(),
            };
            let grid = grid_model.as_ref().map(|grid| {
              let editable = !self.read_only && !self.disabled && column.editor.is_some();
              let being_edited = editing.filter(|editing| (editing.row, editing.column) == (rank, column_index));
              let editor = being_edited.and_then(|editing| {
                let host = grid_host.as_ref()?;
                let editor = column.editor.as_ref()?;
                let ref_ = window.as_ref()?.editor_ref.clone();
                let part = recipe.part(TablePart::Editor);
                let built: Element = match &editor.kind {
                  EditorKind::Text(_) => Input::new(host.draft_text.clone())
                    .id(id_of("editor").unwrap_or_else(|| Arc::from("editor")))
                    .size(InputSize::Sm)
                    .invalid(editing.refused)
                    .label(&column.title)
                    .build(theme),
                  EditorKind::Number(_) => NumberInput::optional(host.draft_number.clone())
                    .id(id_of("editor").unwrap_or_else(|| Arc::from("editor")))
                    .size(InputSize::Sm)
                    .invalid(editing.refused)
                    .label(&column.title)
                    .build(theme),
                  EditorKind::Choice(choices, _) => {
                    let committing = grid.clone();
                    Select::new(
                      host.draft_choice.clone(),
                      choices
                        .iter()
                        .enumerate()
                        .map(|(place, choice)| crate::components::SelectOption::new(place, choice.clone())),
                    )
                    .id(id_of("editor").unwrap_or_else(|| Arc::from("editor")))
                    .size(InputSize::Sm)
                    .invalid(editing.refused)
                    .label(&column.title)
                    .on_change(move |_: &usize| committing.commit(GridStep::Stay))
                    .build(theme)
                  }
                  EditorKind::Checkbox(_) => return None,
                };
                let choice = matches!(editor.kind, EditorKind::Choice(..));
                Some(editor_in(&part, built, choice, grid.clone(), ref_).into())
              });
              GridCell {
                row: rank,
                position,
                editable,
                selected: span.is_some_and(|span| span.anchor != span.extent && span.contains(rank, column_index)),
                editor,
                entry: entry == Some(position)
                  && column_index == focused_column.min(self.columns.len().saturating_sub(1)),
                column_index: column_index + 1 + usize::from(checkboxes),
                checkbox: column.editor.as_ref().is_some_and(CellEditor::is_checkbox),
                element: window
                  .as_ref()
                  .and_then(|window| window.cell_refs.get(&(rank, column_index)).cloned())
                  .unwrap_or_default(),
                grid: grid.clone(),
                id: id_of(&format!("cell-{index}-{column_index}")),
              }
            });
            TableCell {
              column: column_index,
              content,
              grid,
              part,
              align: column.align,
              pinned: sticky_of(column_index),
              // A cell of the range has the range's fill, over what scrolls under it too.
              fill: sticky_of(column_index)
                .filter(|_| !mark.is_some_and(|mark| mark.fill))
                .map(|_| stuck_fill.clone()),
              controls: column.controls,
              last: cell_ends(column_index),
              gap: geometry.gap,
              lead: if column_index == 0 && sticky && !checkboxes {
                geometry.padding_start
              } else {
                0.0
              },
              trail: trail_of(column_index),
            }
          })
          .collect();
        let name = self.columns.first().and_then(|column| match &column.content {
          Content::Text(text) => Some(Arc::<str>::from(text(row))),
          _ => None,
        });
        TableRow {
          index,
          position,
          selected,
          disabled: self.disabled,
          name,
          cells,
          checkbox: checkboxes,
          pinned_start: sticky,
          pinned_end: end_pins > 0,
          lead_fill: sticky.then(|| stuck_fill.clone()),
          entry: entry == Some(position),
          recipe,
          forced: self.forced,
          tracks: tracks.clone(),
          columns_built: columns_built.clone(),
          grid: grid_model.is_some(),
          geometry,
          actions: (!self.disabled).then(|| model.actions(index)),
          select_label: self.words.select_row.clone(),
          host: window.as_ref().and_then(|window| window.host.clone()),
          element: window
            .as_ref()
            .and_then(|window| window.refs.get(&position).cloned())
            .unwrap_or_default(),
          id: id_of(&format!("row-{index}")),
        }
      })
      .collect();

    // A mounted grid's headers have a menu, are dragged to another place, and are found by where they are.
    let menu_host = grid_host.as_ref().filter(|_| self.column_menu && !self.disabled);
    let change_layout = menu_host.and_then(|_| self.layout_changer());
    let open_menu = menu_host.and_then(|host| host.menu_open.lock().clone());
    let shown_ids: Arc<Vec<Arc<str>>> = Arc::new(self.columns.iter().map(|column| column.id.clone()).collect());
    let header_drop = menu_host.and_then(|host| *host.header_drop.lock());
    let drop_of = |column: usize| match header_drop {
      Some((from, to)) if to != Some(from) && to != Some(from + 1) => match to {
        Some(to) if to == column => Some(DropSide::Before),
        None if column + 1 == columns_count && from + 1 != columns_count => Some(DropSide::After),
        _ => None,
      },
      _ => None,
    };
    let mut header_refs: HashMap<usize, ElementRef> = HashMap::new();
    let headers = self
      .columns
      .iter()
      .enumerate()
      .filter(|(column_index, _)| built(*column_index))
      .map(|(column_index, column)| {
        let sorted = self
          .sort
          .as_ref()
          .filter(|sort| sort.column == column.id)
          .map(|sort| sort.direction);
        let sortable = column.sortable && !self.disabled;
        let open = open_menu.as_deref() == Some(&*column.id);
        let drop = drop_of(column_index);
        let header_case = case
          .clone()
          .state(TableState::Sortable, sortable)
          .state(TableState::Sorted, sorted.is_some())
          .state(TableState::Pinned, sticky_of(column_index).is_some())
          .state(TableState::Open, open)
          .state(TableState::DropBefore, drop == Some(DropSide::Before))
          .state(TableState::DropAfter, drop == Some(DropSide::After));
        let sorts = self.write_sort.is_some() || !self.on_sort_change.is_empty();
        let menu = menu_host.zip(change_layout.clone()).map(|(host, change)| {
          let element = ElementRef::new();
          header_refs.insert(column_index, element);
          let (layout, titles) = &**self
            .arranged
            .as_ref()
            .expect("a grid with a layout changer is arranged");
          let id = column.id.clone();
          let choose: Arc<dyn Fn(MenuChoice) + Send + Sync> = {
            let (id, layout, change) = (id.clone(), layout.clone(), change.clone());
            let (write_sort, sort_heard) = (self.write_sort.clone(), self.on_sort_change.clone());
            Arc::new(move |choice: MenuChoice| match choice {
              MenuChoice::Sort(direction) => {
                let next = direction.map(|direction| TableSort {
                  column: id.clone(),
                  direction,
                });
                if let Some(write) = &write_sort {
                  write(next.clone());
                }
                for handler in &sort_heard {
                  handler(next.as_ref());
                }
              }
              MenuChoice::Pin(pin) => change(layout.clone().with_pin(&id, pin)),
              MenuChoice::Fit => change(layout.clone().with_fit(&id)),
              MenuChoice::Hide => change(layout.clone().with_hidden(&id, true)),
              MenuChoice::Toggle(other) => {
                let hidden = layout.place(&other).is_some_and(|place| place.hidden);
                change(layout.clone().with_hidden(&other, !hidden));
              }
              MenuChoice::ShowAll => change(layout.clone().with_all_shown()),
            })
          };
          let set_open: Arc<dyn Fn(bool) + Send + Sync> = {
            let (host, id) = (host.clone(), id.clone());
            Arc::new(move |open: bool| {
              let mut held = host.menu_open.lock();
              if open {
                *held = Some(id.clone());
              } else if held.as_deref() == Some(&*id) {
                *held = None;
              }
              std::mem::drop(held);
              host.rebuild();
            })
          };
          let move_to: Arc<dyn Fn(Option<usize>) + Send + Sync> = {
            let (id, layout, change, shown) = (id.clone(), layout.clone(), change.clone(), shown_ids.clone());
            Arc::new(move |before: Option<usize>| {
              let before = before.and_then(|place| shown.get(place)).map(|id| &**id);
              change(layout.clone().with_moved(&id, before));
            })
          };
          let move_by: Arc<dyn Fn(isize) + Send + Sync> = {
            let move_to = move_to.clone();
            let count = shown_ids.len();
            Arc::new(move |by: isize| {
              let to = (column_index as isize + by).clamp(0, count as isize - 1) as usize;
              if to == column_index {
                return;
              }
              // Before the column at its new place going back, before the one after it going on.
              move_to(if to < column_index {
                Some(to)
              } else {
                (to + 1 < count).then_some(to + 1)
              });
            })
          };
          Arc::new(ColumnMenuLink {
            id: id.clone(),
            title: column.title.clone(),
            sortable: column.sortable && sorts,
            sorted,
            pin: column.pin,
            can_hide: self.columns.len() > 1,
            columns: titles
              .iter()
              .map(|(id, title)| {
                let hidden = layout.place(id).is_some_and(|place| place.hidden);
                (id.clone(), title.clone(), !hidden)
              })
              .collect(),
            open,
            host: host.clone(),
            choose,
            set_open,
            move_by,
            move_to,
            place: column_index,
          })
        });
        let press = (sortable && (self.write_sort.is_some() || !self.on_sort_change.is_empty())).then(|| {
          let (write, handlers, next) = (
            self.write_sort.clone(),
            self.on_sort_change.clone(),
            TableSort::after_press(self.sort.as_ref(), &column.id),
          );
          Arc::new(move || {
            if let Some(write) = &write {
              write(next.clone());
            }
            for handler in &handlers {
              handler(next.as_ref());
            }
          }) as Arc<dyn Fn() + Send + Sync>
        });
        let resizable = column.resizable.unwrap_or(self.resizable) && !self.disabled;
        let resize = window
          .as_ref()
          .and_then(|window| window.host.clone())
          .filter(|_| resizable)
          .map(|host| {
            let floor = column.track.min.max(LEAST_COLUMN);
            Arc::new(ResizeLink {
              layout: change_layout.clone().and_then(|change| {
                let (layout, _) = &**self.arranged.as_ref()?;
                let layout = layout.clone();
                Some(
                  Arc::new(move |id: &str, width: f32| change(layout.clone().with_width(id, width)))
                    as Arc<dyn Fn(&str, f32) + Send + Sync>,
                )
              }),
              host,
              column: column_index,
              id: column.id.clone(),
              least: floor,
              most: column.track.max,
              extra: geometry.gap * f32::from(u8::from(column_index + 1 < columns_count))
                + if column_index == 0 && sticky && !checkboxes {
                  geometry.padding_start
                } else {
                  0.0
                }
                + trail_of(column_index),
              start: Mutex::new(0.0),
              on_resize: self.on_column_resize.clone(),
              on_resize_end: self.on_column_resize_end.clone(),
              label: self
                .words
                .resize
                .as_ref()
                .map_or_else(|| format!("Resize {}", column.title), |label| label(&column.title)),
            })
          });
        TableHeader {
          column: column_index,
          title: column.title.clone(),
          title_hidden: column.title_hidden,
          align: column.align,
          sortable,
          sorted,
          pinned: sticky_of(column_index),
          last: cell_ends(column_index),
          fill: sticky_of(column_index).and(header_fill.clone()),
          gap: geometry.gap,
          lead: if column_index == 0 && sticky && !checkboxes {
            geometry.padding_start
          } else {
            0.0
          },
          trail: trail_of(column_index),
          recipe: self.resolve(theme, &header_case),
          press,
          resize,
          element: header_refs.get(&column_index).cloned(),
          key: column.id.clone(),
          menu,
          drop,
          id: id_of(&format!("header-{}", column.id)),
        }
      })
      .collect();
    if let Some(host) = menu_host {
      *host.header_refs.lock() = header_refs;
    }

    // Every row's key is looked up once, however many are selected.
    let held: HashSet<&K> = self.selected.iter().collect();
    let all = !keys.is_empty() && keys.iter().all(|key| held.contains(key));
    let some = keys.iter().any(|key| held.contains(key));
    let select_all = (checkboxes && !self.disabled).then(|| {
      let model = model.clone();
      Arc::new(move || model.select_shown(!all)) as Arc<dyn Fn() + Send + Sync>
    });

    let total = self.total.unwrap_or(count);
    let selected_count = self.selected.len();
    let count_text = match &self.words.count {
      Some(text) => text(total, selectable.then_some(selected_count)),
      None if selectable => format!("{selected_count} of {total} {} selected", rows_word(total)),
      None => format!("{total} {}", rows_word(total)),
    };

    let body_window = window.as_ref().map(|window| BodyWindow {
      row_height: window.row_height,
      count: line_count,
    });
    let empty = (count == 0 && !self.loading).then(|| {
      self.empty.clone().unwrap_or_else(|| {
        EmptyState::new(&self.words.empty)
          .icon(icons::INBOX)
          .size(EmptyStateSize::Sm)
          .build(theme)
      })
    });

    TableView {
      headers,
      rows,
      groups,
      grouped,
      selectable,
      checkboxes,
      grid: grid_model.is_some(),
      pinned_start: sticky,
      pinned_end: end_pins > 0,
      lead_inset,
      selection: match (all, some) {
        (true, _) => Some(true),
        (false, true) => None,
        (false, false) => Some(false),
      },
      select_all,
      select_all_label: self.words.select_all.clone(),
      loading: self.loading,
      loading_rows: self.loading_rows,
      empty,
      footer: footer.then_some(count_text),
      pagination: self.pagination.clone(),
      label: self.label.clone(),
      total,
      height: self.height,
      max_height: self.max_height,
      disabled: self.disabled,
      last_row: row_recipe(false, !footer),
      recipe,
      geometry,
      tracks,
      columns_built,
      window: body_window,
      scroll: window.as_ref().map(|window| window.scroll.clone()),
      on_scroll: window.as_ref().and_then(|window| window.on_scroll.clone()),
      focus_rows: window.as_ref().map(|window| window.rows.clone()),
      scroll_ref: window
        .as_ref()
        .and_then(|window| window.host.as_ref())
        .map(|host| host.state.scroll_ref.clone()),
      id: self.id.clone(),
      key: self.key.clone(),
    }
  }
}

impl<R: Send + Sync + 'static, K: Clone + Eq + Hash + Send + Sync + 'static> From<Table<R, K>> for Element {
  fn from(table: Table<R, K>) -> Self {
    table.build(&current_theme())
  }
}

/// What a column says of a row as text: its text, or the value its editor edits, or nothing for a cell the app
/// builds. What the rows are grouped by.
fn column_text<R>(column: &TableColumn<R>, row: &R) -> String {
  if let Content::Text(text) = &column.content {
    return text(row);
  }
  match column.editor.as_ref().map(|editor| &editor.kind) {
    Some(EditorKind::Text(text)) => text(row),
    Some(EditorKind::Number(number)) => number(row).to_string(),
    Some(EditorKind::Choice(choices, chosen)) => choices
      .get(chosen(row))
      .map(|choice| choice.to_string())
      .unwrap_or_default(),
    Some(EditorKind::Checkbox(checked)) => checked(row).to_string(),
    None => String::new(),
  }
}

fn rows_word(count: usize) -> &'static str {
  if count == 1 { "row" } else { "rows" }
}

/// The numbers of the recipe a table lays its tracks out with, in pixels.
#[derive(Clone, Copy, Debug, Default)]
pub struct Geometry {
  /// The room between two columns.
  pub gap: f32,
  /// The padding at the start of a row.
  pub padding_start: f32,
  /// The padding at the end of a row.
  pub padding_end: f32,
  /// The width of the column of checkboxes.
  pub select: f32,
}

impl Geometry {
  fn of(recipe: &ResolvedRecipe) -> Self {
    let theme = recipe.theme();
    let spacing =
      |value: Option<crate::node::SpacingValue>| value.map(|value| value.resolve(&theme.spacing(), 0.0)).unwrap_or(0.0);
    let row = recipe.part(TablePart::Row).rest();
    let select = recipe.part(TablePart::Select).rest();
    Self {
      gap: spacing(row.gap),
      padding_start: spacing(row.padding_start.or(row.padding_x)),
      padding_end: spacing(row.padding_end.or(row.padding_x)),
      select: px(select.width, theme).unwrap_or(16.0),
    }
  }
}

/// A length of the recipe in pixels, where it is one.
fn px(length: Option<Length>, theme: &Theme) -> Option<f32> {
  match length?.resolve(theme)? {
    Dimension::Px(px) => Some(px),
    _ => None,
  }
}

/// The rows and the selection, which every row's behavior reads.
struct Model<K> {
  keys: Arc<Vec<K>>,
  selected: Vec<K>,
  select: Option<Arc<dyn Fn(Vec<K>) + Send + Sync>>,
  on_selection_change: Vec<SelectionHeard<K>>,
  on_row_click: Vec<Told<K>>,
  on_row_activate: Vec<Told<K>>,
}

/// How a press or a key selects a row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Pick {
  /// That row alone.
  Only,
  /// The row in or out of what is selected, the rest as it is.
  Toggle,
  /// The rows from the last selected one to it.
  Range,
}

impl<K: Clone + Eq + Hash + Send + Sync + 'static> Model<K> {
  fn actions(self: &Arc<Self>, index: usize) -> Arc<RowActions> {
    let (pick, click, activate, all) = (self.clone(), self.clone(), self.clone(), self.clone());
    Arc::new(RowActions {
      pick: Arc::new(move |how| pick.pick(index, how)),
      click: Arc::new(move || {
        for handler in &click.on_row_click {
          handler(&click.keys[index]);
        }
      }),
      activate: Arc::new(move || {
        for handler in &activate.on_row_activate {
          handler(&activate.keys[index]);
        }
      }),
      select_all: Arc::new(move || all.select_shown(true)),
      selectable: self.select.is_some(),
    })
  }

  fn pick(&self, index: usize, pick: Pick) {
    let key = &self.keys[index];
    let next = match pick {
      Pick::Only => vec![key.clone()],
      Pick::Toggle => {
        let mut next = self.selected.clone();
        match next.iter().position(|held| held == key) {
          Some(at) => {
            next.remove(at);
          }
          None => next.push(key.clone()),
        }
        next
      }
      Pick::Range => {
        let anchor = self
          .selected
          .last()
          .and_then(|last| self.keys.iter().position(|held| held == last))
          .unwrap_or(index);
        let (start, end) = (anchor.min(index), anchor.max(index));
        // What was selected outside the range stays; the anchor is last, where the next range starts.
        let range: HashSet<&K> = self.keys[start..=end].iter().collect();
        let mut next: Vec<K> = self
          .selected
          .iter()
          .filter(|held| !range.contains(held))
          .cloned()
          .collect();
        next.extend((start..=end).filter(|at| *at != anchor).map(|at| self.keys[at].clone()));
        next.push(self.keys[anchor].clone());
        next
      }
    };
    self.write(next);
  }

  /// Every row the table shows selected, or none of them, the rest of the selection as it is.
  fn select_shown(&self, all: bool) {
    let shown: HashSet<&K> = self.keys.iter().collect();
    let mut next: Vec<K> = self
      .selected
      .iter()
      .filter(|held| !shown.contains(held))
      .cloned()
      .collect();
    if all {
      next.extend(self.keys.iter().cloned());
    }
    self.write(next);
  }

  /// Asks for `next` to be selected and tells every handler of it: nothing where it is what is selected now.
  fn write(&self, next: Vec<K>) {
    if let Some(select) = &self.select
      && next != self.selected
    {
      select(next.clone());
      for handler in &self.on_selection_change {
        handler(&next);
      }
    }
  }
}

/// What a row that takes input does, with the type of the table's keys out of sight.
struct RowActions {
  pick: Arc<dyn Fn(Pick) + Send + Sync>,
  click: Arc<dyn Fn() + Send + Sync>,
  activate: Arc<dyn Fn() + Send + Sync>,
  select_all: Arc<dyn Fn() + Send + Sync>,
  selectable: bool,
}

/// The rows a mounted table builds, and what its host does for them.
#[derive(Clone)]
struct Window {
  /// The places of the rows built, in order.
  indices: Vec<usize>,
  /// The row that has the focus, which stays built wherever the rows are scrolled.
  focused: Option<usize>,
  row_height: f32,
  scroll: ScrollState,
  refs: HashMap<usize, ElementRef>,
  /// Which columns are built, where the table builds those in view only: `None` for every one.
  columns: Option<Arc<Vec<bool>>>,
  /// The refs of the cells of a grid, by row and column, and of the editor of the cell being edited.
  cell_refs: HashMap<(usize, usize), ElementRef>,
  editor_ref: ElementRef,
  host: Option<Arc<HostLink>>,
  /// The rows as the body's focus group steps over them, built or not.
  rows: Arc<dyn FocusRows>,
  /// The lines the table shows: its rows, or its groups and the rows of the open ones.
  lines: Arc<Lines>,
  on_scroll: Option<Arc<dyn Fn(ScrollEvent) + Send + Sync>>,
}

/// Where the rows of a mounted table are: the room the rows that are not built take.
#[derive(Clone, Copy, Debug)]
pub struct BodyWindow {
  /// The height of a row.
  pub row_height: f32,
  /// How many rows there are.
  pub count: usize,
}

/// A table as a renderer sees it: its headers, the rows it builds, and the style of its parts.
pub struct TableView {
  /// The headers of the columns, in order.
  pub headers: Vec<TableHeader>,
  /// The rows built, in order: every row, or those of a mounted table in view and around it.
  pub rows: Vec<TableRow>,
  /// The groups' rows built, where the rows are grouped: each says its place among the lines, as a row does.
  pub groups: Vec<TableGroup>,
  /// Whether the rows are grouped: the body is a nested list, Right opening a group and Left closing it.
  pub grouped: bool,
  /// Whether its rows are selected.
  pub selectable: bool,
  /// Whether it has the column of checkboxes.
  pub checkboxes: bool,
  /// Whether its cells take the focus and are edited: a mounted data grid.
  pub grid: bool,
  /// Whether the cells at the start stay while the rows scroll sideways ([`ColumnPin::Start`]).
  pub pinned_start: bool,
  /// Whether the cells at the end stay ([`ColumnPin::End`]).
  pub pinned_end: bool,
  /// Where the first column stays from the start, after the checkbox's.
  pub lead_inset: f32,
  /// Whether every row is selected: `Some(true)` all, `Some(false)` none, `None` some.
  pub selection: Option<bool>,
  select_all: Option<Arc<dyn Fn() + Send + Sync>>,
  /// The name of the header's checkbox.
  select_all_label: Arc<str>,
  pub loading: bool,
  pub loading_rows: usize,
  /// What stands under the header where there are no rows.
  pub empty: Option<Element>,
  /// What the footer says, where the table has one.
  pub footer: Option<String>,
  pub pagination: Option<Element>,
  pub label: Option<Arc<str>>,
  /// How many rows there are in all.
  pub total: usize,
  pub height: Option<Length>,
  pub max_height: Option<Length>,
  pub disabled: bool,
  /// The style of each part of the table, its rows at rest.
  pub recipe: ResolvedRecipe,
  /// The style of the last row, with nothing under it in the box where the table has no footer.
  pub last_row: ResolvedRecipe,
  pub geometry: Geometry,
  /// The tracks of the columns, which the header and every row lay their cells out in.
  pub tracks: ColumnTracks,
  /// Which columns are built, where a mounted table builds those in view only: `None` for every one.
  pub columns_built: Option<Arc<Vec<bool>>>,
  /// Where the rows a mounted table does not build are.
  pub window: Option<BodyWindow>,
  scroll: Option<ScrollState>,
  on_scroll: Option<Arc<dyn Fn(ScrollEvent) + Send + Sync>>,
  focus_rows: Option<Arc<dyn FocusRows>>,
  /// What a mounted grid knows its view by, where a drag of cells past its edge scrolls it.
  scroll_ref: Option<ElementRef>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl TableView {
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// The box in the style of its part, with nothing in it yet. It is a table to what reads the tree of the
  /// window, a grid where its rows are selected, with how many rows (the header's among them) and columns.
  pub fn root(&self) -> Column {
    // The tracks are the columns and the checkboxes': there however a renderer took the headers.
    let columns = self.tracks.len();
    let mut root = Column::new()
      .align_items(Alignment::Stretch)
      .part(&self.part(TablePart::Root))
      .describe("a11y_role", if self.selectable || self.grid { "grid" } else { "table" })
      .describe("a11y_rowcount", (self.total + 1).to_string())
      .describe("a11y_colcount", columns.to_string());
    if let Some(label) = &self.label {
      root = root.describe("a11y_name", label.clone());
    }
    if self.selectable {
      root = root.describe("a11y_multiselectable", "true");
    }
    if self.loading {
      root = root.describe("a11y_busy", "true");
    }
    if let Some(height) = self.height.and_then(|height| height.resolve(self.theme())) {
      root = root.height(height);
    }
    if let Some(height) = self.max_height.and_then(|height| height.resolve(self.theme())) {
      root = root.max_height(height);
    }
    if let Some(id) = &self.id {
      root = root.id(id.clone());
    }
    if let Some(key) = &self.key {
      root = root.key(key.clone());
    }
    root
  }

  /// The header row: the header's checkbox, then a header for each column, in the tracks of the columns. It
  /// stays at the top while the rows scroll.
  pub fn header(&mut self) -> Row {
    let part = self.part(TablePart::Header);
    let mut header = Row::new()
      .part(&part)
      .spacing(0.0)
      // Each cell is as high as the row, so that one that stays covers what scrolls under it, and sets what it
      // holds in the middle of that height itself.
      .align_items(Alignment::Stretch)
      .column_tracks(&self.tracks)
      .sticky(Sticky::top(0.0))
      .focus_group(FocusGroup::horizontal().one_stop(true).wrap(false))
      .describe("a11y_role", "row")
      .describe("a11y_rowindex", "1");
    if self.pinned_start {
      header = header.padding_left(0.0);
    }
    if self.pinned_end {
      header = header.padding_right(0.0);
    }
    if self.checkboxes {
      header = header.child(self.select_all());
    }
    let headers = std::mem::take(&mut self.headers);
    let placed = headers.into_iter().map(|header| (header.column, header.render()));
    header.with_children(in_tracks(
      self.columns_built.as_ref().map(|built| built.as_slice()),
      placed,
    ))
  }

  /// The header's checkbox, in the cell of the column of checkboxes: checked where every row is selected,
  /// mixed where some are. A press selects every row, or none where all are.
  pub fn select_all(&self) -> Element {
    let part = self.part(TablePart::Select);
    let mut checkbox = Checkbox::new(&self.select_all_label)
      .label_hidden(true)
      .checked(self.selection == Some(true))
      .indeterminate(self.selection.is_none())
      .disabled(self.disabled);
    if let Some(press) = self.select_all.clone() {
      checkbox = checkbox.on_change(move |_| press());
    }
    if let Some(id) = &self.id {
      checkbox = checkbox.id(format!("{id}-select-all"));
    }
    let mut cell = lead_cell(&part, self.geometry, self.pinned_start).child(checkbox.build(self.theme()));
    if self.pinned_start {
      // It covers what scrolls under it, in the header's fill.
      cell = cell.sticky(Sticky::left(0.0));
      if let Some(fill) = self.part(TablePart::Header).rest().background {
        cell = cell.background(fill);
      }
    }
    cell.into()
  }

  /// The rows built in their column, one stop of Tab that the arrow keys move through, with the room of the
  /// rows a mounted table does not build before and after them.
  pub fn body(&mut self) -> Column {
    // The rows and the groups' rows in the order of their lines.
    let mut lines: Vec<(usize, Element)> = std::mem::take(&mut self.rows)
      .into_iter()
      .map(|row| (row.position, row.render()))
      .chain(
        std::mem::take(&mut self.groups)
          .into_iter()
          .map(|group| (group.position, group.render())),
      )
      .collect();
    lines.sort_by_key(|(position, _)| *position);
    let group = FocusGroup::vertical().one_stop(true).wrap(false).nested(self.grouped);
    let mut body = Column::new()
      .align_items(Alignment::Stretch)
      .describe("a11y_role", "rowgroup");
    let (Some(window), Some(focus_rows)) = (self.window, self.focus_rows.clone()) else {
      // Every row is built: the group steps over the rows themselves, PageUp and PageDown by what the table's
      // scroll shows.
      return body
        .focus_group(group.pages(true))
        .with_children(lines.into_iter().map(|(_, line)| line));
    };
    // A mounted table builds the rows around its view: the group steps over all of them, and a row it goes to
    // that is not built is revealed and focused once it is.
    body = body.focus_group(group).focus_rows(focus_rows);
    let spacer = |before: usize, rows: usize| {
      Row::new()
        .height(rows as f32 * window.row_height)
        .key(format!("before-{before}"))
    };
    let mut next = 0;
    for (position, line) in lines {
      if position > next {
        body = body.child(spacer(position, position - next));
      }
      next = position + 1;
      body = body.child(line);
    }
    if window.count > next {
      body = body.child(spacer(window.count, window.count - next));
    }
    body
  }

  /// Rows of skeletons in the tracks of the columns, where the rows will be: a line in each cell, the first
  /// column's of a width that changes from row to row.
  pub fn skeletons(&self) -> Column {
    const FIRST: [f32; 4] = [150.0, 190.0, 120.0, 170.0];
    let part = self.part(TablePart::Row);
    let cell = self.part(TablePart::Cell);
    let columns = self.tracks.len() - usize::from(self.checkboxes);
    Column::new()
      .align_items(Alignment::Stretch)
      .with_children((0..self.loading_rows).map(|index| {
        // The last row's part, with no line under it, is its only one: a part given after the tracks would take
        // them away, and the row would fall out of the columns.
        let row_part = if self.loading_rows == index + 1 && self.footer.is_none() {
          self.last_row.part(TablePart::Row)
        } else {
          part.clone()
        };
        let mut row = Row::new()
          .part(&row_part)
          .spacing(0.0)
          .align_items(Alignment::Stretch)
          .column_tracks(&self.tracks)
          .describe("a11y_hidden", "true");
        if self.checkboxes {
          row = row.child(Row::new());
        }
        row.with_children((0..columns).map(|column| {
          let width = if column == 0 { FIRST[index % FIRST.len()] } else { 70.0 };
          let line = Skeleton::line().width(width).build(self.theme());
          let mut wrapper = Row::new().part(&cell).align_items(Alignment::Center).child(line);
          if column + 1 < columns {
            wrapper = wrapper.padding_right(self.geometry.gap);
          }
          wrapper
        }))
      }))
  }

  /// What stands under the header where there are no rows, as wide as the table.
  pub fn empty(&mut self) -> Option<Row> {
    let empty = self.empty.take()?;
    Some(
      Row::new()
        .justify(Justify::Center)
        .part(&self.part(TablePart::Empty))
        .child(empty),
    )
  }

  /// The footer: the count of the rows at its start, the pagination at its end.
  pub fn footer(&mut self) -> Option<Row> {
    let text = self.footer.take()?;
    let count = self.part(TablePart::Count);
    let mut footer = Row::new()
      .align_items(Alignment::Center)
      .justify(Justify::SpaceBetween)
      .part(&self.part(TablePart::Footer))
      .child(Row::new().part(&count).child(count.text(&text).nowrap()));
    if let Some(pagination) = self.pagination.take() {
      footer = footer.child(pagination);
    }
    Some(footer)
  }

  /// How the default renderer builds a table: the header and the rows (or the empty state, or the skeletons)
  /// scrolling both ways in the box, and the footer under them.
  pub fn render(mut self) -> Element {
    let mut content = Column::new()
      .align_items(Alignment::Stretch)
      .measures_column_tracks(&self.tracks)
      .child(self.header());
    // The empty state is in the middle of the table's box, not of its columns, which may be wider: it stands
    // under what scrolls, which holds the header alone.
    let empty = if self.loading { None } else { self.empty() };
    if self.loading {
      content = content.child(self.skeletons());
    } else if empty.is_none() {
      content = content.child(self.body());
    }
    // Its bars run beside the rows: the one down under the header that stays, the one across after the first
    // column that stays.
    let mut scroll = ScrollBoth::new(content)
      .fill_width(true)
      .scrollbars_clear_of_sticky(true)
      // The bar across stands in a strip of its own under the rows while they are wider than the view: no row
      // is ever under it, and a table whose columns fit has no strip.
      .reserve_bar_across(true)
      .part(&self.part(TablePart::Scroll));
    if let Some(empty) = empty {
      let mut root = self.root().child(scroll.flex_full(0.0, 1.0, None));
      let mut empty = Column::new().align_items(Alignment::Stretch).child(empty);
      if self.height.is_some() {
        empty = empty.flex(1.0).justify(Justify::Center);
      }
      root = root.child(empty);
      if let Some(footer) = self.footer() {
        root = root.child(footer);
      }
      return root.into();
    }
    if self.height.is_some() {
      scroll = scroll.flex_full(1.0, 1.0, None);
    } else {
      scroll = scroll.flex_full(0.0, 1.0, None);
    }
    if let Some(state) = self.scroll.clone() {
      scroll = scroll.with_scroll_state(state);
    }
    if let Some(element) = self.scroll_ref.clone() {
      scroll = scroll.ref_element(element);
    }
    if let Some(on_scroll) = self.on_scroll.clone() {
      scroll = scroll.on_scroll(move |event: ScrollEvent| on_scroll(event));
    }
    let mut root = self.root().child(scroll);
    if let Some(footer) = self.footer() {
      root = root.child(footer);
    }
    root.into()
  }
}

/// The cells of a row, each with the column it is of, in the tracks of the columns: where a run of columns is
/// not built (`built` says so), one room spans their tracks in place of a cell each.
pub fn in_tracks(built: Option<&[bool]>, cells: impl IntoIterator<Item = (usize, Element)>) -> Vec<Element> {
  let mut placed = Vec::new();
  let mut next = 0;
  let room = |from: usize, to: usize, placed: &mut Vec<Element>| {
    if to > from {
      placed.push(Row::new().track_span(to - from).key(format!("columns-{from}")).into());
    }
  };
  for (column, cell) in cells {
    room(next, column, &mut placed);
    placed.push(cell);
    next = column + 1;
  }
  if let Some(built) = built {
    room(next, built.len(), &mut placed);
  }
  placed
}

/// The cell of the column of checkboxes, the header's or a row's: as wide as its track, its padding at the
/// start where the cells at the start stay.
fn lead_cell(part: &ResolvedPart, geometry: Geometry, sticky: bool) -> Row {
  let mut cell = Row::new()
    .align_items(Alignment::Center)
    .part(part)
    .width(Dimension::Auto)
    .padding_right(geometry.gap);
  if sticky {
    cell = cell.padding_left(geometry.padding_start);
  }
  cell
}

/// The header of one column as a renderer sees it.
pub struct TableHeader {
  /// Which column it is of, from 0.
  pub column: usize,
  pub title: Arc<str>,
  /// Whether its title is said to what reads the tree and not drawn: the column of a row's actions.
  pub title_hidden: bool,
  pub align: ColumnAlign,
  /// Whether a press of it sorts the rows.
  pub sortable: bool,
  /// Which way the rows are sorted by it, if they are.
  pub sorted: Option<SortDirection>,
  /// Where it stays while the rows scroll sideways, if its column is pinned.
  pub pinned: Option<Sticky>,
  /// Whether it is the last column, with no room after it.
  pub last: bool,
  /// The fill of a header that stays: the header row's, over which the others scroll.
  pub fill: Option<BackgroundColor>,
  gap: f32,
  lead: f32,
  trail: f32,
  /// The style of each part of the header, in its case.
  pub recipe: ResolvedRecipe,
  press: Option<Arc<dyn Fn() + Send + Sync>>,
  resize: Option<Arc<ResizeLink>>,
  /// Where a mounted grid finds the header, which a column dragged by its header goes before.
  element: Option<ElementRef>,
  /// The column's id, which keeps the header the same element wherever the column is.
  key: Arc<str>,
  menu: Option<Arc<ColumnMenuLink>>,
  /// Where a column dragged by its header goes, drawn on this one.
  drop: Option<DropSide>,
  id: Option<Arc<str>>,
}

impl TableHeader {
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part)
  }

  /// The cell in the column's track: the room after it before the next column, its content where the column
  /// has it. It is a column header to what reads the tree, with which way it is sorted where it sorts.
  pub fn cell(&self) -> Row {
    let mut cell = Row::new()
      // Where a column dragged by its header goes, drawn as a line at its start or its end.
      .part(&self.part(TablePart::DropLine))
      .align_items(Alignment::Center)
      .justify(justify(self.align))
      .describe("a11y_role", "columnheader")
      .describe("a11y_name", self.title.clone())
      // A header is the same element wherever the columns built around it start, and wherever its column is
      // moved to.
      .key(format!("header-{}", self.key));
    if self.sortable || self.sorted.is_some() {
      let sort = match self.sorted {
        Some(SortDirection::Ascending) => "ascending",
        Some(SortDirection::Descending) => "descending",
        None => "none",
      };
      cell = cell.describe("a11y_sort", sort);
    }
    if !self.last {
      cell = cell.padding_right(self.gap);
    }
    if self.trail > 0.0 {
      cell = cell.padding_right(self.trail);
    }
    if self.lead > 0.0 {
      cell = cell.padding_left(self.lead);
    }
    if let Some(pinned) = self.pinned {
      cell = cell.sticky(pinned);
    }
    if let Some(fill) = self.fill.clone() {
      cell = cell.background(fill);
    }
    if let Some(element) = &self.element {
      cell = cell.ref_element(element.clone());
    }
    if self.menu.is_some() {
      // Its menu's trigger shows while the pointer is on the header or the focus is in it.
      cell = cell.interaction_group(true);
    }
    cell
  }

  /// What opens the column's menu, at the end of its header: `None` where the header has no menu. It takes no
  /// focus: Alt+Down on the header opens the menu too.
  pub fn menu_trigger(&self) -> Option<Element> {
    let link = self.menu.clone()?;
    let part = self.part(TablePart::MenuTrigger);
    let toggle = link.clone();
    let mut trigger = pressable(&part, false)
      .focusable(false)
      .align_items(Alignment::Center)
      .describe("a11y_name", format!("{} column menu", link.title))
      .describe("a11y_haspopup", "menu")
      .describe("a11y_expanded", if link.open { "true" } else { "false" })
      .on_click(move |event: MouseEvent| {
        // A press of it is its own: it does not sort the rows.
        event.stop_propagation();
        (toggle.set_open)(!toggle.open);
      })
      .child(glyph(
        &self.part(TablePart::SortIcon),
        Name::new(icons::MORE_VERTICAL),
        false,
      ));
    if let Some(id) = &self.id {
      trigger = trigger.id(format!("{id}-menu"));
    }
    if link.open {
      trigger = trigger.ref_element(link.host.menu_anchor.clone());
    }
    Some(trigger.into())
  }

  /// The column's menu, while it is open: the sort, the pin, its width to fit its content, hiding it, and the
  /// columns to show or hide.
  pub fn menu(&self) -> Option<Element> {
    let link = self.menu.clone().filter(|link| link.open)?;
    let choose = |choice: MenuChoice| {
      let choose = link.choose.clone();
      move || choose(choice.clone())
    };
    let set_open = link.set_open.clone();
    let mut menu = Menu::controlled(true, link.host.menu_anchor.clone(), move |open| set_open(open))
      .label(&format!("{} column", link.title))
      .placement(Placement::BottomEnd);
    if let Some(id) = &self.id {
      menu = menu.id(format!("{id}-menu-panel"));
    }
    if link.sortable {
      menu = menu
        .item(
          MenuItem::new("Sort ascending")
            .icon(icons::ARROW_UP)
            .checked(link.sorted == Some(SortDirection::Ascending))
            .on_select(choose(MenuChoice::Sort(Some(SortDirection::Ascending)))),
        )
        .item(
          MenuItem::new("Sort descending")
            .icon(icons::ARROW_DOWN)
            .checked(link.sorted == Some(SortDirection::Descending))
            .on_select(choose(MenuChoice::Sort(Some(SortDirection::Descending)))),
        )
        .item(
          MenuItem::new("Clear sort")
            .icon(icons::CHEVRONS_VERTICAL)
            .disabled(link.sorted.is_none())
            .on_select(choose(MenuChoice::Sort(None))),
        )
        .separator();
    }
    let hidden = link.columns.iter().any(|(_, _, shown)| !shown);
    let shown = link.columns.iter().filter(|(_, _, shown)| *shown).count();
    let columns = Menu::new(link.host.submenu_open.clone(), link.host.submenu_anchor.clone())
      .label("Columns")
      .items(link.columns.iter().map(|(id, title, on)| {
        MenuItem::new(title)
          .checked(*on)
          // The last column shown stays.
          .disabled(*on && shown == 1)
          .on_select(choose(MenuChoice::Toggle(id.clone())))
      }))
      .separator()
      .item(
        MenuItem::new("Show all columns")
          .disabled(!hidden)
          .on_select(choose(MenuChoice::ShowAll)),
      );
    Some(
      menu
        .item(
          MenuItem::new("Pin to start")
            .icon(icons::PIN)
            .checked(link.pin == ColumnPin::Start)
            .on_select(choose(MenuChoice::Pin(ColumnPin::Start))),
        )
        .item(
          MenuItem::new("Pin to end")
            .icon(icons::PIN)
            .checked(link.pin == ColumnPin::End)
            .on_select(choose(MenuChoice::Pin(ColumnPin::End))),
        )
        .item(
          MenuItem::new("Unpin")
            .icon(icons::CLOSE)
            .disabled(link.pin == ColumnPin::None)
            .on_select(choose(MenuChoice::Pin(ColumnPin::None))),
        )
        .separator()
        .item(
          MenuItem::new("Size to fit")
            .icon(icons::RESIZE_HORIZONTAL)
            .on_select(choose(MenuChoice::Fit)),
        )
        .item(
          MenuItem::new("Hide column")
            .icon(icons::EYE_OFF)
            .disabled(!link.can_hide)
            .on_select(choose(MenuChoice::Hide)),
        )
        .item(MenuItem::new("Columns").icon(icons::EYE).submenu(columns))
        .build(self.recipe.theme()),
    )
  }

  /// The arrow of a column the rows are sorted by, up or down, and the mark of one that sorts.
  pub fn sort_icon(&self) -> Option<Element> {
    let name = match self.sorted {
      Some(SortDirection::Ascending) => icons::ARROW_UP,
      Some(SortDirection::Descending) => icons::ARROW_DOWN,
      None if self.sortable => icons::CHEVRONS_VERTICAL,
      None => return None,
    };
    Some(glyph(&self.part(TablePart::SortIcon), Name::new(name), false))
  }

  /// What is dragged at the end of the header to make the column wider or narrower: `None` for a column that is
  /// not resized. It is a separator to what reads the tree, with the column's width as its value; the keys that
  /// resize the column are its header's (Ctrl and Left or Right).
  pub fn resizer(&self) -> Option<Row> {
    let link = self.resize.clone()?;
    let width = link.width();
    let (start, moved, end) = (link.clone(), link.clone(), link.clone());
    Some(
      Row::new()
        .part(&self.part(TablePart::Resizer))
        .justify(Justify::Center)
        .align_items(Alignment::Stretch)
        .height(Dimension::Pct(100.0))
        .interaction_group(true)
        .describe("a11y_role", "separator")
        .describe("a11y_orientation", "vertical")
        .describe("a11y_name", link.label.clone())
        .describe("a11y_valuenow", format!("{}", width.round()))
        .start_drag_buttons(MouseButtonMask::LEFT)
        .on_drag_start(move |_: DragEvent| start.begin())
        .on_drag_move(move |event: DragEvent| moved.drag(event.total_delta_x))
        .on_drag_end(move |_: DragEvent| end.finish())
        // A press of it is its own: it does not sort the rows.
        .on_click(|event: MouseEvent| event.stop_propagation())
        .child(Row::new().part(&self.part(TablePart::ResizeLine))),
    )
  }

  /// How the default renderer builds a header: its title and its arrow, pressed to sort where it sorts, and its
  /// resizer at its end where the column is resized.
  pub fn render(self) -> Element {
    let part = self.part(TablePart::HeaderCell);
    let title = if self.title_hidden {
      None
    } else {
      // It gives way to the room the column has, and ends in an ellipsis.
      Some(
        part
          .text(&self.title)
          .nowrap()
          .text_overflow(crate::node::node_kind::TextOverflow::Elipsis)
          .flex_full(0.0, 1.0, None)
          .min_width(Dimension::Px(0.0)),
      )
    };
    let resizer = self.resizer();
    let mut cell = self.cell();
    let label: Element = match self.press.clone().filter(|_| self.sortable) {
      None => {
        let mut label = Row::new()
          .align_items(Alignment::Center)
          .flex_full(0.0, 1.0, None)
          .min_width(Dimension::Px(0.0))
          .part(&part);
        if let Some(title) = title {
          label = label.child(title);
        }
        if let Some(icon) = self.sort_icon() {
          label = label.child(icon);
        }
        // A header that is resized or has a menu and is not sorted is a stop of the header's keys.
        if self.resize.is_some() || self.menu.is_some() {
          label = label.tab_index(0).describe("a11y_name", self.title.clone());
          if let Some(id) = &self.id {
            label = label.id(id.clone());
          }
        }
        label.into()
      }
      Some(press) => {
        let mut button = pressable(&part, false)
          .align_items(Alignment::Center)
          .flex_full(0.0, 1.0, None)
          .min_width(Dimension::Px(0.0))
          .describe("a11y_name", format!("Sort by {}", self.title))
          .on_click(move |_| press());
        if let Some(title) = title {
          button = button.child(title);
        }
        if let Some(icon) = self.sort_icon() {
          button = button.child(icon);
        }
        if let Some(id) = &self.id {
          button = button.id(id.clone());
        }
        button.into()
      }
    };
    // A header with a menu is dragged to another place among the columns.
    let label = match &self.menu {
      Some(link) => {
        let (start, moved, end) = (link.clone(), link.clone(), link.clone());
        Row::new()
          .flex_full(0.0, 1.0, None)
          .min_width(Dimension::Px(0.0))
          .align_items(Alignment::Center)
          .start_drag_buttons(MouseButtonMask::LEFT)
          .on_drag_start(move |_: DragEvent| {
            *start.host.header_drop.lock() = Some((start.place, Some(start.place)));
          })
          .on_drag_move(move |event: DragEvent| {
            let to = moved.host.drop_place(event.x);
            let mut held = moved.host.header_drop.lock();
            if held.is_some_and(|(_, at)| at != to) {
              *held = Some((moved.place, to));
              drop(held);
              moved.host.rebuild();
            }
          })
          .on_drag_end(move |_: DragEvent| {
            let held = end.host.header_drop.lock().take();
            end.host.rebuild();
            if let Some((from, to)) = held
              && to != Some(from)
              && to != Some(from + 1)
            {
              (end.move_to)(to);
            }
          })
          .child(label)
          .into()
      }
      None => label,
    };
    let trigger = self.menu_trigger();
    let menu = self.menu();
    let (resize_link, menu_link) = (self.resize.clone(), self.menu.clone());
    if resize_link.is_some() || menu_link.is_some() {
      cell = cell.on_key_down(move |event: KeyboardEvent| {
        if !event.focus_within {
          return;
        }
        let command = event.ctrl || event.meta;
        if let Some(link) = &menu_link {
          // Alt+Down, the menu key or Shift+F10 opens the column's menu; Ctrl+Shift+PageUp and PageDown move the
          // column, as a browser moves a tab.
          let key = event.key.as_str();
          if (event.alt && !command && key == "ArrowDown") || key == "ContextMenu" || (event.shift && key == "F10") {
            event.prevent_default();
            (link.set_open)(true);
            return;
          }
          if command && event.shift && !event.alt && matches!(key, "PageUp" | "PageDown") {
            event.prevent_default();
            (link.move_by)(if key == "PageUp" { -1 } else { 1 });
            return;
          }
        }
        let Some(link) = &resize_link else {
          return;
        };
        if !command || event.alt {
          return;
        }
        let step = if event.shift { RESIZE_STEP_SHIFT } else { RESIZE_STEP };
        let by = match event.key.as_str() {
          "ArrowLeft" => -step,
          "ArrowRight" => step,
          _ => return,
        };
        event.prevent_default();
        link.step(by);
      });
    }
    if resizer.is_none() && trigger.is_none() {
      return cell.child(label).into();
    }
    if resizer.is_some() {
      // The label takes the room the resizer leaves, which stands in the room after the column, on its edge.
      let half = px(self.part(TablePart::Resizer).rest().width, self.recipe.theme()).unwrap_or(8.0) / 2.0;
      let after = if self.last { self.trail } else { self.gap };
      cell = cell.padding_right((after / 2.0 - half).max(0.0));
    }
    // The title where its column puts it, in the room the trigger at the header's end leaves.
    let titled = Row::new()
      .flex_full(1.0, 1.0, Some(0.0))
      .min_width(Dimension::Px(0.0))
      .justify(justify(self.align))
      .align_items(Alignment::Center)
      .child(label);
    match trigger {
      Some(trigger) => {
        cell = cell.child(
          Row::new()
            .flex_full(1.0, 1.0, Some(0.0))
            .min_width(Dimension::Px(0.0))
            .align_items(Alignment::Center)
            .spacing(4.0)
            .child(titled)
            .child(trigger),
        );
      }
      None => cell = cell.child(titled),
    }
    if let Some(resizer) = resizer {
      cell = cell.child(resizer);
    }
    if let Some(menu) = menu {
      cell = cell.child(menu);
    }
    cell.into()
  }
}

/// A column's resizer and its table's host: what a drag or a key makes the column, and who is told.
struct ResizeLink {
  /// What keeps the width a person made the column in a grid's arrangement of its columns.
  layout: Option<Arc<dyn Fn(&str, f32) + Send + Sync>>,
  host: Arc<HostLink>,
  column: usize,
  id: Arc<str>,
  /// The least and the most a person makes the column.
  least: f32,
  most: f32,
  /// The room of its track that is not the column's own: the gap after it, and the row's padding it holds.
  extra: f32,
  /// The column's width when the drag started.
  start: Mutex<f32>,
  on_resize: Vec<ResizeHeard>,
  on_resize_end: Vec<ResizeHeard>,
  label: String,
}

impl ResizeLink {
  /// The column's width now: its track's, less what of it is not the column's own.
  fn width(&self) -> f32 {
    (self.host.track_width(self.column) - self.extra).max(0.0)
  }

  fn begin(&self) {
    *self.start.lock() = self.width();
  }

  fn drag(&self, by: f32) {
    let start = *self.start.lock();
    self.set(start + by);
  }

  fn finish(&self) {
    let width = self.width();
    if (width - *self.start.lock()).abs() > 0.01 {
      self.keep(width);
      for handler in &self.on_resize_end {
        handler(&self.id, width);
      }
    }
  }

  /// The width a person made the column, kept in the grid's arrangement of its columns, which is then what it
  /// is drawn at.
  fn keep(&self, width: f32) {
    if let Some(layout) = &self.layout {
      layout(&self.id, width);
      self.host.state.resized.lock().remove(&self.id);
    }
  }

  /// A key: narrower or wider by `by`, and that is the end of it.
  fn step(&self, by: f32) {
    let before = self.width();
    let after = self.set(before + by);
    if (after - before).abs() > 0.01 {
      self.keep(after);
      for handler in &self.on_resize_end {
        handler(&self.id, after);
      }
    }
  }

  /// Makes the column `width`, between its least and its most: its width now.
  fn set(&self, width: f32) -> f32 {
    let width = width.clamp(self.least, self.most.max(self.least));
    if (width - self.width()).abs() > 0.01 && self.host.resize(&self.id, width) {
      for handler in &self.on_resize {
        handler(&self.id, width);
      }
    }
    width
  }
}

fn justify(align: ColumnAlign) -> Justify {
  match align {
    ColumnAlign::Start => Justify::Start,
    ColumnAlign::Center => Justify::Center,
    ColumnAlign::End => Justify::End,
  }
}

/// One cell of a row as a renderer sees it.
pub struct TableCell {
  /// Which column it is of, from 0.
  pub column: usize,
  /// What the column built from the row.
  pub content: Element,
  /// What it is and does in a data grid: `None` in a table.
  pub grid: Option<GridCell>,
  /// The style of the cell, in the case of its column and its row.
  pub part: ResolvedPart,
  pub align: ColumnAlign,
  /// Where it stays while the rows scroll sideways, if its column is pinned.
  pub pinned: Option<Sticky>,
  /// Whether a press in it is its own, not the row's.
  pub controls: bool,
  /// Whether it is the last of the row, with no room after it.
  pub last: bool,
  /// The fill of a cell that stays: the row's, over which the others scroll.
  pub fill: Option<BackgroundColor>,
  gap: f32,
  lead: f32,
  trail: f32,
}

impl TableCell {
  /// How the default renderer builds a cell: its content where its column has it, in the room of its
  /// track. What takes the focus in it is out of the arrow keys of the rows: Tab reaches it from its row.
  pub fn render(mut self) -> Element {
    let mut cell = Row::new()
      .align_items(Alignment::Center)
      .justify(justify(self.align))
      .part(&self.part)
      // A cell is the same element wherever the columns built around it start.
      .key(format!("cell-{}", self.column));
    match self.grid.take() {
      // A cell of a grid is a stop of the grid's keys, and holds its editor while it is edited.
      Some(mut grid) => {
        let editor = grid.editor.take();
        cell = grid
          .dress(self.column, cell, editor.is_some())
          .child(editor.unwrap_or(self.content));
      }
      None => {
        cell = cell
          .describe("a11y_role", "cell")
          .focus_group(FocusGroup::none())
          .child(self.content)
      }
    }
    if !self.last {
      cell = cell.padding_right(self.gap);
    }
    if self.trail > 0.0 {
      cell = cell.padding_right(self.trail);
    }
    if self.lead > 0.0 {
      cell = cell.padding_left(self.lead);
    }
    if let Some(pinned) = self.pinned {
      cell = cell.sticky(pinned);
    }
    if let Some(fill) = self.fill {
      cell = cell.background(fill);
    }
    if self.controls {
      cell = cell.on_click(|event: MouseEvent| event.stop_propagation());
    }
    cell.into()
  }
}

/// One row as a renderer sees it: where it is, whether it is selected, its cells, the style of its parts,
/// and its behavior ready to be dressed.
pub struct TableRow {
  /// Its place among the rows, from 0.
  pub index: usize,
  /// Its line among the lines shown, the groups' rows counted: its place where the rows are not grouped.
  pub position: usize,
  pub selected: bool,
  pub disabled: bool,
  /// What it is called to what reads the tree: the text of its first column, where that is text.
  pub name: Option<Arc<str>>,
  /// Its cells, a column's each.
  pub cells: Vec<TableCell>,
  /// Whether it has a checkbox before its cells.
  pub checkbox: bool,
  /// Whether the cells at its start stay while the rows scroll sideways.
  pub pinned_start: bool,
  /// Whether the cells at its end stay.
  pub pinned_end: bool,
  /// The fill of the cells that stay, the row's, over which the others scroll.
  pub lead_fill: Option<BackgroundColor>,
  /// Whether Tab enters the rows at it.
  pub entry: bool,
  /// The style of each part of this row, in its case.
  pub recipe: ResolvedRecipe,
  forced: Option<Interaction>,
  tracks: ColumnTracks,
  columns_built: Option<Arc<Vec<bool>>>,
  /// Whether its cells take the focus, and not the row: a row of a data grid.
  grid: bool,
  geometry: Geometry,
  actions: Option<Arc<RowActions>>,
  /// The name of its checkbox.
  select_label: Arc<str>,
  host: Option<Arc<HostLink>>,
  element: ElementRef,
  id: Option<Arc<str>>,
}

impl TableRow {
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part).forced(self.forced)
  }

  /// The row's behavior with the style of its part, its cells in the tracks of the columns: the element that
  /// is pressed, selected and focused, with nothing in it yet. It is a row to what reads the tree, at its
  /// place (the header's is 1), selected or not where the table selects.
  pub fn button(&self) -> components::Button {
    let mut row = pressable(&self.part(TablePart::Row), self.disabled)
      .justify(Justify::Start)
      // Each cell is as high as the row: a first column that stays covers all of what scrolls under it, not only
      // the line of its text. A cell sets what it holds in the middle of that height.
      .align_items(Alignment::Stretch)
      .spacing(0.0)
      .column_tracks(&self.tracks)
      // The cells that stay cover the row's veil: they take it from the row under the pointer and held down.
      .interaction_group(true)
      .stops_inside(true)
      .ref_element(self.element.clone())
      .describe("a11y_role", "row")
      .describe("a11y_rowindex", (self.position + 2).to_string())
      // A row is the same element wherever the rows built around it start.
      .key(format!("row-{}", self.index));
    if self.grid {
      // The cells are the stops of a grid's keys, Left and Right a group of the row's own.
      row = row.focusable(false).focus_group(FocusGroup::horizontal().wrap(false));
    } else {
      row = row.focus_entry(self.entry);
      if self.host.is_some() {
        row = row.focus_row(self.position);
      }
    }
    if self.pinned_start {
      row = row.padding_left(0.0);
    }
    if self.pinned_end {
      row = row.padding_right(0.0);
    }
    if let Some(name) = &self.name {
      row = row.describe("a11y_name", name.clone()).focus_label(name.clone());
    }
    if let Some(actions) = &self.actions
      && actions.selectable
    {
      row = row.describe("a11y_selected", if self.selected { "true" } else { "false" });
    }
    if let Some(actions) = self.actions.clone() {
      // A press selects at once, and a second press of the row soon after at the same place activates it. The
      // row has no `on_dblclick`: the engine holds back the click of an element that has one, and the focus with
      // it, until it knows it is not the first of a double press.
      let press = actions.clone();
      let index = self.index;
      row = row.on_click(move |event: MouseEvent| {
        if second_press(index, &event) {
          (press.activate)();
          return;
        }
        if press.selectable {
          let pick = if event.shift {
            Pick::Range
          } else if event.ctrl || event.meta || event.from_keyboard {
            // Space adds the focused row or takes it out, as Ctrl with a press does.
            Pick::Toggle
          } else {
            Pick::Only
          };
          (press.pick)(pick);
        }
        (press.click)();
      });
      let element = self.element.clone();
      row = row.on_key_down(move |event: KeyboardEvent| {
        // A key on a button in the row is the button's.
        if !element.focused() || event.composing || event.alt {
          return;
        }
        let command = event.ctrl || event.meta;
        match event.key.as_str() {
          // Enter activates; it does not press the row, which would select it.
          "Enter" if !command && !event.shift => {
            event.prevent_default();
            (actions.activate)();
          }
          "a" | "A" if command && actions.selectable && !event.shift => {
            event.prevent_default();
            (actions.select_all)();
          }
          _ => {}
        }
      });
      if let Some(host) = &self.host {
        let (host, position) = (host.clone(), self.position);
        row = row.on_focus(move || host.focused(position));
      }
    }
    if let Some(id) = &self.id {
      row = row.id(id.clone());
    }
    row
  }

  /// The checkbox of the row, in the cell of the column of checkboxes: a press of it adds the row or takes it
  /// out, with Shift the rows from the last selected one. The checkbox itself takes neither the focus nor the
  /// press: Space on the row is its press.
  pub fn checkbox(&self) -> Element {
    let part = self.part(TablePart::Select);
    let checkbox = Checkbox::new(&self.select_label)
      .label_hidden(true)
      .checked(self.selected)
      .disabled(self.disabled)
      .build(self.recipe.theme());
    let mut cell = lead_cell(&part, self.geometry, self.pinned_start).child(Row::new().inert(true).child(checkbox));
    if self.pinned_start {
      cell = cell.sticky(Sticky::left(0.0));
    }
    if let Some(fill) = self.lead_fill.clone() {
      cell = cell.background(fill);
    }
    if let Some(actions) = self.actions.clone() {
      cell = cell.on_click(move |event: MouseEvent| {
        event.stop_propagation();
        (actions.pick)(if event.shift { Pick::Range } else { Pick::Toggle });
      });
    }
    cell.into()
  }

  /// How the default renderer builds a row: its checkbox, then its cells.
  pub fn render(mut self) -> Element {
    let mut row = self.button();
    if self.checkbox {
      row = row.child(self.checkbox());
    }
    let cells = std::mem::take(&mut self.cells);
    let placed = cells.into_iter().map(|cell| (cell.column, cell.render()));
    row
      .with_children(in_tracks(
        self.columns_built.as_ref().map(|built| built.as_slice()),
        placed,
      ))
      .into()
  }
}

thread_local! {
  /// The last plain press of a row: which row, when and where. A second one of it soon after at the same place,
  /// as the engine counts a double press, activates it.
  static LAST_PRESS: Cell<Option<(usize, Instant, (f32, f32))>> = const { Cell::new(None) };
}

/// Whether a press of the row `index` is the second of a double press: then the next is a first again. A press
/// with Shift, Ctrl or the system key, or from the keyboard, changes the selection and is never one.
fn second_press(index: usize, event: &MouseEvent) -> bool {
  let plain = !(event.from_keyboard || event.shift || event.ctrl || event.meta || event.alt);
  let now = Instant::now();
  let near = |(x, y): (f32, f32)| (x - event.x).powi(2) + (y - event.y).powi(2) <= DOUBLE_CLICK_DISTANCE.powi(2);
  LAST_PRESS.with(|last| {
    let second = plain
      && last
        .get()
        .is_some_and(|(row, at, place)| row == index && now.duration_since(at) <= DOUBLE_CLICK_INTERVAL && near(place));
    last.set((plain && !second).then_some((index, now, (event.x, event.y))));
    second
  })
}

/// What a person typed or chose in a cell of a [`DataGrid`](super::DataGrid), told to the app with the row and
/// the column ([`CellEdit`]).
#[derive(Clone, Debug, PartialEq)]
pub enum CellValue {
  /// A text typed in.
  Text(String),
  /// A number typed in.
  Number(f64),
  /// The place of the choice chosen among the column's choices.
  Choice(usize),
  /// A box ticked, or not.
  Checked(bool),
  /// A number or a choice cleared: Delete on its cell. What it is emptied to is the app's, which may refuse it.
  Empty,
}

/// An edit a person committed in a cell: which row (its key), which column (its id), and the new value. The app
/// that takes it changes its rows, and the grid shows them at the next render.
#[derive(Clone, Debug)]
pub struct CellEdit<'a, K> {
  pub row: &'a K,
  pub column: &'a str,
  pub value: CellValue,
}

type EditHeard<K> = Arc<dyn Fn(&CellEdit<K>) -> bool + Send + Sync>;

/// How the cells of a column of a [`DataGrid`](super::DataGrid) are edited in place, with `ui`'s own control in
/// the cell, and the value a row's cell starts the edit from.
pub struct CellEditor<R> {
  kind: EditorKind<R>,
}

enum EditorKind<R> {
  Text(TextFn<R>),
  Number(Arc<dyn Fn(&R) -> f64 + Send + Sync>),
  Choice(Arc<Vec<Arc<str>>>, Arc<dyn Fn(&R) -> usize + Send + Sync>),
  Checkbox(Arc<dyn Fn(&R) -> bool + Send + Sync>),
}

impl<R> Clone for CellEditor<R> {
  fn clone(&self) -> Self {
    Self {
      kind: match &self.kind {
        EditorKind::Text(text) => EditorKind::Text(text.clone()),
        EditorKind::Number(number) => EditorKind::Number(number.clone()),
        EditorKind::Choice(choices, chosen) => EditorKind::Choice(choices.clone(), chosen.clone()),
        EditorKind::Checkbox(checked) => EditorKind::Checkbox(checked.clone()),
      },
    }
  }
}

impl<R> CellEditor<R> {
  /// A text typed in an input: `ui::Input`, from the row's text.
  pub fn text(value: impl Fn(&R) -> String + Send + Sync + 'static) -> Self {
    Self {
      kind: EditorKind::Text(Arc::new(value)),
    }
  }

  /// A number typed in a number input: `ui::NumberInput`, from the row's number.
  pub fn number(value: impl Fn(&R) -> f64 + Send + Sync + 'static) -> Self {
    Self {
      kind: EditorKind::Number(Arc::new(value)),
    }
  }

  /// One of `choices`, chosen in a select: `ui::Select`, from the place of the row's choice.
  pub fn choice<S: AsRef<str>>(
    choices: impl IntoIterator<Item = S>,
    chosen: impl Fn(&R) -> usize + Send + Sync + 'static,
  ) -> Self {
    Self {
      kind: EditorKind::Choice(
        Arc::new(choices.into_iter().map(|choice| Arc::from(choice.as_ref())).collect()),
        Arc::new(chosen),
      ),
    }
  }

  /// A box ticked or not, in the cell itself: a press of it or Space turns it, with no editor to open. A column
  /// edited with one that builds nothing of its own shows the box.
  pub fn checkbox(checked: impl Fn(&R) -> bool + Send + Sync + 'static) -> Self {
    Self {
      kind: EditorKind::Checkbox(Arc::new(checked)),
    }
  }

  fn is_checkbox(&self) -> bool {
    matches!(self.kind, EditorKind::Checkbox(_))
  }
}

/// Where the focus goes from a cell whose edit was committed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum GridStep {
  /// It stays on the cell.
  Stay,
  /// The cell under it: Enter.
  Down,
  /// The cell after it in the row: Tab.
  On,
  /// The cell before it: Shift+Tab.
  Back,
}

/// The edit going on in a cell: where, and whether the app refused what was committed.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Editing {
  row: usize,
  column: usize,
  refused: bool,
}

/// What the focus is to go to once a render has it: a cell, or the editor of the cell being edited.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FocusAim {
  Cell(usize, usize),
  Editor,
}

/// What a cell of a grid does, with the types of the table's rows and keys out of sight.
trait GridCells: Send + Sync {
  /// The cell `(row, column)` has the focus: an edit going on elsewhere is committed, or put back where it is
  /// refused.
  fn focused(&self, row: usize, column: usize);
  /// Starts editing the cell, from what is typed where a key started it.
  fn start(&self, row: usize, column: usize, typed: Option<String>);
  /// Turns the box of a cell edited with a checkbox.
  fn toggle(&self, row: usize, column: usize);
  /// Commits the edit going on, and moves the focus as `step` says where the app takes it.
  fn commit(&self, step: GridStep);
  /// Puts the edit going on back, and gives the focus back to its cell.
  fn cancel(&self);
  /// Moves the focus to the cell `(row, column)`, where there is one: whether there is.
  fn move_to(&self, row: usize, column: usize) -> bool;
  fn select(&self, row: usize, pick: Pick);
  fn activate(&self, row: usize);
  /// How many rows and columns the grid has.
  fn size(&self) -> (usize, usize);
  /// The row a press of a cell is on: the handlers of a row's press are told.
  fn click(&self, row: usize);
  /// The corner of the range of cells that moves, where there is a range.
  fn extent(&self) -> Option<(usize, usize)>;
  /// The range from its anchor (or `from`, or the focused cell where there is none) to `to`, scrolled into view.
  fn extend(&self, from: Option<(usize, usize)>, to: (usize, usize));
  /// No range: whether there was one.
  fn clear_range(&self) -> bool;
  /// Every cell in the range.
  fn select_cells(&self);
  /// The range's cells, or the one at `at`, as tab-separated text; a cut clears those that are edited.
  fn copy(&self, at: (usize, usize), cut: bool) -> String;
  /// Pastes tab-separated text from the range's first cell, or `at`.
  fn paste(&self, at: (usize, usize), text: &str);
  /// Clears what is edited of the range's cells, or the one at `at`.
  fn clear(&self, at: (usize, usize));
  /// A drag of cells starts at `at`, and is at `(x, y)` now.
  fn drag_start(&self, at: (usize, usize));
  fn drag_to(&self, x: f32, y: f32);
  /// How many rows a page is.
  fn page(&self) -> usize;
}

/// The cells of a grid as its host works them.
struct GridModel<R, K> {
  host: Arc<HostState>,
  /// The range of cells, among the rows and the columns shown, and as the app holds it; what asks for another
  /// and who is told.
  span: Option<Span>,
  range: Option<CellRange<K>>,
  write_range: Option<RangeWrite<K>>,
  on_range: Vec<RangeHeard<K>>,
  /// The text of each column, where it is a text: what a copy takes of a cell that is not edited.
  texts: Arc<Vec<Option<TextFn<R>>>>,
  rows: Arc<Vec<R>>,
  /// The keys of the rows shown, by their ranks, and the lines they are on.
  keys: Arc<Vec<K>>,
  lines: Arc<Lines>,
  /// The id and the editor of each column, where it is edited.
  editors: Arc<Vec<(Arc<str>, Option<CellEditor<R>>)>>,
  on_edit: Vec<EditHeard<K>>,
  model: Arc<Model<K>>,
  read_only: bool,
}

impl<R: Send + Sync + 'static, K: Clone + Eq + Hash + Send + Sync + 'static> GridModel<R, K> {
  fn editor(&self, column: usize) -> Option<&CellEditor<R>> {
    if self.read_only {
      return None;
    }
    self.editors.get(column).and_then(|(_, editor)| editor.as_ref())
  }

  /// Tells the app `value` for the cell: whether every handler took it.
  fn tell(&self, row: usize, column: usize, value: CellValue) -> bool {
    let (Some(key), Some((id, _))) = (self.keys.get(row), self.editors.get(column)) else {
      return false;
    };
    let edit = CellEdit {
      row: key,
      column: id,
      value,
    };
    // Every handler is told, and the edit is taken only where none refused it.
    self.on_edit.iter().fold(true, |taken, handler| handler(&edit) && taken)
  }

  /// The value typed or chosen in the editor of the cell being edited.
  fn drafted(&self, column: usize) -> Option<CellValue> {
    Some(match &self.editor(column)?.kind {
      EditorKind::Text(_) => CellValue::Text(self.host.draft_text.get()),
      EditorKind::Number(_) => CellValue::Number(self.host.draft_number.get()?),
      EditorKind::Choice(..) => CellValue::Choice(self.host.draft_choice.get()),
      EditorKind::Checkbox(_) => return None,
    })
  }

  fn aim(&self, aim: FocusAim) {
    *self.host.pending_focus.lock() = Some(aim);
    if let FocusAim::Cell(row, column) = aim {
      *self.host.focused.lock() = Some(self.position(row));
      *self.host.focused_column.lock() = Some(column);
    }
    self.host.rebuild();
  }

  /// The row shown at `rank`.
  fn row(&self, rank: usize) -> Option<&R> {
    self.rows.get(*self.lines.order.get(rank)?)
  }

  /// The line of the row shown at `rank`.
  fn position(&self, rank: usize) -> usize {
    self.lines.position_of_rank.get(rank).copied().unwrap_or(rank)
  }

  /// The cell that has the focus, where it is a cell: its rank and its column.
  fn focused_cell(&self) -> Option<(usize, usize)> {
    let position = (*self.host.focused.lock())?;
    Some((self.lines.rank_at(position)?, (*self.host.focused_column.lock())?))
  }
}

impl<R: Send + Sync + 'static, K: Clone + Eq + Hash + Send + Sync + 'static> GridCells for GridModel<R, K> {
  fn focused(&self, row: usize, column: usize) {
    let position = self.position(row);
    let before = (*self.host.focused.lock(), *self.host.focused_column.lock());
    *self.host.focused.lock() = Some(position);
    *self.host.focused_column.lock() = Some(column);
    // The focus moved to another cell: the range is gone, and the cell is where the next one starts.
    if before != (Some(position), Some(column)) {
      self.write_span(None);
    }
    let editing = *self.host.editing.lock();
    if let Some(editing) = editing
      && (editing.row, editing.column) != (row, column)
    {
      // The focus left the cell being edited for another: what was typed is committed, or put back.
      if let Some(value) = self.drafted(editing.column) {
        self.tell(editing.row, editing.column, value);
      }
      *self.host.editing.lock() = None;
      self.host.rebuild();
    }
  }

  fn start(&self, row: usize, column: usize, typed: Option<String>) {
    let (Some(editor), Some(values)) = (self.editor(column), self.row(row)) else {
      return;
    };
    match &editor.kind {
      EditorKind::Text(text) => self.host.draft_text.set(typed.unwrap_or_else(|| text(values))),
      EditorKind::Number(number) => self.host.draft_number.set(
        typed
          .and_then(|typed| typed.parse::<f64>().ok())
          .or(Some(number(values))),
      ),
      EditorKind::Choice(_, chosen) => self.host.draft_choice.set(chosen(values)),
      EditorKind::Checkbox(_) => {
        self.toggle(row, column);
        return;
      }
    }
    *self.host.editing.lock() = Some(Editing {
      row,
      column,
      refused: false,
    });
    self.aim(FocusAim::Editor);
  }

  fn toggle(&self, row: usize, column: usize) {
    let (Some(editor), Some(values)) = (self.editor(column), self.row(row)) else {
      return;
    };
    if let EditorKind::Checkbox(checked) = &editor.kind {
      self.tell(row, column, CellValue::Checked(!checked(values)));
    }
  }

  fn commit(&self, step: GridStep) {
    let Some(editing) = *self.host.editing.lock() else {
      return;
    };
    let Some(value) = self.drafted(editing.column) else {
      return;
    };
    if !self.tell(editing.row, editing.column, value) {
      // Refused: the editor stays, said to be wrong, and keeps the focus.
      *self.host.editing.lock() = Some(Editing {
        refused: true,
        ..editing
      });
      self.aim(FocusAim::Editor);
      return;
    }
    *self.host.editing.lock() = None;
    let (row, column) = (editing.row, editing.column);
    let columns = self.editors.len();
    let target = match step {
      GridStep::Stay => (row, column),
      GridStep::Down if row + 1 < self.keys.len() => (row + 1, column),
      GridStep::On if column + 1 < columns => (row, column + 1),
      GridStep::Back if column > 0 => (row, column - 1),
      _ => (row, column),
    };
    self.aim(FocusAim::Cell(target.0, target.1));
  }

  fn cancel(&self) {
    let Some(editing) = self.host.editing.lock().take() else {
      return;
    };
    self.aim(FocusAim::Cell(editing.row, editing.column));
  }

  fn move_to(&self, row: usize, column: usize) -> bool {
    if row >= self.keys.len() || column >= self.editors.len() {
      return false;
    }
    self.aim(FocusAim::Cell(row, column));
    true
  }

  fn select(&self, row: usize, pick: Pick) {
    if let Some(index) = self.lines.order.get(row) {
      self.model.pick(*index, pick);
    }
  }

  fn activate(&self, row: usize) {
    if let Some(key) = self.keys.get(row) {
      for handler in &self.model.on_row_activate {
        handler(key);
      }
    }
  }

  fn size(&self) -> (usize, usize) {
    (self.keys.len(), self.editors.len())
  }

  fn click(&self, row: usize) {
    if let Some(key) = self.keys.get(row) {
      for handler in &self.model.on_row_click {
        handler(key);
      }
    }
  }

  fn extent(&self) -> Option<(usize, usize)> {
    self.span.map(|span| span.extent)
  }

  fn extend(&self, from: Option<(usize, usize)>, to: (usize, usize)) {
    let anchor = self
      .span
      .map(|span| span.anchor)
      .or(from)
      .or_else(|| self.focused_cell())
      .unwrap_or(to);
    let span = Span { anchor, extent: to };
    // A range of one cell is the focused cell alone.
    self.write_span((anchor != to).then_some(span));
    self.host.reveal_cell(self.position(to.0), to.1);
  }

  fn clear_range(&self) -> bool {
    let had = self.span.is_some();
    self.write_span(None);
    had
  }

  fn select_cells(&self) {
    let (rows, columns) = self.size();
    if rows == 0 || columns == 0 {
      return;
    }
    let anchor = self.focused_cell().unwrap_or((0, 0));
    // The focus stays where it is, at a corner of the range.
    let extent = (
      if anchor.0 == 0 { rows - 1 } else { 0 },
      if anchor.1 == 0 { columns - 1 } else { 0 },
    );
    let span = if anchor.0 == 0 || anchor.0 == rows - 1 {
      Span { anchor, extent }
    } else {
      Span {
        anchor: (0, 0),
        extent: (rows - 1, columns - 1),
      }
    };
    self.write_span(Some(span));
  }

  fn copy(&self, at: (usize, usize), cut: bool) -> String {
    self.copy_cells(at, cut)
  }

  fn paste(&self, at: (usize, usize), text: &str) {
    self.paste_cells(at, text);
  }

  fn clear(&self, at: (usize, usize)) {
    self.clear_cells(at);
  }

  fn drag_start(&self, at: (usize, usize)) {
    *self.host.drag_anchor.lock() = Some(at);
    *self.host.drag_last.lock() = Some(at);
    // A drag has no click to focus the cell it started at: the focus goes there, where the range starts.
    self.aim(FocusAim::Cell(at.0, at.1));
  }

  fn drag_to(&self, x: f32, y: f32) {
    self.host.scroll_toward(x, y);
    let Some(anchor) = *self.host.drag_anchor.lock() else {
      return;
    };
    let Some(cell) = self.host.cell_at(x, y) else {
      return;
    };
    // A drag's handlers may be an earlier render's: what it last asked for is the host's.
    if self.host.drag_last.lock().replace(cell) == Some(cell) {
      return;
    }
    self.write_span_now((anchor != cell).then_some(Span { anchor, extent: cell }));
  }

  fn page(&self) -> usize {
    FocusRows::page(&*self.host)
  }
}

/// What a cell of a grid is and does, as a renderer sees it beside the cell's own: where it is, whether it is
/// edited or being edited, and what its keys and presses do.
pub struct GridCell {
  /// The row it is in: its place among the rows shown, from 0.
  pub row: usize,
  /// The row's line among the lines shown, the groups' rows counted.
  pub position: usize,
  /// Whether a person edits it: its column has an editor and the grid is not read only.
  pub editable: bool,
  /// Whether it is in the range of cells selected.
  pub selected: bool,
  /// The editor in it while it is being edited, in place of its content.
  pub editor: Option<Element>,
  /// Whether it is the cell Tab enters the grid at.
  pub entry: bool,
  /// Its place to what reads the tree: its column's among the grid's, the checkboxes' counted, from 1.
  pub column_index: usize,
  checkbox: bool,
  element: ElementRef,
  grid: Arc<dyn GridCells>,
  id: Option<Arc<str>>,
}

thread_local! {
  /// The last plain press of a cell of a grid: which, when and where. A second one of it soon after at the
  /// same place edits it.
  static LAST_CELL_PRESS: Cell<Option<((usize, usize), Instant, (f32, f32))>> = const { Cell::new(None) };
}

fn second_cell_press(cell: (usize, usize), event: &MouseEvent) -> bool {
  let plain = !(event.from_keyboard || event.shift || event.ctrl || event.meta || event.alt);
  let now = Instant::now();
  let near = |(x, y): (f32, f32)| (x - event.x).powi(2) + (y - event.y).powi(2) <= DOUBLE_CLICK_DISTANCE.powi(2);
  LAST_CELL_PRESS.with(|last| {
    let second = plain
      && last.get().is_some_and(|(at, when, place)| {
        at == cell && now.duration_since(when) <= DOUBLE_CLICK_INTERVAL && near(place)
      });
    last.set((plain && !second).then_some((cell, now, (event.x, event.y))));
    second
  })
}

impl GridCell {
  /// The cell's behavior on the element of the cell: a stop of the grid's keys, its row and its column said to
  /// them, its keys, and a double press that edits it.
  fn dress(&self, column: usize, cell: Row, editing: bool) -> Row {
    let (row, grid, element) = (self.row, self.grid.clone(), self.element.clone());
    let editable = self.editable;
    let checkbox = self.checkbox;
    let mut cell = cell
      .tab_index(0)
      .focus_row(self.position)
      .focus_column(column)
      .focus_entry(self.entry)
      .ref_element(self.element.clone())
      .describe("a11y_role", "gridcell")
      .describe("a11y_colindex", self.column_index.to_string())
      .describe("a11y_readonly", if editable { "false" } else { "true" })
      .describe("a11y_rowindex", (self.position + 2).to_string())
      .on_focus({
        let grid = grid.clone();
        move || grid.focused(row, column)
      })
      .on_click({
        let grid = grid.clone();
        move |event: MouseEvent| {
          // A press of a cell is the cell's: it does not select the row, as its checkbox and Space do.
          event.stop_propagation();
          if event.shift {
            // The press took the range to the cell: the focus stays where the range started.
            event.prevent_default();
            return;
          }
          if event.ctrl || event.meta || event.alt {
            return;
          }
          // A second press edits a cell that is edited, and activates the row of one that is not.
          if second_cell_press((row, column), &event) {
            if editable {
              grid.start(row, column, None);
            } else {
              grid.activate(row);
            }
            return;
          }
          grid.click(row);
        }
      });
    if self.selected {
      cell = cell.describe("a11y_selected", "true");
    }
    if let Some(id) = &self.id {
      cell = cell.id(id.clone());
    }
    // A cell being edited leaves its keys, its presses and its clipboard to its editor.
    if editing {
      return cell;
    }
    cell = cell
      // A press with Shift takes the range to the cell and leaves the focus where the range started; a press
      // without it starts again from the cell.
      .on_mouse_down({
        let grid = grid.clone();
        move |event: MouseEvent| {
          if event.button != MouseButton::Left {
            return;
          }
          if event.shift {
            event.prevent_default();
            grid.extend(None, (row, column));
          } else {
            grid.clear_range();
          }
        }
      })
      // A drag from a cell selects the cells from it to the one under the pointer.
      .start_drag_buttons(MouseButtonMask::LEFT)
      .on_drag_start({
        let grid = grid.clone();
        move |_: DragEvent| grid.drag_start((row, column))
      })
      .on_drag_move({
        let grid = grid.clone();
        move |event: DragEvent| grid.drag_to(event.x, event.y)
      })
      .on_copy({
        let grid = grid.clone();
        move |event: CopyEvent| event.set_text(grid.copy((row, column), event.is_cut()))
      })
      .on_paste({
        let grid = grid.clone();
        move |event: PasteEvent| {
          if let Some(text) = event.text() {
            grid.paste((row, column), text);
          }
        }
      });
    cell.on_key_down(move |event: KeyboardEvent| {
      // A key in the cell is the cell's only where the cell has the focus itself, not its editor.
      if !element.focused() || event.composing || event.alt {
        return;
      }
      let command = event.ctrl || event.meta;
      let (rows, columns) = grid.size();
      let (last_row, last_column) = (rows.saturating_sub(1), columns.saturating_sub(1));
      // Shift with a key that moves takes the range's moving corner there, and leaves the focus where it is.
      let (at_row, at_column) = grid.extent().unwrap_or((row, column));
      let reach = match event.key.as_str() {
        _ if !event.shift => None,
        "ArrowUp" if !command => Some((at_row.saturating_sub(1), at_column)),
        "ArrowDown" if !command => Some(((at_row + 1).min(last_row), at_column)),
        "ArrowLeft" if !command => Some((at_row, at_column.saturating_sub(1))),
        "ArrowRight" if !command => Some((at_row, (at_column + 1).min(last_column))),
        "PageUp" if !command => Some((at_row.saturating_sub(grid.page()), at_column)),
        "PageDown" if !command => Some(((at_row + grid.page()).min(last_row), at_column)),
        "Home" if command => Some((0, 0)),
        "End" if command => Some((last_row, last_column)),
        "Home" => Some((at_row, 0)),
        "End" => Some((at_row, last_column)),
        _ => None,
      };
      if let Some(to) = reach {
        event.prevent_default();
        grid.extend(Some((row, column)), to);
        return;
      }
      match event.key.as_str() {
        "Enter" | "F2" if !command && editable && !checkbox => {
          event.prevent_default();
          grid.start(row, column, None);
        }
        "Enter" if !command => {
          event.prevent_default();
          grid.activate(row);
        }
        " " if !command && checkbox && editable => {
          event.prevent_default();
          grid.toggle(row, column);
        }
        " " if !command => {
          event.prevent_default();
          grid.select(row, if event.shift { Pick::Range } else { Pick::Toggle });
        }
        "Tab" if !command => {
          let to = if event.shift {
            column.checked_sub(1)
          } else {
            Some(column + 1)
          };
          if to.is_some_and(|to| grid.move_to(row, to)) {
            event.prevent_default();
          }
        }
        // The ends of the row, and with Ctrl of the grid: cells that may not be built, as the columns and the
        // rows out of view are not.
        "Home" | "End" => {
          event.prevent_default();
          let end = event.key == "End";
          let to_row = match (command, end) {
            (true, true) => last_row,
            (true, false) => 0,
            (false, _) => row,
          };
          grid.move_to(to_row, if end { last_column } else { 0 });
        }
        // Every cell: the rows are selected with the header's checkbox.
        "a" | "A" if command && !event.shift => {
          event.prevent_default();
          grid.select_cells();
        }
        // Delete clears what is edited of the range, or of the cell; Shift+Delete is a cut, the engine's.
        "Delete" | "Backspace" if !command && !event.shift => {
          event.prevent_default();
          grid.clear((row, column));
        }
        "Escape" if !command => {
          if grid.clear_range() {
            event.prevent_default();
          }
        }
        // What is typed in an editable cell starts its edit with it.
        key if !command && editable && !checkbox && key.chars().count() == 1 && !key.trim().is_empty() => {
          event.prevent_default();
          grid.start(row, column, Some(key.to_owned()));
        }
        _ => {}
      }
    })
  }
}

/// The editor of the cell being edited, in its part: its keys commit it and move on, or put it back.
fn editor_in(part: &ResolvedPart, editor: Element, choice: bool, grid: Arc<dyn GridCells>, element: ElementRef) -> Row {
  Row::new()
    .part(part)
    .align_items(Alignment::Center)
    .width(Dimension::Pct(100.0))
    .ref_element(element)
    .on_key_down(move |event: KeyboardEvent| {
      if !event.focus_within || event.composing {
        return;
      }
      match event.key.as_str() {
        // A select keeps Enter for its menu: its choice commits it.
        "Enter" if !choice && !event.shift && !event.alt => {
          event.prevent_default();
          grid.commit(GridStep::Down);
        }
        "Tab" if !event.alt => {
          event.prevent_default();
          grid.commit(if event.shift { GridStep::Back } else { GridStep::On });
        }
        "Escape" => {
          event.prevent_default();
          grid.cancel();
        }
        _ => {}
      }
    })
    .child(editor)
}

/// What a mounted table's rows tell its host: where the focus is.
struct HostLink {
  state: Arc<HostState>,
}

impl HostLink {
  fn focused(&self, index: usize) {
    *self.state.focused.lock() = Some(index);
  }

  /// How wide the track of the column `column` is now.
  fn track_width(&self, column: usize) -> f32 {
    let metrics = *self.state.metrics.lock();
    let Some(tracks) = self.state.tracks.lock().as_ref().map(|(_, tracks)| tracks.clone()) else {
      return 0.0;
    };
    let viewport = self.state.scroll.viewport_width();
    let available = (viewport > 0.0).then(|| (viewport - metrics.padding_start - metrics.padding_end).max(0.0));
    tracks
      .widths(available, 0.0)
      .get(column + metrics.lead_tracks)
      .copied()
      .unwrap_or(0.0)
  }

  /// Keeps the column of `id` at `width`, and renders the table again: whether that is another width.
  fn resize(&self, id: &Arc<str>, width: f32) -> bool {
    let mut resized = self.state.resized.lock();
    if resized.get(id).is_some_and(|held| (held - width).abs() < 0.01) {
      return false;
    }
    resized.insert(id.clone(), width);
    drop(resized);
    self.state.rebuild();
    true
  }
}

/// The rows of a mounted table as the engine's focus group steps over them: Up and Down, Home and End, PageUp
/// and PageDown go to any row, and one that is not built is scrolled to and built, and focused once it is.
impl FocusRows for HostState {
  fn count(&self) -> usize {
    self.metrics.lock().count
  }

  fn label(&self, _row: usize) -> Option<String> {
    None
  }

  fn enabled(&self, _row: usize) -> bool {
    !self.metrics.lock().disabled
  }

  fn level(&self, row: usize) -> u32 {
    self.lines.lock().level(row)
  }

  fn expanded(&self, row: usize) -> Option<bool> {
    let lines = self.lines.lock();
    match lines.lines.get(row)? {
      Shown::Group(group) => Some(lines.groups[*group].open),
      Shown::Row(_) => None,
    }
  }

  fn set_expanded(&self, row: usize, open: bool) {
    let group = match self.lines.lock().lines.get(row) {
      Some(Shown::Group(group)) => *group,
      _ => return,
    };
    let toggle = self.group_toggle.lock().clone();
    if let Some(toggle) = toggle {
      toggle(group, open);
    }
  }

  /// The rows the view holds under the header, less one, so that the row a page goes from stays in view.
  fn page(&self) -> usize {
    let metrics = *self.metrics.lock();
    if metrics.row_height <= 0.0 {
      return 1;
    }
    ((self.scroll.viewport_height() - metrics.header_height) / metrics.row_height)
      .floor()
      .max(2.0) as usize
      - 1
  }

  fn reveal(&self, row: usize) {
    let metrics = *self.metrics.lock();
    if row >= metrics.count || metrics.row_height <= 0.0 {
      return;
    }
    // The row is scrolled to where it shows under the header, and kept built as the focused row is.
    let top = row as f32 * metrics.row_height;
    let room = (self.scroll.viewport_height() - metrics.header_height).max(metrics.row_height);
    let scroll_y = self.scroll.scroll_y();
    let y = if top < scroll_y {
      top
    } else if top + metrics.row_height > scroll_y + room {
      top + metrics.row_height - room
    } else {
      scroll_y
    };
    self.scroll.set_scroll(self.scroll.scroll_x(), y.max(0.0));
    *self.focused.lock() = Some(row);
    self.rebuild();
  }
}

/// The sizes a mounted table's window is reckoned by.
#[derive(Clone, Copy, Debug, Default)]
struct Metrics {
  row_height: f32,
  header_height: f32,
  count: usize,
  disabled: bool,
  /// The room before the first track of a row, and after the last.
  padding_start: f32,
  padding_end: f32,
  /// The tracks before the first column's: the checkboxes'.
  lead_tracks: usize,
  /// The columns that stay at the start and at the end, built wherever the rows are scrolled.
  pins: (usize, usize),
}

/// Rows built past each edge of what is in view.
const OVERSCAN: usize = 8;
/// Columns built past each side of what is in view.
const OVERSCAN_COLUMNS: usize = 2;
/// Rows built before the table knows how high it is.
const FIRST_ROWS: usize = 40;

/// What a mounted table keeps from render to render.
struct HostState {
  scroll: ScrollState,
  /// The tracks of the columns, kept while the columns are the same, with what they measured.
  tracks: Mutex<Option<(Vec<TrackSpec>, ColumnTracks)>>,
  /// The places of the rows built.
  built: Mutex<Vec<usize>>,
  /// Which columns are built, where the columns in view only are.
  columns_built: Mutex<Option<Arc<Vec<bool>>>>,
  /// The column that has, or last had, the focus: built wherever the rows are scrolled.
  focused_column: Mutex<Option<usize>>,
  /// The widths a person made the columns, by their ids.
  resized: Mutex<HashMap<Arc<str>, f32>>,
  /// What is typed or chosen in the editor of the cell being edited, and which cell that is.
  draft_text: Signal<String>,
  draft_number: Signal<Option<f64>>,
  draft_choice: Signal<usize>,
  editing: Mutex<Option<Editing>>,
  /// Where the focus is to go once a render has it.
  pending_focus: Mutex<Option<FocusAim>>,
  /// The refs of the cells built at the last render, by row and column, which a drag of cells finds its cell by.
  cell_refs: Mutex<HashMap<(usize, usize), ElementRef>>,
  /// What scrolls, where a drag of cells past its edge scrolls it.
  scroll_ref: ElementRef,
  /// The cell a drag of cells started at, and the cell it last reached.
  drag_anchor: Mutex<Option<(usize, usize)>>,
  drag_last: Mutex<Option<(usize, usize)>>,
  /// The column whose menu is open, what the menu hangs from, and its list of columns.
  menu_open: Mutex<Option<Arc<str>>>,
  menu_anchor: ElementRef,
  submenu_open: Signal<bool>,
  submenu_anchor: ElementRef,
  /// Where each header built is, by its column, and a column dragged by its header: from where to before which.
  header_refs: Mutex<HashMap<usize, ElementRef>>,
  header_drop: Mutex<Option<(usize, Option<usize>)>>,
  /// The lines shown, and what opens or closes the group of one: the engine's nested group asks it.
  lines: Mutex<Arc<Lines>>,
  group_toggle: Mutex<Option<Arc<dyn Fn(usize, bool) + Send + Sync>>>,
  metrics: Mutex<Metrics>,
  /// The row that has, or last had, the focus, or that a key goes to.
  focused: Mutex<Option<usize>>,
  revision: Signal<u64>,
}

impl HostState {
  /// The rows in view and `margin` rows past each edge, from the scroll as it is.
  fn range(&self, margin: usize) -> Option<(usize, usize)> {
    let metrics = *self.metrics.lock();
    let viewport = self.scroll.viewport_height();
    if metrics.row_height <= 0.0 || viewport <= 0.0 {
      return None;
    }
    let scroll_y = self.scroll.scroll_y();
    let first = (scroll_y / metrics.row_height).floor().max(0.0) as usize;
    let last = ((scroll_y + viewport) / metrics.row_height).ceil().max(0.0) as usize;
    Some((first.saturating_sub(margin), (last + margin).min(metrics.count)))
  }

  /// Whether the rows that are to be built, those in view and half the overscan around them, are not all
  /// built, or rows far from them are (the first rows built before the table knew its height): the table is
  /// rendered again with the rows around what is in view.
  fn needs_rows(&self) -> bool {
    let (Some((first, last)), Some((far_first, far_last))) = (self.range(OVERSCAN / 2), self.range(OVERSCAN * 2))
    else {
      return false;
    };
    let focused = *self.focused.lock();
    let built = self.built.lock();
    (first..last).any(|index| !built.contains(&index))
      || built
        .iter()
        .any(|index| (*index < far_first || *index >= far_last) && Some(*index) != focused)
  }

  fn rebuild(&self) {
    self.revision.update(|revision| *revision = revision.wrapping_add(1));
  }

  /// Which columns are in view at `scroll_x`, with [`OVERSCAN_COLUMNS`] past each side, the pinned ones and the
  /// focused one: `None` while the table does not know its width, or where every column is in view anyway.
  fn columns_at(&self, scroll_x: f32) -> Option<Vec<bool>> {
    let metrics = *self.metrics.lock();
    let tracks = self.tracks.lock().as_ref()?.1.clone();
    let viewport = self.scroll.viewport_width();
    if viewport <= 0.0 {
      return None;
    }
    let available = (viewport - metrics.padding_start - metrics.padding_end).max(0.0);
    let widths = tracks.widths(Some(available), 0.0);
    let columns = widths.len().saturating_sub(metrics.lead_tracks);
    let mut x = metrics.padding_start;
    let mut seen = vec![false; columns];
    for (track, width) in widths.iter().enumerate() {
      if let Some(column) = track.checked_sub(metrics.lead_tracks) {
        seen[column] = x + width >= scroll_x && x <= scroll_x + viewport;
      }
      x += width;
    }
    if seen.iter().all(|seen| *seen) {
      return None;
    }
    let mut built = seen.clone();
    for (column, seen) in seen.iter().enumerate() {
      if *seen {
        let (from, to) = (
          column.saturating_sub(OVERSCAN_COLUMNS),
          (column + OVERSCAN_COLUMNS + 1).min(columns),
        );
        built[from..to].iter_mut().for_each(|built| *built = true);
      }
    }
    let (start, end) = metrics.pins;
    built[..start.min(columns)].iter_mut().for_each(|built| *built = true);
    built[columns.saturating_sub(end)..]
      .iter_mut()
      .for_each(|built| *built = true);
    if let Some(column) = *self.focused_column.lock()
      && column < columns
    {
      built[column] = true;
    }
    Some(built)
  }

  /// Whether a column in view at `scroll_x` is not built, or the table built only some columns and now sees
  /// them all: it is rendered again.
  fn needs_columns(&self, scroll_x: f32) -> bool {
    let wanted = self.columns_at(scroll_x);
    let built = self.columns_built.lock().clone();
    match (wanted, built) {
      (None, None) => false,
      (None, Some(built)) => built.iter().any(|built| !built),
      (Some(_), None) => false,
      (Some(wanted), Some(built)) => {
        // Only a column that came into view: one that left keeps its cells until the next render.
        wanted
          .iter()
          .zip(built.iter())
          .enumerate()
          .any(|(column, (wanted, built))| *wanted && !built && self.seen_at(column, scroll_x))
      }
    }
  }

  /// Whether the column `column` is in view at `scroll_x`.
  fn seen_at(&self, column: usize, scroll_x: f32) -> bool {
    let metrics = *self.metrics.lock();
    let Some(tracks) = self.tracks.lock().as_ref().map(|(_, tracks)| tracks.clone()) else {
      return true;
    };
    let viewport = self.scroll.viewport_width();
    let available = (viewport - metrics.padding_start - metrics.padding_end).max(0.0);
    let widths = tracks.widths(Some(available), 0.0);
    let track = column + metrics.lead_tracks;
    let x = metrics.padding_start + widths[..track.min(widths.len())].iter().sum::<f32>();
    let width = widths.get(track).copied().unwrap_or(0.0);
    x + width >= scroll_x && x <= scroll_x + viewport
  }
}

struct TableProps<R, K> {
  table: Arc<Mutex<Table<R, K>>>,
}

impl<R, K> PartialEq for TableProps<R, K> {
  fn eq(&self, _: &Self) -> bool {
    false
  }
}

#[cfg(feature = "devtools")]
impl<R, K> crate::app::component::DevtoolsInspectable for TableProps<R, K> {}

/// What keeps a mounted table: the scroll of its rows, the widths of its columns, which rows are built.
struct TableHost<R, K> {
  state: Arc<HostState>,
  /// The range of cells of a grid whose app holds none.
  own_range: Arc<Mutex<Option<CellRange<K>>>>,
  /// The arrangement of a grid's columns whose app holds none.
  own_layout: Arc<Mutex<Option<ColumnLayout>>>,
  /// The groups open of a table whose app holds none.
  own_open: Arc<Mutex<Vec<GroupKey>>>,
  _marker: PhantomData<fn() -> R>,
}

impl<R: Send + Sync + 'static, K: Clone + Eq + Hash + Send + Sync + 'static> Component for TableHost<R, K> {
  type Props = TableProps<R, K>;

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      state: Arc::new(HostState {
        scroll: ScrollState::new(),
        tracks: Mutex::new(None),
        built: Mutex::new(Vec::new()),
        columns_built: Mutex::new(None),
        focused_column: Mutex::new(None),
        resized: Mutex::new(HashMap::new()),
        draft_text: ctx.signal(String::new()),
        draft_number: ctx.signal(None),
        draft_choice: ctx.signal(0),
        editing: Mutex::new(None),
        pending_focus: Mutex::new(None),
        cell_refs: Mutex::new(HashMap::new()),
        scroll_ref: ElementRef::new(),
        drag_anchor: Mutex::new(None),
        drag_last: Mutex::new(None),
        menu_open: Mutex::new(None),
        menu_anchor: ElementRef::new(),
        submenu_open: ctx.signal(false),
        submenu_anchor: ElementRef::new(),
        header_refs: Mutex::new(HashMap::new()),
        header_drop: Mutex::new(None),
        lines: Mutex::new(Arc::new(Lines::default())),
        group_toggle: Mutex::new(None),
        metrics: Mutex::new(Metrics::default()),
        focused: Mutex::new(None),
        revision: ctx.signal(0_u64),
      }),
      own_range: Arc::new(Mutex::new(None)),
      own_layout: Arc::new(Mutex::new(None)),
      own_open: Arc::new(Mutex::new(Vec::new())),
      _marker: PhantomData,
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let mut table = ctx.props::<TableProps<R, K>>().table.lock().clone();
    let _ = self.state.revision.get();
    // The widths a person made the columns are the table's.
    table.resized = Arc::new(self.state.resized.lock().clone());
    // A grid whose app holds no range of cells keeps its own.
    if table.grid && table.write_cell_range.is_none() {
      table.cell_range = self.own_range.lock().clone();
      let (own, state) = (self.own_range.clone(), self.state.clone());
      table.write_cell_range = Some(Arc::new(move |next| {
        *own.lock() = next;
        state.rebuild();
      }));
    }
    // The groups a table whose app holds none keeps open itself.
    if !table.group_by.is_empty() && table.write_open.is_none() {
      table.open_groups = self.own_open.lock().clone();
      let (own, state) = (self.own_open.clone(), self.state.clone());
      table.write_open = Some(Arc::new(move |next| {
        *own.lock() = next;
        state.rebuild();
      }));
    }
    table.find_grouping();
    // A grid's columns as a person arranged them: in their order, the hidden ones out, their widths and pins.
    if table.grid {
      if table.write_column_layout.is_none() {
        table.column_layout = self.own_layout.lock().clone();
        let (own, state) = (self.own_layout.clone(), self.state.clone());
        table.write_column_layout = Some(Arc::new(move |next| {
          *own.lock() = Some(next);
          state.rebuild();
        }));
      }
      let columns = std::mem::take(&mut table.columns);
      let full = table.column_layout.clone().unwrap_or_default().full(&columns);
      let titles = full
        .columns
        .iter()
        .filter_map(|place| {
          let column = columns.iter().find(|column| column.id == place.id)?;
          Some((column.id.clone(), column.title.clone()))
        })
        .collect();
      table.columns = full.arrange(columns);
      table.arranged = Some(Arc::new((full, titles)));
    }
    let theme = current_theme();
    let state = &self.state;

    // The tracks are kept while the columns are the same: what they measured stays.
    let specs = table.track_specs(&theme);
    let tracks = {
      let mut held = state.tracks.lock();
      match &*held {
        Some((kept, tracks)) if *kept == specs => tracks.clone(),
        _ => {
          let tracks = ColumnTracks::new(specs.clone());
          *held = Some((specs, tracks.clone()));
          tracks
        }
      }
    };

    let recipe = table.resolve(&theme, &table.case());
    let row_height = px(recipe.part(TablePart::Row).rest().height, &theme).unwrap_or(0.0);
    let header_height = px(recipe.part(TablePart::Header).rest().height, &theme).unwrap_or(0.0);
    // The lines shown are what the window is reckoned in: the rows, or the groups and the rows of the open ones.
    let lines = Arc::new(table.lines());
    *state.lines.lock() = lines.clone();
    let count = lines.lines.len();
    let geometry = Geometry::of(&recipe);
    let pins = table.pins();
    let lead_tracks = usize::from(table.with_checkboxes());
    *state.metrics.lock() = Metrics {
      row_height,
      header_height,
      count,
      disabled: table.disabled,
      // A row whose cells stay at an end holds its padding there in its pinned cell, in a track.
      padding_start: if pins.0 > 0 { 0.0 } else { geometry.padding_start },
      padding_end: if pins.1 > 0 { 0.0 } else { geometry.padding_end },
      lead_tracks,
      pins,
    };
    // The columns in view, and the pinned and the focused one; every column before the table knows its width.
    let columns_built = state.columns_at(state.scroll.scroll_x()).map(Arc::new);
    *state.columns_built.lock() = columns_built.clone();

    // A recipe whose rows have no height in pixels builds every row: where they are cannot be reckoned.
    let window = (row_height > 0.0).then(|| {
      let (first, last) = state.range(OVERSCAN).unwrap_or((0, FIRST_ROWS.min(count)));
      let focused = state.focused.lock().filter(|index| *index < count);
      let mut indices: Vec<usize> = (first..last.min(count)).collect();
      if let Some(focused) = focused
        && !indices.contains(&focused)
      {
        indices.push(focused);
        indices.sort_unstable();
      }
      *state.built.lock() = indices.clone();
      let refs: HashMap<usize, ElementRef> = indices.iter().map(|index| (*index, ElementRef::new())).collect();
      // A grid's cells have refs of their own, by the rank of their row, which the focus is sent to.
      let cell_refs: HashMap<(usize, usize), ElementRef> = if table.grid {
        let built = |column: usize| columns_built.as_ref().is_none_or(|built| built[column]);
        indices
          .iter()
          .filter_map(|position| lines.rank_at(*position))
          .flat_map(|row| {
            (0..table.columns.len())
              .filter(|column| built(*column))
              .map(move |column| ((row, column), ElementRef::new()))
          })
          .collect()
      } else {
        HashMap::new()
      };
      *state.cell_refs.lock() = cell_refs.clone();
      let editor_ref = ElementRef::new();
      let pending = *state.pending_focus.lock();
      match pending {
        Some(FocusAim::Editor) => {
          ctx.focus(&editor_ref);
          *state.pending_focus.lock() = None;
        }
        Some(FocusAim::Cell(row, column)) => match cell_refs.get(&(row, column)) {
          Some(target) => {
            ctx.focus(target);
            *state.pending_focus.lock() = None;
          }
          // A row not built yet is scrolled to; the focus goes to it once it is.
          None
            if lines
              .position_of_rank
              .get(row)
              .is_some_and(|position| !indices.contains(position)) =>
          {
            state.reveal(lines.position_of_rank[row])
          }
          None => *state.pending_focus.lock() = None,
        },
        None => {}
      }
      let watcher = state.clone();
      let on_scroll: Arc<dyn Fn(ScrollEvent) + Send + Sync> = Arc::new(move |event: ScrollEvent| {
        // Handlers run before the scroll moves: the rows are reckoned from where it is going.
        let metrics = *watcher.metrics.lock();
        let viewport = watcher.scroll.viewport_height();
        let scroll_y = (watcher.scroll.scroll_y() - event.delta_y).max(0.0);
        if metrics.row_height <= 0.0 || viewport <= 0.0 {
          return;
        }
        let first = (scroll_y / metrics.row_height).floor() as usize;
        let last = (((scroll_y + viewport) / metrics.row_height).ceil() as usize).min(metrics.count);
        let built = watcher.built.lock().clone();
        let scroll_x = (watcher.scroll.scroll_x() - event.delta_x).max(0.0);
        if (first.saturating_sub(OVERSCAN / 2)..(last + OVERSCAN / 2).min(metrics.count))
          .any(|index| !built.contains(&index))
          || watcher.needs_columns(scroll_x)
        {
          watcher.rebuild();
        }
      });
      Window {
        indices,
        focused,
        row_height,
        scroll: state.scroll.clone(),
        refs,
        cell_refs,
        editor_ref,
        columns: columns_built,
        host: Some(Arc::new(HostLink { state: state.clone() })),
        rows: state.clone(),
        lines: lines.clone(),
        on_scroll: Some(on_scroll),
      }
    });
    let view = table.view(&theme, tracks, window);
    match theme.renderer::<Table<R, K>>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }

  fn after_layout(&self) {
    // A scroll the handler did not see (the bar dragged, a row scrolled into view) or a height first known.
    if self.state.needs_rows() || self.state.needs_columns(self.state.scroll.scroll_x()) {
      self.state.rebuild();
    }
  }
}

impl<R: Send + Sync + 'static, K: Send + Sync + 'static> Themed for Table<R, K> {
  const NAME: &'static str = "table";
  type Renderer = TableRenderer;

  /// The design's table (`pencil/lurq-design-system.pen`: Table, Table/Empty, Table/Loading, and the Data
  /// Grid for what a table whose rows are selected has): a box with the large radius, an inner edge of 1 in
  /// the border's color and the background's fill. The header is 40 high on the muted fill with a line of 1
  /// under it, its titles 13/500 in the muted foreground; a row is 44 high with a line of 1 under it but the
  /// last, its text 13 in the foreground. Both are padded 16 at each side with 16 between columns, 12 where
  /// the rows have checkboxes, whose column is 16 wide. The column the rows are sorted by has its title in the
  /// foreground and an arrow of 14 4 after it. A selected row is in the highlight (the board's `$accent`). A
  /// column of numbers is mono 12 in the muted foreground at its end. The footer is 44 high, padded 16 at the
  /// start and 8 at the end, its count 13 in the muted foreground. Where the board draws nothing: under the
  /// pointer and held down a row has the veils of those states, a sortable header's title is in the
  /// foreground under the pointer, the focus from the keyboard is the ring inside a row, a sortable header
  /// has a muted mark that it sorts, and a disabled table is faded. The densities are a step of the theme's
  /// rows around the board's 44: 36 compact, 52 comfortable and touch, the header 4 lower.
  fn recipe() -> Recipe {
    use PaletteColor::{Background, Foreground, Highlight, Muted, MutedForeground, Ring, StateHover, StatePressed};
    use TablePart::{Count, Empty, Footer, Header, HeaderCell, Root, Row, Select, SortIcon};
    use TableState::{Disabled, Last, Numeric, Selectable, Selected, Sortable, Sorted};
    let line = || Line::inside(BorderSize::Sm, PaletteColor::Border);
    let range_edge = || Line::inside(BorderSize::Md, Ring);
    Recipe::new()
      .base(Root, |s| {
        s.width(Length::Fill)
          .radius(RadiusSize::Lg)
          .border(line())
          .background(Background)
          .foreground(Foreground)
      })
      // What scrolls clips what is in it to its own corners, the box's: a clip of its own inside the box would cut
      // the header's fill square over the box's round corners.
      .base(TablePart::Scroll, |s| s.radius(RadiusSize::Lg))
      .state(TableState::Footer, TablePart::Scroll, |s| {
        s.corners(RadiusSize::Lg, RadiusSize::Lg, 0.0, 0.0)
      })
      .base(Header, |s| {
        s.height(40.0)
          .padding_x(16.0)
          .gap(16.0)
          .background(Muted)
          .underline(line())
      })
      .base(HeaderCell, |s| {
        s.gap(4.0)
          .typography(TypographyStyle::Body)
          .font_size(13.0)
          .weight(FontWeight::Medium)
          .foreground(MutedForeground)
      })
      .base(SortIcon, |s| s.square(IconSize::Sm))
      // A column's edge: 8 of room that takes the pointer, a line of 2 in the ring's color under it and while it is
      // dragged, nothing at rest, as the board draws no line between headers.
      .base(TablePart::Resizer, |s| s.width(8.0).cursor(CursorIcon::ColResize))
      .base(TablePart::ResizeLine, |s| s.width(2.0).radius(RadiusSize::Full))
      // A grid's column menu: a ghost button of 24 at the end of the header, its mark muted, shown while the
      // pointer is on the header, the focus is in it, or the menu is open.
      .base(TablePart::MenuTrigger, |s| {
        fading(
          s.square(24.0)
            .radius(RadiusSize::Sm)
            .foreground(MutedForeground)
            .cursor(CursorIcon::Pointer)
            .opacity(0.0),
        )
      })
      .within(TablePart::MenuTrigger, |s| s.opacity(1.0))
      .state(TableState::Open, TablePart::MenuTrigger, |s| s.opacity(1.0).background(Muted))
      .hovered(TablePart::MenuTrigger, |s| s.veil(StateHover).foreground(Foreground))
      // Where a column dragged by its header goes: a line of 2 in the ring's color.
      .state(TableState::DropBefore, TablePart::DropLine, |s| s.start_line(range_edge()))
      .state(TableState::DropAfter, TablePart::DropLine, |s| s.end_line(range_edge()))
      .within_hovered(TablePart::ResizeLine, |s| s.background(Ring))
      .within_pressed(TablePart::ResizeLine, |s| s.background(Ring))
      .state(Sortable, HeaderCell, |s| fading(s.radius(RadiusSize::Sm).cursor(CursorIcon::Pointer)))
      .hovered(HeaderCell, |s| s.foreground(Foreground))
      .focus_visible(HeaderCell, |s| s.focus_ring())
      .state(Sorted, HeaderCell, |s| s.foreground(Foreground))
      .base(Row, |s| {
        fading(
          s.height(44.0)
            .padding_x(16.0)
            .gap(16.0)
            .underline(line())
            .cursor(CursorIcon::Pointer),
        )
      })
      .hovered(Row, |s| s.veil(StateHover))
      .pressed(Row, |s| s.veil(StatePressed))
      .focus_visible(Row, |s| s.outline(Outline::new(BorderSize::Md, Ring).offset(-2.0)))
      .state(Selected, Row, |s| s.background(Highlight))
      .state(Last, Row, |s| s.underline(Line::inside(0.0, PaletteColor::Border)))
      .state(Selectable, Header, |s| s.gap(12.0))
      .state(Selectable, Row, |s| s.gap(12.0))
      .base(TablePart::Cell, |s| {
        s.typography(TypographyStyle::Body)
          .font_size(13.0)
          .foreground(Foreground)
      })
      .state(TableState::Mono, TablePart::Cell, |s| s.font_family(FontFamily::Mono))
      .state(TableState::Muted, TablePart::Cell, |s| s.foreground(MutedForeground))
      .state(Numeric, TablePart::Cell, |s| {
        s.font_family(FontFamily::Mono)
          .font_size(12.0)
          .foreground(MutedForeground)
      })
      .base(Select, |s| s.width(16.0).cursor(CursorIcon::Pointer))
      // A cell of a data grid takes the focus: from the keyboard, the ring inside it, as a row's.
      .focus_visible(TablePart::Cell, |s| s.outline(Outline::new(BorderSize::Md, Ring).offset(-2.0)))
      .base(TablePart::Editor, |s| s.width(Length::Fill))
      // The range of cells of a grid: one area in the selection's fill, edged in the ring's color, the focused cell
      // in it on its row's fill.
      .state(TableState::InRange, TablePart::Cell, |s| s.background(PaletteColor::Selection))
      .state(TableState::RangeTop, TablePart::Cell, |s| s.top_line(range_edge()))
      .state(TableState::RangeBottom, TablePart::Cell, |s| s.underline(range_edge()))
      .state(TableState::RangeStart, TablePart::Cell, |s| s.start_line(range_edge()))
      .state(TableState::RangeEnd, TablePart::Cell, |s| s.end_line(range_edge()))
      // The cells that stay cover the row's veil with its fill: they have it themselves.
      .when(Selector::new().state(TableState::Pinned).within().hovered(), TablePart::Cell, |s| {
        s.veil(StateHover)
      })
      .when(Selector::new().state(TableState::Pinned).within().pressed(), TablePart::Cell, |s| {
        s.veil(StatePressed)
      })
      .when(Selector::new().state(TableState::Pinned).within().hovered(), Select, |s| s.veil(StateHover))
      .when(Selector::new().state(TableState::Pinned).within().pressed(), Select, |s| s.veil(StatePressed))
      // A group's row: as high as a row, on the muted fill, its chevron muted, its value 13/500 in the foreground
      // and its count mono 12 muted after it.
      .base(TablePart::GroupRow, |s| {
        fading(
          s.height(44.0)
            .background(Muted)
            .underline(line())
            .cursor(CursorIcon::Pointer),
        )
      })
      .hovered(TablePart::GroupRow, |s| s.veil(StateHover))
      .pressed(TablePart::GroupRow, |s| s.veil(StatePressed))
      .focus_visible(TablePart::GroupRow, |s| s.outline(Outline::new(BorderSize::Md, Ring).offset(-2.0)))
      .base(TablePart::Chevron, |s| s.square(IconSize::Sm).foreground(MutedForeground))
      .base(TablePart::GroupLabel, |s| {
        s.typography(TypographyStyle::Body)
          .font_size(13.0)
          .weight(FontWeight::Medium)
          .foreground(Foreground)
      })
      .base(TablePart::GroupCount, |s| {
        s.font_family(FontFamily::Mono)
          .font_size(12.0)
          .foreground(MutedForeground)
      })
      .density(Density::Compact, TablePart::GroupRow, |s| s.height(Density::Default))
      .density(Density::Comfortable, TablePart::GroupRow, |s| s.height(52.0))
      .density(Density::Touch, TablePart::GroupRow, |s| s.height(52.0))
      .base(Empty, |s| s.width(Length::Fill))
      .base(Footer, |s| s.height(44.0).padding_start(16.0).padding_end(8.0))
      .base(Count, |s| {
        s.typography(TypographyStyle::Body)
          .font_size(13.0)
          .foreground(MutedForeground)
      })
      .density(Density::Compact, Row, |s| s.height(Density::Default))
      .density(Density::Compact, Header, |s| s.height(32.0))
      .density(Density::Comfortable, Row, |s| s.height(52.0))
      .density(Density::Comfortable, Header, |s| s.height(48.0))
      .density(Density::Touch, Row, |s| s.height(52.0))
      .density(Density::Touch, Header, |s| s.height(48.0))
      .state(Disabled, Root, |s| s.opacity(OpacityLevel::Disabled))
      .state(Disabled, Row, |s| s.cursor(CursorIcon::Default))
  }
}
