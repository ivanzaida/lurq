//! A combobox: a select a person types in, whose list what is typed filters, bound to a signal.

use std::{
  any::Any,
  collections::HashMap,
  hash::{DefaultHasher, Hash, Hasher},
  panic::Location,
  sync::{Arc, LazyLock, Mutex, Weak},
};

use super::{
  field::{FieldControl, InField},
  icon::Icon,
  icons, names,
  part_style::{Length, PartStyle},
  recipe::{Case, Interaction, Name, Recipe, ResolvedPart, ResolvedRecipe},
  select::{self, engine_style, px},
  shared::{fading, pressable},
  spinner::Spinner,
  text_box::{self, BoxStates, EditHandler, Judgment, TextBoxSize},
  theme::{Themed, current_theme},
};
use crate::{
  app::{
    ctx::OutsidePress,
    events::{KeyboardEvent, MouseEvent, TextInputEvent},
    theme::{IconSize, PaletteColor, Theme},
  },
  components::{self, Row, SelectOption, SelectTriggerState},
  core::{Signal, SignalValue},
  layout::Alignment,
  node::{
    CursorIcon, Element, NodeUpdate, SelectStyle, dimension::Dimension, node_kind::SelectChangeCallback,
    select_state::WeakSelectState,
  },
};

/// The parts of a combobox a recipe styles. Those it shares with a select have a select's names, and the engine draws
/// its list as it draws a select's.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ComboboxPart {
  /// The box that is typed in and that the list opens under: its size, its edge, its fill, its ring, and the type of
  /// what is typed. Its gap is the room between what is typed and the chevron.
  Trigger,
  /// What is typed, over the trigger's type and color.
  Value,
  /// What the box says while nothing is typed, over the value's type and color.
  Placeholder,
  /// The glyph at the end of the box: its side and its color.
  Chevron,
  /// The panel of the open list: its fill, edge, corners, shadow and padding. Its gap is the room between the box
  /// and it; its height is the most it is, past which it scrolls.
  Panel,
  /// An option of the open list: its height, padding, corners, type and color. Under the pointer or the keys it is
  /// `hovered`. Its gap is the room between its label and the check.
  Item,
  /// The mark at the end of the chosen item: its side and its color.
  Check,
  /// The second, smaller line of an option that has one ([`SelectOption::detail`]).
  Detail,
  /// The short text at the end of an option that has one ([`SelectOption::hint`]).
  Hint,
  /// The label over a group of options ([`SelectOption::group`]).
  Group,
  /// The line between two groups: its fill is the line's color, its height the line's, and its padding the room
  /// around it.
  Separator,
  /// The row after the options that cannot be chosen: "No results" while what is typed leaves none, and the
  /// spinner with "Loading results…" while more are loading. Its gap is the room between the spinner and the text.
  Status,
  /// The spinner of the status row while more options are loading: its color.
  Spinner,
  /// What takes the value and what is typed away, at the end of the box ([`Combobox::clearable`]): its side, its
  /// corners and its color.
  Clear,
  /// The cross in it: its side.
  ClearIcon,
}

names!(ComboboxPart {
  Trigger => "trigger",
  Value => "value",
  Placeholder => "placeholder",
  Chevron => "chevron",
  Panel => "panel",
  Item => "item",
  Check => "check",
  Detail => "detail",
  Hint => "hint",
  Group => "group",
  Separator => "separator",
  Status => "status",
  Spinner => "spinner",
  Clear => "clear",
  ClearIcon => "clear_icon",
});

/// The sizes of a combobox: the heights of the theme's controls, as an input and a select have them.
pub type ComboboxSize = TextBoxSize;

/// The states of a combobox a recipe styles, besides what the pointer and the keyboard do to it. They have the
/// names of a select's.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ComboboxState {
  /// Its list is open. The engine says when; the recipe's rules for it are what the box looks like then.
  Open,
  /// What is chosen is wrong, or nothing is and something has to be: its edge is in the destructive color.
  Invalid,
  /// What is chosen is allowed, with a caveat: its edge is in the warning color.
  Warning,
  /// What is chosen was checked and is right: its edge is in the success color.
  Valid,
  /// What is chosen can be read and not changed: its list does not open, and what is typed is not taken.
  ReadOnly,
  /// It takes no input. Of an item, resolved on its parts: it cannot be chosen ([`SelectOption::disabled`]).
  Disabled,
  /// It is as wide as what holds it.
  FullWidth,
  /// Of an item, resolved on its parts: it is the one chosen. Its rules are for `Item` and `Check`.
  Selected,
  /// It is in a fluid field ([`Field::fluid`](super::Field::fluid)): the field's label is inside its box, over
  /// what is typed.
  Fluid,
}

names!(ComboboxState {
  Open => "open",
  Invalid => "invalid",
  Warning => "warning",
  Valid => "valid",
  ReadOnly => "read_only",
  Disabled => "disabled",
  FullWidth => "full_width",
  Selected => "selected",
  Fluid => "fluid",
});

/// What builds a combobox's nodes: a theme's in place of the default one.
pub type ComboboxRenderer = Arc<dyn Fn(ComboboxView) -> Element + Send + Sync>;

/// Whether an option of `label` is shown while `query` is typed.
pub(crate) type Filter = Arc<dyn Fn(&str, &str) -> bool + Send + Sync>;

/// The default filter: the options whose label contains what was typed, without regard to case.
pub(crate) fn contains(label: &str, query: &str) -> bool {
  label.to_lowercase().contains(&query.to_lowercase())
}

/// What the engine's select of a combobox or a multi select is built with, besides its options and its value.
pub(crate) struct Engine {
  pub style: SelectStyle,
  pub placeholder: Option<Arc<str>>,
  pub input: components::TextInput,
  pub trigger: Arc<dyn Fn(SelectTriggerState) -> Element + Send + Sync>,
  pub status: Option<Element>,
  pub free_text: bool,
  /// Whether the list opens and takes a choice: not where what is chosen is read only.
  pub read_only: bool,
}

/// What a select, a combobox or a multi select the app holds the value of (`controlled`) is across renders: where
/// in the app's code it is built and its id or key, out of the range of the ids of signals.
fn controlled_binding(at: &'static Location<'static>, name: Option<&str>) -> usize {
  let mut hasher = DefaultHasher::new();
  (at.file(), at.line(), at.column(), name).hash(&mut hasher);
  (hasher.finish() as usize) | (1 << (usize::BITS - 1))
}

/// The value of none of a single value of type `T` where a value of it can be none without an `Option`: the empty
/// text of a `String`.
fn empty_text<T: Clone + 'static>() -> Option<T> {
  (&String::new() as &dyn Any).downcast_ref::<T>().cloned()
}

/// The engine's select `select` with its choices made through `on_change`, its identity across renders `binding`,
/// and read only where it says.
fn wired(select: Element, binding: usize, read_only: bool, on_change: SelectChangeCallback) -> Element {
  let node = select
    .into_node()
    .select_on_change(on_change)
    .select_binding(binding)
    .select_read_only(read_only);
  Element::from_node(node)
}

/// What a select, a combobox or a multi select chooses from and what it holds, whatever the type of its values.
pub(crate) trait Choices: Send + Sync {
  /// What it is across renders: the id of the signal it is bound to, or, held by the app, where it is built.
  fn binding(&self) -> usize;
  /// The id or key of the component: the binding of a controlled one is made with it too, so that two built at the
  /// one place in the app's code are told apart.
  fn name(&mut self, name: Option<&str>);
  /// The label of each option, in order.
  fn labels(&self) -> Vec<Arc<str>>;
  /// Where each option is in the groups: the group it is in, by the index of the option that starts it, and its
  /// label.
  fn groups(&self) -> Vec<Option<(usize, Arc<str>)>>;
  fn disabled(&self) -> Vec<bool>;
  /// The options chosen, by their index. Reading it in a render renders again when it changes.
  fn chosen(&self) -> Vec<usize>;
  /// A value of a single one that is no option's, as text: what was typed and kept. Only a value of `String` can be.
  fn custom(&self) -> Option<String>;
  /// Makes `text` the value, where a value of the type can be text; whether it did.
  fn set_custom(&self, text: &str) -> bool;
  /// Chooses an option: the value of a single one, one more of several.
  fn choose(&self, index: usize);
  /// Takes an option out of what several hold.
  fn unchoose(&self, index: usize);
  /// Takes the value away: of several, all of it; of a single one, where it can be none ([`Choices::can_clear`]).
  fn clear(&self);
  /// Whether the value can be taken away: a single one's only where it can be none.
  fn can_clear(&self) -> bool;
  /// The engine's select of the options `shown`, as a combobox: what `engine` says.
  fn build(self: Arc<Self>, shown: &[usize], engine: Engine) -> Element;
  /// The engine's select of every option, in `style`, as a plain select.
  fn build_plain(self: Arc<Self>, style: SelectStyle, placeholder: Option<Arc<str>>, read_only: bool) -> Element;
}

/// The choices of a single value of type `T`, which hear its change in that type: what a select's and a combobox's
/// builder holds, and gives on as [`Choices`] as it builds.
pub(crate) trait OneOf<T>: Choices {
  /// Adds a handler of a change of the value.
  fn on_change(&mut self, handler: Arc<dyn Fn(&T) + Send + Sync>);
}

/// The choices of several values of type `T`, which hear their change as the whole new set: what a multi select's
/// builder holds.
pub(crate) trait ManyOf<T>: Choices {
  /// Adds a handler of a change of the values.
  fn on_change(&mut self, handler: Arc<dyn Fn(&[T]) + Send + Sync>);
}

/// The options of `options` at `shown`, each that comes first of its group in them starting it.
fn shown_options<T: Clone>(
  options: &[SelectOption<T>],
  groups: &[Option<(usize, Arc<str>)>],
  shown: &[usize],
) -> Vec<SelectOption<T>> {
  let mut last = None;
  shown
    .iter()
    .filter_map(|index| {
      let option = options.get(*index)?.clone();
      let group = groups.get(*index).cloned().flatten();
      let start = group.as_ref().map(|(start, _)| *start);
      let option = match group {
        // The option that starts its group may be filtered out: the first shown of the group starts it.
        Some((_, label)) if start != last && option.starts_group().is_none() => option.group(label),
        _ => option,
      };
      last = start;
      Some(option)
    })
    .collect()
}

fn groups_of<T>(options: &[SelectOption<T>]) -> Vec<Option<(usize, Arc<str>)>> {
  let mut current = None;
  options
    .iter()
    .enumerate()
    .map(|(index, option)| {
      if let Some(label) = option.starts_group() {
        current = Some((index, Arc::from(label)));
      }
      current.clone()
    })
    .collect()
}

/// The engine's select in what `engine` says: a combobox around its input.
fn dress<T>(select: components::Select<T>, engine: Engine) -> components::Select<T>
where
  T: Clone + PartialEq + Send + Sync + SignalValue + 'static,
  Vec<T>: SignalValue,
{
  let trigger = engine.trigger;
  // A press outside a combobox closes its list and reaches what it lands on, as a press outside a text field does:
  // the focus leaves the box for what was pressed, or for nothing.
  let mut select = select
    .outside_press(OutsidePress::PassThrough)
    .style(engine.style)
    .combobox(engine.input)
    .free_text(engine.free_text)
    .trigger(move |state| trigger(state));
  if let Some(placeholder) = engine.placeholder {
    select = select.placeholder(placeholder);
  }
  if let Some(status) = engine.status {
    select = select.status(status);
  }
  select
}

/// A single value and the options it is one of.
pub(crate) struct One<T: SignalValue> {
  /// The app's signal, or, of a value the app holds, one of this build that holds it.
  value: Signal<T>,
  options: Vec<SelectOption<T>>,
  /// Where in the app's code a controlled one is built; `None` for one bound to the app's signal, which it sets.
  at: Option<&'static Location<'static>>,
  binding: usize,
  /// The value of none, where the value can be none: `None` of an optional one, the empty text of a `String`.
  empty: Option<T>,
  told: Vec<Arc<dyn Fn(&T) + Send + Sync>>,
}

impl<T> One<T>
where
  T: Clone + PartialEq + Send + Sync + SignalValue + 'static,
{
  /// Bound to the app's signal: a choice sets it.
  pub(crate) fn bound(value: Signal<T>, options: Vec<SelectOption<T>>) -> Self {
    Self {
      binding: value.id(),
      value,
      options,
      at: None,
      empty: empty_text(),
      told: Vec::new(),
    }
  }

  /// The app's value, built at `at` in its code: a choice is told, and what the app answers with is shown.
  pub(crate) fn controlled(value: T, options: Vec<SelectOption<T>>, at: &'static Location<'static>) -> Self {
    Self {
      value: Signal::new(value),
      options,
      at: Some(at),
      binding: controlled_binding(at, None),
      empty: empty_text(),
      told: Vec::new(),
    }
  }

  /// Whose value of none is `empty`: an optional one's `None`.
  pub(crate) fn with_empty(mut self, empty: T) -> Self {
    self.empty = Some(empty);
    self
  }

  /// Makes `next` the value, where it is another: sets the app's signal of one bound to it, and tells the handlers.
  fn set(&self, next: T) {
    if self.value.get_untracked() == next {
      return;
    }
    match self.at {
      None => self.value.set(next.clone()),
      // The app answers: what the box shows of the value is what it answers with, whatever was chosen.
      Some(_) => forget_shown(self.binding),
    }
    for handler in &self.told {
      handler(&next);
    }
  }
}

impl<T> OneOf<T> for One<T>
where
  T: Clone + PartialEq + Send + Sync + SignalValue + 'static,
  Vec<T>: SignalValue,
{
  fn on_change(&mut self, handler: Arc<dyn Fn(&T) + Send + Sync>) {
    self.told.push(handler);
  }
}

impl<T> Choices for One<T>
where
  T: Clone + PartialEq + Send + Sync + SignalValue + 'static,
  Vec<T>: SignalValue,
{
  fn binding(&self) -> usize {
    self.binding
  }

  fn name(&mut self, name: Option<&str>) {
    if let Some(at) = self.at {
      self.binding = controlled_binding(at, name);
    }
  }

  fn labels(&self) -> Vec<Arc<str>> {
    self.options.iter().map(|option| Arc::from(option.label())).collect()
  }

  fn groups(&self) -> Vec<Option<(usize, Arc<str>)>> {
    groups_of(&self.options)
  }

  fn disabled(&self) -> Vec<bool> {
    self.options.iter().map(SelectOption::is_disabled).collect()
  }

  fn chosen(&self) -> Vec<usize> {
    let value = self.value.get();
    self
      .options
      .iter()
      .position(|option| *option.value() == value)
      .into_iter()
      .collect()
  }

  fn custom(&self) -> Option<String> {
    let text = (&self.value as &dyn Any).downcast_ref::<Signal<String>>()?.get();
    let chosen = self
      .options
      .iter()
      .any(|option| (option.value() as &dyn Any).downcast_ref::<String>() == Some(&text));
    (!chosen && !text.is_empty()).then_some(text)
  }

  fn set_custom(&self, text: &str) -> bool {
    match (&text.to_owned() as &dyn Any).downcast_ref::<T>() {
      Some(value) => {
        self.set(value.clone());
        true
      }
      None => false,
    }
  }

  fn choose(&self, index: usize) {
    if let Some(option) = self.options.get(index).filter(|option| !option.is_disabled()) {
      self.set(option.value().clone());
    }
  }

  fn unchoose(&self, _index: usize) {}

  fn clear(&self) {
    if let Some(empty) = &self.empty {
      self.set(empty.clone());
    }
  }

  fn can_clear(&self) -> bool {
    self.empty.is_some()
  }

  fn build(self: Arc<Self>, shown: &[usize], engine: Engine) -> Element {
    let options = shown_options(&self.options, &self.groups(), shown);
    let read_only = engine.read_only;
    let select = dress(components::Select::new(self.value.clone()).options(options), engine);
    let (me, shown) = (self.clone(), shown.to_vec());
    wired(
      select.into(),
      self.binding,
      read_only,
      Arc::new(move |index| {
        if let Some(option) = shown.get(index) {
          me.choose(*option);
        }
      }),
    )
  }

  fn build_plain(self: Arc<Self>, style: SelectStyle, placeholder: Option<Arc<str>>, read_only: bool) -> Element {
    let mut select = components::Select::new(self.value.clone())
      .options(self.options.clone())
      .style(style);
    if let Some(placeholder) = placeholder {
      select = select.placeholder(placeholder);
    }
    let me = self.clone();
    wired(
      select.into(),
      self.binding,
      read_only,
      Arc::new(move |index| me.choose(index)),
    )
  }
}

/// Several values and the options they are of.
pub(crate) struct Many<T>
where
  Vec<T>: SignalValue,
{
  /// The app's signal, or, of values the app holds, one of this build that holds them.
  value: Signal<Vec<T>>,
  options: Vec<SelectOption<T>>,
  /// Where in the app's code a controlled one is built; `None` for one bound to the app's signal, which it sets.
  at: Option<&'static Location<'static>>,
  binding: usize,
  told: Vec<Arc<dyn Fn(&[T]) + Send + Sync>>,
}

impl<T> Many<T>
where
  T: Clone + PartialEq + Send + Sync + SignalValue + 'static,
  Vec<T>: SignalValue,
{
  /// Bound to the app's signal: a choice sets it.
  pub(crate) fn bound(value: Signal<Vec<T>>, options: Vec<SelectOption<T>>) -> Self {
    Self {
      binding: value.id(),
      value,
      options,
      at: None,
      told: Vec::new(),
    }
  }

  /// The app's values, built at `at` in its code: a choice is told, and what the app answers with is shown.
  pub(crate) fn controlled(values: Vec<T>, options: Vec<SelectOption<T>>, at: &'static Location<'static>) -> Self {
    Self {
      value: Signal::new(values),
      options,
      at: Some(at),
      binding: controlled_binding(at, None),
      told: Vec::new(),
    }
  }

  /// Makes `next` the values, where they are others: sets the app's signal of one bound to it, and tells the
  /// handlers the whole of them.
  fn set(&self, next: Vec<T>) {
    if self.value.get_untracked() == next {
      return;
    }
    if self.at.is_none() {
      self.value.set(next.clone());
    }
    for handler in &self.told {
      handler(&next);
    }
  }
}

impl<T> ManyOf<T> for Many<T>
where
  T: Clone + PartialEq + Send + Sync + SignalValue + 'static,
  Vec<T>: SignalValue,
{
  fn on_change(&mut self, handler: Arc<dyn Fn(&[T]) + Send + Sync>) {
    self.told.push(handler);
  }
}

impl<T> Choices for Many<T>
where
  T: Clone + PartialEq + Send + Sync + SignalValue + 'static,
  Vec<T>: SignalValue,
{
  fn binding(&self) -> usize {
    self.binding
  }

  fn name(&mut self, name: Option<&str>) {
    if let Some(at) = self.at {
      self.binding = controlled_binding(at, name);
    }
  }

  fn labels(&self) -> Vec<Arc<str>> {
    self.options.iter().map(|option| Arc::from(option.label())).collect()
  }

  fn groups(&self) -> Vec<Option<(usize, Arc<str>)>> {
    groups_of(&self.options)
  }

  fn disabled(&self) -> Vec<bool> {
    self.options.iter().map(SelectOption::is_disabled).collect()
  }

  /// In the order they were chosen in: the order of the tags.
  fn chosen(&self) -> Vec<usize> {
    self
      .value
      .get()
      .iter()
      .filter_map(|value| self.options.iter().position(|option| option.value() == value))
      .collect()
  }

  fn custom(&self) -> Option<String> {
    None
  }

  fn set_custom(&self, _text: &str) -> bool {
    false
  }

  fn choose(&self, index: usize) {
    if let Some(option) = self.options.get(index).filter(|option| !option.is_disabled()) {
      let mut next = self.value.get_untracked();
      if !next.contains(option.value()) {
        next.push(option.value().clone());
      }
      self.set(next);
    }
  }

  fn unchoose(&self, index: usize) {
    if let Some(option) = self.options.get(index) {
      let mut next = self.value.get_untracked();
      next.retain(|held| held != option.value());
      self.set(next);
    }
  }

  fn clear(&self) {
    self.set(Vec::new());
  }

  fn can_clear(&self) -> bool {
    true
  }

  fn build(self: Arc<Self>, shown: &[usize], engine: Engine) -> Element {
    let options = shown_options(&self.options, &self.groups(), shown);
    let read_only = engine.read_only;
    let select = dress(
      components::Select::multiple(self.value.clone()).options(options),
      engine,
    );
    let (me, shown) = (self.clone(), shown.to_vec());
    wired(
      select.into(),
      self.binding,
      read_only,
      Arc::new(move |index| {
        if let Some(&option) = shown.get(index) {
          me.toggle(option);
        }
      }),
    )
  }

  fn build_plain(self: Arc<Self>, style: SelectStyle, placeholder: Option<Arc<str>>, read_only: bool) -> Element {
    let mut select = components::Select::multiple(self.value.clone())
      .options(self.options.clone())
      .style(style);
    if let Some(placeholder) = placeholder {
      select = select.placeholder(placeholder);
    }
    let me = self.clone();
    wired(
      select.into(),
      self.binding,
      read_only,
      Arc::new(move |index| me.toggle(index)),
    )
  }
}

impl<T> Many<T>
where
  T: Clone + PartialEq + Send + Sync + SignalValue + 'static,
  Vec<T>: SignalValue,
{
  /// The option at `index` in what is held, or out of it.
  fn toggle(&self, index: usize) {
    let held = self
      .options
      .get(index)
      .is_some_and(|option| self.value.get_untracked().contains(option.value()));
    if held {
      self.unchoose(index);
    } else {
      self.choose(index);
    }
  }
}

/// What is typed in a combobox that the app gave no signal of its own for: one for each signal a combobox is bound to,
/// while one is in the tree. `alive` is held by the input of the last one built; when nothing holds it the combobox
/// left the tree, and what was typed in it goes with it.
struct Typed {
  query: Signal<String>,
  /// What the box showed of the value when it was last built.
  shown: Option<String>,
  alive: Weak<()>,
}

static TYPED: LazyLock<Mutex<HashMap<usize, Typed>>> = LazyLock::new(Default::default);

/// What is typed in the combobox bound to the signal `binding`, and what keeps it while the combobox is in the tree.
/// `shown` is what the box shows of the value: when it is another than the last build's, the app or a choice changed
/// the value, and what is typed becomes it. `None` keeps what is typed whatever the value is.
pub(crate) fn typed(binding: usize, shown: Option<&str>) -> (Signal<String>, Arc<()>) {
  let mut all = TYPED.lock().unwrap();
  all.retain(|_, typed| typed.alive.strong_count() > 0);
  if let Some(typed) = all.get_mut(&binding)
    && let Some(alive) = typed.alive.upgrade()
  {
    if let Some(shown) = shown
      && typed.shown.as_deref() != Some(shown)
    {
      typed.shown = Some(shown.to_owned());
      if typed.query.get_untracked() != shown {
        typed.query.set(shown.to_owned());
      }
    }
    return (typed.query.clone(), alive);
  }
  let alive = Arc::new(());
  let query = Signal::new(shown.unwrap_or_default().to_owned());
  all.insert(
    binding,
    Typed {
      query: query.clone(),
      shown: shown.map(str::to_owned),
      alive: Arc::downgrade(&alive),
    },
  );
  (query, alive)
}

/// Makes the next build of the combobox bound to `binding` put what the box shows of the value in what is typed:
/// the app holds the value, and the box shows what it answers with.
fn forget_shown(binding: usize) {
  if let Some(typed) = TYPED.lock().unwrap().get_mut(&binding) {
    typed.shown = None;
  }
}

/// The options shown while `query` is typed: all of them while it is nothing or what the box shows of the value,
/// else those `filter` keeps.
pub(crate) fn filtered(labels: &[Arc<str>], query: &str, shown: Option<&str>, filter: &Filter) -> Vec<usize> {
  let all = query.trim().is_empty() || Some(query) == shown;
  (0..labels.len())
    .filter(|index| all || filter(&labels[*index], query))
    .collect()
}

/// A combobox: a select a person types in, whose list what is typed filters. A framework, a country, an assignee
/// among many.
///
/// ```
/// use lurq::{components::SelectOption, core::Signal, ui::Combobox};
///
/// let framework = Signal::new("lurq");
/// let combobox = Combobox::new(framework, [("lurq", "lurq"), ("gpui", "GPUI"), ("slint", "Slint")])
///   .placeholder("Select framework");
/// // What is typed and is no option's label is kept: a value of `String` becomes it.
/// let tag = Signal::new(String::new());
/// let free = Combobox::new(tag, [SelectOption::new("rust".to_owned(), "Rust")]).allow_custom(true);
/// // Held by the app: it is told of a choice, and the box shows what it answers with.
/// let held = Combobox::controlled("gpui", [("lurq", "lurq"), ("gpui", "GPUI")], |framework| {
///   println!("{framework}")
/// });
/// ```
///
/// It is the engine's [`Select`](crate::components::Select) as a combobox ([`Select::combobox`]
/// (crate::components::Select::combobox)), in the design's dress: the box of a select with a text input in it, which
/// keeps the focus while the list is open. [`Combobox::on_change`] hears a choice. Typing opens the list and filters
/// it, with the first option that matches highlighted; Down and Up move the highlight, which a row shows as it shows
/// the pointer, Enter chooses, Escape closes the list and a second Escape clears what was typed, and Tab chooses
/// nothing and goes on. Choosing puts the option's label in the box, and the list opens whole again until something
/// else is typed. When the focus leaves, what was typed goes back to the chosen option's label, unless custom values
/// are allowed. The options are a select's: a detail, a hint, elements before and after the label, groups, disabled.
pub struct Combobox<V = ()> {
  choices: Box<dyn OneOf<V>>,
  placeholder: Option<Arc<str>>,
  boxed: BoxStates,
  full_width: bool,
  states: Vec<Name>,
  rules: Recipe,
  forced: Option<Interaction>,
  filter: Filter,
  on_query: Vec<EditHandler>,
  on_submit: Vec<SubmitHandler>,
  loading: bool,
  empty_text: Arc<str>,
  loading_text: Arc<str>,
  allow_custom: bool,
  clearable: bool,
  clear_label: Arc<str>,
  query: Option<Signal<String>>,
  name: Option<Arc<str>>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

/// What is told of Enter in a combobox: what the box holds.
pub(crate) type SubmitHandler = Arc<dyn Fn(&str) + Send + Sync>;

impl<T> Combobox<T>
where
  T: Clone + PartialEq + Send + Sync + SignalValue + 'static,
  Vec<T>: SignalValue,
{
  /// A combobox of `options` bound to `value`: the option whose value the signal has is chosen, and choosing another
  /// sets it. An option is a `(value, label)` or a [`SelectOption`].
  pub fn new(value: Signal<T>, options: impl IntoIterator<Item = impl Into<SelectOption<T>>>) -> Self {
    Self::of(Box::new(One::bound(
      value,
      options.into_iter().map(Into::into).collect(),
    )))
  }

  /// A combobox of the app's `value`, which the app holds: a choice is told to `on_change`, and the box shows what
  /// the app answers with. Two built at the one place in the app's code, in a loop, are told apart by their `id` or
  /// `key`.
  #[track_caller]
  pub fn controlled(
    value: T,
    options: impl IntoIterator<Item = impl Into<SelectOption<T>>>,
    on_change: impl Fn(&T) + Send + Sync + 'static,
  ) -> Self {
    let options = options.into_iter().map(Into::into).collect();
    Self::of(Box::new(One::controlled(value, options, Location::caller()))).on_change(on_change)
  }
}

impl<T> Combobox<Option<T>>
where
  T: Clone + PartialEq + Send + Sync + SignalValue + 'static,
  Option<T>: SignalValue,
  Vec<Option<T>>: SignalValue,
{
  /// A combobox bound to `value`, which may hold no value: `None` is no option's, and the box shows its placeholder;
  /// [`Combobox::clearable`] takes the value away.
  pub fn optional(value: Signal<Option<T>>, options: impl IntoIterator<Item = impl Into<SelectOption<T>>>) -> Self {
    let options = options.into_iter().map(|option| option.into().map(Some)).collect();
    Self::of(Box::new(One::bound(value, options).with_empty(None)))
  }
}

impl<V: 'static> Combobox<V> {
  fn of(choices: Box<dyn OneOf<V>>) -> Self {
    Self {
      choices,
      placeholder: None,
      boxed: BoxStates::default(),
      full_width: false,
      states: Vec::new(),
      rules: Recipe::new(),
      forced: None,
      filter: Arc::new(contains),
      on_query: Vec::new(),
      on_submit: Vec::new(),
      loading: false,
      empty_text: super::current_locale().common.no_results.clone(),
      loading_text: super::current_locale().common.loading_results.clone(),
      allow_custom: false,
      clearable: false,
      clear_label: super::current_locale().common.clear.clone(),
      query: None,
      name: None,
      id: None,
      key: None,
    }
  }

  /// Called with the new value when a person chooses an option, or keeps a value of its own
  /// ([`Combobox::allow_custom`]), or clears it: not when the app sets the signal, and not for what is chosen
  /// already. The handler takes the type of the value: `on_change(|framework: &&str| ..)` of a combobox of `&str`,
  /// `&Option<T>` of an [`optional`](Combobox::optional) one; a handler of another type does not compile.
  pub fn on_change(mut self, handler: impl Fn(&V) + Send + Sync + 'static) -> Self {
    self.choices.on_change(Arc::new(handler));
    self
  }

  /// What the box says while nothing is typed.
  pub fn placeholder(mut self, placeholder: &str) -> Self {
    self.placeholder = Some(Arc::from(placeholder));
    self
  }

  /// What the combobox is called, to what reads the tree. The field it is in shows its label.
  pub fn label(mut self, label: &str) -> Self {
    self.boxed.label = Some(Arc::from(label));
    self
  }

  /// A [`ComboboxSize`], or a size the theme's recipe names.
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

  /// Whether what is chosen can be read and not changed: the list does not open, what is typed is not taken, and
  /// there is no clear control. It keeps the focus and is read by what reads the tree.
  pub fn read_only(mut self, read_only: bool) -> Self {
    self.boxed.read_only = read_only;
    self
  }

  /// Whether the combobox takes no input.
  pub fn disabled(mut self, disabled: bool) -> Self {
    self.boxed.disabled = disabled;
    self
  }

  /// Whether the combobox is as wide as what holds it, in place of the design's 260.
  pub fn full_width(mut self, full_width: bool) -> Self {
    self.full_width = full_width;
    self
  }

  /// Which options are shown while something is typed: `filter(label, typed)`. By default those whose label
  /// contains what was typed, without regard to case.
  pub fn filter(mut self, filter: impl Fn(&str, &str) -> bool + Send + Sync + 'static) -> Self {
    self.filter = Arc::new(filter);
    self
  }

  /// Called with what the box will hold at each edit: for an app that fetches its options as a person types. Its
  /// options then are what it fetched, which it may show all of with a `filter` that keeps every one.
  pub fn on_query(mut self, handler: impl Fn(&str) + Send + Sync + 'static) -> Self {
    self
      .on_query
      .push(Arc::new(move |event: &TextInputEvent| handler(event.new_value())));
    self
  }

  /// Called on Enter with what the box holds, where Enter chooses nothing: the list is closed, or nothing in it is
  /// highlighted. Enter on a highlighted option chooses it and submits nothing.
  pub fn on_submit(mut self, handler: impl Fn(&str) + Send + Sync + 'static) -> Self {
    self.on_submit.push(Arc::new(handler));
    self
  }

  /// Whether more options are loading: the list ends in a row with a spinner that says so.
  pub fn loading(mut self, loading: bool) -> Self {
    self.loading = loading;
    self
  }

  /// What the list says while what is typed leaves no option: "No results" unless it is told otherwise.
  pub fn empty_text(mut self, text: &str) -> Self {
    self.empty_text = Arc::from(text);
    self
  }

  /// What the row of [`Combobox::loading`] says: "Loading results…" unless it is told otherwise.
  pub fn loading_text(mut self, text: &str) -> Self {
    self.loading_text = Arc::from(text);
    self
  }

  /// Whether what is typed may be no option's label. Typing then highlights nothing, and Enter, or the focus
  /// leaving, keeps what was typed: an option whose label it is, without regard to case, is chosen, and anything
  /// else becomes the value of a combobox of `String`. The app hears it as it is typed ([`Combobox::on_query`]).
  pub fn allow_custom(mut self, allow: bool) -> Self {
    self.allow_custom = allow;
    self
  }

  /// A cross at the end of the box, while something is chosen, that takes the value and what is typed away: the
  /// value is `None` of an [`optional`](Combobox::optional) combobox and the empty text of a combobox of `String`.
  /// A combobox of a value that cannot be none has no cross.
  pub fn clearable(mut self, clearable: bool) -> Self {
    self.clearable = clearable;
    self
  }

  /// What the clear control is called to what reads the tree: "Clear" unless it is told otherwise.
  pub fn clear_label(mut self, label: &str) -> Self {
    self.clear_label = Arc::from(label);
    self
  }

  /// The signal of what is typed, where the app keeps it: to read it, or to set it. Without one the combobox keeps
  /// it itself while it is in the tree, one for each signal of a value.
  pub fn query(mut self, query: Signal<String>) -> Self {
    self.query = Some(query);
    self
  }

  /// Puts the combobox in a state the theme's recipe names, or takes it out. `state(ComboboxState::Open, true)` draws
  /// the box as it is while the list is open, with the list closed: for a gallery or a test.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// The name the combobox's value has in the form it is in: the label of the chosen option.
  #[cfg(feature = "form")]
  pub fn name(mut self, name: impl Into<Arc<str>>) -> Self {
    self.name = Some(name.into());
    self
  }

  /// Styles one part of this combobox over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(PartStyle) -> PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for this combobox.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// Draws the box as it is under the pointer or focused, with nothing pointing at it: for a gallery or a test.
  pub fn force(mut self, interaction: Option<Interaction>) -> Self {
    self.forced = interaction;
    self
  }

  /// The id of the combobox's node: the box the list opens under. Its input has the id with `-input` after it.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// Builds the combobox in `theme`. Converting a combobox into an element does this in the theme of what is being
  /// rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let case = self
      .boxed
      .case(self.states.clone())
      .state(ComboboxState::FullWidth, self.full_width);
    let mut choices = self.choices;
    choices.name(self.id.as_deref().or(self.key.as_deref()));
    let choices: Box<dyn Choices> = choices;
    let choices: Arc<dyn Choices> = Arc::from(choices);
    let labels = choices.labels();
    let chosen = choices.chosen();
    // What the box shows of the value: the chosen option's label, or a value of its own that was typed and kept.
    let shown = match chosen.first() {
      Some(index) => labels.get(*index).map(|label| label.to_string()),
      None => choices.custom(),
    }
    .unwrap_or_default();
    let (query, alive) = match self.query {
      Some(query) => (query, Arc::new(())),
      None => typed(choices.binding(), Some(&shown)),
    };
    let typed_now = query.get();
    let shown_options = filtered(&labels, &typed_now, Some(&shown), &self.filter);
    let view = ComboboxView {
      placeholder: self.placeholder,
      label: self.boxed.label.clone(),
      invalid: self.boxed.judgment() == Some(Judgment::Invalid),
      disabled: self.boxed.disabled,
      loading: self.loading,
      recipe: theme.resolve::<Combobox>(&case, Some(&self.rules)),
      case,
      rules: self.rules,
      forced: self.forced,
      clearable: self.clearable && choices.can_clear(),
      clear_label: self.clear_label,
      choices,
      labels,
      shown_text: shown,
      shown: shown_options,
      query,
      alive,
      on_query: self.on_query,
      on_submit: self.on_submit,
      opened: Arc::new(Mutex::new(None)),
      empty_text: self.empty_text,
      loading_text: self.loading_text,
      allow_custom: self.allow_custom,
      boxed: self.boxed,
      name: self.name,
      id: self.id,
      key: self.key,
    };
    match theme.renderer::<Combobox>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }
}

impl<V: 'static> From<Combobox<V>> for Element {
  fn from(combobox: Combobox<V>) -> Self {
    combobox.build(&current_theme())
  }
}

impl<V: 'static> FieldControl for Combobox<V> {
  /// Called by the field's label unless it has a name of its own, judged as the field is unless the app judged the
  /// combobox itself, and disabled where the field is.
  fn in_field(mut self, field: &InField) -> Element {
    self.boxed.in_field(field);
    self.into()
  }

  fn has_fluid_form(&self) -> bool {
    true
  }
}

/// A combobox as a renderer sees it: what it was given, the style of its parts, and its behavior ready to be dressed.
pub struct ComboboxView {
  pub placeholder: Option<Arc<str>>,
  /// What it is called, to what reads the tree.
  pub label: Option<Arc<str>>,
  /// Whether what is chosen is wrong, as the combobox shows it.
  pub invalid: bool,
  pub disabled: bool,
  /// Whether more options are loading.
  pub loading: bool,
  /// The style of each part of this combobox.
  pub recipe: ResolvedRecipe,
  case: Case,
  rules: Recipe,
  forced: Option<Interaction>,
  choices: Arc<dyn Choices>,
  labels: Vec<Arc<str>>,
  shown_text: String,
  shown: Vec<usize>,
  query: Signal<String>,
  alive: Arc<()>,
  on_query: Vec<EditHandler>,
  on_submit: Vec<SubmitHandler>,
  /// The engine's select once it is built: whether its list is open, for what Enter does.
  opened: Arc<Mutex<Option<WeakSelectState>>>,
  empty_text: Arc<str>,
  loading_text: Arc<str>,
  allow_custom: bool,
  clearable: bool,
  clear_label: Arc<str>,
  boxed: BoxStates,
  // Read only where the engine has forms.
  #[cfg_attr(not(feature = "form"), allow(dead_code))]
  name: Option<Arc<str>>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl ComboboxView {
  /// The style of a part, as this combobox is to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part).forced(self.forced)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// The recipe resolved with `state` on as well: what the box is while the list is open, or an item while it is
  /// chosen or disabled.
  pub fn in_state(&self, state: impl Into<Name>) -> ResolvedRecipe {
    let case = self.case.clone().state(state, true);
    self.theme().resolve::<Combobox>(&case, Some(&self.rules))
  }

  /// What is typed: the signal the input is bound to.
  pub fn query(&self) -> Signal<String> {
    self.query.clone()
  }

  /// The labels of the options the list shows for what is typed.
  pub fn shown(&self) -> Vec<Arc<str>> {
    self
      .shown
      .iter()
      .filter_map(|index| self.labels.get(*index).cloned())
      .collect()
  }

  /// The engine's style for the box and its list, from the recipe, as a select's.
  pub fn select_style(&self) -> SelectStyle {
    engine_style(&self.recipe, self.forced, &|state| self.in_state(state))
  }

  /// The text input in the box, in the type of the value and of the placeholder, with no box of its own: a stop of
  /// Tab, called by the combobox's label. What leaving it does to what was typed is set on it here.
  pub fn input(&self) -> components::TextInput {
    let theme = self.theme();
    let value = self
      .part(ComboboxPart::Trigger)
      .rest()
      .merged(&self.part(ComboboxPart::Value).rest());
    let placeholder = value.clone().merged(&self.part(ComboboxPart::Placeholder).rest());
    let control = components::TextInput::new(self.query.clone())
      .single_line()
      .text_style(text_box::ink(theme, &value))
      .placeholder_style(text_box::ink(theme, &placeholder))
      .min_width(0.0)
      .flex(1.0);
    let mut control = text_box::text_box(control, &self.boxed, self.on_query.clone());
    if let Some(placeholder) = &self.placeholder {
      control = control.placeholder(placeholder);
    }
    if let Some(id) = &self.id {
      control = control.id(format!("{id}-input"));
    }
    let leave = self.leave();
    control = control.on_blur(move || leave());
    if self.allow_custom {
      let keep = self.leave();
      control = control.on_key_down(move |event: KeyboardEvent| {
        if event.focus_within && !event.composing && event.key == "Enter" {
          keep();
        }
      });
    }
    if !self.on_submit.is_empty() {
      let (submit, query, opened) = (self.on_submit.clone(), self.query.clone(), self.opened.clone());
      control = control.on_key_down(move |event: KeyboardEvent| {
        let plain = !event.ctrl && !event.alt && !event.meta && !event.shift;
        if !event.focus_within || event.composing || !plain || event.key != "Enter" {
          return;
        }
        // Enter on a highlighted option of the open list chooses it: the engine does that after the handlers.
        let chooses = opened
          .lock()
          .unwrap()
          .as_ref()
          .and_then(WeakSelectState::upgrade)
          .is_some_and(|state| state.is_open() && state.highlighted().is_some());
        if !chooses {
          let text = query.get_untracked();
          for handler in &submit {
            handler(&text);
          }
        }
      });
    }
    control
  }

  /// The cross at the end of the box that takes the value and what is typed away, while something is chosen and the
  /// combobox is clearable. It is called "Clear" to what reads the tree, or its `clear_label`.
  pub fn clear(&self) -> Option<Element> {
    if !self.clearable || self.boxed.read_only || (self.shown_text.is_empty() && self.query.get().is_empty()) {
      return None;
    }
    let (choices, query) = (self.choices.clone(), self.query.clone());
    Some(clear_control(
      &self.part(ComboboxPart::Clear),
      &self.part(ComboboxPart::ClearIcon),
      self.theme(),
      self.disabled,
      &self.clear_label,
      move || {
        choices.clear();
        if !query.get_untracked().is_empty() {
          query.set(String::new());
        }
      },
    ))
  }

  /// What leaving the box does to what was typed: it goes back to what the box shows of the value, or, where custom
  /// values are allowed, it is kept.
  fn leave(&self) -> impl Fn() + Send + Sync + 'static {
    let (choices, query, shown, custom) = (
      self.choices.clone(),
      self.query.clone(),
      self.shown_text.clone(),
      self.allow_custom,
    );
    let (labels, disabled, alive) = (self.labels.clone(), self.choices.disabled(), self.alive.clone());
    move || {
      // Held while the input is: what was typed is kept for the combobox while it is in the tree.
      let _ = &alive;
      let text = query.get_untracked();
      if !custom {
        if text != shown {
          query.set(shown.clone());
        }
        return;
      }
      let named = labels
        .iter()
        .enumerate()
        .position(|(index, label)| !disabled[index] && label.to_lowercase() == text.trim().to_lowercase());
      match named {
        Some(index) => {
          choices.choose(index);
          if text != *labels[index] {
            query.set(labels[index].to_string());
          }
        }
        None => {
          choices.set_custom(text.trim());
        }
      }
    }
  }

  /// The glyph at the end of the box, in the style of its part: the board's two chevrons, up and down. The pointer
  /// passes through it to the box.
  pub fn chevron(&self) -> Element {
    let part = self.part(ComboboxPart::Chevron);
    let side = part.rest().width.unwrap_or(Length::Icon(IconSize::Md));
    Row::new()
      .part(&part)
      .flex_shrink(0.0)
      .pointer_events_none()
      .child(Icon::new(icons::CHEVRONS_VERTICAL).size(side).build(self.theme()))
      .into()
  }

  /// The row after the options: the spinner and "Loading results…" while more are loading, "No results" while what is
  /// typed leaves none, and nothing otherwise.
  pub fn status(&self) -> Option<Element> {
    status_row(
      &self.recipe,
      self.loading,
      self.shown.is_empty(),
      &self.loading_text,
      &self.empty_text,
    )
  }

  /// The engine's select as a combobox around the input, of the options what is typed leaves, in `style`, with the
  /// box's size, opacity and cursor; `trigger` lays out what is in the box. It is a combobox to what reads the tree,
  /// and the input in it is the stop of Tab.
  pub fn select(
    self,
    style: SelectStyle,
    trigger: impl Fn(SelectTriggerState) -> Element + Send + Sync + 'static,
  ) -> Element {
    let engine = Engine {
      style,
      placeholder: self.placeholder.clone(),
      input: self.input(),
      trigger: Arc::new(trigger),
      status: self.status(),
      free_text: self.allow_custom,
      read_only: self.boxed.read_only,
    };
    let node = self.choices.clone().build(&self.shown, engine).into_node();
    *self.opened.lock().unwrap() = node.clone_select_state().map(|state| state.downgrade());
    let select = Element::from_node(node);
    finish(
      select,
      &self.part(ComboboxPart::Trigger).rest(),
      self.theme(),
      Finish {
        fixed_height: true,
        disabled: self.disabled,
        label: self.label.as_ref(),
        name: self.name.as_ref(),
        id: self.id.as_ref(),
        key: self.key.as_ref(),
      },
    )
  }

  /// How the default renderer builds a combobox: the input, the clear control where it has one, and the chevron in
  /// the box of a select.
  pub fn render(self) -> Element {
    let style = self.select_style();
    let gap = self.part(ComboboxPart::Trigger).rest().gap;
    let end = Mutex::new(Some((self.clear(), self.chevron())));
    self.select(style, move |state| {
      let mut row = Row::new().width(Dimension::Pct(100.0)).align_items(Alignment::Center);
      if let Some(gap) = gap {
        row = row.spacing(gap);
      }
      if let Some(input) = state.input {
        row = row.child(input);
      }
      if let Some((clear, chevron)) = end.lock().unwrap().take() {
        if let Some(clear) = clear {
          row = row.child(clear);
        }
        row = row.child(chevron);
      }
      row.into()
    })
  }
}

/// What is done to the engine's select of a combobox or a multi select once it is built.
pub(crate) struct Finish<'a> {
  /// Whether the box is as high as its part says, or at least as high: a multi select's grows with its tags.
  pub fixed_height: bool,
  pub disabled: bool,
  pub label: Option<&'a Arc<str>>,
  pub name: Option<&'a Arc<str>>,
  pub id: Option<&'a Arc<str>>,
  pub key: Option<&'a Arc<str>>,
}

/// The engine's select with the trigger's size, opacity and cursor, disabled, named and found by its id.
pub(crate) fn finish(select: Element, trigger: &PartStyle, theme: &Theme, finish: Finish) -> Element {
  let mut node = select.into_node();
  if let Some(width) = trigger.width.and_then(|width| width.resolve(theme)) {
    NodeUpdate::width(&mut node, width);
  }
  if let Some(width) = trigger.max_width.and_then(|width| width.resolve(theme)) {
    NodeUpdate::max_width(&mut node, width);
  }
  if let Some(height) = trigger.height.and_then(|height| height.resolve(theme)) {
    if finish.fixed_height {
      NodeUpdate::height(&mut node, height);
    } else {
      NodeUpdate::min_height(&mut node, height);
    }
  }
  if let Some(opacity) = select::opacity(trigger.opacity, theme) {
    NodeUpdate::opacity(&mut node, opacity);
  }
  if let Some(cursor) = trigger.cursor {
    NodeUpdate::cursor(&mut node, cursor);
  }
  NodeUpdate::disabled(&mut node, finish.disabled);
  if let Some(label) = finish.label {
    NodeUpdate::describe(&mut node, Arc::from("a11y_name"), label.clone());
  }
  #[cfg(feature = "form")]
  if let Some(name) = finish.name {
    NodeUpdate::name(&mut node, name.clone());
  }
  #[cfg(not(feature = "form"))]
  let _ = finish.name;
  let mut element = Element::from_node(node);
  if let Some(id) = finish.id {
    element = element.id(id.clone());
  }
  if let Some(key) = finish.key {
    element = element.key(key.clone());
  }
  element
}

/// The row after the options of `recipe`'s list: the spinner and `loading_text` while more are loading, `empty`
/// while there are none, and nothing otherwise.
pub(crate) fn status_row(
  recipe: &ResolvedRecipe,
  loading: bool,
  none: bool,
  loading_text: &str,
  empty: &str,
) -> Option<Element> {
  if !loading && !none {
    return None;
  }
  let part = recipe.part(ComboboxPart::Status);
  let mut row = Row::new()
    .width(Dimension::Pct(100.0))
    .align_items(Alignment::Center)
    .part(&part);
  if loading {
    let spinner = recipe.part(ComboboxPart::Spinner);
    row = row.child(Row::new().part(&spinner).child(Spinner::new()));
  }
  let text = if loading { loading_text } else { empty };
  Some(row.child(part.text(text).nowrap()).into())
}

/// What is pressed in the box without opening its list: a press of a tag's cross takes the tag away, and one of the
/// clear control clears, and neither reaches the select, which a press opens.
pub(crate) fn apart(element: Element) -> Element {
  Row::new()
    .flex_shrink(0.0)
    .overflow_visible()
    .on_mouse_down(|event: MouseEvent| event.stop_propagation())
    .child(element)
    .into()
}

/// The cross at the end of a box that takes what it holds away: in the style of `part` and of `icon` for its cross,
/// a `button` called `label` to what reads the tree, pressed apart from the select.
pub(crate) fn clear_control(
  part: &ResolvedPart,
  icon: &ResolvedPart,
  theme: &Theme,
  disabled: bool,
  label: &Arc<str>,
  clear: impl Fn() + Send + Sync + 'static,
) -> Element {
  let side = icon.rest().width.unwrap_or(Length::Icon(IconSize::Sm));
  let mut control = pressable(part, disabled)
    .describe("a11y_role", "button")
    .describe("a11y_name", label.clone())
    .flex_shrink(0.0)
    .child(Icon::new(icons::CLOSE).size(side).build(theme));
  if !disabled {
    control = control.on_click(move |_| clear());
  }
  apart(control.into())
}

/// The rules of the clear control a combobox and a multi select have, by the names of their parts: 16 square, 3
/// round, in the muted foreground and the foreground under the pointer, its cross 12.
pub(crate) fn clear_rules(recipe: Recipe) -> Recipe {
  recipe
    .base(ComboboxPart::Clear, |s| {
      fading(
        s.square(16.0)
          .radius(3.0)
          .foreground(PaletteColor::MutedForeground)
          .cursor(CursorIcon::Pointer),
      )
    })
    .base(ComboboxPart::ClearIcon, |s| s.square(12.0))
    .hovered(ComboboxPart::Clear, |s| s.foreground(PaletteColor::Foreground))
    .focus_visible(ComboboxPart::Clear, |s| s.focus_ring())
}

/// The rules of the list a combobox and a multi select add to a select's: the status row, 32 high, 8 from the edge
/// of the list, in the muted foreground at 13, the spinner 8 before its text in the foreground.
pub(crate) fn status_rules(recipe: Recipe) -> Recipe {
  recipe
    .base(ComboboxPart::Status, |s| {
      s.height(32.0)
        .padding_x(8.0)
        .gap(8.0)
        .foreground(PaletteColor::MutedForeground)
        .font_size(13.0)
    })
    .base(ComboboxPart::Spinner, |s| s.foreground(PaletteColor::Foreground))
    // As high as the options at each density.
    .row_density(super::Selector::new(), ComboboxPart::Status, 32.0)
}

/// Every combobox has the one recipe and renderer, whatever its values are.
impl<V: 'static> Themed for Combobox<V> {
  const NAME: &'static str = "combobox";
  type Renderer = ComboboxRenderer;

  /// The design's combobox (`pencil/lurq-design-system.pen`, Combobox, and Menu Item for its list): a select's box,
  /// 260 by 36, the background's fill in an edge of 1 in the color of an input's, 6 round, what is typed 12 from its
  /// edge in the body's type and the placeholder in the muted foreground, two chevrons of 16 at its end in the muted
  /// foreground, the cursor of text. Under the pointer its edge is the ring's color; while the focus is in it and
  /// while the list is open it has a line of 2 inside the box in that color; the destructive, warning and success
  /// colors for what is chosen. A disabled one is half opaque on the muted fill. Small is 32 with text of 13, large 40
  /// with text of 15. The list is a select's: 4 under the box on the popover's fill, in an edge of the border's
  /// color, 8 round, with the middle shadow and 4 of padding; an item is 32 high, 8 in, 4 round, on the highlight
  /// under the pointer or the keys, the chosen one with a check of 16 at its end. After the options, a row of 32 in
  /// the muted foreground at 13 says there are none, or, with a spinner of 16 in the foreground 8 before it, that
  /// more are loading. The cross that clears is 16, in the muted foreground, the foreground under the pointer. In a
  /// fluid field the box is a select's fluid one: what is typed, the cross and the chevrons on one line under the
  /// field's label.
  fn recipe() -> Recipe {
    let recipe = <select::Select as Themed>::recipe().base(ComboboxPart::Trigger, |s| s.cursor(CursorIcon::Text));
    clear_rules(status_rules(recipe))
  }
}

/// A side in pixels of a part, for what the engine needs one of: the check, a box of a list of checkboxes.
pub(crate) fn side(part: &PartStyle, theme: &Theme, default: f32) -> f32 {
  px(part.width, theme).unwrap_or(default)
}
