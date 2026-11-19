//! A column view: a hierarchy as columns side by side, each showing what the item chosen in the one before it
//! holds, as a file browser's Miller columns.

use std::sync::Arc;

use parking_lot::Mutex;

use super::{
  icons, names,
  part_style::{Length, PartStyle},
  recipe::{Case, Interaction, Name, Recipe, ResolvedPart, ResolvedRecipe, Selector},
  shared::{fading, glyph, pressable},
  spinner::{Spinner, SpinnerPart},
  theme::{Themed, current_theme},
  tree_view::{TreeItem, second_press},
};
use crate::{
  app::{
    component::Component,
    ctx::Ctx,
    events::{KeyboardEvent, MouseButton, MouseEvent},
    theme::{BorderSize, Density, IconSize, OpacityLevel, PaletteColor, RadiusSize, Theme, TypographyStyle},
  },
  components::{self, Column, Row, ScrollHorizontal, ScrollVertical},
  core::{Signal, SignalValue},
  layout::{
    Alignment,
    layout_kind::{Justify, ScrollState},
  },
  node::{
    CursorIcon, Element, FocusGroup, border::Border, color::Color, dimension::Dimension, node_kind::TextOverflow,
    outline::Outline,
  },
};

/// The parts of a column view a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ColumnViewPart {
  /// The box around the columns: its size, its fill, its edge and its corners.
  Root,
  /// One column: its least width, its padding, the room between its items, and the line at its end.
  Column,
  /// One item: what is pressed, chosen and focused.
  Item,
  /// The icon before the label.
  Icon,
  /// What the item says.
  Label,
  /// What the app puts at the item's end, before its chevron: a count, a badge.
  Trailing,
  /// Around what is at the end of an item marked to show it on hover: hidden until the pointer is on the item or
  /// the focus is on it.
  Reveal,
  /// The chevron at the end of an item that holds others.
  Chevron,
  /// What a column shows while the app brings what its item holds: the room around its spinner.
  Status,
  /// The spinner of that column: its side.
  Spinner,
}

names!(ColumnViewPart {
  Root => "root",
  Column => "column",
  Item => "item",
  Icon => "icon",
  Label => "label",
  Trailing => "trailing",
  Reveal => "reveal",
  Chevron => "chevron",
  Status => "status",
  Spinner => "spinner",
});

/// The states of a column view a recipe styles, besides what the pointer and the keyboard do to it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ColumnViewState {
  /// Of an item: it is on the path chosen, and what it holds is the next column.
  Selected,
  /// Of an item: it is the end of the path, the one chosen last.
  Current,
  /// Of a column: it is the last one, with no line at its end.
  Last,
  /// Of an item: it takes no input. Of the view: nothing in it does.
  Disabled,
}

names!(ColumnViewState {
  Selected => "selected",
  Current => "current",
  Last => "last",
  Disabled => "disabled",
});

/// What builds a column view's nodes: a theme's in place of the default one.
pub type ColumnViewRenderer = Arc<dyn Fn(ColumnViewView) -> Element + Send + Sync>;

type Told<T> = Arc<dyn Fn(&T) + Send + Sync>;
type ToldPath<T> = Arc<dyn Fn(&[T]) + Send + Sync>;

/// A hierarchy as columns side by side: the first shows what is at the top, and each one after it what the item
/// chosen in the one before holds. The folders of a disk, the groups of a catalogue, the sections of a manual.
///
/// ```
/// use lurq::{core::Signal, ui::{ColumnView, TreeItem}};
///
/// # fn files(path: Signal<Vec<&'static str>>) {
/// let files = ColumnView::new(path, [
///   TreeItem::folder("crates", "crates").item(
///     TreeItem::folder("lurq", "lurq")
///       .item(TreeItem::folder("src", "src"))
///       .item(TreeItem::file("Cargo.toml", "Cargo.toml")),
///   ),
///   TreeItem::folder("docs", "docs"),
/// ])
/// .label("Files")
/// .on_activate(|file| println!("open {file}"));
/// // The path held by the app, which is told each new one.
/// let held = ColumnView::controlled(["docs"], [TreeItem::folder("docs", "docs")], |path| println!("{path:?}"));
/// # }
/// ```
///
/// Its items are a tree's ([`TreeItem`]): a value, a label, an icon, what stands at its end, disabled, and the items
/// it holds. What it holds is the path chosen: the values from an item at the top down to the item chosen last,
/// the app's signal ([`new`](Self::new)) or what the app says with a handler it is told by
/// ([`controlled`](Self::controlled)). A press of an item ends the path at it, and the column after its own shows
/// what it holds. An item marked [`lazy`](TreeItem::lazy) that has no items yet has a column with a spinner: the app
/// hears the path and gives the item its children.
///
/// The view is one stop of Tab, at the item chosen last or the first. The choice follows the focus: Up and Down go
/// through the items of a column, Home and End to its first and its last; Right goes into what the item holds, to
/// its first item, and Left back to the item the column is of. A letter goes to the next item that starts with it.
/// Enter and a double press activate the item ([`on_activate`](Self::on_activate)). The columns share the view's
/// width, none narrower than its part says, and scroll sideways where they do not fit;
/// [mounted](Self::mount), the view starts at the end of its path and brings the last column into sight as the
/// path grows.
pub struct ColumnView<T: 'static = ()> {
  items: Vec<TreeItem<T>>,
  path: Vec<T>,
  write: Option<Arc<dyn Fn(Vec<T>) + Send + Sync>>,
  on_change: Vec<ToldPath<T>>,
  on_activate: Vec<Told<T>>,
  press_activates: bool,
  activate_branches: bool,
  open_on_hover: bool,
  /// Whether the item Tab would stop on takes the focus as it is first built: in a panel that opens, a cascader's.
  pub(crate) autofocus: bool,
  disabled: bool,
  label: Option<Arc<str>>,
  loading_label: Arc<str>,
  states: Vec<Name>,
  rules: Recipe,
  forced: Option<Interaction>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl<T: Clone + PartialEq + Send + Sync + 'static> ColumnView<T> {
  /// A column view of `items` bound to `path`: the items whose values the signal has, from the top down, are
  /// chosen, and choosing another sets it to the path that ends at that item.
  pub fn new(path: Signal<Vec<T>>, items: impl IntoIterator<Item = TreeItem<T>>) -> Self
  where
    Vec<T>: SignalValue,
  {
    let mut view = Self::of(items);
    view.path = path.get();
    view.write = Some(Arc::new(move |next| path.set(next)));
    view
  }

  /// A column view of `items` whose path the app holds: `path`, and `on_change` is told the path a press or a key
  /// asks for; the view shows what the app then gives. Two built at the one place in the app's code, in a loop, are
  /// told apart by their `id` or `key`.
  pub fn controlled(
    path: impl IntoIterator<Item = T>,
    items: impl IntoIterator<Item = TreeItem<T>>,
    on_change: impl Fn(&[T]) + Send + Sync + 'static,
  ) -> Self {
    let mut view = Self::of(items);
    view.path = path.into_iter().collect();
    view.on_change(on_change)
  }

  fn of(items: impl IntoIterator<Item = TreeItem<T>>) -> Self {
    Self {
      items: items.into_iter().collect(),
      path: Vec::new(),
      write: None,
      on_change: Vec::new(),
      on_activate: Vec::new(),
      press_activates: false,
      activate_branches: true,
      open_on_hover: false,
      autofocus: false,
      disabled: false,
      label: None,
      loading_label: super::current_locale().common.loading.clone(),
      states: Vec::new(),
      rules: Recipe::new(),
      forced: None,
      id: None,
      key: None,
    }
  }

  /// Called with the whole new path after a press or a key changes it, after the signal is set: the values from the
  /// top down to the item chosen. Not called for the path chosen already, nor when the app sets the signal. A second
  /// handler is called after the first.
  pub fn on_change(mut self, handler: impl Fn(&[T]) + Send + Sync + 'static) -> Self {
    self.on_change.push(Arc::new(handler));
    self
  }

  /// Called with the item a double press or Enter activates: open the file.
  pub fn on_activate(mut self, handler: impl Fn(&T) + Send + Sync + 'static) -> Self {
    self.on_activate.push(Arc::new(handler));
    self
  }

  /// Whether a single press of an item activates it as well as choosing it, in place of a double press: a menu of
  /// columns, a [cascader](super::Cascader)'s.
  pub fn press_activates(mut self, activates: bool) -> Self {
    self.press_activates = activates;
    self
  }

  /// Whether an item that holds others is activated by Enter and by the press that activates. It is unless told
  /// otherwise; without it Enter goes into what the item holds, as Right does, and only an item that holds nothing
  /// is activated: a cascader of which only the leaves are chosen.
  pub fn activate_branches(mut self, activate: bool) -> Self {
    self.activate_branches = activate;
    self
  }

  /// Whether the pointer on an item that holds others chooses it, opening its column, as a menu's submenu opens.
  pub fn open_on_hover(mut self, on_hover: bool) -> Self {
    self.open_on_hover = on_hover;
    self
  }

  /// Whether no item takes input.
  pub fn disabled(mut self, disabled: bool) -> Self {
    self.disabled = disabled;
    self
  }

  /// What the view is called, to what reads the tree of the window. It draws nothing.
  pub fn label(mut self, label: &str) -> Self {
    self.label = Some(Arc::from(label));
    self
  }

  /// What the spinner of a column whose items are on their way is called to what reads the tree: "Loading" unless
  /// it is told otherwise.
  pub fn loading_label(mut self, label: &str) -> Self {
    self.loading_label = Arc::from(label);
    self
  }

  /// Puts the view in a state the theme's recipe names, or takes it out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// Styles one part of this view over what the theme gives it: `style(ColumnViewPart::Root, |s| s.height(320.0))`.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(PartStyle) -> PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for this view.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// Draws the item Tab would stop on (the one chosen last, else the first that is not disabled) as it is under the
  /// pointer, held down or focused from the keyboard, with nothing pointing at it: for a gallery or a test.
  pub fn force(mut self, interaction: Option<Interaction>) -> Self {
    self.forced = interaction;
    self
  }

  /// The id of the root. Each column has this id with `-column-` and its place after it, from 0.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// Builds the view in `theme`, with nothing kept. Converting a view into an element does this in the theme of
  /// what is being rendered.
  pub fn build(self, theme: &Theme) -> Element {
    self.build_with(theme, None)
  }

  /// The view mounted where it is declared: it keeps where its columns are scrolled to, starts at the end of its
  /// path, and brings the last column into sight when the path grows.
  pub fn mount(self, ctx: &mut Ctx) -> Element {
    let key = self.key.clone();
    let props = ColumnViewProps { view: self };
    match key {
      Some(key) => ctx.mount_keyed::<ColumnViewHost<T>>(&key, props).key(key),
      None => ctx.mount::<ColumnViewHost<T>>(props),
    }
  }

  fn build_with(&self, theme: &Theme, kept: Option<Kept>) -> Element {
    let view = self.view(theme, kept);
    match theme.renderer::<ColumnView<T>>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }

  /// The view as a renderer sees it, in `theme`: a column for the top and one for each item of the path that holds
  /// others.
  fn view(&self, theme: &Theme, kept: Option<Kept>) -> ColumnViewView {
    let base = Case {
      variant: None,
      size: None,
      density: None,
      breakpoint: super::current_breakpoint(),
      states: self.states.clone(),
    };
    let root = theme
      .resolve::<ColumnView<T>>(
        &base.clone().state(ColumnViewState::Disabled, self.disabled),
        Some(&self.rules),
      )
      .owned_by(self.id.as_ref());
    let resolve = |case: Case| {
      theme
        .resolve::<ColumnView<T>>(&case, Some(&self.rules))
        .owned_like(&root)
    };
    // An item is in one of six cases: each is resolved once, however many items there are.
    let cases: Vec<ResolvedRecipe> = (0..6)
      .map(|case| {
        let (chosen, disabled) = (case / 2, case % 2 == 1);
        resolve(
          base
            .clone()
            .state(ColumnViewState::Selected, chosen > 0)
            .state(ColumnViewState::Current, chosen == 2)
            .state(ColumnViewState::Disabled, disabled),
        )
      })
      .collect();
    let last_column = resolve(base.clone().state(ColumnViewState::Last, true));

    // The columns: the top, then what each item of the path holds, down to the first value no item has.
    struct Level<'a, T> {
      items: &'a [TreeItem<T>],
      /// Which of them is on the path.
      chosen: Option<usize>,
      /// The key of the item the column is of, and what it says.
      parent: Option<(Arc<str>, Arc<str>)>,
      loading: bool,
    }
    let mut levels: Vec<Level<'_, T>> = Vec::new();
    let mut resolved: Vec<T> = Vec::new();
    let mut items: &[TreeItem<T>] = &self.items;
    let mut parent: Option<(Arc<str>, Arc<str>)> = None;
    let mut loading = false;
    loop {
      let chosen = self
        .path
        .get(levels.len())
        .and_then(|value| items.iter().position(|item| item.value == *value));
      levels.push(Level {
        items,
        chosen,
        parent: parent.clone(),
        loading,
      });
      let Some(index) = chosen else {
        break;
      };
      let item = &items[index];
      resolved.push(item.value.clone());
      if !item.lazy && item.items.is_empty() {
        break;
      }
      let key = item_key(parent.as_ref().map(|(key, _)| &**key), index, item);
      loading = item.lazy && item.items.is_empty();
      parent = Some((key, item.label.clone()));
      items = &item.items;
    }

    let keys: Vec<Vec<Arc<str>>> = levels
      .iter()
      .map(|level| {
        let parent = level.parent.as_ref().map(|(key, _)| &**key);
        level
          .items
          .iter()
          .enumerate()
          .map(|(index, item)| item_key(parent, index, item))
          .collect()
      })
      .collect();
    let model = Arc::new(Model {
      columns: levels
        .iter()
        .zip(&keys)
        .map(|(level, keys)| {
          level
            .items
            .iter()
            .zip(keys)
            .enumerate()
            .map(|(index, (item, key))| Cell {
              value: item.value.clone(),
              key: key.clone(),
              enabled: !self.disabled && !item.disabled,
              child: item
                .items
                .iter()
                .enumerate()
                .find(|(_, child)| !child.disabled)
                .map(|(at, child)| (child.value.clone(), item_key(Some(key), at, child))),
              chosen: level.chosen == Some(index),
            })
            .collect()
        })
        .collect(),
      path: resolved.clone(),
      write: self.write.clone(),
      on_change: self.on_change.clone(),
      on_activate: self.on_activate.clone(),
    });

    // Tab stops at the item chosen last, where it takes input, else at the first that does.
    let end = resolved.len().checked_sub(1);
    let entry = end
      .and_then(|column| Some((column, levels[column].chosen?)))
      .filter(|(column, index)| model.columns[*column][*index].enabled)
      .or_else(|| {
        model
          .columns
          .iter()
          .enumerate()
          .find_map(|(column, cells)| cells.iter().position(|cell| cell.enabled).map(|index| (column, index)))
      });

    let count = levels.len();
    let columns = levels
      .into_iter()
      .zip(keys)
      .enumerate()
      .map(|(column, (level, keys))| {
        let siblings = level.items.len();
        let items = level
          .items
          .iter()
          .zip(keys)
          .enumerate()
          .map(|(index, (item, key))| {
            let selected = level.chosen == Some(index);
            let current = selected && end == Some(column);
            let disabled = self.disabled || item.disabled;
            let recipe = &cases[(usize::from(selected) + usize::from(current)) * 2 + usize::from(disabled)];
            // An item names its parts after its id, else after its key, where the app gave it one.
            let recipe = match &item.key {
              Some(key) if item.id.is_none() => recipe.keyed_item(key, ColumnViewPart::Item),
              _ => recipe.item(item.id.as_ref(), ColumnViewPart::Item),
            };
            let opens = item.lazy || !item.items.is_empty();
            let actions = (!disabled).then(|| {
              let (choose, go, activate) = (model.clone(), model.clone(), model.clone());
              Arc::new(ItemActions {
                choose: Arc::new(move || choose.choose(column, index)),
                go: Arc::new(move |way| go.go(column, index, way)),
                activate: Arc::new(move || activate.activate(column, index)),
                press_activates: self.press_activates && (self.activate_branches || !opens),
                enter_goes_in: !self.activate_branches && opens,
                hover_chooses: self.open_on_hover && opens,
              })
            });
            ColumnViewItem {
              label: item.label.clone(),
              icon: if selected {
                item.open_icon.or(item.icon)
              } else {
                item.icon
              },
              opens,
              selected,
              current,
              disabled,
              trailing: item.trailing.clone(),
              trailing_on_hover: item.trailing_on_hover,
              level: column as u32 + 1,
              position: (index + 1, siblings),
              entry: entry == Some((column, index)),
              autofocus: self.autofocus && entry == Some((column, index)),
              index,
              recipe,
              forced: self.forced.filter(|_| entry == Some((column, index))),
              actions,
              focus_key: key,
              id: item.id.clone(),
            }
          })
          .collect();
        ColumnViewColumn {
          label: level.parent.map(|(_, label)| label),
          items,
          loading: level.loading,
          last: column + 1 == count,
          loading_label: self.loading_label.clone(),
          recipe: if column + 1 == count {
            last_column.clone()
          } else {
            root.clone()
          },
          id: self.id.as_ref().map(|id| Arc::from(format!("{id}-column-{column}"))),
        }
      })
      .collect();

    ColumnViewView {
      columns,
      label: self.label.clone(),
      disabled: self.disabled,
      recipe: root,
      kept,
      id: self.id.clone(),
      key: self.key.clone(),
    }
  }
}

impl<T: Clone + PartialEq + Send + Sync + 'static> From<ColumnView<T>> for Element {
  fn from(view: ColumnView<T>) -> Self {
    view.build(&current_theme())
  }
}

/// What an item is told apart by among every item shown, and what the keys find it by: the key the app gave it, or
/// where it is from the top, `0/2/1`.
fn item_key<T>(parent: Option<&str>, index: usize, item: &TreeItem<T>) -> Arc<str> {
  match (&item.key, parent) {
    (Some(key), _) => key.clone(),
    (None, Some(parent)) => Arc::from(format!("{parent}/{index}")),
    (None, None) => Arc::from(index.to_string()),
  }
}

/// Where a key takes the choice from an item.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Way {
  Back,
  On,
  First,
  Last,
  /// Right: into what the item holds.
  In,
  /// Left: back to the item the column is of.
  Out,
}

/// An item of a column as the keys see it.
struct Cell<T> {
  value: T,
  key: Arc<str>,
  enabled: bool,
  /// The first item it holds that takes input: where Right goes.
  child: Option<(T, Arc<str>)>,
  chosen: bool,
}

/// The columns shown and the path chosen, which every item's behavior reads.
struct Model<T> {
  columns: Vec<Vec<Cell<T>>>,
  /// The path as the items have it: the values of the app's path that are items, from the top.
  path: Vec<T>,
  write: Option<Arc<dyn Fn(Vec<T>) + Send + Sync>>,
  on_change: Vec<ToldPath<T>>,
  on_activate: Vec<Told<T>>,
}

impl<T: Clone + PartialEq> Model<T> {
  /// The path that ends at the item `index` of `column`.
  fn path_to(&self, column: usize, index: usize) -> Vec<T> {
    let mut path: Vec<T> = self.path.iter().take(column).cloned().collect();
    path.push(self.columns[column][index].value.clone());
    path
  }

  /// Sets the path where it is another, then tells the handlers.
  fn set(&self, next: Vec<T>) {
    if next == self.path {
      return;
    }
    if let Some(write) = &self.write {
      write(next.clone());
    }
    for handler in &self.on_change {
      handler(&next);
    }
  }

  fn choose(&self, column: usize, index: usize) {
    self.set(self.path_to(column, index));
  }

  fn activate(&self, column: usize, index: usize) {
    for handler in &self.on_activate {
      handler(&self.columns[column][index].value);
    }
  }

  /// Takes the choice `way` from the item `index` of `column`, and gives the key of the item it went to: `None`
  /// where there is none that way.
  fn go(&self, column: usize, index: usize, way: Way) -> Option<Arc<str>> {
    let cells = &self.columns[column];
    let within = |to: Option<usize>| {
      let to = to?;
      self.set(self.path_to(column, to));
      Some(cells[to].key.clone())
    };
    let enabled = |at: &usize| cells[*at].enabled;
    match way {
      Way::Back => within((0..index).rev().find(enabled)),
      Way::On => within((index + 1..cells.len()).find(enabled)),
      Way::First => within((0..index).find(enabled)),
      Way::Last => within((index + 1..cells.len()).rev().find(enabled)),
      Way::In => {
        let mut path = self.path_to(column, index);
        let child = cells[index].child.clone();
        if let Some((value, _)) = &child {
          path.push(value.clone());
        }
        self.set(path);
        child.map(|(_, key)| key)
      }
      Way::Out => {
        let before = column.checked_sub(1)?;
        let parent = self.columns[before].iter().position(|cell| cell.chosen)?;
        self.set(self.path_to(before, parent));
        Some(self.columns[before][parent].key.clone())
      }
    }
  }
}

/// What an item that takes input does, with the type of the view's values out of sight.
struct ItemActions {
  choose: Arc<dyn Fn() + Send + Sync>,
  go: Arc<dyn Fn(Way) -> Option<Arc<str>> + Send + Sync>,
  activate: Arc<dyn Fn() + Send + Sync>,
  /// Whether a single press activates the item after it chooses it.
  press_activates: bool,
  /// Whether Enter goes into what the item holds, in place of activating it.
  enter_goes_in: bool,
  /// Whether the pointer coming onto the item chooses it.
  hover_chooses: bool,
}

/// What a mounted column view keeps from one render to the next.
#[derive(Clone)]
struct Kept {
  /// Where the columns are scrolled to, sideways.
  scroll: ScrollState,
  /// How many columns the last render had: one more brings the last into sight.
  columns: Arc<Mutex<usize>>,
}

/// A column view as a renderer sees it: its columns, and the style of its root.
pub struct ColumnViewView {
  /// The columns, from the top's.
  pub columns: Vec<ColumnViewColumn>,
  /// What the view is called, to what reads the tree of the window.
  pub label: Option<Arc<str>>,
  pub disabled: bool,
  /// The style of each part of the view as a whole.
  pub recipe: ResolvedRecipe,
  kept: Option<Kept>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl ColumnViewView {
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// The root in the style of its part, with nothing in it yet, and the behavior of the keyboard: one stop of Tab,
  /// whose choice follows the focus, and typeahead. It is a group to what reads the tree, called by its label.
  pub fn root(&self) -> Row {
    let mut root = Row::new()
      .align_items(Alignment::Stretch)
      .clip()
      .focus_group(
        FocusGroup::vertical()
          .one_stop(true)
          .activate(true)
          .typeahead(true)
          .wrap(false),
      )
      .describe("a11y_role", "group")
      .part(&self.part(ColumnViewPart::Root));
    if let Some(label) = &self.label {
      root = root.describe("a11y_name", label.clone());
    }
    if self.disabled {
      root = root.describe("a11y_disabled", "true");
    }
    if let Some(id) = &self.id {
      root = root.id(id.clone());
    }
    if let Some(key) = &self.key {
      root = root.key(key.clone());
    }
    root
  }

  /// How the default renderer builds a view: its columns side by side, sharing its width, in what scrolls them
  /// sideways where they do not fit.
  pub fn render(mut self) -> Element {
    let columns = std::mem::take(&mut self.columns);
    let count = columns.len();
    let row = Row::new()
      .height(Dimension::Pct(100.0))
      .align_items(Alignment::Stretch)
      .with_children(columns.into_iter().map(ColumnViewColumn::render));
    let mut scroll = ScrollHorizontal::new(row)
      .fill_width(true)
      .key("columns")
      .flex_full(1.0, 1.0, Some(0.0))
      .min_width(0.0)
      .height(Dimension::Pct(100.0));
    if let Some(kept) = &self.kept {
      scroll = scroll.with_scroll_state(kept.scroll.clone());
      // More columns than the last render had, and the first render's: the path grew, and its end is brought
      // into sight.
      let mut before = kept.columns.lock();
      if count > *before {
        kept.scroll.scroll_to_right_pending();
      }
      *before = count;
    }
    self.root().child(scroll).into()
  }
}

/// One column of a column view as a renderer sees it: its items, and the style of its parts.
pub struct ColumnViewColumn {
  /// What the item it is of says: `None` for the first column, of what is at the top.
  pub label: Option<Arc<str>>,
  /// Its items, in order.
  pub items: Vec<ColumnViewItem>,
  /// Whether its items are on their way: the item it is of is lazy and holds none yet.
  pub loading: bool,
  /// Whether it is the last column.
  pub last: bool,
  loading_label: Arc<str>,
  /// The style of each part of this column.
  pub recipe: ResolvedRecipe,
  id: Option<Arc<str>>,
}

impl ColumnViewColumn {
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// The spinner of a column whose items are on their way, in the room of its part.
  pub fn status(&self) -> Element {
    let side = self
      .part(ColumnViewPart::Spinner)
      .rest()
      .width
      .unwrap_or(Length::Icon(IconSize::Md));
    Row::new()
      .justify(Justify::Center)
      .part(&self.part(ColumnViewPart::Status))
      .child(
        Spinner::new()
          .label(&self.loading_label)
          .label_hidden(true)
          .rules(&self.recipe.lend(ColumnViewPart::Spinner))
          .style(SpinnerPart::Indicator, |s| s.square(side))
          .build(self.theme()),
      )
      .into()
  }

  /// How the default renderer builds a column: its items one under the other, in what scrolls them where they are
  /// more than its height. It is a listbox to what reads the tree, called by the item it is of.
  pub fn render(mut self) -> Element {
    let part = self.part(ColumnViewPart::Column);
    let style = part.rest();
    let theme = self.theme().clone();
    let least = super::select::px(style.min_width, &theme).unwrap_or(0.0);
    let mut list = Column::new().align_items(Alignment::Stretch);
    if let Some(gap) = style.gap {
      list = list.spacing(gap);
    }
    let status = self.loading.then(|| self.status());
    let list = list
      .with_children(std::mem::take(&mut self.items).into_iter().map(ColumnViewItem::render))
      .with_children(status);
    // The part's padding is around the items, inside what scrolls: the bar stands at the column's edge.
    let [top, end, bottom, start] = style.paddings();
    let none = || crate::node::SpacingValue::from(0.0);
    let padded = Column::new()
      .align_items(Alignment::Stretch)
      .padding_top(top.unwrap_or_else(none))
      .padding_right(end.unwrap_or_else(none))
      .padding_bottom(bottom.unwrap_or_else(none))
      .padding_left(start.unwrap_or_else(none))
      .child(list);
    let mut column = Column::new()
      .align_items(Alignment::Stretch)
      .flex_full(1.0, 0.0, Some(least))
      .min_width(least)
      .describe("a11y_role", "listbox")
      .part(&part)
      // The padding is inside the scroll, and the least width is the column's own.
      .padding(0.0)
      .child(ScrollVertical::new(padded).flex_full(1.0, 1.0, Some(0.0)).min_height(0.0));
    if let Some(label) = &self.label {
      column = column.describe("a11y_name", label.clone());
    }
    if let Some(id) = &self.id {
      column = column.id(id.clone());
    }
    column.into()
  }
}

/// One item of a column view as a renderer sees it: what it says and where it is, the style of its parts, and its
/// behavior ready to be dressed.
pub struct ColumnViewItem {
  pub label: Arc<str>,
  /// The icon before its label: the open one while it is on the path, where it has one.
  pub icon: Option<Name>,
  /// Whether it holds others: it has a chevron, and chosen, a column after its own.
  pub opens: bool,
  /// Whether it is on the path.
  pub selected: bool,
  /// Whether it is the end of the path.
  pub current: bool,
  pub disabled: bool,
  /// What stands at its end, before its chevron.
  pub trailing: Option<Element>,
  /// Whether what stands at its end shows only while the item is pointed at or holds the focus.
  pub trailing_on_hover: bool,
  /// Which column it is in, from 1.
  pub level: u32,
  /// Where it is among the items of its column, from 1, and how many they are.
  pub position: (usize, usize),
  /// Whether Tab enters the view at it.
  pub entry: bool,
  autofocus: bool,
  index: usize,
  /// The style of each part of this item, in its case.
  pub recipe: ResolvedRecipe,
  forced: Option<Interaction>,
  actions: Option<Arc<ItemActions>>,
  focus_key: Arc<str>,
  id: Option<Arc<str>>,
}

impl ColumnViewItem {
  /// The style of a part, as this item is to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part).forced(self.forced)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// The item's behavior with the style of its part: what is pressed, chosen and focused, with nothing in it yet.
  /// A press chooses it, a double press and Enter activate it, and the arrows, Home and End take the choice from
  /// it. It is an option to what reads the tree, selected while it is on the path.
  pub fn button(&self) -> components::Button {
    let flag = |on: bool| if on { "true" } else { "false" };
    let mut item = pressable(&self.part(ColumnViewPart::Item), self.disabled)
      .justify(Justify::Start)
      .align_items(Alignment::Center)
      .key(self.focus_key.clone())
      .describe("a11y_role", "option")
      .describe("a11y_name", self.label.clone())
      .describe("a11y_level", self.level.to_string())
      .describe("a11y_posinset", self.position.0.to_string())
      .describe("a11y_setsize", self.position.1.to_string())
      .describe("a11y_selected", flag(self.selected))
      .focus_label(self.label.clone())
      .focus_entry(self.entry)
      .interaction_group(true);
    if self.autofocus {
      item = item.autofocus(true);
    }
    if self.opens {
      item = item.describe("a11y_expanded", flag(self.selected));
    }
    if let Some(actions) = self.actions.clone() {
      let (press, index) = (actions.clone(), self.index);
      item = item.on_click(move |event: MouseEvent| {
        if !event.from_keyboard && event.button != MouseButton::Left {
          return;
        }
        if press.press_activates && !event.from_keyboard {
          (press.choose)();
          (press.activate)();
        // A second press of the item soon after at the same place activates it: it was chosen by the first.
        } else if !event.from_keyboard && second_press(index, &event) {
          (press.activate)();
        } else {
          (press.choose)();
        }
      });
      if actions.hover_chooses {
        let hover = actions.clone();
        item = item.on_mouse_enter(move || (hover.choose)());
      }
      item = item.on_key_down(move |event: KeyboardEvent| {
        if !event.focus_within || event.composing || event.alt || event.ctrl || event.meta || event.shift {
          return;
        }
        let way = match event.key.as_str() {
          "Enter" if actions.enter_goes_in => Way::In,
          "Enter" => {
            event.prevent_default();
            (actions.activate)();
            return;
          }
          "ArrowUp" => Way::Back,
          "ArrowDown" => Way::On,
          "Home" => Way::First,
          "End" => Way::Last,
          "ArrowRight" => Way::In,
          "ArrowLeft" => Way::Out,
          _ => return,
        };
        // The key is the view's whether or not there is an item that way: the group around the items moves
        // nothing by it.
        event.prevent_default();
        if let Some(key) = (actions.go)(way) {
          event.focus_key(key);
        }
      });
    }
    if let Some(id) = &self.id {
      item = item.id(id.clone());
    }
    item
  }

  /// The icon before the label, if the item has one.
  pub fn icon(&self) -> Option<Element> {
    self
      .icon
      .map(|name| glyph(&self.part(ColumnViewPart::Icon), name, false))
  }

  /// What the item says, in the room the icon and what is at its end leave: a longer label ends in an ellipsis.
  pub fn label(&self) -> Row {
    let part = self.part(ColumnViewPart::Label);
    Row::new().flex_full(1.0, 1.0, Some(0.0)).min_width(0.0).child(
      part
        .text(&self.label)
        .nowrap()
        .text_overflow(TextOverflow::Elipsis)
        .part(&part),
    )
  }

  /// The chevron at the end of an item that holds others: `None` for one that holds none.
  pub fn chevron(&self) -> Option<Element> {
    self.opens.then(|| {
      glyph(
        &self.part(ColumnViewPart::Chevron),
        Name::new(icons::CHEVRON_RIGHT),
        false,
      )
    })
  }

  /// How the default renderer builds an item: its icon, its label, what is at its end and its chevron.
  pub fn render(mut self) -> Element {
    let mut item = self.button();
    if let Some(icon) = self.icon() {
      item = item.child(icon);
    }
    item = item.child(self.label());
    if let Some(trailing) = self.trailing.take() {
      let trailing: Element = if self.trailing_on_hover {
        Row::new()
          .align_items(Alignment::Center)
          .part(&self.part(ColumnViewPart::Reveal))
          .child(trailing)
          .into()
      } else {
        trailing
      };
      item = item.child(
        Row::new()
          .align_items(Alignment::Center)
          .flex_shrink(0.0)
          .part(&self.part(ColumnViewPart::Trailing))
          .child(trailing),
      );
    }
    if let Some(chevron) = self.chevron() {
      item = item.child(chevron);
    }
    item.into()
  }
}

impl<T: Clone + PartialEq + Send + Sync + 'static> crate::ui::Styled for ColumnView<T> {
  type Component = Self;

  fn rules(self, rules: &crate::ui::Recipe) -> Self {
    Self::rules(self, rules)
  }

  fn state(self, state: impl Into<crate::ui::Name>, on: bool) -> Self {
    Self::state(self, state, on)
  }
}

/// Every column view has the one recipe and renderer, whatever its values are.
impl<T: Send + Sync + 'static> Themed for ColumnView<T> {
  const NAME: &'static str = "column_view";
  type Renderer = ColumnViewRenderer;

  fn parts() -> crate::ui::Declared {
    ColumnViewPart::declared()
  }

  fn states() -> crate::ui::Declared {
    ColumnViewState::declared()
  }

  /// The design's column view (`pencil/lurq-design-system.pen`, Column View): a box of 520 by 176 on the background
  /// in an edge of 1 in the border's color, 8 round, that cuts what is in it. Its columns share its width, each
  /// with a line of 1 in the border's color at its end but the last, padded 4, its items 1 apart. An item is 26
  /// high with the small radius, 6 from its sides, and holds an icon of 14 in the muted foreground, its label in
  /// the body's type at 13 and, where it holds others, a chevron of 12 in the muted foreground, 6 apart. An item on
  /// the path is in the highlight (the board's `$accent`); the one the path ends at is on the info fill, its icon,
  /// its label and its chevron in white. The board draws no other state and no column narrower than a third of
  /// 520: under the pointer and held down an item has the veils of those states, the focus from the keyboard is
  /// the ring inside it, a disabled one is faded, and a column is 160 wide at least, past which the columns scroll
  /// sideways.
  fn recipe() -> Recipe {
    use ColumnViewPart::{Chevron, Icon, Item, Label, Reveal, Root, Spinner, Status, Trailing};
    use ColumnViewState::{Current, Disabled, Last, Selected};
    use PaletteColor::{Background, Foreground, Highlight, Info, MutedForeground, Ring, StateHover, StatePressed};
    let line = |width: f32| Border::inside(width, PaletteColor::Border);
    let white = Color::from_hex("#FFFFFF");
    Recipe::new()
      .base(Root, |s| {
        s.width(520.0)
          .max_width(Length::Fill)
          .height(176.0)
          .radius(RadiusSize::Lg)
          .background(Background)
          .foreground(Foreground)
          .border(Border::inside(BorderSize::Sm, PaletteColor::Border))
      })
      .base(ColumnViewPart::Column, |s| {
        s.min_width(160.0).padding_x(4.0).padding_y(4.0).gap(1.0).end_line(line(1.0))
      })
      .state(Last, ColumnViewPart::Column, |s| s.end_line(line(0.0)))
      .base(Item, |s| {
        fading(
          s.height(26.0)
            .padding_x(6.0)
            .gap(6.0)
            .radius(RadiusSize::Sm)
            .cursor(CursorIcon::Pointer),
        )
      })
      // A row of the density's height: 26 at the default, 18, 34 and 38 at the others.
      .row_density(Selector::new(), Item, 26.0)
      .hovered(Item, |s| s.veil(StateHover))
      .pressed(Item, |s| s.veil(StatePressed))
      .focus_visible(Item, |s| s.outline(Outline::new(BorderSize::Md, Ring).offset(-2.0)))
      .base(Icon, |s| s.square(IconSize::Sm).foreground(MutedForeground))
      .base(Label, |s| s.typography(TypographyStyle::Body).font_size(13.0))
      .base(Trailing, |s| {
        s.gap(4.0)
          .typography(TypographyStyle::Caption)
          .foreground(MutedForeground)
      })
      .base(Reveal, |s| s.opacity(0.0))
      .within(Reveal, |s| s.opacity(1.0))
      .density(Density::Touch, Reveal, |s| s.opacity(1.0))
      .base(Chevron, |s| s.square(IconSize::Xs).foreground(MutedForeground))
      .base(Status, |s| s.padding_y(8.0))
      .base(Spinner, |s| s.square(IconSize::Md))
      .state(Selected, Item, |s| s.background(Highlight))
      .state(Current, Item, |s| s.background(Info))
      .state(Current, Icon, |s| s.foreground(white))
      .state(Current, Label, |s| s.foreground(white))
      .state(Current, Trailing, |s| s.foreground(white))
      .state(Current, Chevron, |s| s.foreground(white))
      .state(Disabled, Item, |s| s.opacity(OpacityLevel::Disabled).cursor(CursorIcon::Default))
  }
}

struct ColumnViewProps<T: 'static> {
  view: ColumnView<T>,
}

/// A view built again is drawn again: its path is read from the app's signal as it is built.
impl<T> PartialEq for ColumnViewProps<T> {
  fn eq(&self, _: &Self) -> bool {
    false
  }
}

impl<T> crate::app::component::DevtoolsInspectable for ColumnViewProps<T> {
  fn inspect(&self, formatter: &mut crate::app::component::DevtoolsFormatter<'_>) {
    formatter.value("ColumnView", format!("{} items at the top", self.view.items.len()));
  }
}

/// What a mounted column view keeps between renders.
struct ColumnViewHost<T> {
  kept: Kept,
  _values: std::marker::PhantomData<fn() -> T>,
}

impl<T: Clone + PartialEq + Send + Sync + 'static> Component for ColumnViewHost<T> {
  type Props = ColumnViewProps<T>;

  fn create(_: &mut Ctx) -> Self {
    Self {
      kept: Kept {
        scroll: ScrollState::new(),
        columns: Arc::new(Mutex::new(0)),
      },
      _values: std::marker::PhantomData,
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    ctx
      .props::<ColumnViewProps<T>>()
      .view
      .build_with(&current_theme(), Some(self.kept.clone()))
  }
}
