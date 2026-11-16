//! A tree view: things inside things, files in folders, each level opened and closed in place.

use std::{
  cell::Cell,
  collections::HashMap,
  marker::PhantomData,
  sync::{
    Arc, Mutex,
    atomic::{AtomicU64, Ordering},
  },
};

use super::{
  icons, names,
  opening_rows::{Moving, OpeningRows, RowMotion},
  part_style::Length,
  recipe::{Case, Interaction, Name, Recipe, ResolvedPart, ResolvedRecipe},
  selection::{ListSelection, Pick},
  shared::{IconChoice, fading, glyph, pressable},
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
  node::{CursorIcon, Element, FocusGroup, FocusRows, FocusStep, SpacingValue, dimension::Dimension, outline::Outline},
  // The engine's clock: the standard one is not there in a browser.
  time::Instant,
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
  /// The chevron of a row that opens: down while it is open, turned to the end while it is closed. A row that does
  /// not open has the room of one, empty; a row whose children are on their way has a spinner in it.
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
  /// What opens and closes under a row that opens, in a tree that builds every row: as high as the rows it holds
  /// while the row is open, of no height while it is closed, with a transition between the two.
  Panel,
  /// The rows in a row's panel: they fade in as it opens and out as it closes.
  Body,
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
  Panel => "panel",
  Body => "body",
});

/// The states of a row of a tree view a recipe styles, besides what the
/// pointer and the keyboard do to it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TreeViewState {
  /// The row is selected.
  Selected,
  /// The row is open: what it holds is shown under it.
  Open,
  /// The row takes no input.
  Disabled,
}

names!(TreeViewState {
  Selected => "selected",
  Open => "open",
  Disabled => "disabled",
});

/// What builds a tree view's nodes: a theme's in place of the default one.
pub type TreeViewRenderer = Arc<dyn Fn(TreeViewView) -> Element + Send + Sync>;

/// One thing in a tree: a folder, a file, a group of settings. It is told
/// apart by its `value`, which the tree's open and selected rows are named by.
///
/// ```
/// use lurq::ui::{TreeItem, icons};
///
/// let src = TreeItem::folder("src", "src")
///   .item(TreeItem::file("src/lib.rs", "lib.rs"))
///   .item(TreeItem::file("src/main.rs", "main.rs").disabled(true));
/// // Children the app brings when the row is first opened.
/// let remote = TreeItem::new("origin", "origin").icon(icons::BRANCH).lazy(true);
/// ```
#[derive(Clone)]
pub struct TreeItem<T> {
  // Read by what else shows a hierarchy of these: a tree select's panel, a column view's columns.
  pub(crate) value: T,
  pub(crate) label: Arc<str>,
  pub(crate) icon: Option<Name>,
  pub(crate) open_icon: Option<Name>,
  pub(crate) trailing: Option<Element>,
  pub(crate) trailing_on_hover: bool,
  pub(crate) disabled: bool,
  pub(crate) lazy: bool,
  pub(crate) items: Vec<TreeItem<T>>,
  pub(crate) id: Option<Arc<str>>,
  pub(crate) key: Option<Arc<str>>,
}

impl<T> TreeItem<T> {
  /// An item of `value` that says `label`, with no icon.
  pub fn new(value: T, label: &str) -> Self {
    Self {
      value,
      label: Arc::from(label),
      icon: None,
      open_icon: None,
      trailing: None,
      trailing_on_hover: false,
      disabled: false,
      lazy: false,
      items: Vec::new(),
      id: None,
      key: None,
    }
  }

  /// A folder: an item with the icon of a folder, and of an open one while it is open.
  pub fn folder(value: T, label: &str) -> Self {
    Self::new(value, label)
      .icon(icons::FOLDER)
      .open_icon(icons::FOLDER_OPEN)
  }

  /// A file: an item with the icon of a file.
  pub fn file(value: T, label: &str) -> Self {
    Self::new(value, label).icon(icons::FILE)
  }

  /// The icon before the label, by its name in the theme's set; `None` for none, in place of a folder's or a
  /// file's.
  pub fn icon(mut self, icon: impl Into<IconChoice>) -> Self {
    self.icon = icon.into().0;
    self
  }

  /// The icon before the label while the item is open, in place of its [`icon`](Self::icon); `None` for the same
  /// icon open and closed.
  pub fn open_icon(mut self, icon: impl Into<IconChoice>) -> Self {
    self.open_icon = icon.into().0;
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
  /// open with no children its chevron is a spinner. The app is told it opened (`TreeView::on_open_change`) and
  /// gives it its children, or makes it not lazy where it has none.
  pub fn lazy(mut self, lazy: bool) -> Self {
    self.lazy = lazy;
    self
  }

  /// An item inside this one, after those it holds already.
  pub fn item(mut self, item: TreeItem<T>) -> Self {
    self.items.push(item);
    self
  }

  /// Items inside this one, after those it holds already.
  pub fn items(mut self, items: impl IntoIterator<Item = TreeItem<T>>) -> Self {
    self.items.extend(items);
    self
  }

  /// The id of the row's element: what is pressed and focused.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  /// What the row is told apart by among the rows, in place of where it is among its siblings: a row that keeps
  /// it keeps its node, and its focus, as rows are put in or taken out before it.
  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }
}

/// How many rows a tree's selection holds, which its constructor says: one ([`OneSelected`], `TreeView::new`,
/// `TreeView::controlled`) or any number ([`ManySelected`], `TreeView::multiple`, `TreeView::controlled_multiple`).
/// It is what [`TreeView::on_change`] is told: the row, `&T`, or the rows, `&[T]`.
pub trait RowSelection<T>: Send + Sync + 'static {
  /// What `on_change` is told.
  type Value: ?Sized;
  /// Whether any number of rows may be selected.
  const MANY: bool;
  /// What `on_change` is told of the rows now selected: of one row, none where none is.
  fn told(selected: &[T]) -> Option<&Self::Value>;
}

/// One row selected at most: what `TreeView::new` and `TreeView::controlled` make.
pub struct OneSelected;

/// Any number of rows selected, with Ctrl and Shift: what `TreeView::multiple` and `TreeView::controlled_multiple`
/// make.
pub struct ManySelected;

impl<T: 'static> RowSelection<T> for OneSelected {
  type Value = T;
  const MANY: bool = false;

  fn told(selected: &[T]) -> Option<&T> {
    selected.last()
  }
}

impl<T: 'static> RowSelection<T> for ManySelected {
  type Value = [T];
  const MANY: bool = true;

  fn told(selected: &[T]) -> Option<&[T]> {
    Some(selected)
  }
}

type Told<T> = Arc<dyn Fn(&T) + Send + Sync>;
type ToldOpen<T> = Arc<dyn Fn(&T, bool) + Send + Sync>;

/// Things inside things, one level under the other: the files of a
/// project in their folders, the layers of a scene, the settings of an app
/// in their groups.
///
/// ```
/// use lurq::{core::Signal, ui::{TreeItem, TreeView}};
///
/// # fn files(open: Signal<Vec<&'static str>>, selected: Signal<Option<&'static str>>) {
/// let tree = TreeView::new(selected, [
///   TreeItem::folder("crates", "crates").item(
///     TreeItem::folder("lurq", "lurq")
///       .item(TreeItem::file("lib.rs", "lib.rs"))
///       .item(TreeItem::file("theme.rs", "theme.rs")),
///   ),
///   TreeItem::file("Cargo.toml", "Cargo.toml"),
/// ])
/// .label("Files")
/// .open(open)
/// .on_activate(|file| println!("open {file}"));
/// // The selection held by the app, which is told each new one.
/// let held = TreeView::controlled(Some("Cargo.toml"), [TreeItem::file("Cargo.toml", "Cargo.toml")], |chosen| {
///   println!("{chosen:?}")
/// });
/// # }
/// ```
///
/// What is open and what is selected are the app's: a signal the tree reads
/// and writes ([`new`](Self::new), [`open`](Self::open)), or what the app says
/// with a handler it is told by ([`controlled`](Self::controlled),
/// [`open_ids`](Self::open_ids)).
/// The rows are one flat list of what is shown: what a closed row holds is
/// not built, and it opens and closes at once.
///
/// A click of a row selects it, a press of its chevron opens or closes it,
/// and a double click activates it. The tree is one stop of Tab, at the
/// selected row or the first. Up and Down move through the rows that are
/// shown, Home and End to the first and the last; Right opens a closed row and
/// goes into an open one; Left closes an open row and goes out of a closed one
/// or one that does not open; `*` opens every row at the level of the focused
/// one; a letter goes to the next row that starts with it. Space selects the
/// focused row and Enter activates it: a row that opens opens or closes, and
/// [`on_activate`](Self::on_activate) is told. With
/// [`TreeView::multiple`] a press with Ctrl (or Cmd) adds a row or takes
/// it out, one with Shift selects the rows from the last selected one to it,
/// Space adds or takes out the focused row, Shift with Up, Down, Home or End
/// moves and selects the rows on the way, and Ctrl+A selects them all.
pub struct TreeView<T: 'static, M: RowSelection<T> = OneSelected> {
  items: Vec<TreeItem<T>>,
  open: Vec<T>,
  toggle: Option<Arc<dyn Fn(&T, bool) + Send + Sync>>,
  selected: Vec<T>,
  select: Option<Arc<dyn Fn(Vec<T>) + Send + Sync>>,
  guides: bool,
  /// Whether a press of a row adds it to what is selected or takes it out, with no key held, and no second press
  /// activates: the tree of a tree select of any number.
  pub(crate) press_toggles: bool,
  /// Whether the row Tab would stop on takes the focus as the overlay the tree is in opens: the tree of a tree
  /// select's panel.
  pub(crate) autofocus: bool,
  disabled: bool,
  label: Option<Arc<str>>,
  on_change: Vec<Arc<dyn Fn(&M::Value) + Send + Sync>>,
  on_activate: Vec<Told<T>>,
  on_open_change: Vec<ToldOpen<T>>,
  states: Vec<Name>,
  rules: Recipe,
  forced: Option<Interaction>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
  _selection: PhantomData<fn() -> M>,
}

impl<T: Clone + PartialEq + Send + Sync + 'static> TreeView<T, OneSelected> {
  /// A tree of `items`, each with what it holds, of which one row is selected at most, bound to `selected`: a click
  /// or Space selects a row, and sets the signal. None is open until the app says.
  pub fn new(selected: Signal<Option<T>>, items: impl IntoIterator<Item = TreeItem<T>>) -> Self
  where
    Option<T>: SignalValue,
  {
    let mut tree = Self::of(items);
    tree.selected = selected.get().into_iter().collect();
    tree.select = Some(Arc::new(move |chosen: Vec<T>| selected.set(chosen.into_iter().last())));
    tree
  }

  /// A tree of `items` of which the app holds the one row selected: `selected`, and `on_change` is told the row a
  /// click or Space selects; the tree shows what the app then gives. Two built at the one place in the app's code,
  /// in a loop, are told apart by their `id` or `key`.
  pub fn controlled(
    selected: Option<T>,
    items: impl IntoIterator<Item = TreeItem<T>>,
    on_change: impl Fn(&T) + Send + Sync + 'static,
  ) -> Self {
    let mut tree = Self::of(items);
    tree.selected = selected.into_iter().collect();
    // The app's handler is told by `on_change`, with the handlers that hear the change.
    tree.select = Some(Arc::new(|_| {}));
    tree.on_change(on_change)
  }
}

impl<T: Clone + PartialEq + Send + Sync + 'static> TreeView<T, ManySelected> {
  /// A tree of `items` of which any number of rows are selected, with Ctrl and Shift as a list of files has them,
  /// bound to `selected`. The last of it is where a range selected with Shift starts.
  pub fn multiple(selected: Signal<Vec<T>>, items: impl IntoIterator<Item = TreeItem<T>>) -> Self
  where
    Vec<T>: SignalValue,
  {
    let mut tree = Self::of(items);
    tree.selected = selected.get();
    tree.select = Some(Arc::new(move |chosen| selected.set(chosen)));
    tree
  }

  /// A tree of `items` of which the app holds the rows selected: those of `selected`, and `on_change` is told the
  /// whole new selection after each press or key that changes it; the tree shows what the app then gives. The last
  /// is where a range selected with Shift starts. Two built at the one place in the app's code, in a loop, are told
  /// apart by their `id` or `key`.
  pub fn controlled_multiple(
    selected: impl IntoIterator<Item = T>,
    items: impl IntoIterator<Item = TreeItem<T>>,
    on_change: impl Fn(&[T]) + Send + Sync + 'static,
  ) -> Self {
    let mut tree = Self::of(items);
    tree.selected = selected.into_iter().collect();
    tree.select = Some(Arc::new(|_| {}));
    tree.on_change(on_change)
  }
}

impl<T: Clone + PartialEq + Send + Sync + 'static, M: RowSelection<T>> TreeView<T, M> {
  fn of(items: impl IntoIterator<Item = TreeItem<T>>) -> Self {
    Self {
      items: items.into_iter().collect(),
      open: Vec::new(),
      toggle: None,
      selected: Vec::new(),
      select: None,
      guides: false,
      press_toggles: false,
      autofocus: false,
      disabled: false,
      label: None,
      on_change: Vec::new(),
      on_activate: Vec::new(),
      on_open_change: Vec::new(),
      states: Vec::new(),
      rules: Recipe::new(),
      forced: None,
      id: None,
      key: None,
      _selection: PhantomData,
    }
  }

  /// The rows that are open: those whose values the signal has. Opening a row adds it, closing one takes it out.
  pub fn open(mut self, open: Signal<Vec<T>>) -> Self
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
  pub fn open_ids(
    mut self,
    open: impl IntoIterator<Item = T>,
    toggle: impl Fn(&T, bool) + Send + Sync + 'static,
  ) -> Self {
    self.open = open.into_iter().collect();
    self.toggle = Some(Arc::new(toggle));
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

  /// Called with what a click, Space, a key with Shift or Ctrl+A selected: the row (`&T`) of a tree of one, the whole
  /// new selection (`&[T]`) of a tree of any number. Not called for a row that was selected already, nor when the app
  /// sets the selection.
  pub fn on_change(mut self, handler: impl Fn(&M::Value) + Send + Sync + 'static) -> Self {
    self.on_change.push(Arc::new(handler));
    self
  }

  /// Called with the row a double press or Enter activates: open the file.
  pub fn on_activate(mut self, handler: impl Fn(&T) + Send + Sync + 'static) -> Self {
    self.on_activate.push(Arc::new(handler));
    self
  }

  /// Called with each row a press or a key opens (`true`) or closes: where a lazy item's children are brought as
  /// it opens.
  pub fn on_open_change(mut self, handler: impl Fn(&T, bool) + Send + Sync + 'static) -> Self {
    self.on_open_change.push(Arc::new(handler));
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

  /// Draws the row Tab would stop on (the first selected, else the first that is not disabled) as it is under
  /// the pointer, held down or focused from the keyboard, with nothing pointing at it: for a gallery or a test.
  /// The other rows are drawn as they are.
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
    let (view, ..) = self.view(theme, None, None);
    match theme.renderer::<TreeView<T>>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }

  /// The tree as a renderer sees it, in `theme`: every shown row, with a key of each that stays its own while
  /// rows open and close before it. `page` says how many rows PageUp and PageDown move by, in a tree in a virtual
  /// list; `motion`, how its rows open and close there: the rows of a closing row are in the view too, kept while
  /// they close, and each row says which row in motion it is under and whether it is kept.
  fn view(
    &self,
    theme: &Theme,
    page: Option<Page>,
    motion: Option<(&Arc<OpeningRows>, &Moving)>,
  ) -> (TreeViewView, Vec<String>, Vec<Under>) {
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
    let mut recipe = |selected: bool, open: bool, disabled: bool| {
      recipes
        .entry((selected, open, disabled))
        .or_insert_with(|| {
          let case = base
            .clone()
            .state(TreeViewState::Selected, selected)
            .state(TreeViewState::Open, open)
            .state(TreeViewState::Disabled, disabled);
          theme.resolve::<TreeView<T>>(&case, Some(&self.rules))
        })
        .clone()
    };

    let mut flat = Vec::new();
    let moving = motion.map(|(_, moving)| moving);
    flatten(
      &self.items,
      1,
      "",
      &self.open,
      self.disabled,
      (moving, None, false),
      &mut flat,
    );

    let multiple = M::MANY;
    // What writes a new selection tells the handlers of the change too: the row of a tree of one, the rows of one of
    // any number.
    let write = self.select.clone().map(|write| {
      let told = self.on_change.clone();
      Arc::new(move |next: Vec<T>| {
        let held = next.clone();
        write(next);
        if let Some(value) = M::told(&held) {
          for handler in &told {
            handler(value);
          }
        }
      }) as Arc<dyn Fn(Vec<T>) + Send + Sync>
    });
    let model = Arc::new(Model {
      selection: ListSelection::new(
        flat.iter().map(|row| row.value.clone()).collect(),
        // A row kept while it closes is not reached: it is going.
        flat.iter().map(|row| !row.disabled && !row.kept).collect(),
        self.selected.clone(),
        multiple,
        write,
      ),
      on_activate: self.on_activate.clone(),
      page,
    });
    let entry = flat
      .iter()
      .position(|row| !row.disabled && !row.kept && self.selected.contains(&row.value));
    // The forced interaction is drawn on the row Tab would stop on alone, as it can only be: the entry, else
    // the first row that is not disabled.
    let forced_at = entry.or_else(|| flat.iter().position(|row| !row.disabled && !row.kept));

    let keys = flat.iter().map(|row| row.key.clone()).collect();
    let under = flat
      .iter_mut()
      .map(|row| Under {
        parent: row.under.take(),
        kept: row.kept,
      })
      .collect();
    let motion = motion.map(|(motion, _)| motion.clone());
    let rows = flat
      .into_iter()
      .enumerate()
      .map(|(index, row)| {
        let selected = self.selected.contains(&row.value);
        let path: Arc<str> = row.key.into();
        let toggle = (row.opens && !row.disabled && !row.kept)
          .then(|| self.toggle.clone())
          .flatten()
          .map(|write| {
            let (value, told) = (row.value.clone(), self.on_open_change.clone());
            let (motion, path) = (motion.clone(), path.clone());
            Arc::new(move |open: bool| {
              // In a virtual list what it holds starts to open or close as it does.
              if let Some(motion) = &motion {
                motion.started(&path, open);
              }
              write(&value, open);
              for handler in &told {
                handler(&value, open);
              }
            }) as Arc<dyn Fn(bool) + Send + Sync>
          });
        let actions = (!row.disabled && !row.kept).then(|| {
          let model = model.clone();
          Arc::new(RowActions {
            pick: {
              let model = model.clone();
              Arc::new(move |pick| model.pick(index, pick))
            },
            select_all: {
              let model = model.clone();
              Arc::new(move || model.select_all())
            },
            extend: {
              let model = model.clone();
              Arc::new(move |reach| model.extend(index, reach))
            },
            activate: {
              let (toggle, open) = (toggle.clone(), row.open);
              Arc::new(move || {
                if let Some(toggle) = &toggle {
                  toggle(!open);
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
          open: row.open,
          loading: row.loading,
          selected,
          disabled: row.disabled,
          icon: if row.open { row.open_icon.or(row.icon) } else { row.icon },
          trailing: row.trailing,
          trailing_on_hover: row.trailing_on_hover,
          position: (row.position, row.siblings),
          guides: self.guides,
          entry: entry == Some(index),
          index,
          multiple,
          press_toggles: self.press_toggles,
          autofocus: self.autofocus && forced_at == Some(index),
          recipe: recipe(selected, row.open, row.disabled),
          forced: self.forced.filter(|_| forced_at == Some(index)),
          toggle,
          actions,
          row: None,
          path,
          id: row.id,
          key: row.app_key,
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
    (view, keys, under)
  }
}

/// Which row in motion a row of a tree in a virtual list is under, and whether it is kept while that row closes.
struct Under {
  parent: Option<Arc<str>>,
  kept: bool,
}

impl<T: Clone + PartialEq + Send + Sync + 'static, M: RowSelection<T>> From<TreeView<T, M>> for Element {
  fn from(tree: TreeView<T, M>) -> Self {
    tree.build(&current_theme())
  }
}

/// A row as the tree lays it out: an item that is shown, with where it is.
struct Flat<T> {
  value: T,
  label: Arc<str>,
  icon: Option<Name>,
  open_icon: Option<Name>,
  trailing: Option<Element>,
  trailing_on_hover: bool,
  disabled: bool,
  opens: bool,
  open: bool,
  loading: bool,
  level: u32,
  position: usize,
  siblings: usize,
  id: Option<Arc<str>>,
  /// The key the app gave it.
  app_key: Option<Arc<str>>,
  /// Where it is among the items, from the top: `0/2/1`, or the key the app gave it.
  key: String,
  /// The outermost row it is under that opens or closes, in a tree in a virtual list.
  under: Option<Arc<str>>,
  /// Whether it is under a row that closes, kept while it does.
  kept: bool,
}

/// What a tree in a virtual list flattens its rows with: where its rows are in motion, the outermost row in motion
/// those of the level are under, and whether they are kept while it closes.
type Motion<'a> = (Option<&'a Moving>, Option<Arc<str>>, bool);

/// The rows of `items` at `level` and of what is open in them, in the order they are shown, and in a tree in a
/// virtual list those of what is closing, kept while it closes.
fn flatten<T: Clone + PartialEq>(
  items: &[TreeItem<T>],
  level: u32,
  parent: &str,
  open: &[T],
  disabled: bool,
  (moving, under, kept): Motion<'_>,
  out: &mut Vec<Flat<T>>,
) {
  let siblings = items.len();
  for (position, item) in items.iter().enumerate() {
    let opens = item.lazy || !item.items.is_empty();
    let is_open = opens && open.contains(&item.value);
    let row_disabled = disabled || item.disabled;
    let key = match &item.key {
      Some(key) => key.to_string(),
      None if parent.is_empty() => position.to_string(),
      None => format!("{parent}/{position}"),
    };
    out.push(Flat {
      loading: is_open && item.lazy && item.items.is_empty(),
      value: item.value.clone(),
      label: item.label.clone(),
      icon: item.icon,
      open_icon: item.open_icon,
      trailing: item.trailing.clone(),
      trailing_on_hover: item.trailing_on_hover,
      disabled: row_disabled,
      opens,
      open: is_open,
      level,
      position: position + 1,
      siblings,
      id: item.id.clone(),
      app_key: item.key.clone(),
      key: key.clone(),
      under: under.clone(),
      kept,
    });
    let closing = opens && !is_open && moving.is_some_and(|moving| moving.keeps(&key));
    if is_open || closing {
      // What is in it is under the outermost row in motion: this one, if none it is in is.
      let under = under.clone().or_else(|| {
        moving
          .filter(|moving| moving.moves(&key))
          .map(|_| Arc::from(key.as_str()))
      });
      flatten(
        &item.items,
        level + 1,
        &key,
        open,
        disabled,
        (moving, under, kept || closing),
        out,
      );
    }
  }
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
  selection: ListSelection<T>,
  on_activate: Vec<Told<T>>,
  /// How many rows PageUp and PageDown move by, in a tree in a virtual list.
  page: Option<Page>,
}

/// How many rows PageUp and PageDown move by.
type Page = Arc<dyn Fn() -> usize + Send + Sync>;

impl<T: Clone + PartialEq> Model<T> {
  fn pick(&self, index: usize, pick: Pick) {
    self.selection.pick(index, pick);
  }

  /// Shift with a key on the row `from`: the rows from the anchor to where the focus goes. The tree steps the
  /// selection itself, and not with a press of the row the focus goes to: in a virtual list that row may not be
  /// built.
  fn extend(&self, from: usize, reach: Reach) {
    // Where the group's keys take the focus: the engine's step over the rows that take it, without going round.
    let step = match reach {
      Reach::Back => FocusStep::Back,
      Reach::On => FocusStep::On,
      Reach::First => FocusStep::First,
      Reach::Last => FocusStep::Last,
      Reach::PageBack => FocusStep::PageBack,
      Reach::PageOn => FocusStep::PageOn,
    };
    // PageUp and PageDown are keys of a tree in a virtual list only.
    self.selection.extend(from, step, self.page.as_ref().map(|page| page()));
  }

  fn select_all(&self) {
    self.selection.select_all();
  }

  fn activate(&self, index: usize) {
    for handler in &self.on_activate {
      handler(self.selection.id(index));
    }
  }
}

thread_local! {
  /// The last click of a row: which row of its tree, when and where. A second click of it soon after at the same
  /// place, as the engine counts a double press, activates it.
  static LAST_PRESS: Cell<Option<(usize, Instant, (f32, f32))>> = const { Cell::new(None) };
}

/// Whether a click of the row `index` is the second of a double press: then the next is a first again.
pub(crate) fn second_press(index: usize, event: &MouseEvent) -> bool {
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

/// What a row that takes input does, with the type of the tree's ids out of sight.
struct RowActions {
  pick: Arc<dyn Fn(Pick) + Send + Sync>,
  select_all: Arc<dyn Fn() + Send + Sync>,
  activate: Arc<dyn Fn() + Send + Sync>,
  extend: Arc<dyn Fn(Reach) + Send + Sync>,
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

  /// How the default renderer builds a tree: its rows one under the other, what a row that opens holds in its
  /// [`panel`](TreeViewPart::Panel) under it, which opens and closes over them as an accordion's item does.
  pub fn render(mut self) -> Element {
    let rows = std::mem::take(&mut self.rows);
    let gap = self.part(TreeViewPart::Root).rest().gap.unwrap_or_default();
    let children = nest(&mut rows.into_iter().peekable(), &gap);
    self.root().with_children(children).into()
  }
}

/// The rows from the next on, of its level and of what is in them, each row that opens with its panel under it: the
/// rows in a panel are the room of the root's gap apart, and as far under the row it is under.
fn nest(rows: &mut std::iter::Peekable<std::vec::IntoIter<TreeRow>>, gap: &SpacingValue) -> Vec<Element> {
  let mut out = Vec::new();
  let Some(level) = rows.peek().map(|row| row.level) else {
    return out;
  };
  while let Some(row) = rows.next_if(|next| next.level >= level) {
    if !row.opens {
      out.push(row.render());
      continue;
    }
    let (at, open, path) = (row.level, row.open, row.path.clone());
    let mut panel = Column::new()
      .width(Dimension::Pct(100.0))
      .align_items(Alignment::Stretch)
      .clip()
      .presence()
      .key("panel")
      .part(&row.part(TreeViewPart::Panel));
    if open && rows.peek().is_some_and(|next| next.level > at) {
      panel = panel.child(
        Column::new()
          .key("rows")
          .align_items(Alignment::Stretch)
          .spacing(gap.clone())
          .padding_top(gap.clone())
          .part(&row.part(TreeViewPart::Body))
          .with_children(nest(rows, gap)),
      );
    }
    out.push(
      Column::new()
        .key(format!("node-{path}"))
        .align_items(Alignment::Stretch)
        .child(row.render())
        .child(panel)
        .into(),
    );
  }
  out
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
  pub open: bool,
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
  press_toggles: bool,
  autofocus: bool,
  /// The style of each part of this row, in its case.
  pub recipe: ResolvedRecipe,
  forced: Option<Interaction>,
  toggle: Option<Arc<dyn Fn(bool) + Send + Sync>>,
  actions: Option<Arc<RowActions>>,
  /// Which row of the group's rows it is, in a tree in a virtual list.
  row: Option<usize>,
  /// Where it is among the items, from the top, or the key the app gave it: what its panel is known by.
  path: Arc<str>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
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
    if self.autofocus {
      row = row.autofocus(true);
    }
    if self.opens {
      row = row.describe("a11y_expanded", flag(self.open));
    }
    if self.loading {
      row = row.describe("a11y_busy", "true");
    }
    if let Some(toggle) = self.toggle.clone() {
      row = row.focus_expanded(self.open, move |open| toggle(open));
    }
    if let Some(actions) = self.actions.clone() {
      let multiple = self.multiple;
      // A click selects, as the button comes up over the row it went down on: a press dragged off the row
      // selects nothing. A second click of the row soon after at the same place activates it. The row has no
      // `on_dblclick`: the engine holds back the click of an element that has one until it knows it is not the
      // first of a double press, and the focus with it. Space clicks the focused row: it selects too.
      let press = actions.clone();
      let (index, toggles) = (self.index, self.press_toggles);
      row = row.on_click(move |event: MouseEvent| {
        if event.from_keyboard {
          (press.pick)(Pick::of(&event, multiple));
          return;
        }
        if event.button != MouseButton::Left {
          return;
        }
        if toggles {
          // Each press is a choice of its own: in or out, or with Shift the rows from the last chosen.
          (press.pick)(if event.shift { Pick::Range } else { Pick::Toggle });
        } else if second_press(index, &event) {
          (press.activate)();
        } else {
          (press.pick)(Pick::of(&event, false));
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
    if let Some(key) = &self.key {
      row = row.key(key.clone());
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
    // One chevron, down, turned to the end while the row is closed: it turns as the row opens and closes, over the
    // opening's duration, at once where motion is reduced.
    let theme = self.theme();
    let mut chevron = Row::new()
      .transform(crate::node::transform::Transform2D::rotate_deg(if self.open {
        0.0
      } else {
        -90.0
      }))
      .transition(
        crate::animation::Transition::transform()
          .duration(theme.duration_value(crate::app::theme::MotionDuration::Base))
          .easing(theme.easing_value(crate::app::theme::MotionEasing::Standard)),
      )
      .child(glyph(&part, Name::new(icons::CHEVRON_DOWN), false));
    if let Some(toggle) = self.toggle.clone() {
      let open = !self.open;
      // The press is the chevron's, as the button goes down: it does not select the row as well, and is not the
      // first of a double press of the row.
      chevron = chevron.on_mouse_down(move |event: MouseEvent| {
        event.stop_propagation();
        LAST_PRESS.with(|last| last.set(None));
        if event.button == MouseButton::Left {
          toggle(open);
        }
      });
      chevron = chevron.on_click(|event: MouseEvent| event.stop_propagation());
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

/// Every tree has the one recipe and renderer, whatever its values are and however many it selects.
impl<T: Send + Sync + 'static, M: RowSelection<T>> Themed for TreeView<T, M> {
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
    use TreeViewPart::{Body, Chevron, Content, Guide, Icon, Indent, Label, Panel, Reveal, Root, Row, Trailing};
    let recipe = Recipe::new()
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
      // A row of the density's height, 28 at the default, 20, 36 and 40 at the others; its guide with it.
      .row_density(super::Selector::new(), Row, 28.0)
      .row_density(super::Selector::new(), Guide, 30.0)
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
      });
    // A row opens and closes as an accordion's item does: its panel's height over the theme's base duration, its
    // rows fading in and out.
    super::collapsible::opening(recipe, TreeViewState::Open.into(), Panel.into(), Body.into())
  }
}

impl<T: Clone + PartialEq + Send + Sync + 'static, M: RowSelection<T>> TreeView<T, M> {
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
  /// TreeView::new(Signal::new(None), items)
  ///   .open(open)
  ///   .style(TreeViewPart::Root, |s| s.height(480.0))
  ///   .mount(ctx)
  /// # }
  /// ```
  pub fn mount(self, ctx: &mut Ctx) -> Element {
    ctx.mount::<TreeViewHost<T, M>>(TreeViewHostProps { tree: self })
  }
}

struct TreeViewHostProps<T: 'static, M: RowSelection<T>> {
  tree: TreeView<T, M>,
}

impl<T, M: RowSelection<T>> PartialEq for TreeViewHostProps<T, M> {
  /// A tree built again is drawn again: what is open and selected is read from the app's signals as it is built.
  fn eq(&self, _: &Self) -> bool {
    false
  }
}

#[cfg(feature = "devtools")]
impl<T, M: RowSelection<T>> crate::app::component::DevtoolsInspectable for TreeViewHostProps<T, M> {
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
struct TreeViewHost<T, M> {
  shown: Arc<Mutex<Shown>>,
  rows: Arc<HostRows>,
  generation: AtomicU64,
  /// The rows that open and close, as a property grid's in a virtual list do.
  motion: Arc<OpeningRows>,
  /// Moved on as the motion does, to render the tree.
  wake: Signal<u64>,
  _ids: PhantomData<fn() -> (T, M)>,
}

impl<T: Clone + PartialEq + Send + Sync + 'static, M: RowSelection<T>> Component for TreeViewHost<T, M> {
  type Props = TreeViewHostProps<T, M>;

  fn create(ctx: &mut Ctx) -> Self {
    let wake = ctx.signal(0);
    Self {
      motion: OpeningRows::new(ctx, &wake),
      wake,
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
    // Read so that the motion moving on renders the tree.
    let _ = self.wake.get();
    let moving = self.motion.read(&theme);
    let rows = self.rows.clone();
    let page: Page = Arc::new(move || rows.page());
    let (mut view, keys, under) =
      ctx
        .props::<TreeViewHostProps<T, M>>()
        .tree
        .view(&theme, Some(page), Some((&self.motion, &moving)));
    for (index, row) in view.rows.iter_mut().enumerate() {
      row.row = Some(index);
    }
    let step = row_step(&view);
    // Each row in a frame of its height, which runs from nothing to it as the row it is under opens and back as it
    // closes. The rows under a row of thousands move in turn as many as the view holds, and the rest with the last.
    let in_view = self.rows.page() + 1;
    let mut counts: HashMap<&str, usize> = HashMap::new();
    for row in &under {
      if let Some(parent) = &row.parent {
        *counts.entry(&**parent).or_default() += 1;
      }
    }
    let mut places: HashMap<&str, usize> = HashMap::new();
    let motions: Vec<RowMotion> = under
      .iter()
      .map(|row| match &row.parent {
        Some(parent) => {
          let place = places.entry(&**parent).or_default();
          let motion = moving.row(parent, *place, counts[&**parent].min(in_view), step, row.kept);
          *place += 1;
          motion
        }
        None => moving.still(step),
      })
      .collect();
    self.motion.rendered(&moving);
    *self.rows.data.lock().unwrap() = RowsData {
      keys: keys.clone(),
      labels: view.rows.iter().map(|row| row.label.clone()).collect(),
      levels: view.rows.iter().map(|row| row.level).collect(),
      expanded: view.rows.iter().map(|row| row.opens.then_some(row.open)).collect(),
      // A row kept while the row it is under closes is not reached: it is going.
      enabled: view
        .rows
        .iter()
        .zip(&under)
        .map(|(row, under)| !row.disabled && !under.kept)
        .collect(),
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
        .zip(motions)
        .enumerate()
        .map(|(index, (key, motion))| RowItem {
          index,
          key,
          generation,
          motion,
        })
        .collect(),
    );
    let shown = self.shown.clone();
    let list = VirtualizedList::new_shared(ctx, items)
      .with_scroll_state(self.rows.scroll.clone())
      .reveal_key(self.rows.reveal_key.get())
      .reveal_request(self.rows.reveal_request.get())
      // A row not measured yet is where its height starts while the row it is under moves: what is below is built
      // where it is on the way.
      .estimate_height(|item: &RowItem| item.motion.estimate)
      .width(Dimension::Pct(100.0))
      .flex(1.0)
      .mount_keyed::<TreeRowHost, _, _, _>(
        |item: &RowItem| item.key.clone(),
        move |item: &RowItem| TreeRowProps {
          index: item.index,
          generation: item.generation,
          motion: item.motion.clone(),
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

/// A row of the virtual list: which of the shown rows, from which render, and how it moves.
#[derive(Clone, PartialEq)]
struct RowItem {
  index: usize,
  key: String,
  generation: u64,
  motion: RowMotion,
}

struct TreeRowProps {
  index: usize,
  generation: u64,
  motion: RowMotion,
  shown: Arc<Mutex<Shown>>,
}

impl PartialEq for TreeRowProps {
  fn eq(&self, other: &Self) -> bool {
    self.index == other.index
      && self.generation == other.generation
      && self.motion == other.motion
      && Arc::ptr_eq(&self.shown, &other.shown)
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
      return Column::new().into();
    };
    let row = Column::new()
      .width(Dimension::Pct(100.0))
      .align_items(Alignment::Stretch)
      .spacing(gap)
      .child(row.render())
      // Nothing, after the room under the row.
      .child(Row::new());
    // In a frame of its height, which runs to the height the row it is under gives it as it opens and closes.
    props.motion.frame(row)
  }
}
