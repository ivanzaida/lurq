//! A tree view: things inside things, files in folders, each level opened and closed in place.

use std::{
  cell::Cell,
  collections::HashMap,
  marker::PhantomData,
  sync::{
    Arc, Mutex,
    atomic::{AtomicU64, Ordering},
  },
  time::Instant,
};

use super::{
  icons, names,
  part_style::Length,
  recipe::{Case, Interaction, Name, Recipe, ResolvedPart, ResolvedRecipe},
  shared::{fading, glyph, pressable},
  spinner::{Spinner, SpinnerPart},
  theme::{Themed, current_theme},
};
use crate::{
  app::{
    component::Component,
    ctx::Ctx,
    events::{KeyboardEvent, MouseButton, MouseEvent},
    runtime::{DOUBLE_CLICK_DISTANCE, DOUBLE_CLICK_INTERVAL},
    theme::{BorderSize, Density, IconSize, OpacityLevel, PaletteColor, RadiusSize, Theme, TypographyStyle},
  },
  components::{self, Column, Row, VirtualizedList},
  core::{Signal, SignalValue},
  layout::{
    Alignment,
    layout_kind::{Justify, ScrollState},
    text_style::FontWeight,
  },
  node::{
    CursorIcon, Element, FocusGroup, FocusRows, FocusStep, SpacingValue, dimension::Dimension, outline::Outline,
    step_rows,
  },
};

/// The parts of a tree view a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TreeViewPart {
  /// The rows, one under the other.
  Root,
  /// One row: what is pressed, selected and focused, with its indent and its content.
  Row,
  /// One step of a row's indent, for each level it is in: as wide as a level moves a row in.
  Indent,
  /// The line in a step of the indent, under the chevron of the row the row is in, where the tree draws them.
  Guide,
  /// What a row holds after its indent: its chevron, its icon, its label and what is at its end.
  Content,
  /// The chevron of a row that opens: right while it is closed, down while it is open. A row that does not
  /// open has the room of one, empty; a row whose children are on their way has a spinner in it.
  Chevron,
  /// The icon before the label.
  Icon,
  /// What the row says.
  Label,
  /// What the app puts at the row's end: a count, a badge.
  Trailing,
  /// Around what is at the row's end of an item marked to show it on hover: hidden until the pointer is on the
  /// row or the focus is in it.
  Reveal,
}

names!(TreeViewPart {
  Root => "root",
  Row => "row",
  Indent => "indent",
  Guide => "guide",
  Content => "content",
  Chevron => "chevron",
  Icon => "icon",
  Label => "label",
  Trailing => "trailing",
  Reveal => "reveal",
});

/// The states of a row of a tree view a recipe styles, besides what the
/// pointer and the keyboard do to it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TreeViewState {
  /// The row is selected.
  Selected,
  /// The row is open: what it holds is shown under it.
  Expanded,
  /// The row takes no input.
  Disabled,
}

names!(TreeViewState {
  Selected => "selected",
  Expanded => "expanded",
  Disabled => "disabled",
});

/// What builds a tree view's nodes: a theme's in place of the default one.
pub type TreeViewRenderer = Arc<dyn Fn(TreeViewView) -> Element + Send + Sync>;

/// One thing in a tree: a folder, a file, a group of settings. It is told
/// apart by its `id`, which the tree's open and selected rows are named by.
///
/// ```
/// use lurq::ui::{TreeItem, icons};
///
/// let src = TreeItem::folder("src", "src")
///   .child(TreeItem::file("src/lib.rs", "lib.rs"))
///   .child(TreeItem::file("src/main.rs", "main.rs").disabled(true));
/// // Children the app brings when the row is first opened.
/// let remote = TreeItem::new("origin", "origin").icon(icons::BRANCH).lazy(true);
/// ```
#[derive(Clone)]
pub struct TreeItem<T> {
  id: T,
  label: Arc<str>,
  icon: Option<Name>,
  open_icon: Option<Name>,
  trailing: Option<Element>,
  trailing_on_hover: bool,
  disabled: bool,
  lazy: bool,
  children: Vec<TreeItem<T>>,
  element_id: Option<Arc<str>>,
}

impl<T> TreeItem<T> {
  /// An item of `id` that says `label`, with no icon.
  pub fn new(id: T, label: &str) -> Self {
    Self {
      id,
      label: Arc::from(label),
      icon: None,
      open_icon: None,
      trailing: None,
      trailing_on_hover: false,
      disabled: false,
      lazy: false,
      children: Vec::new(),
      element_id: None,
    }
  }

  /// A folder: an item with the icon of a folder, and of an open one while it is open.
  pub fn folder(id: T, label: &str) -> Self {
    Self::new(id, label).icon(icons::FOLDER).open_icon(icons::FOLDER_OPEN)
  }

  /// A file: an item with the icon of a file.
  pub fn file(id: T, label: &str) -> Self {
    Self::new(id, label).icon(icons::FILE)
  }

  /// The icon before the label, by its name in the theme's set.
  pub fn icon(mut self, name: impl Into<Name>) -> Self {
    self.icon = Some(name.into());
    self
  }

  /// The icon before the label while the item is open, in place of its [`icon`](Self::icon).
  pub fn open_icon(mut self, name: impl Into<Name>) -> Self {
    self.open_icon = Some(name.into());
    self
  }

  /// What stands at the row's end: a count, a badge.
  pub fn trailing(mut self, element: impl Into<Element>) -> Self {
    self.trailing = Some(element.into());
    self
  }

  /// Whether what stands at the row's end shows only while the pointer is on the row or the focus is on it or in
  /// it, and always where the theme is at the touch density: an action, as a new file on a folder's row.
  pub fn trailing_on_hover(mut self, on_hover: bool) -> Self {
    self.trailing_on_hover = on_hover;
    self
  }

  /// Whether the row takes no input: it is not selected, opened or closed, and the keys pass it by. What it
  /// holds is shown if it is open.
  pub fn disabled(mut self, disabled: bool) -> Self {
    self.disabled = disabled;
    self
  }

  /// Whether the item has children the app has not brought yet: it opens as a folder does, and while it is
  /// open with no children its chevron is a spinner. The app is told it opened (`TreeView::on_expand`) and
  /// gives it its children, or makes it not lazy where it has none.
  pub fn lazy(mut self, lazy: bool) -> Self {
    self.lazy = lazy;
    self
  }

  /// An item inside this one.
  pub fn child(mut self, item: TreeItem<T>) -> Self {
    self.children.push(item);
    self
  }

  /// Items inside this one.
  pub fn children(mut self, items: impl IntoIterator<Item = TreeItem<T>>) -> Self {
    self.children.extend(items);
    self
  }

  /// The id of the row's element: what is pressed and focused.
  pub fn element_id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.element_id = Some(id.into());
    self
  }
}

/// What a tree's selection reads and writes: a signal of one row,
/// `Signal<Option<T>>`, or of any number, `Signal<Vec<T>>`, which
/// [`TreeView::selected`] takes either of.
pub struct TreeSelection<T> {
  read: Arc<dyn Fn() -> Vec<T> + Send + Sync>,
  write: Arc<dyn Fn(Vec<T>) + Send + Sync>,
  many: bool,
}

impl<T: Clone + Send + Sync + 'static> From<Signal<Option<T>>> for TreeSelection<T>
where
  Option<T>: SignalValue,
{
  fn from(signal: Signal<Option<T>>) -> Self {
    let read = signal.clone();
    Self {
      read: Arc::new(move || read.get().into_iter().collect()),
      write: Arc::new(move |chosen: Vec<T>| signal.set(chosen.into_iter().last())),
      many: false,
    }
  }
}

impl<T: Clone + Send + Sync + 'static> From<Signal<Vec<T>>> for TreeSelection<T>
where
  Vec<T>: SignalValue,
{
  fn from(signal: Signal<Vec<T>>) -> Self {
    let read = signal.clone();
    Self {
      read: Arc::new(move || read.get()),
      write: Arc::new(move |chosen| signal.set(chosen)),
      many: true,
    }
  }
}

type Told<T> = Arc<dyn Fn(&T) + Send + Sync>;

/// Things inside things, one level under the other: the files of a
/// project in their folders, the layers of a scene, the settings of an app
/// in their groups.
///
/// ```
/// use lurq::{core::Signal, ui::{TreeItem, TreeView}};
///
/// # fn files(open: Signal<Vec<&'static str>>, selected: Signal<Option<&'static str>>) {
/// let tree = TreeView::new([
///   TreeItem::folder("crates", "crates").child(
///     TreeItem::folder("lurq", "lurq")
///       .child(TreeItem::file("lib.rs", "lib.rs"))
///       .child(TreeItem::file("theme.rs", "theme.rs")),
///   ),
///   TreeItem::file("Cargo.toml", "Cargo.toml"),
/// ])
/// .label("Files")
/// .expanded(open)
/// .selected(selected)
/// .on_activate(|file| println!("open {file}"));
/// # }
/// ```
///
/// What is open and what is selected are the app's: a signal the tree reads
/// and writes ([`expanded`](Self::expanded), [`selected`](Self::selected)), or
/// what the app says with a handler it is told by
/// ([`expanded_ids`](Self::expanded_ids), [`selected_ids`](Self::selected_ids)).
/// The rows are one flat list of what is shown: what a closed row holds is
/// not built, and it opens and closes at once.
///
/// A press of a row selects it, a press of its chevron opens or closes it,
/// and a double press activates it. The tree is one stop of Tab, at the
/// selected row or the first. Up and Down move through the rows that are
/// shown, Home and End to the first and the last; Right opens a closed row and
/// goes into an open one; Left closes an open row and goes out of a closed one
/// or one that does not open; `*` opens every row at the level of the focused
/// one; a letter goes to the next row that starts with it. Space selects the
/// focused row and Enter activates it: a row that opens opens or closes, and
/// [`on_activate`](Self::on_activate) is told. With
/// [`multiple`](Self::multiple) a press with Ctrl (or Cmd) adds a row or takes
/// it out, one with Shift selects the rows from the last selected one to it,
/// Space adds or takes out the focused row, Shift with Up, Down, Home or End
/// moves and selects the rows on the way, and Ctrl+A selects them all.
pub struct TreeView<T> {
  items: Vec<TreeItem<T>>,
  open: Vec<T>,
  toggle: Option<Arc<dyn Fn(&T, bool) + Send + Sync>>,
  selected: Vec<T>,
  select: Option<Arc<dyn Fn(Vec<T>) + Send + Sync>>,
  holds_many: bool,
  multiple: bool,
  guides: bool,
  disabled: bool,
  label: Option<Arc<str>>,
  on_select: Vec<Told<T>>,
  on_activate: Vec<Told<T>>,
  on_expand: Vec<Told<T>>,
  states: Vec<Name>,
  rules: Recipe,
  forced: Option<Interaction>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl<T: Clone + PartialEq + Send + Sync + 'static> TreeView<T> {
  /// A tree of `items`, each with what it holds: none open and none selected until the app says.
  pub fn new(items: impl IntoIterator<Item = TreeItem<T>>) -> Self {
    Self {
      items: items.into_iter().collect(),
      open: Vec::new(),
      toggle: None,
      selected: Vec::new(),
      select: None,
      holds_many: false,
      multiple: false,
      guides: false,
      disabled: false,
      label: None,
      on_select: Vec::new(),
      on_activate: Vec::new(),
      on_expand: Vec::new(),
      states: Vec::new(),
      rules: Recipe::new(),
      forced: None,
      id: None,
      key: None,
    }
  }

  /// The rows that are open: those whose ids the signal has. Opening a row adds it, closing one takes it out.
  pub fn expanded(mut self, open: Signal<Vec<T>>) -> Self
  where
    Vec<T>: SignalValue,
  {
    self.open = open.get();
    self.toggle = Some(Arc::new(move |id: &T, opened: bool| {
      open.update(|held| {
        held.retain(|row| row != id);
        if opened {
          held.push(id.clone());
        }
      })
    }));
    self
  }

  /// The rows that are open, as the app holds them: those of `open`, and `toggle` is told of each row a press
  /// or a key asks to open (`true`) or to close.
  pub fn expanded_ids(
    mut self,
    open: impl IntoIterator<Item = T>,
    toggle: impl Fn(&T, bool) + Send + Sync + 'static,
  ) -> Self {
    self.open = open.into_iter().collect();
    self.toggle = Some(Arc::new(toggle));
    self
  }

  /// What is selected: a `Signal<Option<T>>` of one row, or a `Signal<Vec<T>>` of any number with
  /// [`multiple`](Self::multiple). The last of a `Vec` is where a range selected with Shift starts.
  pub fn selected(mut self, selection: impl Into<TreeSelection<T>>) -> Self {
    let selection = selection.into();
    self.selected = (selection.read)();
    self.select = Some(selection.write);
    self.holds_many = selection.many;
    self
  }

  /// What is selected, as the app holds it: the rows of `selected`, and `change` is told what is to be
  /// selected after each press or key that changes it. The last is where a range selected with Shift starts.
  pub fn selected_ids(
    mut self,
    selected: impl IntoIterator<Item = T>,
    change: impl Fn(Vec<T>) + Send + Sync + 'static,
  ) -> Self {
    self.selected = selected.into_iter().collect();
    self.select = Some(Arc::new(change));
    self.holds_many = true;
    self
  }

  /// Whether any number of rows may be selected: with Ctrl and Shift, as a list of files has it. A selection
  /// in a `Signal<Option<T>>` holds one whatever this says.
  pub fn multiple(mut self, multiple: bool) -> Self {
    self.multiple = multiple;
    self
  }

  /// Whether a line runs down each level, under the chevron of the row it is in.
  pub fn guides(mut self, guides: bool) -> Self {
    self.guides = guides;
    self
  }

  /// Whether no row takes input.
  pub fn disabled(mut self, disabled: bool) -> Self {
    self.disabled = disabled;
    self
  }

  /// What the tree is called, to what reads the tree of the window. It draws nothing.
  pub fn label(mut self, label: &str) -> Self {
    self.label = Some(Arc::from(label));
    self
  }

  /// Called with the row a press or Space selects.
  pub fn on_select(mut self, handler: impl Fn(&T) + Send + Sync + 'static) -> Self {
    self.on_select.push(Arc::new(handler));
    self
  }

  /// Called with the row a double press or Enter activates: open the file.
  pub fn on_activate(mut self, handler: impl Fn(&T) + Send + Sync + 'static) -> Self {
    self.on_activate.push(Arc::new(handler));
    self
  }

  /// Called with each row as it is opened, by a press or a key: where a lazy item's children are brought.
  pub fn on_expand(mut self, handler: impl Fn(&T) + Send + Sync + 'static) -> Self {
    self.on_expand.push(Arc::new(handler));
    self
  }

  /// Puts every row in a state the theme's recipe names, or takes it out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// Styles one part of this tree over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(super::PartStyle) -> super::PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for this tree.
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

  /// The id of the root: the rows' column.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// Builds the tree in `theme`. Converting a tree into an element does this in the theme of what is being
  /// rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let (view, _) = self.view(theme, None);
    match theme.renderer::<TreeView<T>>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }

  /// The tree as a renderer sees it, in `theme`: every shown row, with a key of each that stays its own while
  /// rows open and close before it. `page` says how many rows PageUp and PageDown move by, in a tree in a virtual
  /// list.
  fn view(&self, theme: &Theme, page: Option<Page>) -> (TreeViewView, Vec<String>) {
    let breakpoint = super::current_breakpoint();
    let base = Case {
      variant: None,
      size: None,
      density: None,
      breakpoint,
      states: self.states.clone(),
    };
    // A row is in one of eight cases: each is resolved once, however many rows there are.
    let mut recipes: HashMap<(bool, bool, bool), ResolvedRecipe> = HashMap::new();
    let mut recipe = |selected: bool, expanded: bool, disabled: bool| {
      recipes
        .entry((selected, expanded, disabled))
        .or_insert_with(|| {
          let case = base
            .clone()
            .state(TreeViewState::Selected, selected)
            .state(TreeViewState::Expanded, expanded)
            .state(TreeViewState::Disabled, disabled);
          theme.resolve::<TreeView<T>>(&case, Some(&self.rules))
        })
        .clone()
    };

    let mut flat = Vec::new();
    flatten(&self.items, 1, "", &self.open, self.disabled, &mut flat);

    let multiple = self.multiple && self.holds_many;
    let model = Arc::new(Model {
      ids: flat.iter().map(|row| row.id.clone()).collect(),
      enabled: flat.iter().map(|row| !row.disabled).collect(),
      selected: self.selected.clone(),
      multiple,
      select: self.select.clone(),
      on_select: self.on_select.clone(),
      on_activate: self.on_activate.clone(),
      page,
    });
    let entry = flat
      .iter()
      .position(|row| !row.disabled && self.selected.contains(&row.id));

    let keys = flat.iter_mut().map(|row| std::mem::take(&mut row.key)).collect();
    let rows = flat
      .into_iter()
      .enumerate()
      .map(|(index, row)| {
        let selected = self.selected.contains(&row.id);
        let toggle = (row.opens && !row.disabled)
          .then(|| self.toggle.clone())
          .flatten()
          .map(|write| {
            let (id, on_expand) = (row.id.clone(), self.on_expand.clone());
            Arc::new(move |open: bool| {
              write(&id, open);
              if open {
                for handler in &on_expand {
                  handler(&id);
                }
              }
            }) as Arc<dyn Fn(bool) + Send + Sync>
          });
        let actions = (!row.disabled).then(|| {
          let model = model.clone();
          Arc::new(RowActions {
            pick: {
              let model = model.clone();
              Arc::new(move |pick| model.pick(index, pick))
            },
            extend: {
              let model = model.clone();
              Arc::new(move |reach| model.extend(index, reach))
            },
            select_all: {
              let model = model.clone();
              Arc::new(move || model.select_all())
            },
            activate: {
              let (toggle, expanded) = (toggle.clone(), row.expanded);
              Arc::new(move || {
                if let Some(toggle) = &toggle {
                  toggle(!expanded);
                }
                model.activate(index);
              })
            },
          })
        });
        TreeRow {
          label: row.label,
          level: row.level,
          opens: row.opens,
          expanded: row.expanded,
          loading: row.loading,
          selected,
          disabled: row.disabled,
          icon: if row.expanded {
            row.open_icon.or(row.icon)
          } else {
            row.icon
          },
          trailing: row.trailing,
          trailing_on_hover: row.trailing_on_hover,
          position: (row.position, row.siblings),
          guides: self.guides,
          entry: entry == Some(index),
          index,
          multiple,
          recipe: recipe(selected, row.expanded, row.disabled),
          forced: self.forced,
          toggle,
          actions,
          row: None,
          id: row.element_id,
        }
      })
      .collect();

    let view = TreeViewView {
      rows,
      multiple,
      guides: self.guides,
      label: self.label.clone(),
      recipe: recipe(false, false, self.disabled),
      id: self.id.clone(),
      key: self.key.clone(),
    };
    (view, keys)
  }
}

impl<T: Clone + PartialEq + Send + Sync + 'static> From<TreeView<T>> for Element {
  fn from(tree: TreeView<T>) -> Self {
    tree.build(&current_theme())
  }
}

/// A row as the tree lays it out: an item that is shown, with where it is.
struct Flat<T> {
  id: T,
  label: Arc<str>,
  icon: Option<Name>,
  open_icon: Option<Name>,
  trailing: Option<Element>,
  trailing_on_hover: bool,
  disabled: bool,
  opens: bool,
  expanded: bool,
  loading: bool,
  level: u32,
  position: usize,
  siblings: usize,
  element_id: Option<Arc<str>>,
  /// Where it is among the items, from the top: `0/2/1`.
  key: String,
}

/// The rows of `items` at `level` and of what is open in them, in the order they are shown.
fn flatten<T: Clone + PartialEq>(
  items: &[TreeItem<T>],
  level: u32,
  parent: &str,
  open: &[T],
  disabled: bool,
  out: &mut Vec<Flat<T>>,
) {
  let siblings = items.len();
  for (position, item) in items.iter().enumerate() {
    let opens = item.lazy || !item.children.is_empty();
    let expanded = opens && open.contains(&item.id);
    let row_disabled = disabled || item.disabled;
    let key = if parent.is_empty() {
      position.to_string()
    } else {
      format!("{parent}/{position}")
    };
    out.push(Flat {
      loading: expanded && item.lazy && item.children.is_empty(),
      id: item.id.clone(),
      label: item.label.clone(),
      icon: item.icon,
      open_icon: item.open_icon,
      trailing: item.trailing.clone(),
      trailing_on_hover: item.trailing_on_hover,
      disabled: row_disabled,
      opens,
      expanded,
      level,
      position: position + 1,
      siblings,
      element_id: item.element_id.clone(),
      key: key.clone(),
    });
    if expanded {
      flatten(&item.children, level + 1, &key, open, disabled, out);
    }
  }
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

/// Where Shift with a key takes the selection, from the focused row: where the focus goes with the key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Reach {
  Back,
  On,
  First,
  Last,
  PageBack,
  PageOn,
}

/// The shown rows and what is selected of them, which every row's behavior reads.
struct Model<T> {
  ids: Vec<T>,
  enabled: Vec<bool>,
  selected: Vec<T>,
  multiple: bool,
  select: Option<Arc<dyn Fn(Vec<T>) + Send + Sync>>,
  on_select: Vec<Told<T>>,
  on_activate: Vec<Told<T>>,
  /// How many rows PageUp and PageDown move by, in a tree in a virtual list.
  page: Option<Page>,
}

/// How many rows PageUp and PageDown move by.
type Page = Arc<dyn Fn() -> usize + Send + Sync>;

impl<T: Clone + PartialEq> Model<T> {
  fn pick(&self, index: usize, pick: Pick) {
    let id = &self.ids[index];
    let next = match pick {
      _ if !self.multiple => vec![id.clone()],
      Pick::Only => vec![id.clone()],
      Pick::Toggle => {
        let mut next = self.selected.clone();
        match next.iter().position(|held| held == id) {
          Some(at) => {
            next.remove(at);
          }
          None => next.push(id.clone()),
        }
        next
      }
      Pick::Range => self.range(self.anchor().unwrap_or(index), index),
    };
    self.write(next);
    for handler in &self.on_select {
      handler(id);
    }
  }

  /// Shift with a key on the row `from`: the rows from the anchor to where the focus goes.
  fn extend(&self, from: usize, reach: Reach) {
    if !self.multiple {
      return;
    }
    // Where the group's keys take the focus: the engine's step over the rows that take it, without going round.
    let step = match reach {
      Reach::Back => FocusStep::Back,
      Reach::On => FocusStep::On,
      Reach::First => FocusStep::First,
      Reach::Last => FocusStep::Last,
      Reach::PageBack => FocusStep::PageBack,
      Reach::PageOn => FocusStep::PageOn,
    };
    let page = match (&self.page, step.is_page()) {
      (Some(page), true) => page(),
      // PageUp and PageDown are keys of a tree in a virtual list only.
      (None, true) => return,
      _ => 1,
    };
    let to = step_rows(step, from, self.ids.len(), page, false, |index| self.enabled[index]);
    if let Some(to) = to {
      self.write(self.range(self.anchor().unwrap_or(from), to));
    }
  }

  fn select_all(&self) {
    if !self.multiple {
      return;
    }
    let anchor = self.anchor();
    let mut all: Vec<T> = (0..self.ids.len())
      .filter(|index| self.enabled[*index] && Some(*index) != anchor)
      .map(|index| self.ids[index].clone())
      .collect();
    all.extend(anchor.map(|anchor| self.ids[anchor].clone()));
    self.write(all);
  }

  fn activate(&self, index: usize) {
    for handler in &self.on_activate {
      handler(&self.ids[index]);
    }
  }

  /// Where a range starts: the last selected row, where it is shown.
  fn anchor(&self) -> Option<usize> {
    let last = self.selected.last()?;
    self.ids.iter().position(|id| id == last)
  }

  /// The rows from `anchor` to `to` that take input, the anchor last, so that it stays where the next starts.
  fn range(&self, anchor: usize, to: usize) -> Vec<T> {
    let (start, end) = (anchor.min(to), anchor.max(to));
    let mut range: Vec<T> = (start..=end)
      .filter(|index| self.enabled[*index] && *index != anchor)
      .map(|index| self.ids[index].clone())
      .collect();
    range.push(self.ids[anchor].clone());
    range
  }

  fn write(&self, next: Vec<T>) {
    if let Some(select) = &self.select
      && next != self.selected
    {
      select(next);
    }
  }
}

thread_local! {
  /// The last press of a row: which row of its tree, when and where. A second press of it soon after at the same
  /// place, as the engine counts a double press, activates it.
  static LAST_PRESS: Cell<Option<(usize, Instant, (f32, f32))>> = const { Cell::new(None) };
}

/// Whether a press of the row `index` is the second of a double press: then the next is a first again.
fn second_press(index: usize, event: &MouseEvent) -> bool {
  let now = Instant::now();
  let near = |(x, y): (f32, f32)| (x - event.x).powi(2) + (y - event.y).powi(2) <= DOUBLE_CLICK_DISTANCE.powi(2);
  LAST_PRESS.with(|last| {
    let second = last
      .get()
      .is_some_and(|(row, at, place)| row == index && now.duration_since(at) <= DOUBLE_CLICK_INTERVAL && near(place));
    last.set((!second).then_some((index, now, (event.x, event.y))));
    second
  })
}

/// How a press of a row selects, by its keys: Shift a range, Ctrl (Cmd) one more or one less. Space does what
/// Ctrl with a press does where any number are selected (`toggles`).
fn pick_of(event: &MouseEvent, toggles: bool) -> Pick {
  if event.shift {
    Pick::Range
  } else if event.ctrl || event.meta || toggles {
    Pick::Toggle
  } else {
    Pick::Only
  }
}

/// What a row that takes input does, with the type of the tree's ids out of sight.
struct RowActions {
  pick: Arc<dyn Fn(Pick) + Send + Sync>,
  extend: Arc<dyn Fn(Reach) + Send + Sync>,
  select_all: Arc<dyn Fn() + Send + Sync>,
  activate: Arc<dyn Fn() + Send + Sync>,
}

/// A tree as a renderer sees it: its rows, one flat list of what is shown,
/// and the style of its root.
pub struct TreeViewView {
  /// The rows that are shown, in order.
  pub rows: Vec<TreeRow>,
  /// Whether any number of rows may be selected.
  pub multiple: bool,
  /// Whether a line runs down each level.
  pub guides: bool,
  /// What the tree is called, to what reads the tree of the window.
  pub label: Option<Arc<str>>,
  /// The style of each part of the tree as a whole, its rows at rest.
  pub recipe: ResolvedRecipe,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl TreeViewView {
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// The root in the style of its part, with the behavior of the keyboard: one stop of Tab at the selected
  /// row or the first, the arrows through the rows that are shown and between their levels, typeahead. It is
  /// a tree to what reads the tree of the window.
  pub fn root(&self) -> Column {
    let mut root = Column::new()
      .align_items(Alignment::Stretch)
      .focus_group(
        FocusGroup::vertical()
          .nested(true)
          .one_stop(true)
          .typeahead(true)
          .wrap(false),
      )
      .describe("a11y_role", "tree")
      .part(&self.part(TreeViewPart::Root));
    if let Some(label) = &self.label {
      root = root.describe("a11y_name", label.clone());
    }
    if self.multiple {
      root = root.describe("a11y_multiselectable", "true");
    }
    if let Some(id) = &self.id {
      root = root.id(id.clone());
    }
    if let Some(key) = &self.key {
      root = root.key(key.clone());
    }
    root
  }

  /// How the default renderer builds a tree: its rows one under the other.
  pub fn render(mut self) -> Element {
    let rows = std::mem::take(&mut self.rows);
    self.root().with_children(rows.into_iter().map(TreeRow::render)).into()
  }
}

/// One row of a tree as a renderer sees it: what it says and where it is,
/// the style of its parts, and its behavior ready to be dressed.
#[derive(Clone)]
pub struct TreeRow {
  pub label: Arc<str>,
  /// Its level: 1 at the top, 2 in a row at the top, and so on.
  pub level: u32,
  /// Whether it opens: it has children, or the app brings them.
  pub opens: bool,
  /// Whether it is open.
  pub expanded: bool,
  /// Whether it is open and its children are on their way.
  pub loading: bool,
  pub selected: bool,
  pub disabled: bool,
  /// The icon before its label: the open one while it is open, where it has one.
  pub icon: Option<Name>,
  /// What stands at its end.
  pub trailing: Option<Element>,
  /// Whether what stands at its end shows only while the row is pointed at or holds the focus.
  pub trailing_on_hover: bool,
  /// Where it is among the rows of its level in the row it is in, from 1, and how many they are.
  pub position: (usize, usize),
  /// Whether a line runs down each level of its indent.
  pub guides: bool,
  /// Whether Tab enters the tree at it: the first selected row.
  pub entry: bool,
  /// Which of the tree's shown rows it is.
  index: usize,
  multiple: bool,
  /// The style of each part of this row, in its case.
  pub recipe: ResolvedRecipe,
  forced: Option<Interaction>,
  toggle: Option<Arc<dyn Fn(bool) + Send + Sync>>,
  actions: Option<Arc<RowActions>>,
  /// Which row of the group's rows it is, in a tree in a virtual list.
  row: Option<usize>,
  id: Option<Arc<str>>,
}

impl TreeRow {
  /// The style of a part, as this row is to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part).forced(self.forced)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// The row's behavior with the style of its part: the element that is pressed, selected, focused and
  /// disabled, with nothing in it yet. A press selects it, with Ctrl or Shift as the tree allows; a double
  /// press and Enter activate it; Space selects it. It is a tree item to what reads the tree of the window,
  /// at its level, among its siblings, open or not, selected or not.
  pub fn button(&self) -> components::Button {
    let flag = |on: bool| if on { "true" } else { "false" };
    let mut row = pressable(&self.part(TreeViewPart::Row), self.disabled)
      .justify(Justify::Start)
      .align_items(Alignment::Stretch)
      // The guides reach over the room between two rows.
      .overflow_visible()
      .describe("a11y_role", "treeitem")
      .describe("a11y_name", self.label.clone())
      .describe("a11y_level", self.level.to_string())
      .describe("a11y_posinset", self.position.0.to_string())
      .describe("a11y_setsize", self.position.1.to_string())
      .describe("a11y_selected", flag(self.selected))
      .focus_label(self.label.clone())
      .focus_level(self.level)
      .focus_entry(self.entry)
      // What is at its end may show only while it is pointed at or holds the focus.
      .interaction_group(true);
    if let Some(index) = self.row {
      row = row.focus_row(index);
    }
    if self.opens {
      row = row.describe("a11y_expanded", flag(self.expanded));
    }
    if self.loading {
      row = row.describe("a11y_busy", "true");
    }
    if let Some(toggle) = self.toggle.clone() {
      row = row.focus_expanded(self.expanded, move |open| toggle(open));
    }
    if let Some(actions) = self.actions.clone() {
      let multiple = self.multiple;
      // A press selects as the button goes down, as a file manager's does, and a second press of the row soon
      // after at the same place activates it. The row has no `on_dblclick`: the engine holds back the click of an
      // element that has one until it knows it is not the first of a double press, and the focus with it.
      let press = actions.clone();
      let index = self.index;
      row = row.on_mouse_down(move |event: MouseEvent| {
        if event.button != MouseButton::Left {
          return;
        }
        if second_press(index, &event) {
          (press.activate)();
        } else {
          (press.pick)(pick_of(&event, false));
        }
      });
      // Space clicks the focused row: it selects too. A click of the pointer selected at its press.
      let space = actions.clone();
      row = row.on_click(move |event: MouseEvent| {
        if event.from_keyboard {
          (space.pick)(pick_of(&event, multiple));
        }
      });
      row = row.on_key_down(move |event: KeyboardEvent| {
        if !event.focus_within || event.composing || event.alt {
          return;
        }
        let command = event.ctrl || event.meta;
        match event.key.as_str() {
          // Enter activates; it does not press the row, which would select it.
          "Enter" if !command && !event.shift => {
            event.prevent_default();
            (actions.activate)();
          }
          "a" | "A" if command && multiple && !event.shift => {
            event.prevent_default();
            (actions.select_all)();
          }
          // The focus moves on with the key, as it does without Shift; the selection goes with it.
          "ArrowUp" if event.shift && !command => (actions.extend)(Reach::Back),
          "ArrowDown" if event.shift && !command => (actions.extend)(Reach::On),
          "Home" if event.shift && !command => (actions.extend)(Reach::First),
          "End" if event.shift && !command => (actions.extend)(Reach::Last),
          "PageUp" if event.shift && !command => (actions.extend)(Reach::PageBack),
          "PageDown" if event.shift && !command => (actions.extend)(Reach::PageOn),
          _ => {}
        }
      });
    }
    if let Some(id) = &self.id {
      row = row.id(id.clone());
    }
    row
  }

  /// A step of the indent for each level the row is in, each with its guide where the tree draws them:
  /// `None` for a row at the top.
  pub fn indent(&self) -> Option<Row> {
    let steps = self.level.saturating_sub(1);
    if steps == 0 {
      return None;
    }
    let (step, guide) = (self.part(TreeViewPart::Indent), self.part(TreeViewPart::Guide));
    Some(
      Row::new()
        .align_items(Alignment::Stretch)
        .overflow_visible()
        .with_children((0..steps).map(|_| {
          let mut cell = Row::new()
            .justify(Justify::Center)
            .align_items(Alignment::Start)
            .overflow_visible()
            .part(&step);
          if self.guides {
            cell = cell.child(Row::new().part(&guide));
          }
          cell
        })),
    )
  }

  /// The chevron of a row that opens, which a press opens or closes it with; a spinner while its children
  /// are on their way; the room of a chevron, empty, for a row that does not open.
  pub fn chevron(&self) -> Element {
    let part = self.part(TreeViewPart::Chevron);
    if self.loading {
      let side = part.rest().width.unwrap_or(Length::Icon(IconSize::Sm));
      let spinner = Spinner::new()
        .decorative(true)
        .style(SpinnerPart::Indicator, |s| s.square(side));
      return Row::new().part(&part).child(spinner.build(self.theme())).into();
    }
    if !self.opens {
      return Row::new().part(&part).into();
    }
    let name = if self.expanded {
      icons::CHEVRON_DOWN
    } else {
      icons::CHEVRON_RIGHT
    };
    let mut chevron = Row::new().child(glyph(&part, Name::new(name), false));
    if let Some(toggle) = self.toggle.clone() {
      let open = !self.expanded;
      // The press is the chevron's, as the button goes down: it does not select the row as well, and is not the
      // first of a double press of the row.
      chevron = chevron.on_mouse_down(move |event: MouseEvent| {
        event.stop_propagation();
        LAST_PRESS.with(|last| last.set(None));
        if event.button == MouseButton::Left {
          toggle(open);
        }
      });
    }
    chevron.into()
  }

  /// The icon before the label, if the row has one.
  pub fn icon(&self) -> Option<Element> {
    self
      .icon
      .clone()
      .map(|name| glyph(&self.part(TreeViewPart::Icon), name, false))
  }

  /// What the row says, in the room it has: a longer label is cut.
  pub fn label(&self) -> Column {
    let part = self.part(TreeViewPart::Label);
    Column::new()
      .flex_full(1.0, 1.0, None)
      .part(&part)
      .child(part.text(&self.label).nowrap())
  }

  /// How the default renderer builds a row: its indent, then its chevron, its icon, its label and what is at
  /// its end.
  pub fn render(mut self) -> Element {
    let mut row = self.button();
    if let Some(indent) = self.indent() {
      row = row.child(indent);
    }
    let mut content = Row::new()
      .align_items(Alignment::Center)
      .flex_full(1.0, 1.0, None)
      .part(&self.part(TreeViewPart::Content))
      .child(self.chevron());
    if let Some(icon) = self.icon() {
      content = content.child(icon);
    }
    content = content.child(self.label());
    if let Some(trailing) = self.trailing.take() {
      let trailing: Element = if self.trailing_on_hover {
        Row::new()
          .align_items(Alignment::Center)
          .part(&self.part(TreeViewPart::Reveal))
          .child(trailing)
          .into()
      } else {
        trailing
      };
      content = content.child(
        Row::new()
          .align_items(Alignment::Center)
          .part(&self.part(TreeViewPart::Trailing))
          .child(trailing),
      );
    }
    row.child(content).into()
  }
}

impl<T: Send + Sync + 'static> Themed for TreeView<T> {
  const NAME: &'static str = "tree_view";
  type Renderer = TreeViewRenderer;

  /// The design's tree view (`pencil/lurq-design-system.pen`, Tree View): rows of 28, 2 apart, with the
  /// small radius and 8 at each side; each level moves a row in by 16. A row holds a chevron of 14 in the
  /// muted foreground, right while it is closed and down while it is open (a row that does not open has its
  /// room), an icon of 16 in the muted foreground and its label in the body's type at 13, 6 apart. The
  /// selected row is in the highlight (the board's `$accent`), its label at 500 and its icon in the
  /// foreground. The board draws no other state: under the pointer and held down a row has the veils of
  /// those states, the focus from the keyboard is the ring inside it, and a disabled row is faded. A guide
  /// is a line of 1 in the border's color under the middle of the chevron of the row it is in, as high as a
  /// row and the room under it.
  fn recipe() -> Recipe {
    use PaletteColor::{Foreground, Highlight, MutedForeground, Ring, StateHover, StatePressed};
    use TreeViewPart::{Chevron, Content, Guide, Icon, Indent, Label, Reveal, Root, Row, Trailing};
    Recipe::new()
      .base(Root, |s| s.gap(2.0).foreground(Foreground))
      .base(Row, |s| {
        fading(
          s.height(28.0)
            .padding_x(8.0)
            .radius(RadiusSize::Sm)
            .cursor(CursorIcon::Pointer),
        )
      })
      .hovered(Row, |s| s.veil(StateHover))
      .pressed(Row, |s| s.veil(StatePressed))
      .focus_visible(Row, |s| s.outline(Outline::new(BorderSize::Md, Ring).offset(-2.0)))
      // A step is 16: the chevron of 14 and 2 after it, so that the guide in its middle is under the chevron's.
      .base(Indent, |s| s.width(16.0).padding_end(2.0))
      .base(Guide, |s| s.width(1.0).height(30.0).background(PaletteColor::Border))
      .base(Content, |s| s.gap(6.0))
      .base(Chevron, |s| s.square(IconSize::Sm).foreground(MutedForeground))
      .base(Icon, |s| s.square(IconSize::Md).foreground(MutedForeground))
      .base(Label, |s| s.typography(TypographyStyle::Body).font_size(13.0))
      .base(Trailing, |s| {
        s.gap(4.0)
          .typography(TypographyStyle::Caption)
          .foreground(MutedForeground)
      })
      // What an item marks to show on hover: hidden at rest, shown while the row is pointed at or holds the
      // focus, and always under a finger, which points at nothing.
      .base(Reveal, |s| s.opacity(0.0))
      .within(Reveal, |s| s.opacity(1.0))
      .density(Density::Touch, Reveal, |s| s.opacity(1.0))
      .state(TreeViewState::Selected, Row, |s| s.background(Highlight))
      .state(TreeViewState::Selected, Icon, |s| s.foreground(Foreground))
      .state(TreeViewState::Selected, Label, |s| s.weight(FontWeight::Medium))
      .state(TreeViewState::Disabled, Row, |s| {
        s.opacity(OpacityLevel::Disabled).cursor(CursorIcon::Default)
      })
  }
}

impl<T: Clone + PartialEq + Send + Sync + 'static> TreeView<T> {
  /// The tree in the engine's virtual list, for a tree of thousands of rows: only the rows near its view are
  /// built, and the keys reach every row, built or not. Up, Down, Home, End, PageUp, PageDown, Right, Left, `*`
  /// and typeahead step over the rows that are shown (the engine's [`FocusRows`](crate::node::FocusRows)), the
  /// list builds and brings into view the row they go to, and the focus goes to it once it is built; Shift with
  /// them and Ctrl+A select over every row.
  ///
  /// The list fills the tree, which is to be given a height: `.style(TreeViewPart::Root, |s| s.height(..))`, or
  /// the room of what holds it. The rows are drawn by [`TreeRow::render`]; a theme's renderer of the whole tree is
  /// not used, its rules are.
  ///
  /// ```
  /// use lurq::{app::ctx::Ctx, core::Signal, node::Element, ui::{TreeItem, TreeView, TreeViewPart}};
  ///
  /// # fn files(ctx: &mut Ctx, items: Vec<TreeItem<u32>>, open: Signal<Vec<u32>>) -> Element {
  /// TreeView::new(items)
  ///   .expanded(open)
  ///   .style(TreeViewPart::Root, |s| s.height(480.0))
  ///   .mount(ctx)
  /// # }
  /// ```
  pub fn mount(self, ctx: &mut Ctx) -> Element {
    ctx.mount::<TreeViewHost<T>>(TreeViewHostProps { tree: self })
  }
}

struct TreeViewHostProps<T> {
  tree: TreeView<T>,
}

impl<T> PartialEq for TreeViewHostProps<T> {
  /// A tree built again is drawn again: what is open and selected is read from the app's signals as it is built.
  fn eq(&self, _: &Self) -> bool {
    false
  }
}

#[cfg(feature = "devtools")]
impl<T> crate::app::component::DevtoolsInspectable for TreeViewHostProps<T> {
  fn inspect(&self, formatter: &mut crate::app::component::DevtoolsFormatter<'_>) {
    formatter.value("TreeView", format!("{} items at the top", self.tree.items.len()));
  }
}

/// The rows a tree in a virtual list built last, which its row components draw.
#[derive(Default)]
struct Shown {
  rows: Vec<TreeRow>,
  gap: SpacingValue,
}

/// The rows of a tree in a virtual list as the engine's focus group steps over them.
struct HostRows {
  data: Mutex<RowsData>,
  reveal_key: Signal<Option<String>>,
  reveal_request: Signal<u64>,
  scroll: ScrollState,
}

#[derive(Default)]
struct RowsData {
  keys: Vec<String>,
  labels: Vec<Arc<str>>,
  levels: Vec<u32>,
  expanded: Vec<Option<bool>>,
  enabled: Vec<bool>,
  toggles: Vec<Option<Arc<dyn Fn(bool) + Send + Sync>>>,
  /// The height of a row and the room after it.
  step: f32,
}

impl FocusRows for HostRows {
  fn count(&self) -> usize {
    self.data.lock().unwrap().keys.len()
  }

  fn label(&self, row: usize) -> Option<String> {
    self.data.lock().unwrap().labels.get(row).map(|label| label.to_string())
  }

  fn enabled(&self, row: usize) -> bool {
    self.data.lock().unwrap().enabled.get(row).copied().unwrap_or(false)
  }

  fn level(&self, row: usize) -> u32 {
    self.data.lock().unwrap().levels.get(row).copied().unwrap_or(1)
  }

  fn expanded(&self, row: usize) -> Option<bool> {
    let data = self.data.lock().unwrap();
    data.toggles.get(row)?.as_ref()?;
    data.expanded.get(row).copied().flatten()
  }

  fn set_expanded(&self, row: usize, open: bool) {
    let toggle = self.data.lock().unwrap().toggles.get(row).cloned().flatten();
    if let Some(toggle) = toggle {
      toggle(open);
    }
  }

  fn page(&self) -> usize {
    let step = self.data.lock().unwrap().step.max(1.0);
    ((self.scroll.viewport_height() / step).floor() as usize).max(1)
  }

  fn reveal(&self, row: usize) {
    let Some(key) = self.data.lock().unwrap().keys.get(row).cloned() else {
      return;
    };
    self.reveal_key.set(Some(key));
    self.reveal_request.update(|request| *request = request.wrapping_add(1));
  }
}

/// A tree in a virtual list: its root, the focus group, holds the list, and the list a row component for each
/// row near its view.
struct TreeViewHost<T> {
  shown: Arc<Mutex<Shown>>,
  rows: Arc<HostRows>,
  generation: AtomicU64,
  _ids: PhantomData<fn() -> T>,
}

impl<T: Clone + PartialEq + Send + Sync + 'static> Component for TreeViewHost<T> {
  type Props = TreeViewHostProps<T>;

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      shown: Arc::default(),
      rows: Arc::new(HostRows {
        data: Mutex::default(),
        reveal_key: ctx.signal(None),
        reveal_request: ctx.signal(0),
        scroll: ScrollState::new(),
      }),
      generation: AtomicU64::new(0),
      _ids: PhantomData,
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let theme = current_theme();
    let rows = self.rows.clone();
    let page: Page = Arc::new(move || rows.page());
    let (mut view, keys) = ctx.props::<TreeViewHostProps<T>>().tree.view(&theme, Some(page));
    for (index, row) in view.rows.iter_mut().enumerate() {
      row.row = Some(index);
    }
    let step = row_step(&view);
    *self.rows.data.lock().unwrap() = RowsData {
      keys: keys.clone(),
      labels: view.rows.iter().map(|row| row.label.clone()).collect(),
      levels: view.rows.iter().map(|row| row.level).collect(),
      expanded: view.rows.iter().map(|row| row.opens.then_some(row.expanded)).collect(),
      enabled: view.rows.iter().map(|row| !row.disabled).collect(),
      toggles: view.rows.iter().map(|row| row.toggle.clone()).collect(),
      step,
    };
    // Every row component draws again from what this render built: what is selected, open and loading.
    let generation = self.generation.fetch_add(1, Ordering::Relaxed) + 1;
    *self.shown.lock().unwrap() = Shown {
      rows: std::mem::take(&mut view.rows),
      gap: view.part(TreeViewPart::Root).rest().gap.unwrap_or_default(),
    };
    let items: Arc<Vec<RowItem>> = Arc::new(
      keys
        .into_iter()
        .enumerate()
        .map(|(index, key)| RowItem { index, key, generation })
        .collect(),
    );
    let shown = self.shown.clone();
    let list = VirtualizedList::new_shared(ctx, items)
      .with_scroll_state(self.rows.scroll.clone())
      .reveal_key(self.rows.reveal_key.get())
      .reveal_request(self.rows.reveal_request.get())
      .estimate_height(move |_| step)
      .width(Dimension::Pct(100.0))
      .flex(1.0)
      .mount_keyed::<TreeRowHost, _, _, _>(
        |item: &RowItem| item.key.clone(),
        move |item: &RowItem| TreeRowProps {
          index: item.index,
          generation: item.generation,
          shown: shown.clone(),
        },
      );
    let rows: Arc<dyn FocusRows> = self.rows.clone();
    view.root().focus_rows(rows).child(list)
  }
}

/// The height of a row and the room after it, as the recipe gives them: what the list places rows it has not
/// built by, and what a page of rows is.
fn row_step(view: &TreeViewView) -> f32 {
  let theme = view.theme();
  let height = view
    .part(TreeViewPart::Row)
    .rest()
    .height
    .and_then(|height| height.resolve(theme))
    .and_then(|height| match height {
      Dimension::Px(px) => Some(px),
      _ => None,
    })
    .unwrap_or(28.0);
  let gap = view
    .part(TreeViewPart::Root)
    .rest()
    .gap
    .map(|gap| gap.resolve(&theme.spacing(), 0.0))
    .unwrap_or(0.0);
  height + gap
}

/// A row of the virtual list: which of the shown rows, from which render.
#[derive(Clone, PartialEq)]
struct RowItem {
  index: usize,
  key: String,
  generation: u64,
}

struct TreeRowProps {
  index: usize,
  generation: u64,
  shown: Arc<Mutex<Shown>>,
}

impl PartialEq for TreeRowProps {
  fn eq(&self, other: &Self) -> bool {
    self.index == other.index && self.generation == other.generation && Arc::ptr_eq(&self.shown, &other.shown)
  }
}

#[cfg(feature = "devtools")]
impl crate::app::component::DevtoolsInspectable for TreeRowProps {
  fn inspect(&self, formatter: &mut crate::app::component::DevtoolsFormatter<'_>) {
    formatter.value("TreeRow", format!("row {}", self.index));
  }
}

/// One row of a tree in a virtual list, with the room after it that the tree's rows have between them.
struct TreeRowHost;

impl Component for TreeRowHost {
  type Props = TreeRowProps;

  fn create(_: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let props = ctx.props::<TreeRowProps>();
    let (row, gap) = {
      let shown = props.shown.lock().unwrap();
      (shown.rows.get(props.index).cloned(), shown.gap.clone())
    };
    let Some(row) = row else {
      return Column::new();
    };
    Column::new()
      .width(Dimension::Pct(100.0))
      .align_items(Alignment::Stretch)
      .spacing(gap)
      .child(row.render())
      // Nothing, after the room under the row.
      .child(Row::new())
  }
}
