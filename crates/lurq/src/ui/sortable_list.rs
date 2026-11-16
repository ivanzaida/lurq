//! A sortable list: rows a person puts in another order, by a drag of a row or of its handle and by the keys.

use std::sync::Arc;

use parking_lot::Mutex;

use super::{
  icons,
  list_item::{ListItem, ListItemPart},
  names,
  part_style::{PartStyle, PartTransition},
  recipe::{Case, Interaction, Name, Recipe, ResolvedPart, ResolvedRecipe},
  selection::{ListSelection, Pick, select_all_key},
  shared::{fading, glyph},
  theme::{Themed, current_theme},
};
use crate::{
  animation::Transition,
  app::{
    component::Component,
    ctx::Ctx,
    events::{DragEvent, KeyboardEvent, MouseButtonMask, MouseEvent},
    theme::{BorderSize, IconSize, MotionDuration, PaletteColor, RadiusSize, ShadowStyle, Theme},
  },
  components::{Column, Row, Stack},
  core::{ElementRef, Signal, SignalValue},
  layout::{Alignment, layout_kind::Justify, text_style::FontWeight},
  node::{
    CursorIcon, Element, FocusGroup, NodeUpdate, border::Border, color::Color, dimension::Dimension,
    transform::Transform2D,
  },
};

/// How far a press on a row moves before it is a drag: the distance past which the engine clicks nothing on the
/// release either, so the release of a drag selects nothing.
const DRAG_START: f32 = 4.0;

/// How far the row under the pointer is turned while it is dragged, in degrees: the board's lifted row.
pub const SORTABLE_LIST_TILT: f32 = -2.0;

/// The parts of a sortable list a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SortableListPart {
  /// The rows, one under the other: the room between them.
  Root,
  /// One row: what is laid over a list item's root. Its gap is the room between its handle, its icon and its
  /// text.
  Item,
  /// The grip at the row's start, which the row is dragged by.
  Handle,
  /// The icon of a row that has one, after its handle.
  Icon,
  /// What the row says: what is laid over a list item's title.
  Title,
  /// The second line of a row that has one: what is laid over a list item's description.
  Description,
  /// What stands where a dragged row was, until it is let go.
  Ghost,
  /// The room between two rows, and before the first and after the last: where the line shows, it opens to hold
  /// the line with the room between rows on each side of it. Its transition is how the rows move to make that
  /// room.
  Slot,
  /// The line between two rows where what is dragged would go.
  Line,
  /// The dot at the line's start.
  Cap,
  /// The badge on the corner of the row under the pointer that counts the rows dragged, where they are several.
  Count,
  /// What holds the row under the pointer of a drag over the list. Its padding at the start and at the end is how
  /// much narrower than the list that row is drawn, taken off about the pointer.
  Lifted,
}

names!(SortableListPart {
  Root => "root",
  Item => "item",
  Handle => "handle",
  Icon => "icon",
  Title => "title",
  Description => "description",
  Ghost => "ghost",
  Slot => "slot",
  Line => "line",
  Cap => "cap",
  Count => "count",
  Lifted => "lifted",
});

/// The states of a sortable list a recipe styles, besides what the pointer and the keyboard do to its rows.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SortableListState {
  /// Of a row: it is selected.
  Selected,
  /// Of a row: it is the one under the pointer of a drag, drawn over the list.
  Dragged,
  /// Of a row: it takes no input, and stays where it is. Of the list: none of its rows is moved.
  Disabled,
}

names!(SortableListState {
  Selected => "selected",
  Dragged => "dragged",
  Disabled => "disabled",
});

/// What builds a sortable list's nodes: a theme's in place of the default one.
pub type SortableListRenderer = Arc<dyn Fn(SortableListView) -> Element + Send + Sync>;

/// One row of a sortable list: what the app tells it apart by, and what it says.
///
/// ```
/// use lurq::ui::{SortableItem, icons};
///
/// let plain: SortableItem<u8> = (1, "Buttons").into();
/// let rich = SortableItem::new("inputs", "Inputs").description("Text, number, password").icon(icons::FILE);
/// ```
#[derive(Clone)]
pub struct SortableItem<T> {
  value: T,
  title: Arc<str>,
  description: Option<Arc<str>>,
  icon: Option<Name>,
  trailing: Option<Element>,
  disabled: bool,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl<T> SortableItem<T> {
  /// A row of `value` that says `title`.
  pub fn new(value: T, title: &str) -> Self {
    Self {
      value,
      title: Arc::from(title),
      description: None,
      icon: None,
      trailing: None,
      disabled: false,
      id: None,
      key: None,
    }
  }

  /// A second line under the title, in the muted color.
  pub fn description(mut self, description: &str) -> Self {
    self.description = Some(Arc::from(description));
    self
  }

  /// An icon after the handle, by its name in the theme's set.
  pub fn icon(mut self, name: impl Into<Name>) -> Self {
    self.icon = Some(name.into());
    self
  }

  /// What stands at the row's end: a switch, a badge, a button.
  pub fn trailing(mut self, element: impl Into<Element>) -> Self {
    self.trailing = Some(element.into());
    self
  }

  /// Whether the row takes no input: it is not dragged, moved by the keys or selected, and it is dimmed. Other
  /// rows are still put before it and after it.
  pub fn disabled(mut self, disabled: bool) -> Self {
    self.disabled = disabled;
    self
  }

  /// The id of the row's element: what is pressed, focused and dragged. Its handle has this id with `-handle`
  /// after it.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  /// What the row is told apart by among the rows, in place of where it is: a row that keeps it keeps its node as
  /// the rows are put in another order.
  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }
}

impl<T, L: AsRef<str>> From<(T, L)> for SortableItem<T> {
  fn from((value, title): (T, L)) -> Self {
    Self::new(value, title.as_ref())
  }
}

/// Takes the rows at `rows` out of `list` and puts them back together, in the order they were in, so that the
/// first of them is at `to` of the list without them: what [`SortableList::on_reorder_many`] asks for.
///
/// ```
/// let mut list = vec!["a", "b", "c", "d", "e"];
/// lurq::ui::move_rows(&mut list, &[1, 3], 0);
/// assert_eq!(list, ["b", "d", "a", "c", "e"]);
/// ```
pub fn move_rows<X>(list: &mut Vec<X>, rows: &[usize], to: usize) {
  let mut rows: Vec<usize> = rows.iter().copied().filter(|row| *row < list.len()).collect();
  rows.sort_unstable();
  rows.dedup();
  let mut taken = Vec::with_capacity(rows.len());
  for row in rows.iter().rev() {
    taken.push(list.remove(*row));
  }
  let to = to.min(list.len());
  for row in taken {
    list.insert(to, row);
  }
}

type Moved = Arc<dyn Fn(usize, usize) + Send + Sync>;
type MovedMany = Arc<dyn Fn(&[usize], usize) + Send + Sync>;
type Told<T> = Arc<dyn Fn(&[T]) + Send + Sync>;

/// A list whose rows a person puts in another order: the columns of a table, the layers of a scene, the steps of a
/// pipeline.
///
/// ```
/// use lurq::{core::Signal, ui::SortableList};
///
/// # fn render(ctx: &mut lurq::app::ctx::Ctx, sections: Signal<Vec<&'static str>>) {
/// let order = sections.clone();
/// let list = SortableList::new(sections.get().into_iter().map(|name| (name, name)))
///   .label("Sections")
///   .on_reorder(move |from, to| {
///     order.update(|sections| {
///       let section = sections.remove(from);
///       sections.insert(to, section);
///     })
///   })
///   // Mounted, so that the dragged row follows the pointer.
///   .mount(ctx);
/// # }
/// ```
///
/// The list does not keep the order: the app's rows do. A row is dragged by any of it, or by its handle alone
/// ([`handle_only`](Self::handle_only)); past a few pixels the drag begins, the row follows the pointer over the
/// list, what stood in its place is a ghost, and a line shows between which rows it would go. Let go, the app is
/// told where: [`on_reorder(from, to)`](Self::on_reorder), as a strip of tabs tells it. The line stands in the
/// flow of the list, and the rows it goes between move apart to make room for it. Escape during a drag puts
/// the row back and tells nothing.
///
/// The list is one stop of Tab. Up and Down move the focus through the rows; Alt with Up or Down moves the focused
/// row one place, and Alt with Home or End to the first or the last, and the focus goes with it. With a selection
/// ([`selected`](Self::selected)) a press selects a row, with Ctrl one in or out and with Shift a range, Space adds
/// the focused row or takes it out, and Ctrl+A selects them all; a drag of a selected row, or Alt with a key on it,
/// moves every selected row together, the row under the pointer counts them on its corner, and the app is told by
/// [`on_reorder_many`](Self::on_reorder_many).
///
/// A row is a [`ListItem`] in the dress of the parts of the list: its title, its description, what stands at its
/// end. The rows follow the pointer where the list is [mounted](Self::mount); converted into an element on its own
/// it still tells where a row was let go, with nothing drawn on the way.
pub struct SortableList<T: 'static = ()> {
  items: Vec<SortableItem<T>>,
  selected: Option<Vec<T>>,
  select: Option<Arc<dyn Fn(Vec<T>) + Send + Sync>>,
  on_selected_change: Vec<Told<T>>,
  on_reorder: Vec<Moved>,
  on_reorder_many: Vec<MovedMany>,
  handle: bool,
  handle_only: bool,
  disabled: bool,
  label: Option<Arc<str>>,
  states: Vec<Name>,
  rules: Recipe,
  forced: Option<Interaction>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl<T: Clone + PartialEq + Send + Sync + 'static> SortableList<T> {
  /// A list of `items`, in the order the app has them. An item is a `(value, title)` or a [`SortableItem`].
  pub fn new(items: impl IntoIterator<Item = impl Into<SortableItem<T>>>) -> Self {
    Self {
      items: items.into_iter().map(Into::into).collect(),
      selected: None,
      select: None,
      on_selected_change: Vec::new(),
      on_reorder: Vec::new(),
      on_reorder_many: Vec::new(),
      handle: true,
      handle_only: false,
      disabled: false,
      label: None,
      states: Vec::new(),
      rules: Recipe::new(),
      forced: None,
      id: None,
      key: None,
    }
  }

  /// Called when a person moves one row: with where it was and where it goes, as it is taken out of the list and
  /// put back (`rows.insert(to, rows.remove(from))`), by a drag or by Alt with a key. A drag that Escape ends, and
  /// one let go where the row was, tell nothing. The app reorders its rows. A second handler is called after the
  /// first.
  pub fn on_reorder(mut self, handler: impl Fn(usize, usize) + Send + Sync + 'static) -> Self {
    self.on_reorder.push(Arc::new(handler));
    self
  }

  /// Called when a person moves several selected rows together: with where they were, in order, and where the
  /// first of them goes in the list without them ([`move_rows`] does it to a `Vec`). One row moved is told to
  /// [`on_reorder`](Self::on_reorder).
  pub fn on_reorder_many(mut self, handler: impl Fn(&[usize], usize) + Send + Sync + 'static) -> Self {
    self.on_reorder_many.push(Arc::new(handler));
    self
  }

  /// The rows that are selected: those whose values the signal has. A press selects a row, with Ctrl and Shift as
  /// a list of files has them, and the selected rows are dragged together. A list without it has no selection.
  pub fn selected(mut self, selected: Signal<Vec<T>>) -> Self
  where
    Vec<T>: SignalValue,
  {
    self.selected = Some(selected.get());
    self.select = Some(Arc::new(move |next| selected.set(next)));
    self
  }

  /// The rows that are selected, as the app holds them: `selected`, and `on_selected_change` is told the whole new
  /// selection a press or a key asks for.
  pub fn selected_controlled(
    mut self,
    selected: impl IntoIterator<Item = T>,
    on_selected_change: impl Fn(&[T]) + Send + Sync + 'static,
  ) -> Self {
    self.selected = Some(selected.into_iter().collect());
    self.select = Some(Arc::new(|_| {}));
    self.on_selected_change(on_selected_change)
  }

  /// Called with the whole new selection after a press or a key changes it, after the signal is set. Not called
  /// when the app sets the signal.
  pub fn on_selected_change(mut self, handler: impl Fn(&[T]) + Send + Sync + 'static) -> Self {
    self.on_selected_change.push(Arc::new(handler));
    self
  }

  /// Whether each row has its grip at its start. It has unless told otherwise.
  pub fn handle(mut self, handle: bool) -> Self {
    self.handle = handle;
    self
  }

  /// Whether a row is dragged by its grip alone, and not by the rest of it: for rows that hold what is pressed or
  /// selected itself.
  pub fn handle_only(mut self, handle_only: bool) -> Self {
    self.handle_only = handle_only;
    self
  }

  /// Whether no row is moved or selected.
  pub fn disabled(mut self, disabled: bool) -> Self {
    self.disabled = disabled;
    self
  }

  /// What the list is called, to what reads the tree. It draws nothing.
  pub fn label(mut self, label: &str) -> Self {
    self.label = Some(Arc::from(label));
    self
  }

  /// Puts the list in a state the theme's recipe names, or takes it out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// Styles one part of this list over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(PartStyle) -> PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for this list.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// Draws the row Tab would stop on (the first selected, else the first that is not disabled) as it is under the
  /// pointer, held down or focused from the keyboard, with nothing pointing at it: for a gallery or a test.
  pub fn force(mut self, interaction: Option<Interaction>) -> Self {
    self.forced = interaction;
    self
  }

  /// The id of the list's root.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// Builds the list in `theme`, with nothing kept: a drag of a row still tells where it was let go, but nothing
  /// follows the pointer on the way. Converting a list into an element does this in the theme of what is being
  /// rendered.
  pub fn build(self, theme: &Theme) -> Element {
    self.build_with(theme, Kept::new(None))
  }

  /// The list mounted where it is declared, where it keeps a drag from one render to the next: the dragged row
  /// follows the pointer, its place is a ghost, and a line shows where it would go.
  pub fn mount(self, ctx: &mut Ctx) -> Element {
    let key = self.key.clone();
    let props = SortableListProps { list: self };
    match key {
      Some(key) => ctx.mount_keyed::<SortableListHost<T>>(&key, props).key(key),
      None => ctx.mount::<SortableListHost<T>>(props),
    }
  }

  fn build_with(&self, theme: &Theme, kept: Kept) -> Element {
    let view = self.view(theme, kept);
    match theme.renderer::<SortableList<T>>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }

  fn view(&self, theme: &Theme, kept: Kept) -> SortableListView {
    let base = Case {
      variant: None,
      size: None,
      density: None,
      breakpoint: super::current_breakpoint(),
      states: self.states.clone(),
    };
    let resolve = |selected: bool, dragged: bool, disabled: bool| {
      theme.resolve::<SortableList<T>>(
        &base
          .clone()
          .state(SortableListState::Selected, selected)
          .state(SortableListState::Dragged, dragged)
          .state(SortableListState::Disabled, disabled),
        Some(&self.rules),
      )
    };
    let drag = kept.drawn();
    let count = self.items.len();
    let enabled: Vec<bool> = self.items.iter().map(|item| !self.disabled && !item.disabled).collect();
    let chosen: Vec<T> = self
      .selected
      .iter()
      .flatten()
      .filter(|value| self.items.iter().any(|item| item.value == **value))
      .cloned()
      .collect();
    let selects = self.selected.is_some();
    let selection = selects.then(|| {
      let (write, told) = (self.select.clone(), self.on_selected_change.clone());
      Arc::new(ListSelection::new(
        self.items.iter().map(|item| item.value.clone()).collect(),
        enabled.clone(),
        chosen.clone(),
        true,
        Some(Arc::new(move |next: Vec<T>| {
          if let Some(write) = &write {
            write(next.clone());
          }
          for handler in &told {
            handler(&next);
          }
        })),
      ))
    });
    let is_selected: Vec<bool> = self.items.iter().map(|item| chosen.contains(&item.value)).collect();
    // The rows that move with the row at `index`: every selected row where it is one of several, else it alone.
    let moved_with = {
      let (is_selected, enabled) = (is_selected.clone(), enabled.clone());
      Arc::new(move |index: usize| -> Vec<usize> {
        let together: Vec<usize> = (0..is_selected.len())
          .filter(|row| is_selected[*row] && enabled[*row])
          .collect();
        if is_selected[index] && together.len() > 1 {
          together
        } else {
          vec![index]
        }
      })
    };
    let tell = {
      let (one, many) = (self.on_reorder.clone(), self.on_reorder_many.clone());
      Arc::new(move |rows: &[usize], to: usize| match rows {
        [] => {}
        [from] => {
          if *from != to {
            for handler in &one {
              handler(*from, to);
            }
          }
        }
        _ => {
          // Nothing moves where the rows stand together at the place they go to.
          let together = rows.windows(2).all(|pair| pair[1] == pair[0] + 1);
          if together && rows[0] == to {
            return;
          }
          for handler in &many {
            handler(rows, to);
          }
        }
      }) as Tell
    };
    let entry = (0..count)
      .find(|index| enabled[*index] && is_selected[*index])
      .or_else(|| (0..count).find(|index| enabled[*index]));
    let keys: Vec<Arc<str>> = self
      .items
      .iter()
      .enumerate()
      .map(|(index, item)| item.key.clone().unwrap_or_else(|| Arc::from(format!("row-{index}"))))
      .collect();

    let rows = self
      .items
      .iter()
      .enumerate()
      .map(|(index, item)| {
        let dragged = drag.as_ref().is_some_and(|drag| drag.from == index);
        SortableRow {
          title: item.title.clone(),
          description: item.description.clone(),
          icon: item.icon,
          trailing: item.trailing.clone(),
          selected: is_selected[index],
          disabled: !enabled[index],
          ghost: drag.as_ref().is_some_and(|drag| drag.rows.contains(&index)),
          dragged,
          handle: self.handle || self.handle_only,
          handle_only: self.handle_only,
          selects,
          entry: entry == Some(index),
          index,
          count,
          recipe: resolve(is_selected[index], false, !enabled[index]),
          lifted: dragged.then(|| resolve(is_selected[index], true, false)),
          forced: self.forced.filter(|_| entry == Some(index)),
          pick: selection.clone().filter(|_| enabled[index]).map(|selection| {
            Arc::new(move |pick: Pick| selection.pick(index, pick)) as Arc<dyn Fn(Pick) + Send + Sync>
          }),
          select_all: selection
            .clone()
            .filter(|_| enabled[index])
            .map(|selection| Arc::new(move || selection.select_all()) as Arc<dyn Fn() + Send + Sync>),
          moved_with: moved_with.clone(),
          tell: tell.clone(),
          keys: keys.clone(),
          kept: kept.clone(),
          id: item.id.clone(),
        }
      })
      .collect();

    SortableListView {
      rows,
      label: self.label.clone(),
      selects,
      disabled: self.disabled,
      recipe: resolve(false, false, self.disabled),
      drag,
      kept,
      id: self.id.clone(),
      key: self.key.clone(),
    }
  }
}

impl<T: Clone + PartialEq + Send + Sync + 'static> From<SortableList<T>> for Element {
  fn from(list: SortableList<T>) -> Self {
    list.build(&current_theme())
  }
}

/// What tells the app where rows go: those at `rows`, in order, to `to` of the list without them.
type Tell = Arc<dyn Fn(&[usize], usize) + Send + Sync>;

/// A drag of rows, from the press to the release.
#[derive(Clone, Debug)]
struct Drag {
  /// The row under the pointer.
  from: usize,
  /// The rows dragged, in order: that one, or every selected row where it is one of several.
  rows: Vec<usize>,
  /// Where each row's top was from the list's when the drag began, and how high it is.
  places: Vec<(f32, f32)>,
  /// How wide the list is.
  width: f32,
  /// Where along the row the button went down, as a share of the row's width.
  grab: f32,
  /// How far the pointer has moved since the press.
  moved: (f32, f32),
  /// Whether the pointer has moved far enough for this to be a drag and not a press.
  begun: bool,
}

impl Drag {
  /// Between which rows what is dragged would go: before the row at the slot, after the last at the count. It is
  /// where the middle of the row under the pointer is among the middles of the rows.
  fn slot(&self) -> usize {
    let (top, height) = self.places[self.from];
    let middle = top + height / 2.0 + self.moved.1;
    self
      .places
      .iter()
      .filter(|(top, height)| top + height / 2.0 < middle)
      .count()
  }

  /// Where the first of the rows dragged goes, in the list without them.
  fn target(&self) -> usize {
    let slot = self.slot();
    slot - self.rows.iter().filter(|row| **row < slot).count()
  }

  /// Whether letting go here moves nothing: the rows stand together where they would go.
  fn stays(&self) -> bool {
    self.rows.windows(2).all(|pair| pair[1] == pair[0] + 1) && self.rows.first() == Some(&self.target())
  }
}

/// What a sortable list keeps from one render to the next. A mounted list keeps it and is rendered again as a drag
/// moves; a list converted into an element on its own has one that starts again at each build, and is not drawn.
#[derive(Clone)]
struct Kept {
  drag: Arc<Mutex<Option<Drag>>>,
  /// Renders a mounted list again: a drag moved.
  revision: Option<Signal<u64>>,
  rows: Arc<Mutex<Vec<ElementRef>>>,
  list: ElementRef,
}

impl Kept {
  fn new(revision: Option<Signal<u64>>) -> Self {
    Self {
      drag: Arc::default(),
      revision,
      rows: Arc::default(),
      list: ElementRef::new(),
    }
  }

  /// The ref of the row at `index`, the same from one render to the next.
  fn row(&self, index: usize) -> ElementRef {
    let mut rows = self.rows.lock();
    while rows.len() <= index {
      rows.push(ElementRef::new());
    }
    rows[index].clone()
  }

  /// Renders a mounted list again, to draw where a drag is.
  fn moved(&self) {
    if let Some(revision) = &self.revision {
      revision.update(|revision| *revision += 1);
    }
  }

  /// The drag drawn now: one that has begun, of a mounted list.
  fn drawn(&self) -> Option<Drag> {
    self.revision.as_ref()?;
    self.drag.lock().clone().filter(|drag| drag.begun)
  }
}

/// A sortable list as a renderer sees it: its rows, the style of its parts, and the drag under way.
pub struct SortableListView {
  /// The rows, in order.
  pub rows: Vec<SortableRow>,
  /// What the list is called, to what reads the tree.
  pub label: Option<Arc<str>>,
  /// Whether its rows are selected.
  pub selects: bool,
  pub disabled: bool,
  /// The style of each part of the list as a whole.
  pub recipe: ResolvedRecipe,
  drag: Option<Drag>,
  kept: Kept,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl SortableListView {
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// The index of the row under the pointer of a drag, where the list is mounted and a drag has begun.
  pub fn dragged(&self) -> Option<usize> {
    self.drag.as_ref().map(|drag| drag.from)
  }

  /// How many rows are dragged.
  pub fn dragged_count(&self) -> usize {
    self.drag.as_ref().map_or(0, |drag| drag.rows.len())
  }

  /// The rows one under the other in the style of the root, with a [slot](Self::slot) before each and after the
  /// last, which is the room between them: one stop of Tab, whose Up and Down move the focus. It is a list to what
  /// reads the tree, a listbox of any number where its rows are selected, called by its label.
  pub fn list(&mut self) -> Column {
    let mut list = Column::new()
      .align_items(Alignment::Stretch)
      // The dot of a line before the first row reaches past the list's top.
      .overflow_visible()
      .focus_group(
        FocusGroup::vertical()
          .one_stop(true)
          .wrap(false)
          .shift_extends(self.selects),
      )
      .ref_element(self.kept.list.clone())
      .describe("a11y_role", if self.selects { "listbox" } else { "list" })
      .part(&self.part(SortableListPart::Root))
      // The room between two rows is their slot's, which opens round the line: not a gap of the column.
      .spacing(0.0);
    if self.selects {
      list = list.describe("a11y_multiselectable", "true");
    }
    if let Some(label) = &self.label {
      list = list.describe("a11y_name", label.clone());
    }
    if self.disabled {
      list = list.describe("a11y_disabled", "true");
    }
    if let Some(id) = &self.id {
      list = list.id(id.clone());
    }
    let rows = std::mem::take(&mut self.rows);
    let count = rows.len();
    let mut children = Vec::with_capacity(2 * count + 1);
    children.push(self.slot(0, count));
    for (index, row) in rows.into_iter().enumerate() {
      children.push(row.render());
      children.push(self.slot(index + 1, count));
    }
    list.with_children(children)
  }

  /// Between which rows the line shows: before the row at the place, after the last at the count. `None` while
  /// nothing is dragged, and where letting go would move nothing.
  pub fn line_at(&self) -> Option<usize> {
    self.drag.as_ref().filter(|drag| !drag.stays()).map(|drag| drag.slot())
  }

  /// The room before the row at `index` of `count`, and after the last at `count`, in the flow of the list: as
  /// high as the room between two rows (none before the first and after the last), and, where the line shows in
  /// it, open to hold the line with that room on each side of it that has a row. Its height runs from the one to
  /// the other by the transition of its part, so the rows move to make room for the line and do not jump.
  pub fn slot(&self, index: usize, count: usize) -> Element {
    let theme = self.theme();
    let gap = super::select::spacing(self.part(SortableListPart::Root).rest().gap, theme).unwrap_or(0.0);
    let thick = super::select::px(self.part(SortableListPart::Line).rest().height, theme).unwrap_or(0.0);
    let (first, last) = (index == 0, index == count);
    let shown = count > 0 && self.line_at() == Some(index);
    let sides = f32::from(u8::from(!first)) + f32::from(u8::from(!last));
    let height = if shown {
      thick + gap * sides
    } else if first || last {
      0.0
    } else {
      gap
    };
    let mut slot = Column::new()
      .key(format!("slot-{index}"))
      .align_items(Alignment::Stretch)
      .justify(match (first, last) {
        (true, false) => Justify::Start,
        (false, true) => Justify::End,
        _ => Justify::Center,
      })
      .overflow_visible()
      .pointer_events_none()
      .part(&self.part(SortableListPart::Slot))
      .height(height);
    if shown {
      slot = slot.child(self.line());
    }
    slot.into()
  }

  /// The line where what is dragged would go, with the dot at its start, in the styles of their parts.
  pub fn line(&self) -> Element {
    let (line, cap) = (self.part(SortableListPart::Line), self.part(SortableListPart::Cap));
    let thick = super::select::px(line.rest().height, self.theme()).unwrap_or(0.0);
    Row::new()
      .height(thick)
      .flex_shrink(0.0)
      .align_items(Alignment::Center)
      .overflow_visible()
      .child(Row::new().flex_shrink(0.0).part(&cap))
      .child(Row::new().flex(1.0).part(&line))
      .into()
  }

  /// The row under the pointer of a drag, drawn over the list where the pointer has it, turned a little, narrower
  /// than the list by what its part says, with the count of the rows dragged on its corner where they are several.
  pub fn lifted(&self, row: Option<Element>) -> Option<Element> {
    let drag = self.drag.as_ref()?;
    let (top, height) = drag.places[drag.from];
    let [_, end, _, start] = self.part(SortableListPart::Lifted).rest().paddings();
    let theme = self.theme();
    let inset = (super::select::spacing(start, theme).unwrap_or(0.0)
      + super::select::spacing(end, theme).unwrap_or(0.0))
    .min(drag.width);
    let width = drag.width - inset;
    // Narrower about the pointer: what was under it when the button went down stays under it.
    let left = drag.moved.0 + drag.grab * inset;
    let mut lifted = Stack::new()
      .overflow_visible()
      .pointer_events_none()
      .transform(Transform2D::rotate_deg(SORTABLE_LIST_TILT))
      .absolute(left, top + drag.moved.1, width, height)
      .child(Row::new().width(width).overflow_visible().with_children(row));
    if drag.rows.len() > 1 {
      let count = self.part(SortableListPart::Count);
      let side = super::select::px(count.rest().height, self.theme()).unwrap_or(20.0);
      // On the corner at the row's end: 4 past it, 8 over it, as the board has it. Its end stays there however
      // many figures the count has, so it is laid out from the end of a room 4 wider than the row. The number is
      // the part's text alone: the box is the badge's.
      let badge = Row::new()
        .justify(Justify::Center)
        .align_items(Alignment::Center)
        .flex_shrink(0.0)
        .part(&count)
        .child(count.text(&drag.rows.len().to_string()).nowrap());
      lifted = lifted.child(
        Row::new()
          .justify(Justify::End)
          .overflow_visible()
          .absolute(0.0, -8.0, width + 4.0, side)
          .child(badge),
      );
    }
    Some(lifted.into())
  }

  /// How the default renderer builds a list: its rows with the room between them, in which the line shows where
  /// what is dragged would go, and over them, while rows are dragged, the row under the pointer.
  pub fn render(mut self) -> Element {
    // The row under the pointer is built twice while it is dragged: its ghost in the list, and itself over it.
    let lifted_row = self.rows.iter().find(|row| row.dragged).map(|row| row.lifted_element());
    let lifted = self.lifted(lifted_row);
    let list = self.list();
    // No wider than the room it has: the list in it is as wide as its part says at most.
    let mut root = Stack::new()
      .overflow_visible()
      .max_width(Dimension::Pct(100.0))
      .child(list);
    if let Some(key) = &self.key {
      root = root.key(key.clone());
    }
    root.with_children(lifted).into()
  }
}

/// One row of a sortable list as a renderer sees it: what it says, the style of its parts, and its behavior ready
/// to be dressed.
pub struct SortableRow {
  pub title: Arc<str>,
  pub description: Option<Arc<str>>,
  /// The icon after its handle.
  pub icon: Option<Name>,
  /// What stands at its end.
  pub trailing: Option<Element>,
  pub selected: bool,
  pub disabled: bool,
  /// Whether it is dragged: a ghost stands in its place.
  pub ghost: bool,
  /// Whether it is the row under the pointer of a drag.
  pub dragged: bool,
  /// Whether it has its grip.
  pub handle: bool,
  /// Whether it is dragged by its grip alone.
  pub handle_only: bool,
  selects: bool,
  /// Whether Tab enters the list at it.
  pub entry: bool,
  /// Where it is in the list, from 0.
  pub index: usize,
  count: usize,
  /// The style of each part of this row, in its case.
  pub recipe: ResolvedRecipe,
  /// The style of each part of the row as it is drawn over the list, under the pointer.
  lifted: Option<ResolvedRecipe>,
  forced: Option<Interaction>,
  pick: Option<Arc<dyn Fn(Pick) + Send + Sync>>,
  select_all: Option<Arc<dyn Fn() + Send + Sync>>,
  moved_with: Arc<dyn Fn(usize) -> Vec<usize> + Send + Sync>,
  tell: Tell,
  keys: Vec<Arc<str>>,
  kept: Kept,
  id: Option<Arc<str>>,
}

impl SortableRow {
  /// The style of a part, as this row is to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part).forced(self.forced)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// The rules a row's list item is given over the theme's for a list item: those of `Item` for its root, of
  /// `Title` for its title and of `Description` for its description.
  fn item_rules(recipe: &ResolvedRecipe) -> Recipe {
    recipe
      .rules_for(SortableListPart::Item, ListItemPart::Root)
      .extend(&recipe.rules_for(SortableListPart::Title, ListItemPart::Title))
      .extend(&recipe.rules_for(SortableListPart::Description, ListItemPart::Description))
  }

  /// What starts, follows and ends a drag of this row, on `node`: the row, or its handle.
  fn draggable(&self, node: &mut crate::node::node::Node) {
    let (index, count) = (self.index, self.count);
    let (start, moved, end) = (self.kept.clone(), self.kept.clone(), self.kept.clone());
    let (with, tell) = (self.moved_with.clone(), self.tell.clone());
    NodeUpdate::start_drag_buttons(node, MouseButtonMask::LEFT);
    NodeUpdate::on_drag_start(node, move |event: DragEvent| {
      let top = start.list.y();
      let pressed = start.row(index);
      let grab = if pressed.width() > 0.0 {
        ((event.x - pressed.x()) / pressed.width()).clamp(0.0, 1.0)
      } else {
        0.0
      };
      let places = (0..count)
        .map(|row| {
          let at = start.row(row);
          (at.y() - top, at.height())
        })
        .collect();
      *start.drag.lock() = Some(Drag {
        from: index,
        rows: with(index),
        places,
        width: start.list.width(),
        grab,
        moved: (0.0, 0.0),
        begun: false,
      });
    });
    NodeUpdate::on_drag_move(node, move |event: DragEvent| {
      {
        let mut drag = moved.drag.lock();
        let Some(drag) = drag.as_mut() else {
          return;
        };
        drag.moved = (event.total_delta_x, event.total_delta_y);
        if !drag.begun && event.total_delta_x.hypot(event.total_delta_y) > DRAG_START {
          drag.begun = true;
        }
        if !drag.begun {
          return;
        }
      }
      moved.moved();
    });
    NodeUpdate::on_drag_end(node, move |event: DragEvent| {
      let Some(mut drag) = end.drag.lock().take() else {
        return;
      };
      if !drag.begun {
        return;
      }
      end.moved();
      drag.moved = (event.total_delta_x, event.total_delta_y);
      tell(&drag.rows, drag.target());
    });
  }

  /// The grip at the row's start, in the style of its part: what the row is dragged by, where it is dragged by its
  /// grip alone. `None` for a row without one.
  pub fn handle(&self) -> Option<Element> {
    if !self.handle {
      return None;
    }
    let mut handle: Element = glyph(
      &self.part(SortableListPart::Handle),
      Name::new(icons::GRIP_VERTICAL),
      false,
    );
    if self.handle_only && !self.disabled {
      self.draggable(&mut handle.node);
      // The press is the grip's: it does not select the row as well.
      NodeUpdate::on_click(&mut *handle.node, |event: MouseEvent| event.stop_propagation());
    }
    if let Some(id) = &self.id {
      handle = handle.id(format!("{id}-handle"));
    }
    Some(handle)
  }

  /// What stands before the row's text: its grip, then its icon.
  fn leading(&self, recipe: &ResolvedRecipe, handle: Option<Element>) -> Option<Element> {
    let icon = self
      .icon
      .map(|name| glyph(&recipe.part(SortableListPart::Icon).forced(self.forced), name, false));
    if handle.is_none() && icon.is_none() {
      return None;
    }
    let mut leading = Row::new().align_items(Alignment::Center);
    if let Some(gap) = recipe.part(SortableListPart::Item).rest().gap {
      leading = leading.spacing(gap);
    }
    Some(leading.with_children(handle).with_children(icon).into())
  }

  /// The row as a list item in `recipe`'s dress, with nothing that answers the pointer or the keys.
  fn item(&self, recipe: &ResolvedRecipe, handle: Option<Element>) -> ListItem {
    let mut item = ListItem::new(&self.title)
      .selected(self.selected)
      .disabled(self.disabled)
      .force(self.forced)
      .rules(&Self::item_rules(recipe));
    if let Some(description) = &self.description {
      item = item.description(description);
    }
    if let Some(leading) = self.leading(recipe, handle) {
      item = item.leading(leading);
    }
    if let Some(trailing) = &self.trailing {
      item = item.trailing(trailing.clone());
    }
    item
  }

  /// The row as it is drawn over the list while it is dragged: itself, lifted, answering nothing.
  fn lifted_element(&self) -> Element {
    let recipe = self.lifted.as_ref().unwrap_or(&self.recipe);
    let handle = self.handle.then(|| {
      glyph(
        &recipe.part(SortableListPart::Handle),
        Name::new(icons::GRIP_VERTICAL),
        false,
      )
    });
    let mut row = self.item(recipe, handle).build(self.theme());
    NodeUpdate::flex_full(&mut *row.node, 1.0, 1.0, Some(0.0));
    row
  }

  /// How the default renderer builds a row: a list item with the grip and the icon before its text, which is
  /// pressed, focused and dragged; or, while it is dragged, the ghost in its place. It is a list item to what reads
  /// the tree, an option where the rows are selected, and says where it is among them.
  pub fn render(self) -> Element {
    let at = self.kept.row(self.index);
    if self.ghost {
      // As high as the row was: the list keeps its height while rows are dragged.
      let height = at.height();
      let mut ghost = Row::new()
        .ref_element(at)
        .key(self.keys[self.index].clone())
        .part(&self.part(SortableListPart::Ghost));
      if height > 0.0 {
        ghost = ghost.height(height);
      }
      return ghost.into();
    }
    let flag = |on: bool| if on { "true" } else { "false" };
    let pick = self.pick.clone();
    let mut row = self
      .item(&self.recipe, self.handle())
      // A row is pressed so that it takes the focus: its press selects it, where the list has a selection. The
      // release of a drag is no press: the engine clicks nothing after a press that moved as far as a drag does.
      .on_click(move |event: MouseEvent| {
        if let Some(pick) = &pick {
          pick(Pick::of(&event, true));
        }
      })
      .build(self.theme());
    let node = &mut *row.node;
    NodeUpdate::ref_element(node, at);
    NodeUpdate::focus_entry(node, self.entry);
    // The list item said it is a button, being pressed: it is a row of this list.
    #[cfg(any(feature = "devtools", feature = "mcp"))]
    node.debug.debug_attrs.retain(|(name, _)| &**name != "a11y_role");
    NodeUpdate::describe(
      node,
      Arc::from("a11y_role"),
      Arc::from(if self.selects { "option" } else { "listitem" }),
    );
    NodeUpdate::describe(
      node,
      Arc::from("a11y_posinset"),
      Arc::from((self.index + 1).to_string()),
    );
    NodeUpdate::describe(node, Arc::from("a11y_setsize"), Arc::from(self.count.to_string()));
    if self.selects {
      NodeUpdate::describe(node, Arc::from("a11y_selected"), Arc::from(flag(self.selected)));
    }
    if !self.disabled {
      if !self.handle_only {
        self.draggable(node);
      }
      let (index, count) = (self.index, self.count);
      let (kept, with, tell, keys) = (
        self.kept.clone(),
        self.moved_with.clone(),
        self.tell.clone(),
        self.keys.clone(),
      );
      let select_all = self.select_all.clone();
      NodeUpdate::on_key_down(node, move |event: KeyboardEvent| {
        // Escape puts dragged rows back where they were: the release tells nothing.
        if event.key == "Escape" {
          let mut drag = kept.drag.lock();
          if drag.as_ref().is_some_and(|drag| drag.begun) {
            *drag = None;
            drop(drag);
            event.prevent_default();
            kept.moved();
          }
          return;
        }
        if !event.focus_within || event.composing {
          return;
        }
        if let Some(select_all) = &select_all {
          select_all_key(&event, &**select_all);
        }
        if !event.alt || event.ctrl || event.meta || event.shift {
          return;
        }
        // Alt with a key moves the row, or the selected rows it is one of, and the focus goes with it.
        let rows = with(index);
        let (first, many) = (rows[0], rows.len());
        let to = match event.key.as_str() {
          "ArrowUp" => first.checked_sub(1),
          "ArrowDown" => (first + many < count || many > 1).then(|| (first + 1).min(count - many)),
          "Home" => Some(0),
          "End" => Some(count - many),
          _ => return,
        };
        event.prevent_default();
        let Some(to) = to else {
          return;
        };
        tell(&rows, to);
        // Where the focused row stands once the app has moved them: by its own key, or by its place.
        let place = to + rows.iter().position(|row| *row == index).unwrap_or(0);
        let key = if keys[index].starts_with("row-") {
          Arc::from(format!("row-{place}"))
        } else {
          keys[index].clone()
        };
        event.focus_key(key);
      });
    }
    row = row.key(self.keys[self.index].clone());
    if let Some(id) = &self.id {
      row = row.id(id.clone());
    }
    row
  }
}

/// Every sortable list has the one recipe and renderer, whatever its values are.
impl<T: Send + Sync + 'static> Themed for SortableList<T> {
  const NAME: &'static str = "sortable_list";
  type Renderer = SortableListRenderer;

  /// The design's sortable list (`pencil/lurq-design-system.pen`, Sortable List): rows 4 apart, each 36 high on the
  /// card's fill in an edge of 1 in the border's color, 6 round, with a grip of 14 in the muted foreground 6 from
  /// its start and its text 8 after it in the body's type at 13, 10 from its end. While rows are dragged, what
  /// stood in the place of each is a ghost on the muted fill in an edge of 1 in the color of an input's, at 0.6; a
  /// line of 2 in the info color with a dot of 6 at its start stands in the flow between the rows they would go
  /// between, 4 from each, so those two are 10 apart and the list is as much higher; and the row under the pointer
  /// is drawn over the list 40 narrower than it (the board's 260 over a list of 300), with the theme's large
  /// shadow, turned by [`SORTABLE_LIST_TILT`], with a badge of 20 on its corner, in the info color with the count in
  /// white at 11 and 600, where several rows are dragged. The rows move to make room for the line over the theme's
  /// base duration in its standard easing, at once where motion is reduced: the board is still, and that is the
  /// motion a row has that opens in a tree. The board draws no row under the pointer, held down, focused, selected
  /// or disabled: a row has a list item's veils and ring, a selected one is in the highlight, and a disabled one is
  /// faded.
  fn recipe() -> Recipe {
    use PaletteColor::{Card, CardForeground, Highlight, Info, Muted, MutedForeground};
    use SortableListPart::{Cap, Count, Ghost, Handle, Icon, Item, Lifted, Line, Root, Slot, Title};
    use SortableListState::{Dragged, Selected};
    let edge = |color: PaletteColor| Border::inside(BorderSize::Sm, color);
    Recipe::new()
      .base(Root, |s| s.gap(4.0))
      .base(Item, |s| {
        fading(
          s.padding_start(6.0)
            .padding_end(10.0)
            .gap(8.0)
            .radius(RadiusSize::Md)
            .background(Card)
            .foreground(CardForeground)
            .border(edge(PaletteColor::Border))
            .cursor(CursorIcon::Grab),
        )
      })
      .base(Handle, |s| {
        s.square(IconSize::Sm)
          .foreground(MutedForeground)
          .cursor(CursorIcon::Grab)
      })
      .base(Icon, |s| s.square(IconSize::Md).foreground(MutedForeground))
      .base(Title, |s| s.font_size(13.0))
      .base(Ghost, |s| {
        s.height(36.0)
          .radius(RadiusSize::Md)
          .background(Muted)
          .border(edge(PaletteColor::Input))
          .opacity(0.6)
      })
      .base(Slot, |s| {
        s.transition(PartTransition::new(Transition::height(), MotionDuration::Base))
      })
      .base(Lifted, |s| s.padding_x(20.0))
      .base(Line, |s| s.height(2.0).background(Info))
      .base(Cap, |s| s.square(6.0).radius(RadiusSize::Full).background(Info))
      .base(Count, |s| {
        s.min_width(20.0)
          .height(20.0)
          .padding_x(4.0)
          .radius(RadiusSize::Full)
          .background(Info)
          .foreground(Color::from_hex("#FFFFFF"))
          .font_size(11.0)
          .weight(FontWeight::SemiBold)
      })
      .state(Selected, Item, |s| s.background(Highlight))
      .state(Dragged, Item, |s| {
        s.shadow(ShadowStyle::Lg).cursor(CursorIcon::Grabbing)
      })
  }
}

struct SortableListProps<T: 'static> {
  list: SortableList<T>,
}

/// A list built again is drawn again: its rows are the app's as it is built.
impl<T> PartialEq for SortableListProps<T> {
  fn eq(&self, _: &Self) -> bool {
    false
  }
}

#[cfg(feature = "devtools")]
impl<T> crate::app::component::DevtoolsInspectable for SortableListProps<T> {
  fn inspect(&self, formatter: &mut crate::app::component::DevtoolsFormatter<'_>) {
    formatter.value("SortableList", format!("{} rows", self.list.items.len()));
  }
}

/// What a mounted sortable list keeps between renders.
struct SortableListHost<T> {
  kept: Kept,
  _values: std::marker::PhantomData<fn() -> T>,
}

impl<T: Clone + PartialEq + Send + Sync + 'static> Component for SortableListHost<T> {
  type Props = SortableListProps<T>;

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      kept: Kept::new(Some(ctx.signal(0_u64))),
      _values: std::marker::PhantomData,
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    if let Some(revision) = &self.kept.revision {
      // Read so that a drag that moves renders the list again.
      revision.get();
    }
    ctx
      .props::<SortableListProps<T>>()
      .list
      .build_with(&current_theme(), self.kept.clone())
  }
}
