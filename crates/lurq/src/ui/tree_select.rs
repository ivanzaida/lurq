//! A tree select: a value chosen from a tree that opens under a trigger, one or any number.

use std::sync::Arc;

use parking_lot::Mutex;

use super::{
  combobox, names,
  part_style::{Length, PartStyle},
  popover::{Popover, PopoverPart},
  recipe::{Interaction, Name, Recipe, ResolvedPart, ResolvedRecipe},
  select::{Select, SelectState},
  select_trigger::{OpenModel, SelectTrigger},
  shared::{ChangeHandler, fading},
  text_box::{BoxStates, Judgment, TextBoxSize},
  theme::{Themed, current_theme},
  tree_view::{TreeItem, TreeView, TreeViewPart},
};
use crate::{
  app::{
    component::Component,
    ctx::{CollisionStrategy, Ctx, Placement},
    theme::{IconSize, PaletteColor, SpacingSize, Theme},
  },
  components::{self, Column, Row, ScrollVertical},
  core::{ElementRef, Signal, SignalValue},
  node::{CursorIcon, Element, SpacingValue, dimension::Dimension},
};

/// The parts of a tree select a recipe styles. Those it shares with a select have their names.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TreeSelectPart {
  /// The box that is pressed and focused, and that the tree opens under: its size, its edge, its fill, its ring,
  /// and the type of what it says. Its gap is the room between what it says and the chevron.
  Trigger,
  /// What the trigger says of what is chosen: its type and its color, over the trigger's.
  Value,
  /// What the trigger says while nothing is chosen, over the value's type and color.
  Placeholder,
  /// The glyph at the end of the trigger.
  Chevron,
  /// What takes what is chosen away, before the chevron ([`TreeSelect::clearable`]).
  Clear,
  /// The cross in it: its side.
  ClearIcon,
  /// The panel that opens: what is laid over a popover's panel. Its width, its padding.
  Panel,
  /// What holds the tree in the panel: its greatest height, past which the tree scrolls.
  Tree,
  /// A row of the tree: what is laid over a tree view's row.
  Row,
  /// The icon of a row of the tree: what is laid over a tree view's.
  RowIcon,
  /// What a row of the tree says: what is laid over a tree view's label.
  RowLabel,
  /// What the panel says where the tree has no items.
  Empty,
}

names!(TreeSelectPart {
  Trigger => "trigger",
  Value => "value",
  Placeholder => "placeholder",
  Chevron => "chevron",
  Clear => "clear",
  ClearIcon => "clear_icon",
  Panel => "panel",
  Tree => "tree",
  Row => "row",
  RowIcon => "row_icon",
  RowLabel => "row_label",
  Empty => "empty",
});

/// The sizes of a tree select: a select's, the heights of the theme's controls.
pub type TreeSelectSize = TextBoxSize;

/// The states of a tree select a recipe styles: a select's, by the same names.
pub type TreeSelectState = SelectState;

/// What builds a tree select's nodes: a theme's in place of the default one.
pub type TreeSelectRenderer = Arc<dyn Fn(TreeSelectView) -> Element + Send + Sync>;

/// What tells the handlers the new choice, in the form the select holds it.
type Tell<T, V> = fn(&[T], &[ChangeHandler<V>]);

fn tell_one<T>(next: &[T], handlers: &[ChangeHandler<T>]) {
  if let Some(value) = next.last() {
    for handler in handlers {
      handler(value);
    }
  }
}

fn tell_optional<T: Clone>(next: &[T], handlers: &[ChangeHandler<Option<T>>]) {
  let value = next.last().cloned();
  for handler in handlers {
    handler(&value);
  }
}

fn tell_many<T>(next: &[T], handlers: &[ChangeHandler<[T]>]) {
  for handler in handlers {
    handler(next);
  }
}

type Toggle<T> = Arc<dyn Fn(&T, bool) + Send + Sync>;

/// A select whose options are a tree: a file of a project, a category of a catalogue, the folders a search looks
/// in.
///
/// ```
/// use lurq::{core::Signal, ui::{TreeItem, TreeSelect}};
///
/// # fn render(ctx: &mut lurq::app::ctx::Ctx, file: Signal<Option<&'static str>>, folders: Signal<Vec<&'static str>>) {
/// let items = || [
///   TreeItem::folder("crates", "crates").item(
///     TreeItem::folder("lurq", "lurq")
///       .item(TreeItem::file("lib.rs", "lib.rs"))
///       .item(TreeItem::file("theme.rs", "theme.rs")),
///   ),
///   TreeItem::file("Cargo.toml", "Cargo.toml"),
/// ];
/// // One or none, of the files alone: mounted, so that it keeps whether it is open and which folders are.
/// let one = TreeSelect::optional(file, items()).placeholder("Select a file").leaves_only(true).mount(ctx);
/// // Any number.
/// let many = TreeSelect::multiple(folders, items()).label("Folders").mount(ctx);
/// // Held by the app, which is told each choice.
/// let held = TreeSelect::controlled("lib.rs", items(), |file| println!("{file}"));
/// # }
/// ```
///
/// Its trigger is a [`Select`]'s, the same box, sizes, ring and judgments, and its panel holds a
/// [`TreeView`] of its items ([`TreeItem`]). Its forms are those of every component that holds a value: bound to
/// the app's signal, [`new`](Self::new) of one, [`optional`](TreeSelect::optional) of one or none and
/// [`multiple`](TreeSelect::multiple) of any number; held by the app, [`controlled`](Self::controlled),
/// [`controlled_optional`](TreeSelect::controlled_optional) and
/// [`controlled_multiple`](TreeSelect::controlled_multiple). The trigger says the path to what is chosen, as the
/// board's does ("crates / lurq / lib.rs"), and of any number their labels.
///
/// A press, Enter, Space or Down on the trigger opens the panel, with the focus on the chosen row or the first.
/// The keys in it are a tree's: Up and Down through the rows, Right and Left to open, close and go in and out, a
/// letter to the next row that starts with it. Of one, a press of a row or Space chooses it and closes the panel,
/// and Enter does on a row that holds nothing; of any number, a press or Space adds a row or takes it out and the
/// panel stays open. Escape and a press outside close it, and the focus goes back to the trigger.
///
/// The select keeps whether it is open, the trigger its panel hangs from and which branches are open where it is
/// [mounted](Self::mount); the app holds them with [`open`](Self::open) and [`branches`](Self::branches).
/// [`Cascader`](super::Cascader) is its sibling for a tree shown as columns that open one after the other, of which
/// one is chosen.
/// Converted into an element on its own it is the still form of itself.
pub struct TreeSelect<T: 'static = (), V: ?Sized + 'static = T> {
  items: Vec<TreeItem<T>>,
  chosen: Vec<T>,
  write: Arc<dyn Fn(Vec<T>) + Send + Sync>,
  many: bool,
  /// Whether nothing chosen is a value it holds: it can be cleared.
  empties: bool,
  tell: Tell<T, V>,
  on_change: Vec<ChangeHandler<V>>,
  open: Option<Signal<bool>>,
  branches: Option<(Vec<T>, Toggle<T>)>,
  on_branch_change: Vec<Toggle<T>>,
  leaves_only: bool,
  path: bool,
  separator_text: Arc<str>,
  max_shown: Option<usize>,
  placeholder: Option<Arc<str>>,
  boxed: BoxStates,
  full_width: bool,
  clearable: bool,
  clear_label: Arc<str>,
  empty_text: Arc<str>,
  panel_label: Option<Arc<str>>,
  placement: Placement,
  offset: Option<SpacingValue>,
  collision: Option<CollisionStrategy>,
  inline: bool,
  dismiss_on_escape: bool,
  dismiss_on_outside_press: bool,
  on_open_change: Vec<Arc<dyn Fn(bool) + Send + Sync>>,
  on_dismiss: Vec<Arc<dyn Fn() + Send + Sync>>,
  // Read only where the engine has forms.
  #[cfg_attr(not(feature = "form"), allow(dead_code))]
  name: Option<Arc<str>>,
  states: Vec<Name>,
  rules: Recipe,
  forced: Option<Interaction>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl<T: Clone + PartialEq + Send + Sync + 'static> TreeSelect<T, T> {
  /// A tree select of `items` bound to `value`: the row of its value is chosen, and choosing another sets it.
  pub fn new(value: Signal<T>, items: impl IntoIterator<Item = TreeItem<T>>) -> Self
  where
    T: SignalValue,
  {
    let chosen = vec![value.get()];
    let write = Arc::new(move |next: Vec<T>| {
      if let Some(next) = next.into_iter().last() {
        value.set(next);
      }
    });
    Self::of(items, chosen, write, false, false, tell_one::<T>)
  }

  /// A tree select of one whose value the app holds: the row of `value` is chosen, and a choice of another tells
  /// `on_change` the value it is to be; the trigger shows what the app then gives. Two built at the one place in
  /// the app's code, in a loop, are told apart by their `id` or `key`.
  pub fn controlled(
    value: T,
    items: impl IntoIterator<Item = TreeItem<T>>,
    on_change: impl Fn(&T) + Send + Sync + 'static,
  ) -> Self {
    Self::of(items, vec![value], Arc::new(|_| {}), false, false, tell_one::<T>).on_change(on_change)
  }
}

impl<T: Clone + PartialEq + Send + Sync + 'static> TreeSelect<T, Option<T>> {
  /// A tree select of one bound to `value`, which may hold none: the trigger shows the placeholder while it is
  /// `None`, and choosing a row sets it to that row's value.
  pub fn optional(value: Signal<Option<T>>, items: impl IntoIterator<Item = TreeItem<T>>) -> Self
  where
    Option<T>: SignalValue,
  {
    let chosen = value.get().into_iter().collect();
    let write = Arc::new(move |next: Vec<T>| value.set(next.into_iter().last()));
    Self::of(items, chosen, write, false, true, tell_optional::<T>)
  }

  /// A tree select of one or none whose value the app holds: a choice, or a clear, is told to `on_change` as the
  /// value it is to be.
  pub fn controlled_optional(
    value: Option<T>,
    items: impl IntoIterator<Item = TreeItem<T>>,
    on_change: impl Fn(&Option<T>) + Send + Sync + 'static,
  ) -> Self {
    let chosen = value.into_iter().collect();
    Self::of(items, chosen, Arc::new(|_| {}), false, true, tell_optional::<T>).on_change(on_change)
  }
}

impl<T: Clone + PartialEq + Send + Sync + 'static> TreeSelect<T, [T]> {
  /// A tree select of any number bound to `values`: the rows of its values are chosen, and a press of a row adds
  /// it or takes it out.
  pub fn multiple(values: Signal<Vec<T>>, items: impl IntoIterator<Item = TreeItem<T>>) -> Self
  where
    Vec<T>: SignalValue,
  {
    let chosen = values.get();
    let write = Arc::new(move |next: Vec<T>| values.set(next));
    Self::of(items, chosen, write, true, true, tell_many::<T>)
  }

  /// A tree select of any number whose values the app holds: a press of a row, or a clear, tells `on_change` the
  /// whole set it is to be.
  pub fn controlled_multiple(
    values: impl IntoIterator<Item = T>,
    items: impl IntoIterator<Item = TreeItem<T>>,
    on_change: impl Fn(&[T]) + Send + Sync + 'static,
  ) -> Self {
    let chosen = values.into_iter().collect();
    Self::of(items, chosen, Arc::new(|_| {}), true, true, tell_many::<T>).on_change(on_change)
  }
}

impl<T: Clone + PartialEq + Send + Sync + 'static, V: ?Sized + 'static> TreeSelect<T, V> {
  fn of(
    items: impl IntoIterator<Item = TreeItem<T>>,
    chosen: Vec<T>,
    write: Arc<dyn Fn(Vec<T>) + Send + Sync>,
    many: bool,
    empties: bool,
    tell: Tell<T, V>,
  ) -> Self {
    Self {
      items: items.into_iter().collect(),
      chosen,
      write,
      many,
      empties,
      tell,
      on_change: Vec::new(),
      open: None,
      branches: None,
      on_branch_change: Vec::new(),
      leaves_only: false,
      path: true,
      separator_text: Arc::from(" / "),
      max_shown: None,
      placeholder: None,
      boxed: BoxStates::default(),
      full_width: false,
      clearable: false,
      clear_label: Arc::from("Clear"),
      empty_text: Arc::from("No options"),
      panel_label: None,
      placement: Placement::BottomStart,
      offset: None,
      collision: None,
      inline: false,
      dismiss_on_escape: true,
      dismiss_on_outside_press: true,
      on_open_change: Vec::new(),
      on_dismiss: Vec::new(),
      name: None,
      states: Vec::new(),
      rules: Recipe::new(),
      forced: None,
      id: None,
      key: None,
    }
  }

  /// Called with what is chosen after each choice a person makes, after the signal is set: the value of a select
  /// of one, `&Option<T>` of an [`optional`](TreeSelect::optional) one, the whole new set of one of any number.
  /// Not called for the row chosen already, nor when the app sets the signal. A second handler is called after the
  /// first. A handler of another type does not compile:
  ///
  /// ```compile_fail
  /// use lurq::{core::Signal, ui::{TreeItem, TreeSelect}};
  ///
  /// let file = Signal::new("lib.rs");
  /// let select = TreeSelect::new(file, [TreeItem::file("lib.rs", "lib.rs")]).on_change(|files: &[&str]| {});
  /// ```
  pub fn on_change(mut self, handler: impl Fn(&V) + Send + Sync + 'static) -> Self {
    self.on_change.push(Arc::new(handler));
    self
  }

  /// Whether the panel is open, where the app keeps it. Without one a mounted select keeps it itself.
  pub fn open(mut self, open: Signal<bool>) -> Self {
    self.open = Some(open);
    self
  }

  /// The branches of the tree that are open: those whose values the signal has. Opening one adds it, closing one
  /// takes it out. Without it a mounted select keeps them itself, and opens those the chosen rows are in as its
  /// panel opens.
  pub fn branches(mut self, open: Signal<Vec<T>>) -> Self
  where
    Vec<T>: SignalValue,
  {
    let held = open.get();
    self.branches = Some((
      held,
      Arc::new(move |id: &T, opened: bool| {
        open.update(|held| {
          held.retain(|row| row != id);
          if opened {
            held.push(id.clone());
          }
        })
      }),
    ));
    self
  }

  /// The branches that are open, as the app holds them: those of `open`, and `on_branch_change` is told of each
  /// a press or a key asks to open (`true`) or to close.
  pub fn branches_controlled(
    mut self,
    open: impl IntoIterator<Item = T>,
    on_branch_change: impl Fn(&T, bool) + Send + Sync + 'static,
  ) -> Self {
    self.branches = Some((open.into_iter().collect(), Arc::new(|_, _| {})));
    self.on_branch_change(on_branch_change)
  }

  /// Called with each branch a press or a key opens (`true`) or closes: where a lazy item's children are brought.
  pub fn on_branch_change(mut self, handler: impl Fn(&T, bool) + Send + Sync + 'static) -> Self {
    self.on_branch_change.push(Arc::new(handler));
    self
  }

  /// Whether only the rows that hold nothing can be chosen: a press of a row that holds others opens or closes it
  /// in place of choosing it. A select of files, whose folders only lead to them.
  pub fn leaves_only(mut self, leaves_only: bool) -> Self {
    self.leaves_only = leaves_only;
    self
  }

  /// Whether the trigger of a select of one says the whole path to what is chosen, as the board's does ("crates /
  /// lurq / lib.rs"), or its label alone. It says the path unless told otherwise.
  pub fn path(mut self, path: bool) -> Self {
    self.path = path;
    self
  }

  /// What stands between the steps of the path the trigger says: " / " unless it is told otherwise.
  pub fn separator_text(mut self, text: &str) -> Self {
    self.separator_text = Arc::from(text);
    self
  }

  /// How many labels the trigger of a select of any number says at most: the rest are counted after them, "+2".
  pub fn max_shown(mut self, max: usize) -> Self {
    self.max_shown = Some(max);
    self
  }

  /// What the trigger says while nothing is chosen.
  pub fn placeholder(mut self, placeholder: &str) -> Self {
    self.placeholder = Some(Arc::from(placeholder));
    self
  }

  /// What the select is called, to what reads the tree. The field it is in shows its label.
  pub fn label(mut self, label: &str) -> Self {
    self.boxed.label = Some(Arc::from(label));
    self
  }

  /// A [`TreeSelectSize`], or a size the theme's recipe names.
  pub fn size(mut self, size: impl Into<Name>) -> Self {
    self.boxed.size = size.into();
    self
  }

  /// Whether what is chosen is wrong, or nothing is and something has to be. Of invalid, warning and valid, the
  /// first that is set is the one shown.
  pub fn invalid(mut self, invalid: bool) -> Self {
    self.boxed.invalid = invalid;
    self
  }

  /// Whether what is chosen is allowed with a caveat.
  pub fn warning(mut self, warning: bool) -> Self {
    self.boxed.warning = warning;
    self
  }

  /// Whether what is chosen was checked and is right.
  pub fn valid(mut self, valid: bool) -> Self {
    self.boxed.valid = valid;
    self
  }

  /// Whether what is chosen can be read and not changed: the panel does not open. It keeps the focus and is read
  /// by what reads the tree.
  pub fn read_only(mut self, read_only: bool) -> Self {
    self.boxed.read_only = read_only;
    self
  }

  /// Whether the select takes no input.
  pub fn disabled(mut self, disabled: bool) -> Self {
    self.boxed.disabled = disabled;
    self
  }

  /// Whether the select is as wide as what holds it, in place of the design's 260.
  pub fn full_width(mut self, full_width: bool) -> Self {
    self.full_width = full_width;
    self
  }

  /// Whether the trigger has a cross before its chevron, while something is chosen, that takes it away: of a
  /// select that may hold none ([`optional`](TreeSelect::optional), [`multiple`](TreeSelect::multiple)).
  pub fn clearable(mut self, clearable: bool) -> Self {
    self.clearable = clearable;
    self
  }

  /// What the clear control is called to what reads the tree: "Clear" unless it is told otherwise.
  pub fn clear_label(mut self, label: &str) -> Self {
    self.clear_label = Arc::from(label);
    self
  }

  /// What the panel says where the tree has no items: "No options" unless it is told otherwise.
  pub fn empty_text(mut self, text: &str) -> Self {
    self.empty_text = Arc::from(text);
    self
  }

  /// What the panel and its tree are called to what reads the tree, in place of the select's label.
  pub fn panel_label(mut self, label: &str) -> Self {
    self.panel_label = Some(Arc::from(label));
    self
  }

  /// Which side of the trigger the panel is on, and which edge of it it lines up with: under it at its start by
  /// default.
  pub fn placement(mut self, placement: Placement) -> Self {
    self.placement = placement;
    self
  }

  /// How far the panel is from the trigger, along the side it is on: the theme's smallest spacing by default.
  pub fn offset(mut self, offset: impl Into<SpacingValue>) -> Self {
    self.offset = Some(offset.into());
    self
  }

  /// What the panel does where there is no room for it on its side.
  pub fn collision(mut self, collision: CollisionStrategy) -> Self {
    self.collision = Some(collision);
    self
  }

  /// Draws the panel in place under the trigger, open or not: for a gallery, a screenshot, a test.
  pub fn inline(mut self, inline: bool) -> Self {
    self.inline = inline;
    self
  }

  /// Whether Escape closes the panel. `true` by default.
  pub fn dismiss_on_escape(mut self, dismiss: bool) -> Self {
    self.dismiss_on_escape = dismiss;
    self
  }

  /// Whether a press outside the panel and the trigger closes it. `true` by default.
  pub fn dismiss_on_outside_press(mut self, dismiss: bool) -> Self {
    self.dismiss_on_outside_press = dismiss;
    self
  }

  /// Called with whether the panel is open after each change a person makes: a press of the trigger, a choice,
  /// Escape, a press outside. Not called when the app sets the signal.
  pub fn on_open_change(mut self, handler: impl Fn(bool) + Send + Sync + 'static) -> Self {
    self.on_open_change.push(Arc::new(handler));
    self
  }

  /// Called after a person closed the panel by Escape or a press outside it: not by a choice, nor a press of the
  /// trigger.
  pub fn on_dismiss(mut self, handler: impl Fn() + Send + Sync + 'static) -> Self {
    self.on_dismiss.push(Arc::new(handler));
    self
  }

  /// The name the select's value has in the form it is in: what the trigger says.
  #[cfg(feature = "form")]
  pub fn name(mut self, name: impl Into<Arc<str>>) -> Self {
    self.name = Some(name.into());
    self
  }

  /// Puts the select in a state the theme's recipe names ([`TreeSelectState`]), or takes it out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// Styles one part of this select over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(PartStyle) -> PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for this select.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// Draws the trigger as it is under the pointer or focused from the keyboard, with nothing pointing at it: for a
  /// gallery or a test.
  pub fn force(mut self, interaction: Option<Interaction>) -> Self {
    self.forced = interaction;
    self
  }

  /// The id of the trigger: what is pressed and focused. Its panel has this id with `-panel` after it, its tree
  /// with `-tree`, and its clear control with `-clear`.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// Builds the select in `theme`, with nothing kept but what the app gives it. Converting a select into an element
  /// does this in the theme of what is being rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let kept = Kept {
      open: self.open.clone().unwrap_or_else(|| Signal::new(false)),
      anchor: ElementRef::new(),
      branches: Arc::default(),
      shown: Arc::default(),
      revision: None,
    };
    self.build_with(theme, kept)
  }

  /// The select mounted where it is declared: it keeps whether its panel is open, the trigger the panel hangs
  /// from, which the focus goes back to, and which branches are open.
  pub fn mount(self, ctx: &mut Ctx) -> Element {
    let key = self.key.clone();
    let props = TreeSelectProps { select: self };
    match key {
      Some(key) => ctx.mount_keyed::<TreeSelectHost<T, V>>(&key, props).key(key),
      None => ctx.mount::<TreeSelectHost<T, V>>(props),
    }
  }

  fn build_with(&self, theme: &Theme, kept: Kept<T>) -> Element {
    let view = self.view(theme, kept);
    match theme.renderer::<TreeSelect<T, V>>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }

  fn view(&self, theme: &Theme, kept: Kept<T>) -> TreeSelectView {
    let open = match &self.open {
      Some(open) => open.clone(),
      None => kept.open.clone(),
    };
    let is_open = open.get();
    let disabled = self.boxed.disabled;
    let case = self
      .boxed
      .case(self.states.clone())
      .state(SelectState::Open, is_open && !self.inline)
      .state(SelectState::FullWidth, self.full_width);
    let recipe = theme.resolve::<TreeSelect<T, V>>(&case, Some(&self.rules));

    // The chosen that are items, in the app's order, each with the labels down to it.
    let mut paths: Vec<(T, Vec<Arc<str>>)> = Vec::new();
    let mut ancestors: Vec<T> = Vec::new();
    let mut branches_of: Vec<T> = Vec::new();
    walk(
      &self.items,
      &mut Vec::new(),
      &mut Vec::new(),
      &mut |item, labels, above| {
        if self.chosen.contains(&item.value) {
          paths.push((item.value.clone(), labels.to_vec()));
          for id in above {
            if !ancestors.contains(id) {
              ancestors.push(id.clone());
            }
          }
        }
        if item.lazy || !item.items.is_empty() {
          branches_of.push(item.value.clone());
        }
      },
    );
    let chosen: Vec<T> = self
      .chosen
      .iter()
      .filter(|value| paths.iter().any(|(held, _)| held == *value))
      .cloned()
      .collect();
    let labels_of = |value: &T| paths.iter().find(|(held, _)| held == value).map(|(_, labels)| labels);
    let text: Option<String> = if chosen.is_empty() {
      None
    } else if self.many {
      let shown = self.max_shown.unwrap_or(usize::MAX);
      let mut said: Vec<&str> = chosen
        .iter()
        .take(shown)
        .filter_map(|value| labels_of(value)?.last().map(|label| &**label))
        .collect();
      let more = chosen.len().saturating_sub(shown);
      let counted = format!("+{more}");
      let mut text = said.join(", ");
      if more > 0 {
        said.clear();
        text = if text.is_empty() {
          counted
        } else {
          format!("{text} {counted}")
        };
      }
      Some(text)
    } else {
      chosen.last().and_then(labels_of).map(|labels| {
        if self.path {
          labels.join(&self.separator_text)
        } else {
          labels.last().map(|label| label.to_string()).unwrap_or_default()
        }
      })
    };

    // Which branches are open: the app's, else what the select keeps. Those the chosen rows are in are opened as
    // the panel opens, and not again while it is open: a row taken out of what is chosen stays where it is.
    let open_branches: Vec<T> = match &self.branches {
      Some((open, _)) => open.clone(),
      None => {
        let (mut held, mut shown) = (kept.branches.lock(), kept.shown.lock());
        let opens = is_open && !*shown;
        *shown = is_open;
        match held.as_mut() {
          Some(branches) => {
            if opens {
              branches.extend(
                ancestors
                  .into_iter()
                  .filter(|id| !branches.contains(id))
                  .collect::<Vec<_>>(),
              );
            }
            branches.clone()
          }
          None => {
            if is_open {
              *held = Some(ancestors.clone());
            }
            ancestors
          }
        }
      }
    };
    let toggle: Toggle<T> = {
      let (own, told) = (
        self.branches.as_ref().map(|(_, toggle)| toggle.clone()),
        self.on_branch_change.clone(),
      );
      let (held, now, revision) = (kept.branches.clone(), open_branches.clone(), kept.revision.clone());
      Arc::new(move |id: &T, opened: bool| {
        match &own {
          Some(toggle) => toggle(id, opened),
          None => {
            let mut held = held.lock();
            let branches = held.get_or_insert_with(|| now.clone());
            branches.retain(|row| row != id);
            if opened {
              branches.push(id.clone());
            }
            drop(held);
            if let Some(revision) = &revision {
              revision.update(|revision| *revision += 1);
            }
          }
        }
        for handler in &told {
          handler(id, opened);
        }
      })
    };

    let model = Arc::new(OpenModel {
      open: open.clone(),
      on_open_change: self.on_open_change.clone(),
      on_dismiss: self.on_dismiss.clone(),
      inert: disabled || self.boxed.read_only,
    });
    let choose = {
      let (write, tell, on_change) = (self.write.clone(), self.tell, self.on_change.clone());
      let held = chosen.clone();
      Arc::new(move |next: Vec<T>| {
        if next == held {
          return;
        }
        write(next.clone());
        tell(&next, &on_change);
      }) as Arc<dyn Fn(Vec<T>) + Send + Sync>
    };

    let tree_rules = recipe
      .rules_for(TreeSelectPart::Row, TreeViewPart::Row)
      .extend(&recipe.rules_for(TreeSelectPart::RowIcon, TreeViewPart::Icon))
      .extend(&recipe.rules_for(TreeSelectPart::RowLabel, TreeViewPart::Label));
    let name = self.panel_label.clone().or_else(|| self.boxed.label.clone());
    let tree: Option<Element> = (!self.items.is_empty()).then(|| {
      let branch = {
        let (branches, open_now, toggle) = (branches_of.clone(), open_branches.clone(), toggle.clone());
        // A press of a row that holds others, where only the others are chosen: it opens or closes.
        move |id: &T| -> bool {
          if !branches.contains(id) {
            return false;
          }
          toggle(id, !open_now.contains(id));
          true
        }
      };
      let leaves_only = self.leaves_only;
      let toggled = toggle.clone();
      macro_rules! dressed {
        ($tree:expr) => {{
          let mut tree = $tree
            .open_ids(open_branches.clone(), move |id, opened| toggled(id, opened))
            .disabled(disabled)
            .rules(&tree_rules);
          if let Some(name) = &name {
            tree = tree.label(name);
          }
          if let Some(id) = &self.id {
            tree = tree.id(format!("{id}-tree"));
          }
          tree.build(theme)
        }};
      }
      if self.many {
        let (choose, held) = (choose.clone(), chosen.clone());
        let mut tree = TreeView::controlled_multiple(chosen.clone(), self.items.clone(), move |next: &[T]| {
          // One row went in or out. A branch that went in, where only leaves are chosen, opens or closes instead.
          if leaves_only
            && let Some(added) = next.iter().find(|value| !held.contains(value))
            && branch(added)
          {
            return;
          }
          choose(next.to_vec());
        });
        tree.press_toggles = true;
        tree.autofocus = !self.inline;
        dressed!(tree)
      } else {
        let (choose, close) = (choose.clone(), model.clone());
        let (activate, shut, branches) = (choose.clone(), model.clone(), branches_of.clone());
        let mut tree = TreeView::controlled(chosen.last().cloned(), self.items.clone(), move |value: &T| {
          if leaves_only && branch(value) {
            return;
          }
          // The panel closes first, so that a handler that opens a dialog keeps its focus.
          close.set_open(false);
          choose(vec![value.clone()]);
        })
        // Enter on a row that holds nothing chooses it, the chosen one too, which closes the panel; on one that
        // holds others it opens or closes it, as in any tree.
        .on_activate(move |value: &T| {
          if !branches.contains(value) {
            shut.set_open(false);
            activate(vec![value.clone()]);
          }
        });
        tree.autofocus = !self.inline;
        dressed!(tree)
      }
    });

    let clear =
      (self.clearable && self.empties && !chosen.is_empty() && !disabled && !self.boxed.read_only).then(|| {
        let choose = choose.clone();
        Arc::new(move || choose(Vec::new())) as Arc<dyn Fn() + Send + Sync>
      });

    TreeSelectView {
      text: text.map(Arc::from),
      placeholder: self.placeholder.clone(),
      label: self.boxed.label.clone(),
      panel_label: name,
      invalid: self.boxed.judgment() == Some(Judgment::Invalid),
      read_only: self.boxed.read_only,
      disabled,
      open: is_open,
      full_width: self.full_width,
      multiple: self.many,
      inline: self.inline,
      recipe,
      forced: self.forced,
      tree,
      empty_text: self.empty_text.clone(),
      clear,
      clear_label: self.clear_label.clone(),
      placement: self.placement,
      offset: self.offset.clone(),
      collision: self.collision,
      dismiss_on_escape: self.dismiss_on_escape,
      dismiss_on_outside_press: self.dismiss_on_outside_press,
      model,
      signal: open,
      anchor: kept.anchor,
      name: self.name.clone(),
      id: self.id.clone(),
      key: self.key.clone(),
    }
  }
}

/// Calls `visit` with every item of `items` and of what they hold, with the labels down to it and the values of
/// the items it is in.
fn walk<T: Clone>(
  items: &[TreeItem<T>],
  labels: &mut Vec<Arc<str>>,
  above: &mut Vec<T>,
  visit: &mut impl FnMut(&TreeItem<T>, &[Arc<str>], &[T]),
) {
  for item in items {
    labels.push(item.label.clone());
    visit(item, labels, above);
    above.push(item.value.clone());
    walk(&item.items, labels, above, visit);
    above.pop();
    labels.pop();
  }
}

impl<T: Clone + PartialEq + Send + Sync + 'static, V: ?Sized + 'static> From<TreeSelect<T, V>> for Element {
  fn from(select: TreeSelect<T, V>) -> Self {
    select.build(&current_theme())
  }
}

impl<T: Clone + PartialEq + Send + Sync + 'static, V: ?Sized + 'static> super::FieldControl for TreeSelect<T, V> {
  /// Called by the field's label unless it has a name of its own, judged as the field is unless the app judged the
  /// select itself, and disabled where the field is.
  fn in_field(mut self, field: &super::InField) -> Element {
    self.boxed.in_field(field);
    self.into()
  }

  /// Told what the field says, as in `in_field`, and mounted: the same control as outside a field.
  fn mount_in_field(mut self, field: &super::InField, ctx: &mut Ctx) -> Element {
    self.boxed.in_field(field);
    self.mount(ctx)
  }
}

/// What a tree select keeps from one render to the next.
struct Kept<T> {
  open: Signal<bool>,
  anchor: ElementRef,
  /// The branches that are open, once the panel was opened: until then those the chosen rows are in.
  branches: Arc<Mutex<Option<Vec<T>>>>,
  /// Whether the panel was open at the last build: as it opens, the branches the chosen rows are in open.
  shown: Arc<Mutex<bool>>,
  /// Renders a mounted select again: a branch opened or closed.
  revision: Option<Signal<u64>>,
}

impl<T> Clone for Kept<T> {
  fn clone(&self) -> Self {
    Self {
      open: self.open.clone(),
      anchor: self.anchor.clone(),
      branches: self.branches.clone(),
      shown: self.shown.clone(),
      revision: self.revision.clone(),
    }
  }
}

/// A tree select as a renderer sees it: what its trigger says, the style of its parts, and its behavior ready to
/// be dressed.
pub struct TreeSelectView {
  /// What the trigger says of what is chosen: `None` while nothing is.
  pub text: Option<Arc<str>>,
  pub placeholder: Option<Arc<str>>,
  /// What it is called, to what reads the tree.
  pub label: Option<Arc<str>>,
  panel_label: Option<Arc<str>>,
  /// Whether what is chosen is wrong, as the select shows it.
  pub invalid: bool,
  pub read_only: bool,
  pub disabled: bool,
  /// Whether its panel is open.
  pub open: bool,
  pub full_width: bool,
  /// Whether any number may be chosen.
  pub multiple: bool,
  /// Whether the panel is drawn in place.
  pub inline: bool,
  /// The style of each part of this select.
  pub recipe: ResolvedRecipe,
  forced: Option<Interaction>,
  tree: Option<Element>,
  empty_text: Arc<str>,
  clear: Option<Arc<dyn Fn() + Send + Sync>>,
  clear_label: Arc<str>,
  placement: Placement,
  offset: Option<SpacingValue>,
  collision: Option<CollisionStrategy>,
  dismiss_on_escape: bool,
  dismiss_on_outside_press: bool,
  model: Arc<OpenModel>,
  signal: Signal<bool>,
  anchor: ElementRef,
  // Read only where the engine has forms.
  #[cfg_attr(not(feature = "form"), allow(dead_code))]
  name: Option<Arc<str>>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl TreeSelectView {
  /// The style of a part, as this select is to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part).forced(self.forced)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// The trigger as a select's is drawn, the one a cascader has too.
  fn shaped(&self) -> SelectTrigger<'_> {
    SelectTrigger {
      recipe: &self.recipe,
      forced: self.forced,
      text: self.text.as_ref(),
      placeholder: self.placeholder.as_ref(),
      label: self.label.as_ref(),
      open: self.open,
      invalid: self.invalid,
      read_only: self.read_only,
      disabled: self.disabled,
      multiple: self.multiple,
      popup: "tree",
      model: &self.model,
      anchor: &self.anchor,
      name: self.name.as_ref(),
      id: self.id.as_ref(),
    }
  }

  /// What the trigger says, in the room the chevron leaves it: what is chosen, else the placeholder. A longer
  /// text ends in an ellipsis.
  pub fn value(&self) -> Row {
    self.shaped().value()
  }

  /// The cross that takes what is chosen away, where the select shows one: pressed apart from the trigger, and
  /// called "Clear" or its `clear_label` to what reads the tree.
  pub fn clear(&self) -> Option<Element> {
    self.shaped().clear(self.clear.clone(), &self.clear_label)
  }

  /// The glyph at the end of the trigger: a chevron down.
  pub fn chevron(&self) -> Element {
    self.shaped().chevron()
  }

  /// The trigger in the style of its part, with nothing in it yet: what is pressed and focused, and what the panel
  /// hangs from. A press, Enter and Space open and close the panel, and Down and Up open it. It is a combobox that
  /// opens a tree to what reads the tree, and says whether it is open.
  pub fn trigger(&self) -> components::Button {
    self.shaped().trigger()
  }

  /// What the panel holds: the tree, in what scrolls it past the greatest height of its part, or the words of a
  /// select with no items.
  pub fn content(&mut self) -> Element {
    let part = self.part(TreeSelectPart::Tree);
    match self.tree.take() {
      Some(tree) => {
        let most = super::select::px(part.rest().max_height, self.theme());
        let mut scroll = ScrollVertical::new(tree).flex_full(0.0, 1.0, None);
        if let Some(most) = most {
          scroll = scroll.max_height(most);
        }
        scroll.into()
      }
      None => {
        let empty = self.part(TreeSelectPart::Empty);
        Row::new().part(&empty).child(empty.text(&self.empty_text)).into()
      }
    }
  }

  /// The panel: a popover under the trigger holding the tree. Escape and a press outside close it, and with the
  /// focus in it give the focus back to the trigger. Inline, it is the popover's panel drawn in place.
  pub fn panel(&mut self) -> Element {
    let mut rules = self.recipe.rules_for(TreeSelectPart::Panel, PopoverPart::Panel);
    // As wide as a trigger that fills its room, once that was laid out.
    let width = self.anchor.width();
    if self.full_width && width > 0.0 && !self.inline {
      rules = rules.base(PopoverPart::Panel, |s| s.width(width));
    }
    let model = self.model.clone();
    let mut popover = Popover::new(self.signal.clone(), self.anchor.clone())
      .placement(self.placement)
      .rules(&rules)
      .inline(self.inline)
      .dismiss_on_escape(self.dismiss_on_escape)
      .dismiss_on_outside_press(self.dismiss_on_outside_press)
      .on_dismiss(move || model.dismissed())
      .child(self.content());
    if let Some(label) = &self.panel_label {
      popover = popover.label(label);
    }
    if let Some(offset) = self.offset.clone() {
      popover = popover.offset(offset);
    }
    if let Some(collision) = self.collision {
      popover = popover.collision(collision);
    }
    if let Some(id) = &self.id {
      popover = popover.id(format!("{id}-panel"));
    }
    popover.build(self.theme())
  }

  /// How the default renderer builds a select: the trigger with what it says, its cross and its chevron, and the
  /// panel that opens under it.
  pub fn render(mut self) -> Element {
    let mut trigger = self.trigger().child(self.value());
    if let Some(clear) = self.clear() {
      trigger = trigger.child(clear);
    }
    let trigger = trigger.child(self.chevron());
    let panel = self.panel();
    let fill = Dimension::Pct(100.0);
    if self.inline {
      // The panel in place under the trigger, as far from it as the popover would be.
      let gap = self
        .offset
        .clone()
        .unwrap_or_else(|| SpacingSize::Xs.into())
        .resolve(&self.theme().spacing(), 0.0);
      let mut root = Column::new().max_width(fill).spacing(gap).child(trigger).child(panel);
      if self.full_width {
        root = root.width(fill);
      }
      if let Some(key) = &self.key {
        root = root.key(key.clone());
      }
      return root.into();
    }
    // No wider than the room it has: the trigger in it is as wide as its recipe says at most.
    let mut root = Row::new().max_width(fill).child(trigger).child(panel);
    if self.full_width {
      root = root.width(fill);
    }
    if let Some(key) = &self.key {
      root = root.key(key.clone());
    }
    root.into()
  }
}

/// Every tree select has the one recipe and renderer, whatever its values are and however many it holds.
impl<T: Send + Sync + 'static, V: ?Sized + 'static> Themed for TreeSelect<T, V> {
  const NAME: &'static str = "tree_select";
  type Renderer = TreeSelectRenderer;

  /// The design's tree select (`pencil/lurq-design-system.pen`, Tree Select): a select's trigger, 260 by 36 on the
  /// background in an edge of 1 in the color of an input's, 6 round, its text 12 from its edge in the body's type
  /// and a chevron of 16 in the muted foreground 8 after it; 4 under it a panel of 260 on the popover's fill in an
  /// edge of 1 in the border's color, 8 round, with the popover's shadow and 8 of padding, which holds the design's
  /// tree view. The trigger under the pointer, focused, open, judged, read only and disabled is a select's. The
  /// board draws no tree longer than its panel, no clear control and no select with nothing to choose: the tree
  /// scrolls past 280, the cross is a combobox's, and the words of an empty panel are 13 in the muted foreground.
  fn recipe() -> Recipe {
    use TreeSelectPart::{Empty, Panel, Tree};
    let recipe = <Select as Themed>::recipe()
      .base(TreeSelectPart::Trigger, fading)
      .base(Panel, |s| {
        s.width(260.0)
          .max_width(Length::Fill)
          .padding_x(8.0)
          .padding_y(8.0)
          .gap(0.0)
      })
      .base(Tree, |s| s.max_height(280.0))
      .base(TreeSelectPart::Chevron, |s| s.square(IconSize::Md))
      .base(Empty, |s| {
        s.padding_x(8.0)
          .padding_y(6.0)
          .font_size(13.0)
          .foreground(PaletteColor::MutedForeground)
      })
      .state(SelectState::ReadOnly, TreeSelectPart::Trigger, |s| {
        s.cursor(CursorIcon::Default)
      });
    combobox::clear_rules(recipe)
  }
}

struct TreeSelectProps<T: 'static, V: ?Sized + 'static> {
  select: TreeSelect<T, V>,
}

/// A select built again is drawn again: what is chosen is read from the app's signal as it is built.
impl<T, V: ?Sized> PartialEq for TreeSelectProps<T, V> {
  fn eq(&self, _: &Self) -> bool {
    false
  }
}

#[cfg(feature = "devtools")]
impl<T, V: ?Sized> crate::app::component::DevtoolsInspectable for TreeSelectProps<T, V> {
  fn inspect(&self, formatter: &mut crate::app::component::DevtoolsFormatter<'_>) {
    formatter.value("TreeSelect", format!("{} items at the top", self.select.items.len()));
  }
}

/// What a mounted tree select keeps between renders.
struct TreeSelectHost<T, V: ?Sized> {
  kept: Kept<T>,
  _held: std::marker::PhantomData<fn() -> Box<V>>,
}

impl<T: Clone + PartialEq + Send + Sync + 'static, V: ?Sized + 'static> Component for TreeSelectHost<T, V> {
  type Props = TreeSelectProps<T, V>;

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      kept: Kept {
        open: ctx.signal(false),
        anchor: ElementRef::new(),
        branches: Arc::default(),
        shown: Arc::default(),
        revision: Some(ctx.signal(0_u64)),
      },
      _held: std::marker::PhantomData,
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    if let Some(revision) = &self.kept.revision {
      // Read so that a branch that opens or closes renders the select again.
      revision.get();
    }
    ctx
      .props::<TreeSelectProps<T, V>>()
      .select
      .build_with(&current_theme(), self.kept.clone())
  }
}
