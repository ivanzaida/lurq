//! A cascader: a value chosen from a tree whose levels open as columns, one after the other, under a trigger.

use std::sync::Arc;

use parking_lot::Mutex;

use super::{
  column_view::{ColumnView, ColumnViewPart, ColumnViewState},
  combobox, names,
  part_style::{Length, PartStyle},
  popover::{Popover, PopoverPart},
  recipe::{Interaction, Name, Recipe, ResolvedPart, ResolvedRecipe, Selector},
  select::{Select, SelectState},
  select_trigger::{OpenModel, SelectTrigger},
  shared::ChangeHandler,
  text_box::{BoxStates, Judgment, TextBoxSize},
  theme::{Themed, current_theme},
  tree_view::TreeItem,
};
use crate::{
  app::{
    component::Component,
    ctx::{CollisionStrategy, Ctx, Placement},
    theme::{IconSize, PaletteColor, RadiusSize, SpacingSize, Theme, TypographyStyle},
  },
  components::{self, Column, Row},
  core::{ElementRef, Signal, SignalValue},
  layout::text_style::FontWeight,
  node::{CursorIcon, Element, SpacingValue, border::Border, dimension::Dimension},
};

/// The parts of a cascader a recipe styles. Those it shares with a select have their names.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CascaderPart {
  /// The box that is pressed and focused, and that the columns open under: a select's.
  Trigger,
  /// What the trigger says of what is chosen: the path to it.
  Value,
  /// What the trigger says while nothing is chosen.
  Placeholder,
  /// The glyph at the end of the trigger.
  Chevron,
  /// What takes what is chosen away, before the chevron ([`Cascader::clearable`]).
  Clear,
  /// The cross in it: its side.
  ClearIcon,
  /// The panel that opens: what is laid over a popover's panel. Its fill, its edge, its corners, its shadow.
  Panel,
  /// The columns side by side in the panel: what is laid over a column view's box. Its greatest height, past which
  /// a column scrolls.
  Columns,
  /// One column: what is laid over a column view's column. Its width, its padding, the room between its options.
  Column,
  /// An option of a column: what is laid over a column view's item.
  Option,
  /// The icon before an option's label, where it has one: what is laid over a column view's icon.
  OptionIcon,
  /// What an option says: what is laid over a column view's label.
  OptionLabel,
  /// The chevron at the end of an option that holds others: what is laid over a column view's chevron.
  OptionChevron,
  /// What the panel says where there are no options.
  Empty,
}

names!(CascaderPart {
  Trigger => "trigger",
  Value => "value",
  Placeholder => "placeholder",
  Chevron => "chevron",
  Clear => "clear",
  ClearIcon => "clear_icon",
  Panel => "panel",
  Columns => "columns",
  Column => "column",
  Option => "option",
  OptionIcon => "option_icon",
  OptionLabel => "option_label",
  OptionChevron => "option_chevron",
  Empty => "empty",
});

/// The sizes of a cascader: a select's, the heights of the theme's controls.
pub type CascaderSize = TextBoxSize;

/// The states of a cascader a recipe styles: a select's, by the same names. `Selected`, resolved on the parts of an
/// option, is an option on the path open in the columns.
pub type CascaderState = SelectState;

/// What builds a cascader's nodes: a theme's in place of the default one.
pub type CascaderRenderer = Arc<dyn Fn(CascaderView) -> Element + Send + Sync>;

/// What tells the handlers the new choice, in the form the cascader holds it.
type Tell<T, V> = fn(Option<&T>, &[ChangeHandler<V>]);

fn tell_one<T>(next: Option<&T>, handlers: &[ChangeHandler<T>]) {
  if let Some(value) = next {
    for handler in handlers {
      handler(value);
    }
  }
}

fn tell_optional<T: Clone>(next: Option<&T>, handlers: &[ChangeHandler<Option<T>>]) {
  let value = next.cloned();
  for handler in handlers {
    handler(&value);
  }
}

type Write<T> = Arc<dyn Fn(Option<T>) + Send + Sync>;

/// A select whose options are a tree shown as columns that open one after the other, as a menu of submenus does:
/// a place by its continent, its country and its city, a category of a catalogue, a setting by its section.
///
/// ```
/// use lurq::{core::Signal, ui::{Cascader, TreeItem}};
///
/// # fn render(ctx: &mut lurq::app::ctx::Ctx, city: Signal<Option<&'static str>>) {
/// let places = || [
///   TreeItem::new("europe", "Europe").items([
///     TreeItem::new("ukraine", "Ukraine").items([TreeItem::new("kyiv", "Kyiv"), TreeItem::new("lviv", "Lviv")]),
///     TreeItem::new("poland", "Poland").item(TreeItem::new("warsaw", "Warsaw")),
///   ]),
///   TreeItem::new("asia", "Asia").item(TreeItem::new("tokyo", "Tokyo")),
/// ];
/// // One or none: mounted, so that it keeps whether it is open and the columns open in it.
/// let one = Cascader::optional(city, places()).placeholder("Select a city").mount(ctx);
/// // Held by the app, which is told each choice; a country may be chosen as well as a city.
/// let held = Cascader::controlled("kyiv", places(), |place| println!("{place}")).leaves_only(false);
/// # }
/// ```
///
/// Its trigger is a [`Select`]'s, the same box, sizes, ring, judgments and fluid form in a field, and says the path
/// to what is chosen ("Europe / Ukraine / Kyiv"). Its panel is a popover that holds a [`ColumnView`] of its items
/// ([`TreeItem`], a tree's): the first column shows what is at the top, and each one after it what the option chosen
/// in the one before holds. Its forms: bound to the app's signal, [`new`](Self::new) of one and
/// [`optional`](Cascader::optional) of one or none; held by the app, [`controlled`](Self::controlled) and
/// [`controlled_optional`](Cascader::controlled_optional). The values of the items are told apart: a value is one
/// item's. [`TreeSelect`](super::TreeSelect) is its sibling for a tree shown in one column, with any number chosen.
///
/// A press, Enter, Space, Down or Up on the trigger opens the panel, with the columns of the path to what is chosen
/// open and the focus on it. A press of an option that holds others opens its column, as the pointer on it does
/// where it is [`open_on_hover`](Self::open_on_hover); a press of one that holds nothing, or Enter on it, chooses it
/// and closes the panel. In the columns Up and Down go through a column, Home and End to its ends, Right into what
/// an option holds and Left back to the option the column is of, a letter to the next option that starts with it;
/// Enter on an option that holds others goes into it. With [`leaves_only(false)`](Self::leaves_only) an option that
/// holds others is chosen by a press or Enter as well, and the panel stays open. Escape and a press outside close it,
/// and the focus goes back to the trigger. An item marked [`lazy`](TreeItem::lazy) that holds nothing yet opens a
/// column with a spinner, and [`on_load`](Self::on_load) is told it: the app gives it its children.
///
/// The cascader keeps whether it is open, the trigger its panel hangs from and the columns open in it where it is
/// [mounted](Self::mount); the app holds whether it is open with [`open`](Self::open). Converted into an element on
/// its own it is the still form of itself.
pub struct Cascader<T: 'static = (), V: ?Sized + 'static = T> {
  items: Vec<TreeItem<T>>,
  chosen: Option<T>,
  write: Write<T>,
  /// Whether nothing chosen is a value it holds: it can be cleared.
  empties: bool,
  tell: Tell<T, V>,
  on_change: Vec<ChangeHandler<V>>,
  open: Option<Signal<bool>>,
  on_load: Vec<ChangeHandler<T>>,
  leaves_only: bool,
  open_on_hover: bool,
  path: bool,
  separator_text: Arc<str>,
  placeholder: Option<Arc<str>>,
  boxed: BoxStates,
  full_width: bool,
  clearable: bool,
  clear_label: Option<Arc<str>>,
  empty_text: Option<Arc<str>>,
  loading_label: Option<Arc<str>>,
  /// The words it was told to speak in, in place of the app's.
  locale: Option<crate::core::InputWords>,
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

impl<T: Clone + PartialEq + Send + Sync + 'static> Cascader<T, T> {
  /// A cascader of `items` bound to `value`: the path to its item is what the trigger says, and choosing another
  /// sets it.
  pub fn new(value: Signal<T>, items: impl IntoIterator<Item = TreeItem<T>>) -> Self
  where
    T: SignalValue,
  {
    let chosen = Some(value.get());
    let write = Arc::new(move |next: Option<T>| {
      if let Some(next) = next {
        value.set(next);
      }
    });
    Self::of(items, chosen, write, false, tell_one::<T>)
  }

  /// A cascader of one whose value the app holds: a choice of another tells `on_change` the value it is to be, and
  /// the trigger shows what the app then gives. Two built at the one place in the app's code, in a loop, are told
  /// apart by their `id` or `key`.
  pub fn controlled(
    value: T,
    items: impl IntoIterator<Item = TreeItem<T>>,
    on_change: impl Fn(&T) + Send + Sync + 'static,
  ) -> Self {
    Self::of(items, Some(value), Arc::new(|_| {}), false, tell_one::<T>).on_change(on_change)
  }
}

impl<T: Clone + PartialEq + Send + Sync + 'static> Cascader<T, Option<T>> {
  /// A cascader bound to `value`, which may hold none: the trigger shows the placeholder while it is `None`, and
  /// choosing an option sets it to that option's value.
  pub fn optional(value: Signal<Option<T>>, items: impl IntoIterator<Item = TreeItem<T>>) -> Self
  where
    Option<T>: SignalValue,
  {
    let chosen = value.get();
    let write = Arc::new(move |next: Option<T>| value.set(next));
    Self::of(items, chosen, write, true, tell_optional::<T>)
  }

  /// A cascader of one or none whose value the app holds: a choice, or a clear, is told to `on_change` as the value
  /// it is to be.
  pub fn controlled_optional(
    value: Option<T>,
    items: impl IntoIterator<Item = TreeItem<T>>,
    on_change: impl Fn(&Option<T>) + Send + Sync + 'static,
  ) -> Self {
    Self::of(items, value, Arc::new(|_| {}), true, tell_optional::<T>).on_change(on_change)
  }
}

impl<T: Clone + PartialEq + Send + Sync + 'static, V: ?Sized + 'static> Cascader<T, V> {
  fn of(
    items: impl IntoIterator<Item = TreeItem<T>>,
    chosen: Option<T>,
    write: Write<T>,
    empties: bool,
    tell: Tell<T, V>,
  ) -> Self {
    Self {
      items: items.into_iter().collect(),
      chosen,
      write,
      empties,
      tell,
      on_change: Vec::new(),
      open: None,
      on_load: Vec::new(),
      leaves_only: true,
      open_on_hover: false,
      path: true,
      // A mark between the steps, not a word: the same in every language.
      separator_text: Arc::from(" / "),
      placeholder: None,
      boxed: BoxStates::default(),
      full_width: false,
      clearable: false,
      clear_label: None,
      empty_text: None,
      loading_label: None,
      locale: None,
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

  /// Called with what is chosen after each choice a person makes, after the signal is set: the value of a cascader
  /// of one, `&Option<T>` of an [`optional`](Cascader::optional) one. Not called for the option chosen already, nor
  /// when the app sets the signal. A second handler is called after the first. A handler of another type does not
  /// compile:
  ///
  /// ```compile_fail
  /// use lurq::{core::Signal, ui::{Cascader, TreeItem}};
  ///
  /// let city = Signal::new("kyiv");
  /// let cascader = Cascader::new(city, [TreeItem::new("kyiv", "Kyiv")]).on_change(|city: &Option<&str>| {});
  /// ```
  pub fn on_change(mut self, handler: impl Fn(&V) + Send + Sync + 'static) -> Self {
    self.on_change.push(Arc::new(handler));
    self
  }

  /// Whether the panel is open, where the app keeps it. Without one a mounted cascader keeps it itself.
  pub fn open(mut self, open: Signal<bool>) -> Self {
    self.open = Some(open);
    self
  }

  /// Called with an item marked [`lazy`](TreeItem::lazy) that holds nothing yet, when its column opens: where the
  /// app brings its children and gives them to it. A second handler is called after the first.
  pub fn on_load(mut self, handler: impl Fn(&T) + Send + Sync + 'static) -> Self {
    self.on_load.push(Arc::new(handler));
    self
  }

  /// Whether only an option that holds nothing can be chosen, as the board's cascader has it: a press of one that
  /// holds others opens its column. `true` unless told otherwise; without it a press or Enter chooses any option,
  /// and a choice of one that holds others leaves the panel open on its column.
  pub fn leaves_only(mut self, leaves_only: bool) -> Self {
    self.leaves_only = leaves_only;
    self
  }

  /// Whether the pointer on an option that holds others opens its column, as a menu's submenu opens, in place of a
  /// press. A press opens it either way.
  pub fn open_on_hover(mut self, on_hover: bool) -> Self {
    self.open_on_hover = on_hover;
    self
  }

  /// Whether the trigger says the whole path to what is chosen, as the board's does ("Europe / Ukraine / Kyiv"), or
  /// its label alone. It says the path unless told otherwise.
  pub fn path(mut self, path: bool) -> Self {
    self.path = path;
    self
  }

  /// What stands between the steps of the path the trigger says: " / " unless it is told otherwise.
  pub fn separator_text(mut self, text: &str) -> Self {
    self.separator_text = Arc::from(text);
    self
  }

  /// What the trigger says while nothing is chosen.
  pub fn placeholder(mut self, placeholder: &str) -> Self {
    self.placeholder = Some(Arc::from(placeholder));
    self
  }

  /// What the cascader is called, to what reads the tree. The field it is in shows its label.
  pub fn label(mut self, label: &str) -> Self {
    self.boxed.label = Some(Arc::from(label));
    self
  }

  /// A [`CascaderSize`], or a size the theme's recipe names.
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

  /// Whether what is chosen can be read and not changed: the panel does not open.
  pub fn read_only(mut self, read_only: bool) -> Self {
    self.boxed.read_only = read_only;
    self
  }

  /// Whether the cascader takes no input.
  pub fn disabled(mut self, disabled: bool) -> Self {
    self.boxed.disabled = disabled;
    self
  }

  /// Whether the trigger is as wide as what holds it, in place of the design's 260.
  pub fn full_width(mut self, full_width: bool) -> Self {
    self.full_width = full_width;
    self
  }

  /// Whether the trigger has a cross before its chevron, while something is chosen, that takes it away: of a
  /// cascader that may hold none ([`optional`](Cascader::optional)).
  pub fn clearable(mut self, clearable: bool) -> Self {
    self.clearable = clearable;
    self
  }

  /// What the clear control is called to what reads the tree: the locale's "Clear" unless it is told otherwise.
  pub fn clear_label(mut self, label: &str) -> Self {
    self.clear_label = Some(Arc::from(label));
    self
  }

  /// What the panel says where there are no items: the locale's "No options" unless it is told otherwise.
  pub fn empty_text(mut self, text: &str) -> Self {
    self.empty_text = Some(Arc::from(text));
    self
  }

  /// What the spinner of a column whose options are on their way is called to what reads the tree: the locale's
  /// "Loading" unless it is told otherwise.
  pub fn loading_label(mut self, label: &str) -> Self {
    self.loading_label = Some(Arc::from(label));
    self
  }

  /// The words this cascader speaks in, in place of the app's ([`App::set_locale`](crate::app::App::set_locale)):
  /// the name of its cross, what an empty panel says and what the spinner of a column on its way is called. A word
  /// told by itself (`clear_label`, `empty_text`, `loading_label`) stays what it was told.
  pub fn locale(mut self, locale: &crate::core::Locale) -> Self {
    self.locale = Some(locale.input.clone());
    self
  }

  /// What the panel and its columns are called to what reads the tree, in place of the cascader's label.
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

  /// The name the cascader's value has in the form it is in: what the trigger says.
  #[cfg(feature = "form")]
  pub fn name(mut self, name: impl Into<Arc<str>>) -> Self {
    self.name = Some(name.into());
    self
  }

  /// Puts the cascader in a state the theme's recipe names ([`CascaderState`]), or takes it out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// Styles one part of this cascader over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(PartStyle) -> PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for this cascader.
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

  /// The id of the trigger: what is pressed and focused. Its panel has this id with `-panel` after it, its columns
  /// with `-columns` (each column that and `-column-` and its place), and its clear control with `-clear`.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// Builds the cascader in `theme`, with nothing kept but what the app gives it. Converting a cascader into an
  /// element does this in the theme of what is being rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let kept = Kept {
      open: self.open.clone().unwrap_or_else(|| Signal::new(false)),
      anchor: ElementRef::new(),
      path: Arc::default(),
      shown: Arc::default(),
      revision: None,
    };
    self.build_with(theme, kept)
  }

  /// The cascader mounted where it is declared: it keeps whether its panel is open, the trigger the panel hangs
  /// from, which the focus goes back to, and the columns open in it.
  pub fn mount(self, ctx: &mut Ctx) -> Element {
    let key = self.key.clone();
    let props = CascaderProps { cascader: self };
    match key {
      Some(key) => ctx.mount_keyed::<CascaderHost<T, V>>(&key, props).key(key),
      None => ctx.mount::<CascaderHost<T, V>>(props),
    }
  }

  fn build_with(&self, theme: &Theme, kept: Kept<T>) -> Element {
    let view = self.view(theme, kept);
    match theme.renderer::<Cascader<T, V>>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }

  fn view(&self, theme: &Theme, kept: Kept<T>) -> CascaderView {
    let open = self.open.clone().unwrap_or_else(|| kept.open.clone());
    let is_open = open.get();
    let disabled = self.boxed.disabled;
    let case = self
      .boxed
      .case(self.states.clone())
      .state(SelectState::Open, is_open && !self.inline)
      .state(SelectState::FullWidth, self.full_width);
    let recipe = theme
      .resolve::<Cascader<T, V>>(&case, Some(&self.rules))
      .owned_by(self.id.as_ref());
    // The words it was not told: its own locale's, else the app's, read where it is built.
    let words = match &self.locale {
      Some(words) => words.clone(),
      None => super::current_locale().input.clone(),
    };
    let told = |word: &Option<Arc<str>>, default: &Arc<str>| word.clone().unwrap_or_else(|| default.clone());
    let on_path = theme
      .resolve::<Cascader<T, V>>(&case.clone().state(SelectState::Selected, true), Some(&self.rules))
      .owned_like(&recipe);

    // The path to what is chosen: its values and its labels, from the top.
    let chosen_path = self
      .chosen
      .as_ref()
      .and_then(|chosen| path_to(&self.items, chosen))
      .unwrap_or_default();
    let text: Option<Arc<str>> = chosen_path.last().map(|_| {
      let labels: Vec<&str> = chosen_path.iter().map(|(_, label)| &**label).collect();
      if self.path {
        Arc::from(labels.join(&self.separator_text))
      } else {
        Arc::from(*labels.last().unwrap_or(&""))
      }
    });
    let values: Vec<T> = chosen_path.iter().map(|(value, _)| value.clone()).collect();

    // The path open in the columns: the chosen one as the panel opens, then where a person goes.
    let browsed: Vec<T> = {
      let (mut held, mut shown) = (kept.path.lock(), kept.shown.lock());
      if is_open && !*shown || held.is_none() {
        *held = Some(values.clone());
      }
      *shown = is_open;
      held.clone().unwrap_or_default()
    };

    let model = Arc::new(OpenModel {
      open: open.clone(),
      on_open_change: self.on_open_change.clone(),
      on_dismiss: self.on_dismiss.clone(),
      inert: disabled || self.boxed.read_only,
    });
    let choose = {
      let (write, tell, on_change) = (self.write.clone(), self.tell, self.on_change.clone());
      let held = self.chosen.clone().filter(|_| !values.is_empty());
      Arc::new(move |next: Option<T>| {
        if next == held {
          return;
        }
        write(next.clone());
        tell(next.as_ref(), &on_change);
      }) as Arc<dyn Fn(Option<T>) + Send + Sync>
    };

    let name = self.panel_label.clone().or_else(|| self.boxed.label.clone());
    // The values of the items that hold others, and of those that are lazy and hold nothing yet.
    let (mut branches, mut waiting) = (Vec::new(), Vec::new());
    branches_of(&self.items, &mut branches, &mut waiting);
    let columns = (!self.items.is_empty()).then(|| {
      let browse = {
        let (held, revision, waiting, on_load) = (
          kept.path.clone(),
          kept.revision.clone(),
          waiting.clone(),
          self.on_load.clone(),
        );
        move |next: &[T]| {
          *held.lock() = Some(next.to_vec());
          if let Some(revision) = &revision {
            revision.update(|revision| *revision += 1);
          }
          // A lazy item that holds nothing yet: the app is asked for its children.
          if let Some(last) = next.last()
            && waiting.contains(last)
          {
            for handler in &on_load {
              handler(last);
            }
          }
        }
      };
      let activate = {
        let (choose, close, leaves_only) = (choose.clone(), model.clone(), self.leaves_only);
        move |value: &T| {
          if branches.contains(value) {
            // An option that holds others, where it may be chosen: chosen, and the panel stays open on its column.
            if !leaves_only {
              choose(Some(value.clone()));
            }
            return;
          }
          // The panel closes first, so that a handler that opens a dialog keeps its focus.
          close.set_open(false);
          choose(Some(value.clone()));
        }
      };
      let rules = self.column_rules(&recipe, &on_path, &browsed);
      let mut view = ColumnView::controlled(browsed.clone(), self.items.clone(), browse)
        .on_activate(activate)
        .press_activates(true)
        .activate_branches(!self.leaves_only)
        .open_on_hover(self.open_on_hover)
        .loading_label(&told(&self.loading_label, &words.loading))
        .disabled(disabled)
        .rules(&rules);
      view.autofocus = !self.inline;
      if let Some(name) = &name {
        view = view.label(name);
      }
      if let Some(id) = &self.id {
        view = view.id(format!("{id}-columns"));
      }
      view.build(theme)
    });

    let clear = (self.clearable && self.empties && text.is_some() && !disabled && !self.boxed.read_only).then(|| {
      let choose = choose.clone();
      Arc::new(move || choose(None)) as Arc<dyn Fn() + Send + Sync>
    });

    CascaderView {
      text,
      placeholder: self.placeholder.clone(),
      label: self.boxed.label.clone(),
      panel_label: name,
      invalid: self.boxed.judgment() == Some(Judgment::Invalid),
      read_only: self.boxed.read_only,
      disabled,
      open: is_open,
      full_width: self.full_width,
      inline: self.inline,
      columns_width: self.columns_width(&recipe, &browsed),
      recipe,
      forced: self.forced,
      columns,
      empty_text: told(&self.empty_text, &words.no_options),
      clear,
      clear_label: told(&self.clear_label, &words.clear),
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

  /// The lists of options open along `path`: the top's, then what each option of the path that holds others holds.
  fn open_columns<'a>(&'a self, path: &[T]) -> Vec<&'a [TreeItem<T>]> {
    let mut columns: Vec<&[TreeItem<T>]> = vec![&self.items];
    let mut items: &[TreeItem<T>] = &self.items;
    for value in path {
      let Some(item) = items.iter().find(|item| item.value == *value) else {
        break;
      };
      if !item.lazy && item.items.is_empty() {
        break;
      }
      columns.push(&item.items);
      items = &item.items;
    }
    columns
  }

  /// How wide the columns open along `path` are side by side: each the least width of a column.
  fn columns_width(&self, recipe: &ResolvedRecipe, path: &[T]) -> f32 {
    let theme = recipe.theme();
    let column = super::select::px(recipe.part(CascaderPart::Column).rest().min_width, theme).unwrap_or(124.0);
    column * self.open_columns(path).len() as f32
  }

  /// What the cascader lays over the column view's recipe: its parts on the view's, an option on the path as the
  /// recipe's `Selected` case says (the view's last one too, which the view draws apart), and the box as wide as the
  /// columns open along `path` and as high as the longest of them, up to its greatest height.
  fn column_rules(&self, recipe: &ResolvedRecipe, on_path: &ResolvedRecipe, path: &[T]) -> Recipe {
    use CascaderPart::{Column, Columns, Option, OptionChevron, OptionIcon, OptionLabel};
    let theme = recipe.theme();
    let px = |length: core::option::Option<Length>| super::select::px(length, theme);
    let column = recipe.part(Column).rest();
    let row = px(recipe.part(Option).rest().height).unwrap_or(30.0);
    let gap = super::select::spacing(column.gap, theme).unwrap_or(0.0);
    let [top, _, bottom, _] = column.paddings();
    let pad =
      |value: core::option::Option<SpacingValue>| value.map_or(0.0, |value| value.resolve(&theme.spacing(), 0.0));
    let longest = self
      .open_columns(path)
      .iter()
      .map(|items| items.len().max(1))
      .max()
      .unwrap_or(1) as f32;
    let mut height = pad(top) + pad(bottom) + longest * row + (longest - 1.0).max(0.0) * gap;
    if let Some(most) = px(recipe.part(Columns).rest().max_height) {
      height = height.min(most);
    }
    let selected = |part: CascaderPart| on_path.part(part).rest();
    let mut rules = recipe
      .rules_for(Columns, ColumnViewPart::Root)
      .extend(&recipe.rules_for(Column, ColumnViewPart::Column))
      .extend(&recipe.rules_for(Option, ColumnViewPart::Item))
      .extend(&recipe.rules_for(OptionIcon, ColumnViewPart::Icon))
      .extend(&recipe.rules_for(OptionLabel, ColumnViewPart::Label))
      .extend(&recipe.rules_for(OptionChevron, ColumnViewPart::Chevron))
      .base(ColumnViewPart::Root, |s| s.width(Length::Fill).height(height));
    for state in [ColumnViewState::Selected, ColumnViewState::Current] {
      for (part, inner) in [
        (Option, ColumnViewPart::Item),
        (OptionIcon, ColumnViewPart::Icon),
        (OptionLabel, ColumnViewPart::Label),
        (OptionChevron, ColumnViewPart::Chevron),
      ] {
        let style = selected(part);
        rules = rules.state(state, inner, move |_| style);
      }
    }
    rules
  }
}

/// The values and labels from the top down to the item of `value`: `None` where no item has it.
fn path_to<T: Clone + PartialEq>(items: &[TreeItem<T>], value: &T) -> Option<Vec<(T, Arc<str>)>> {
  for item in items {
    if item.value == *value {
      return Some(vec![(item.value.clone(), item.label.clone())]);
    }
    if let Some(mut below) = path_to(&item.items, value) {
      below.insert(0, (item.value.clone(), item.label.clone()));
      return Some(below);
    }
  }
  None
}

/// Adds to `branches` the values of the items that hold others or are lazy, and to `waiting` those of the lazy items
/// that hold nothing yet.
fn branches_of<T: Clone>(items: &[TreeItem<T>], branches: &mut Vec<T>, waiting: &mut Vec<T>) {
  for item in items {
    if item.lazy || !item.items.is_empty() {
      branches.push(item.value.clone());
    }
    if item.lazy && item.items.is_empty() {
      waiting.push(item.value.clone());
    }
    branches_of(&item.items, branches, waiting);
  }
}

impl<T: Clone + PartialEq + Send + Sync + 'static, V: ?Sized + 'static> From<Cascader<T, V>> for Element {
  fn from(cascader: Cascader<T, V>) -> Self {
    cascader.build(&current_theme())
  }
}

impl<T: Clone + PartialEq + Send + Sync + 'static, V: ?Sized + 'static> super::FieldControl for Cascader<T, V> {
  /// Called by the field's label unless it has a name of its own, judged as the field is unless the app judged the
  /// cascader itself, disabled where the field is, and in a fluid field the fluid box.
  fn in_field(mut self, field: &super::InField) -> Element {
    self.boxed.in_field(field);
    if field.fluid {
      self.full_width = true;
    }
    self.into()
  }

  /// Told what the field says, as in `in_field`, and mounted: the same control as outside a field.
  fn mount_in_field(mut self, field: &super::InField, ctx: &mut Ctx) -> Element {
    self.boxed.in_field(field);
    if field.fluid {
      self.full_width = true;
    }
    self.mount(ctx)
  }

  fn has_fluid_form(&self) -> bool {
    true
  }
}

/// What a cascader keeps from one render to the next.
struct Kept<T> {
  open: Signal<bool>,
  anchor: ElementRef,
  /// The path open in the columns, once the panel was opened.
  path: Arc<Mutex<Option<Vec<T>>>>,
  /// Whether the panel was open at the last build: as it opens, the columns of the path to what is chosen open.
  shown: Arc<Mutex<bool>>,
  /// Renders a mounted cascader again: a column opened.
  revision: Option<Signal<u64>>,
}

impl<T> Clone for Kept<T> {
  fn clone(&self) -> Self {
    Self {
      open: self.open.clone(),
      anchor: self.anchor.clone(),
      path: self.path.clone(),
      shown: self.shown.clone(),
      revision: self.revision.clone(),
    }
  }
}

/// A cascader as a renderer sees it: what its trigger says, the style of its parts, and its behavior ready to be
/// dressed.
pub struct CascaderView {
  /// What the trigger says of what is chosen: `None` while nothing is.
  pub text: Option<Arc<str>>,
  pub placeholder: Option<Arc<str>>,
  /// What it is called, to what reads the tree.
  pub label: Option<Arc<str>>,
  panel_label: Option<Arc<str>>,
  /// Whether what is chosen is wrong, as the cascader shows it.
  pub invalid: bool,
  pub read_only: bool,
  pub disabled: bool,
  /// Whether its panel is open.
  pub open: bool,
  pub full_width: bool,
  /// Whether the panel is drawn in place.
  pub inline: bool,
  /// The style of each part of this cascader.
  pub recipe: ResolvedRecipe,
  /// How wide the columns open in the panel are side by side.
  columns_width: f32,
  forced: Option<Interaction>,
  columns: Option<Element>,
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

impl CascaderView {
  /// The style of a part, as this cascader is to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part).forced(self.forced)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// The trigger as a select's is drawn, the one a tree select has.
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
      multiple: false,
      popup: "listbox",
      model: &self.model,
      anchor: &self.anchor,
      name: self.name.as_ref(),
      id: self.id.as_ref(),
    }
  }

  /// What the trigger says, in the room the chevron leaves it: the path to what is chosen, else the placeholder. A
  /// longer text ends in an ellipsis.
  pub fn value(&self) -> Row {
    self.shaped().value()
  }

  /// The cross that takes what is chosen away, where the cascader shows one.
  pub fn clear(&self) -> Option<Element> {
    self.shaped().clear(self.clear.clone(), &self.clear_label)
  }

  /// The glyph at the end of the trigger: a chevron down.
  pub fn chevron(&self) -> Element {
    self.shaped().chevron()
  }

  /// The trigger in the style of its part, with nothing in it yet: a select's. A press, Enter and Space open and
  /// close the panel, and Down and Up open it. It is a combobox that opens a listbox to what reads the tree.
  pub fn trigger(&self) -> components::Button {
    self.shaped().trigger()
  }

  /// What the panel holds: the columns, or the words of a cascader with no options.
  pub fn content(&mut self) -> Element {
    match self.columns.take() {
      Some(columns) => columns,
      None => {
        let empty = self.part(CascaderPart::Empty);
        Row::new().part(&empty).child(empty.text(&self.empty_text)).into()
      }
    }
  }

  /// The panel: a popover under the trigger as wide as the columns open in it, holding them. Escape and a press
  /// outside close it, and with the focus in it give the focus back to the trigger. Inline, it is the popover's panel
  /// drawn in place.
  pub fn panel(&mut self) -> Element {
    let width = self.columns_width;
    let mut rules = self.recipe.rules_for(CascaderPart::Panel, PopoverPart::Panel);
    if self.columns.is_some() {
      rules = rules.base(PopoverPart::Panel, |s| s.width(width).max_width(Length::Fill));
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

  /// How the default renderer builds a cascader: the trigger with what it says, its cross and its chevron, and the
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
      let mut root = Column::new()
        .max_width(fill)
        .spacing(gap)
        .align_items(crate::layout::Alignment::Start)
        .child(trigger)
        .child(panel);
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

impl<T: Clone + PartialEq + Send + Sync + 'static, V: ?Sized + 'static> crate::ui::Styled for Cascader<T, V> {
  type Component = Self;

  fn rules(self, rules: &crate::ui::Recipe) -> Self {
    Self::rules(self, rules)
  }

  fn state(self, state: impl Into<crate::ui::Name>, on: bool) -> Self {
    Self::state(self, state, on)
  }
}

/// Every cascader has the one recipe and renderer, whatever its values are and whatever form it is of.
impl<T: Send + Sync + 'static, V: ?Sized + 'static> Themed for Cascader<T, V> {
  const NAME: &'static str = "cascader";
  /// Its id is on the trigger: the box that is pressed.
  const ROOT: &'static str = "trigger";
  type Renderer = CascaderRenderer;

  fn parts() -> crate::ui::Declared {
    CascaderPart::declared()
  }

  /// The design's cascader (`pencil/lurq-design-system.pen`, Cascader `ACuvJ`): a select's trigger, 260 by 36 on the
  /// background in an edge of 1 in the color of an input's, 6 round, its path 12 from its edge in the body's type and
  /// a chevron of 16 in the muted foreground 8 after it; 4 under it a panel on the popover's fill in an edge of 1 in
  /// the border's color, 8 round, with the popover's shadow, that cuts what is in it and holds the columns side by
  /// side, each 124 wide and padded 4, with a line of 1 in the border's color at its end but the last's. An option is
  /// 30 high, 8 in from its sides, 4 round, its label at 13 in the foreground and, where it holds others, a chevron of
  /// 14 in the muted foreground 4 after it. The options on the path are in the highlight (the board's `$accent`),
  /// their labels at 500, the last as the others. The board draws no option under the pointer, focused or disabled,
  /// no column longer than its panel and no cascader with nothing to choose: under the pointer and held down an
  /// option has the veils of those states, the focus from the keyboard is the ring inside it, a disabled one is
  /// faded, the columns scroll past 280, and the words of an empty panel are 13 in the muted foreground. At each
  /// density an option is as high as a row of the density is.
  fn recipe() -> Recipe {
    use CascaderPart::{Column, Columns, Empty, Option, OptionChevron, OptionIcon, OptionLabel, Panel};
    use PaletteColor::{Foreground, Highlight, MutedForeground, Popover};
    let recipe = <Select as Themed>::recipe()
      .base(CascaderPart::Trigger, super::shared::fading)
      .base(CascaderPart::Chevron, |s| s.square(IconSize::Md))
      .state(SelectState::ReadOnly, CascaderPart::Trigger, |s| {
        s.cursor(CursorIcon::Default)
      })
      // The panel holds the columns with no room around them.
      .base(Panel, |s| s.padding_x(0.0).padding_y(0.0).gap(0.0))
      .base(Columns, |s| {
        s.max_height(280.0)
          .radius(RadiusSize::Lg)
          .background(Popover)
          .border(Border::inside(0.0, PaletteColor::Border))
      })
      .base(Column, |s| s.min_width(124.0).padding_x(4.0).padding_y(4.0).gap(0.0))
      .base(Option, |s| s.height(30.0).padding_x(8.0).gap(4.0).radius(RadiusSize::Sm))
      .row_density(Selector::new(), Option, 30.0)
      .base(OptionIcon, |s| s.square(IconSize::Sm).foreground(MutedForeground))
      .base(OptionLabel, |s| {
        s.typography(TypographyStyle::Body)
          .font_size(13.0)
          .foreground(Foreground)
      })
      .base(OptionChevron, |s| s.square(IconSize::Sm).foreground(MutedForeground))
      .state(SelectState::Selected, Option, |s| s.background(Highlight))
      .state(SelectState::Selected, OptionLabel, |s| s.weight(FontWeight::Medium))
      .base(Empty, |s| {
        s.padding_x(12.0)
          .padding_y(8.0)
          .font_size(13.0)
          .foreground(MutedForeground)
      });
    combobox::clear_rules(recipe)
  }
}

struct CascaderProps<T: 'static, V: ?Sized + 'static> {
  cascader: Cascader<T, V>,
}

/// A cascader built again is drawn again: what is chosen is read from the app's signal as it is built.
impl<T, V: ?Sized> PartialEq for CascaderProps<T, V> {
  fn eq(&self, _: &Self) -> bool {
    false
  }
}

impl<T, V: ?Sized> crate::app::component::DevtoolsInspectable for CascaderProps<T, V> {
  fn inspect(&self, formatter: &mut crate::app::component::DevtoolsFormatter<'_>) {
    formatter.value("Cascader", format!("{} items at the top", self.cascader.items.len()));
  }
}

/// What a mounted cascader keeps between renders.
struct CascaderHost<T, V: ?Sized> {
  kept: Kept<T>,
  _held: std::marker::PhantomData<fn() -> Box<V>>,
}

impl<T: Clone + PartialEq + Send + Sync + 'static, V: ?Sized + 'static> Component for CascaderHost<T, V> {
  type Props = CascaderProps<T, V>;

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      kept: Kept {
        open: ctx.signal(false),
        anchor: ElementRef::new(),
        path: Arc::default(),
        shown: Arc::default(),
        revision: Some(ctx.signal(0_u64)),
      },
      _held: std::marker::PhantomData,
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    if let Some(revision) = &self.kept.revision {
      // Read so that a column that opens renders the cascader again.
      revision.get();
    }
    ctx
      .props::<CascaderProps<T, V>>()
      .cascader
      .build_with(&current_theme(), self.kept.clone())
  }
}
