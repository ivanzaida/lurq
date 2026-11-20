//! A select: one value chosen from a list that opens under a trigger, bound to a signal.

use std::{panic::Location, sync::Arc};

use super::{
  combobox::{Choices, One, OneOf},
  icon::Icon,
  icons, names,
  part_style::{FontFamily, Length, Opacity, PartStyle},
  recipe::{Case, Interaction, Name, Recipe, ResolvedPart, ResolvedRecipe, Selector},
  text_box::{self, TextBoxSize},
  theme::{Themed, current_theme},
};
use crate::{
  app::theme::{
    BorderSize, ControlSize, IconSize, OpacityLevel, PaletteColor, RadiusSize, ShadowStyle, Theme, TypographyStyle,
  },
  components::SelectOption,
  core::{Signal, SignalValue},
  layout::text_style::{FontWeight, TextStyle},
  node::{
    CursorIcon, Element, NodeUpdate, Outline, SelectCheckmarkPosition, SelectIcon, SelectPartStyle, SelectStyle,
    SpacingValue, TextColor,
    border::{Border, Borders},
    dimension::Dimension,
    padding::Padding,
  },
};

/// The parts of a select a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SelectPart {
  /// The box that is pressed and focused, and that the list opens under: its size, its edge, its fill, its ring, and
  /// the type of what it says. Its gap is the room between what it says and the chevron.
  Trigger,
  /// The label of the chosen option in the trigger: its type and its color, over the trigger's.
  Value,
  /// What the trigger says while nothing is chosen, over the value's type and color.
  Placeholder,
  /// The glyph at the end of the trigger: its side and its color.
  Chevron,
  /// The panel of the open list: its fill, edge, corners, shadow and padding. Its gap is the room between the trigger
  /// and it; its height is the most it is, past which it scrolls.
  Panel,
  /// An option of the open list: its height, padding, corners, type and color. Under the pointer or the keys it is
  /// `hovered`. Its gap is the room between its label and the check.
  Item,
  /// The mark at the end of the chosen item: its side and its color.
  Check,
  /// The second, smaller line of an option that has one ([`SelectOption::detail`]).
  Detail,
  /// The short text at the end of an option that has one ([`SelectOption::hint`]): "beta", "soon".
  Hint,
  /// The label over a group of options ([`SelectOption::group`]): its padding, type and color.
  Group,
  /// The line between two groups: its fill is the line's color, its height the line's, and its padding the room
  /// around it.
  Separator,
}

names!(SelectPart {
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
});

/// The sizes of a select: the heights of the theme's controls, as an input has them: `Sm` 32 with text of 13, `Md`
/// 36 with text of 14, `Lg` 40 with text of 15.
pub type SelectSize = TextBoxSize;

/// The states of a select a recipe styles, besides what the pointer and the
/// keyboard do to it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SelectState {
  /// Its list is open. The engine says when; the recipe's rules for it are what the trigger looks like then.
  Open,
  /// What is chosen is wrong, or nothing is and something has to be: its edge is in the destructive color.
  Invalid,
  /// What is chosen is allowed, with a caveat: its edge is in the warning color.
  Warning,
  /// What is chosen was checked and is right: its edge is in the success color.
  Valid,
  /// What is chosen can be read and not changed: its list does not open. The muted fill in the border's color.
  ReadOnly,
  /// It takes no input. Of an item, resolved on its parts: it cannot be chosen ([`SelectOption::disabled`]).
  Disabled,
  /// It is as wide as what holds it.
  FullWidth,
  /// Of an item, resolved on its parts: it is the one chosen. Its rules are for `Item` and `Check`.
  Selected,
  /// It is in a fluid field ([`Field::fluid`](super::Field::fluid)): the field's label is inside its trigger, over
  /// what is chosen.
  Fluid,
}

names!(SelectState {
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

/// What builds a select's nodes: a theme's in place of the default one.
pub type SelectRenderer = Arc<dyn Fn(SelectView) -> Element + Send + Sync>;

/// A select: one value chosen from a list that opens under a trigger. A time
/// zone, a theme, a page size.
///
/// ```
/// use lurq::{components::SelectOption, core::Signal, ui::Select};
///
/// let theme = Signal::new("system");
/// let select = Select::new(theme, [("system", "System"), ("light", "Light"), ("dark", "Dark")]);
/// // Nothing chosen yet: a value that is no option's shows the placeholder.
/// let zone = Signal::new(String::new());
/// let zones = Select::new(zone, [
///   SelectOption::new("Europe/Kyiv".to_owned(), "Kyiv"),
///   SelectOption::new("Europe/Lisbon".to_owned(), "Lisbon").disabled(true),
/// ])
/// .placeholder("Select a time zone")
/// .invalid(true);
/// // Held by the app: it is told of a choice, and the trigger shows what it answers with.
/// let size = Select::controlled(20, [(10, "10"), (20, "20"), (50, "50")], |rows| println!("{rows} a page"));
/// ```
///
/// It is the engine's [`Select`](crate::components::Select) in the design's
/// dress: a press or Enter, Space and the arrow keys open its list, the arrows
/// and the first letters move through it, and a press or Enter sets the
/// signal to the option's value and closes it. What the app sets on the signal
/// is what it shows; [`Select::on_change`] hears a choice. It stands beside an [`Input`](super::Input) in a form: the
/// same box, the same sizes, the same ring, and the same judgments of what it
/// holds (`invalid`, `warning`, `valid`). An option can start a group, which
/// the list draws with its label over it and a line above it.
pub struct Select<V = ()> {
  choices: Box<dyn OneOf<V>>,
  placeholder: Option<Arc<str>>,
  label: Option<Arc<str>>,
  size: Name,
  invalid: bool,
  warning: bool,
  valid: bool,
  read_only: bool,
  disabled: bool,
  full_width: bool,
  fit_options: bool,
  states: Vec<Name>,
  rules: Recipe,
  forced: Option<Interaction>,
  name: Option<Arc<str>>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl<T> Select<T>
where
  T: Clone + PartialEq + Send + Sync + SignalValue + 'static,
  Vec<T>: SignalValue,
{
  /// A select of `options` bound to `value`: the option whose value the
  /// signal has is chosen, and choosing another sets it. An option is a
  /// `(value, label)` or a [`SelectOption`], which can have a detail, an
  /// element before or after its label, or be disabled.
  pub fn new(value: Signal<T>, options: impl IntoIterator<Item = impl Into<SelectOption<T>>>) -> Self {
    Self::of(Box::new(One::bound(
      value,
      options.into_iter().map(Into::into).collect(),
    )))
  }

  /// A select of the app's `value`, which the app holds: a choice is told to `on_change`, and the trigger shows what
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

impl<T> Select<Option<T>>
where
  T: Clone + PartialEq + Send + Sync + SignalValue + 'static,
  Option<T>: SignalValue,
  Vec<Option<T>>: SignalValue,
{
  /// A select bound to `value`, which may hold no value: `None` is no option's, and the trigger shows the
  /// placeholder.
  pub fn optional(value: Signal<Option<T>>, options: impl IntoIterator<Item = impl Into<SelectOption<T>>>) -> Self {
    let options = options.into_iter().map(|option| option.into().map(Some)).collect();
    Self::of(Box::new(One::bound(value, options).with_empty(None)))
  }
}

impl<V: 'static> Select<V> {
  fn of(choices: Box<dyn OneOf<V>>) -> Self {
    Self {
      choices,
      placeholder: None,
      label: None,
      size: SelectSize::default().into(),
      invalid: false,
      warning: false,
      valid: false,
      read_only: false,
      disabled: false,
      full_width: false,
      fit_options: false,
      states: Vec::new(),
      rules: Recipe::new(),
      forced: None,
      name: None,
      id: None,
      key: None,
    }
  }

  /// Called with the new value when a person chooses an option: not when the app sets the signal, and not for the
  /// option chosen already. The handler takes the type of the value, `&Option<T>` of an
  /// [`optional`](Select::optional) one; a handler of another type does not compile:
  ///
  /// ```compile_fail
  /// use lurq::{core::Signal, ui::Select};
  ///
  /// let theme = Signal::new("light".to_owned());
  /// let select = Select::new(theme, [("light".to_owned(), "Light")]).on_change(|rows: &u32| {});
  /// ```
  pub fn on_change(mut self, handler: impl Fn(&V) + Send + Sync + 'static) -> Self {
    self.choices.on_change(Arc::new(handler));
    self
  }

  /// What the trigger says while no option is chosen.
  pub fn placeholder(mut self, placeholder: &str) -> Self {
    self.placeholder = Some(Arc::from(placeholder));
    self
  }

  /// What the select is called, to what reads the tree. The field it is in shows its label.
  pub fn label(mut self, label: &str) -> Self {
    self.label = Some(Arc::from(label));
    self
  }

  /// A [`SelectSize`], or a size the theme's recipe names.
  pub fn size(mut self, size: impl Into<Name>) -> Self {
    self.size = size.into();
    self
  }

  /// Whether what is chosen is wrong, or nothing is and something has to be. Of invalid, warning and valid, the
  /// first that is set is the one shown, as an input shows them.
  pub fn invalid(mut self, invalid: bool) -> Self {
    self.invalid = invalid;
    self
  }

  /// Whether what is chosen is allowed with a caveat.
  pub fn warning(mut self, warning: bool) -> Self {
    self.warning = warning;
    self
  }

  /// Whether what is chosen was checked and is right.
  pub fn valid(mut self, valid: bool) -> Self {
    self.valid = valid;
    self
  }

  /// Whether what is chosen can be read and not changed: the list does not open. It keeps the focus and is read by
  /// what reads the tree; the trigger is on the muted fill in the border's color.
  pub fn read_only(mut self, read_only: bool) -> Self {
    self.read_only = read_only;
    self
  }

  /// Whether the select takes no input.
  pub fn disabled(mut self, disabled: bool) -> Self {
    self.disabled = disabled;
    self
  }

  /// Whether the select is as wide as what holds it, in place of the design's 260.
  pub fn full_width(mut self, full_width: bool) -> Self {
    self.full_width = full_width;
    self
  }

  /// Whether the list is as wide as its longest option, in place of as wide as its trigger: for a trigger that
  /// hugs the one value it shows, as a toolbar's control does. The panel's `min_width` still holds.
  pub fn fit_options(mut self, fit_options: bool) -> Self {
    self.fit_options = fit_options;
    self
  }

  /// Puts the select in a state the theme's recipe names, or takes it out.
  /// `state(SelectState::Open, true)` draws the trigger as it is while the
  /// list is open, with the list closed: for a gallery or a test.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// The name the select's value has in the form it is in: the label of the chosen option.
  #[cfg(feature = "form")]
  pub fn name(mut self, name: impl Into<Arc<str>>) -> Self {
    self.name = Some(name.into());
    self
  }

  /// Styles one part of this select over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(PartStyle) -> PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for this select: what the component
  /// it is in says of the select in this place.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// Draws the trigger as it is under the pointer or focused from the
  /// keyboard, with nothing pointing at it: for a gallery or a test.
  pub fn force(mut self, interaction: Option<Interaction>) -> Self {
    self.forced = interaction;
    self
  }

  /// The id of the select's node: what is pressed and focused.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// Builds the select in `theme`. Converting a select into an element does
  /// this in the theme of what is being rendered.
  pub fn build(self, theme: &Theme) -> Element {
    // One judgment is shown: what is wrong over what is doubtful over what is right.
    let invalid = self.invalid;
    let warning = self.warning && !invalid;
    let valid = self.valid && !invalid && !warning;
    let case = Case {
      variant: None,
      size: Some(self.size),
      density: None,
      breakpoint: super::current_breakpoint(),
      states: self.states.clone(),
    }
    .state(SelectState::Invalid, invalid)
    .state(SelectState::Warning, warning)
    .state(SelectState::Valid, valid)
    .state(SelectState::ReadOnly, self.read_only)
    .state(SelectState::Disabled, self.disabled)
    .state(SelectState::FullWidth, self.full_width);
    let mut choices = self.choices;
    choices.name(self.id.as_deref().or(self.key.as_deref()));
    let choices: Box<dyn Choices> = choices;
    let view = SelectView {
      placeholder: self.placeholder,
      label: self.label,
      invalid,
      read_only: self.read_only,
      disabled: self.disabled,
      recipe: theme
        .resolve::<Select>(&case, Some(&self.rules))
        .owned_by(self.id.as_ref()),
      case,
      rules: self.rules,
      forced: self.forced,
      fit_options: self.fit_options,
      choices: Arc::from(choices),
      name: self.name,
      id: self.id,
      key: self.key,
    };
    match theme.renderer::<Select>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }
}

impl<V: 'static> From<Select<V>> for Element {
  fn from(select: Select<V>) -> Self {
    select.build(&current_theme())
  }
}

/// A select of any type of value, as what holds one is given it: an [`InputAddon`](super::InputAddon) of an input
/// group, the page size of a [`Pagination`](super::Pagination). The holder sizes it, names it and lays its rules over
/// it as it builds it; its handlers keep the type of its value.
pub struct AnySelect(Box<dyn Held>);

/// What a holder of a select does to it as it builds it.
#[derive(Default)]
pub(crate) struct Dress<'a> {
  pub size: Option<Name>,
  pub label: Option<&'a str>,
  pub rules: Option<&'a Recipe>,
  pub disabled: bool,
}

/// A select out of sight of the type of its value.
trait Held: Send + Sync {
  fn build_dressed(self: Box<Self>, theme: &Theme, dress: Dress<'_>) -> Element;
}

impl<V: 'static> Held for Select<V> {
  fn build_dressed(self: Box<Self>, theme: &Theme, dress: Dress<'_>) -> Element {
    let mut select = *self;
    if let Some(size) = dress.size {
      select = select.size(size);
    }
    if let Some(label) = dress.label {
      select = select.label(label);
    }
    if let Some(rules) = dress.rules {
      select = select.rules(rules);
    }
    if dress.disabled {
      select = select.disabled(true);
    }
    select.build(theme)
  }
}

impl<V: 'static> From<Select<V>> for AnySelect {
  fn from(select: Select<V>) -> Self {
    Self(Box::new(select))
  }
}

impl AnySelect {
  /// Builds the select in `theme`, as `dress` says.
  pub(crate) fn build(self, theme: &Theme, dress: Dress<'_>) -> Element {
    self.0.build_dressed(theme, dress)
  }
}

/// A select as a renderer sees it: what it was given, the style of its parts,
/// and its behavior ready to be dressed.
pub struct SelectView {
  pub placeholder: Option<Arc<str>>,
  /// What it is called, to what reads the tree.
  pub label: Option<Arc<str>>,
  /// Whether what is chosen is wrong, as the select shows it.
  pub invalid: bool,
  /// Whether what is chosen can be read and not changed.
  pub read_only: bool,
  pub disabled: bool,
  /// The style of each part of this select.
  pub recipe: ResolvedRecipe,
  case: Case,
  rules: Recipe,
  forced: Option<Interaction>,
  /// Whether the list is as wide as its longest option.
  fit_options: bool,
  choices: Arc<dyn Choices>,
  // Read only where the engine has forms.
  #[cfg_attr(not(feature = "form"), allow(dead_code))]
  name: Option<Arc<str>>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl SelectView {
  /// The style of a part, as this select is to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part).forced(self.forced)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// The recipe resolved with `state` on as well: what the trigger is while
  /// the list is open, or an item while it is chosen or disabled.
  pub fn in_state(&self, state: impl Into<Name>) -> ResolvedRecipe {
    let case = self.case.clone().state(state, true);
    self.theme().resolve::<Select>(&case, Some(&self.rules))
  }

  /// The engine's style for the select, from the recipe: the trigger at rest,
  /// under the pointer, focused from the keyboard and open; its value, its
  /// placeholder and its chevron; the panel of the list; an item at rest,
  /// under the pointer or the keys, chosen and disabled, with its check and
  /// its detail.
  pub fn select_style(&self) -> SelectStyle {
    engine_style(&self.recipe, self.forced, &|state| self.in_state(state)).menu_fits_options(self.fit_options)
  }

  /// The engine's select bound to the value and its options, in `style`, with
  /// the trigger's size, opacity and cursor: what is pressed, opened, moved
  /// through with the keys, focused and disabled. It is a stop of Tab
  /// wherever it is, and a combobox to what reads the tree.
  pub fn select(self, style: SelectStyle) -> Element {
    let theme = self.theme().clone();
    let trigger = self.part(SelectPart::Trigger).rest();
    let mut node = self
      .choices
      .clone()
      .build_plain(style, self.placeholder.clone(), self.read_only)
      .into_node();
    if let Some(width) = trigger.width.and_then(|width| width.resolve(&theme)) {
      NodeUpdate::width(&mut node, width);
    }
    // Its width is the most it takes: in a narrower room it is as wide as the room.
    if let Some(width) = trigger.max_width.and_then(|width| width.resolve(&theme)) {
      NodeUpdate::max_width(&mut node, width);
    }
    if let Some(height) = trigger.height.and_then(|height| height.resolve(&theme)) {
      NodeUpdate::height(&mut node, height);
    }
    if let Some(opacity) = opacity(trigger.opacity, &theme) {
      NodeUpdate::opacity(&mut node, opacity);
    }
    if let Some(cursor) = trigger.cursor {
      NodeUpdate::cursor(&mut node, cursor);
    }
    // A select is a stop of Tab only in a form or with an index of its own; a control of the design system is one
    // wherever it is.
    NodeUpdate::tab_index(&mut node, 0);
    NodeUpdate::disabled(&mut node, self.disabled);
    if let Some(label) = &self.label {
      NodeUpdate::describe(&mut node, Arc::from("a11y_name"), label.clone());
    }
    if self.read_only {
      NodeUpdate::describe(&mut node, Arc::from("a11y_readonly"), Arc::from("true"));
    }
    #[cfg(feature = "form")]
    if let Some(name) = &self.name {
      NodeUpdate::name(&mut node, name.clone());
    }
    let mut select = Element::from_node(node);
    if let Some(id) = &self.id {
      select = select.id(id.clone());
    }
    if let Some(key) = &self.key {
      select = select.key(key.clone());
    }
    select
  }

  /// How the default renderer builds a select: the engine's, in the recipe's style.
  pub fn render(self) -> Element {
    let style = self.select_style();
    self.select(style)
  }
}

/// The type of a part's text as the engine paints it: its typography, in the color of its foreground, which is named.
fn ink(theme: &Theme, style: &PartStyle) -> TextStyle {
  let mut text = style.text.resolve(theme);
  match &style.foreground {
    Some(TextColor::Color(color)) => text.color = *color,
    Some(TextColor::Palette(role)) => text.color = theme.palette_color(role.clone()),
    None => {}
  }
  text
}

/// The engine's style for a select from a recipe of the parts a select has, by their names ([`SelectPart`]): the
/// trigger at rest, under the pointer, focused from the keyboard and open; its value, its placeholder and its chevron;
/// the panel of the list; an item at rest, under the pointer or the keys, chosen and disabled, with its check and its
/// detail. `forced` is the trigger's; `in_state` resolves the recipe with a state of [`SelectState`] on as well. A
/// combobox's list is drawn by this too.
pub(crate) fn engine_style(
  recipe: &ResolvedRecipe,
  forced: Option<Interaction>,
  in_state: &dyn Fn(Name) -> ResolvedRecipe,
) -> SelectStyle {
  use SelectPart::{Check, Chevron, Detail, Group, Hint, Item, Panel, Placeholder, Separator, Trigger, Value};
  let theme = recipe.theme();
  let trigger = recipe.part(Trigger).forced(forced);
  let rest = trigger.rest();
  let value = rest.clone().merged(&recipe.part(Value).forced(forced).rest());
  let placeholder = value.clone().merged(&recipe.part(Placeholder).forced(forced).rest());
  let mut text = engine_part(&rest, theme).text(ink(theme, &value));
  if let Some(color) = &value.foreground {
    text = text.text_color(color.clone());
  }
  // The style is written field by field, not through its builders: each builder takes the whole style and gives it
  // back, and a build without optimizations keeps a copy of it on the stack for each, some 450 KB of stack here,
  // which overflowed the main thread's where a select is drawn deep in a page (a table's filter row).
  let mut style = SelectStyle::unstyled();
  style.trigger = text;
  style.placeholder_text = Some(ink(theme, &placeholder));
  if let Some(gap) = rest.gap {
    style.trigger_gap = gap;
  }
  let states = trigger.styles();
  if !states.hovered.is_empty() {
    style.trigger_hovered = Some(engine_part(&states.hovered, theme));
  }
  if !states.focus_visible.is_empty() {
    style.trigger_focused = Some(engine_part(&states.focus_visible, theme));
  }
  // Merged over the trigger last while the list is open: the whole of the trigger as the open case has it.
  style.trigger_open = Some(engine_part(
    &in_state(SelectState::Open.into()).part(Trigger).rest(),
    theme,
  ));

  let chevron = recipe.part(Chevron).forced(forced).rest();
  style.chevron = icon(icons::CHEVRON_DOWN, &chevron, theme);
  if let Some(color) = &chevron.foreground {
    style.chevron_color = Some(color.clone().into());
  }

  // The list is not forced: a forced interaction is the trigger's.
  let menu = recipe.part(Panel).rest();
  // Its height is the most it is, which the engine says apart: not the least the panel is.
  style.menu = engine_part(
    &PartStyle {
      height: None,
      ..menu.clone()
    },
    theme,
  );
  style.menu_gap = spacing(menu.gap, theme).unwrap_or(4.0);
  if let Some(height) = px(menu.height, theme) {
    style.max_menu_height = height;
  }
  // The list is as wide as its trigger, or as its least width where that is more: a narrow trigger's list holds
  // its options whole.
  if let Some(width) = px(menu.min_width, theme) {
    style.min_menu_width = width;
  }

  let item = recipe.part(Item);
  let chosen = in_state(SelectState::Selected.into()).part(Item);
  style.option = engine_part(&item.rest(), theme);
  style.option_selected = Some(engine_part(&chosen.rest(), theme));
  style.option_disabled = Some(engine_part(
    &in_state(SelectState::Disabled.into()).part(Item).rest(),
    theme,
  ));
  style.option_detail = engine_part(&recipe.part(Detail).rest(), theme);
  style.option_hint = engine_part(&recipe.part(Hint).rest(), theme);
  style.group_label = engine_part(&recipe.part(Group).rest(), theme);
  style.separator = engine_part(&recipe.part(Separator).rest(), theme);
  // The item under the keys looks as the one under the pointer does.
  if !item.styles().hovered.is_empty() {
    style.option_hovered = Some(engine_part(&item.styles().hovered, theme));
  }
  if !chosen.styles().hovered.is_empty() {
    style.option_selected_hovered = Some(engine_part(&chosen.styles().hovered, theme));
  }
  if let Some(gap) = item.rest().gap {
    style.checkmark_gap = gap.into();
    style.option_adornment_gap = gap.into();
  }

  let check = in_state(SelectState::Selected.into()).part(Check).rest();
  style.checkmark = icon(icons::CHECK, &check, theme);
  style.checkmark_position = SelectCheckmarkPosition::Trailing;
  style.single_checkmark = true;
  if let Some(side) = px(check.width, theme) {
    style.checkmark_size = Some(side.max(0.0));
  }
  if let Some(color) = &check.foreground {
    style.checkmark_color = Some(color.clone().into());
  }
  style
}

/// A length in pixels: `None` for one that is not.
pub(crate) fn px(length: Option<Length>, theme: &Theme) -> Option<f32> {
  match length?.resolve(theme)? {
    Dimension::Px(px) => Some(px),
    _ => None,
  }
}

pub(crate) fn spacing(value: Option<SpacingValue>, theme: &Theme) -> Option<f32> {
  value.map(|value| value.resolve(&theme.spacing(), 0.0))
}

pub(crate) fn opacity(opacity: Option<Opacity>, theme: &Theme) -> Option<f32> {
  match opacity? {
    Opacity::Value(value) => Some(value),
    Opacity::Level(level) => theme.opacities().try_get(level),
  }
}

/// A glyph of the theme's set in the side of `style`, in the color the engine
/// gives it: the chevron's or the check's.
pub(crate) fn icon(name: &'static str, style: &PartStyle, theme: &Theme) -> SelectIcon {
  let (side, theme) = (style.width.unwrap_or(Length::Icon(IconSize::Md)), theme.clone());
  SelectIcon::element(move |color| {
    let mut icon = Icon::new(name).size(side);
    if let Some(color) = color {
      icon = icon.color(color);
    }
    icon.build(&theme)
  })
}

/// A part's style as the engine's select paints a part: its fill, edge,
/// corners, shadow, ring, opacity, padding, height (at least), type and color.
pub(crate) fn engine_part(style: &PartStyle, theme: &Theme) -> SelectPartStyle {
  let mut part = SelectPartStyle::new();
  if let Some(color) = &style.background {
    part = part.background(color.clone());
  }
  // A line along the bottom is laid over the border of every side, as on any part.
  match (&style.border, &style.underline) {
    (Some(border), None) => part = part.border_custom(Borders::all(border.clone())),
    (border, Some(line)) => {
      let mut borders = border.clone().map(Borders::all).unwrap_or_default();
      borders.bottom = Some(line.clone());
      part = part.border_custom(borders);
    }
    (None, None) => {}
  }
  if let Some(radius) = style.radius {
    part.border_radius = Some(radius);
  }
  if let Some(shadow) = &style.shadow {
    part = part.box_shadow(shadow.clone());
  }
  let ring = match (&style.outline, style.focus_ring) {
    (Some(outline), _) => Some(outline.clone()),
    (None, Some(true)) => Some(theme.focus_ring().outline()),
    _ => None,
  };
  if let Some(ring) = ring {
    part = part.outline(ring);
  }
  if let Some(opacity) = opacity(style.opacity, theme) {
    part = part.opacity(opacity);
  }
  let [top, end, bottom, start] = style.paddings();
  if top.is_some() || end.is_some() || bottom.is_some() || start.is_some() {
    let none = SpacingValue::from(0.0);
    part = part.padding(
      Padding::new()
        .top(top.unwrap_or(none))
        .right(end.unwrap_or(none))
        .bottom(bottom.unwrap_or(none))
        .left(start.unwrap_or(none)),
    );
  }
  if let Some(height) = px(style.height, theme).or_else(|| px(style.min_height, theme)) {
    part = part.min_height(height);
  }
  if style.sets_text() {
    part = part.text(style.text.resolve(theme));
  }
  if let Some(color) = &style.foreground {
    part = part.text_color(color.clone());
  }
  part
}

impl<V: 'static> crate::ui::Styled for Select<V> {
  type Component = Self;

  fn rules(self, rules: &crate::ui::Recipe) -> Self {
    Self::rules(self, rules)
  }

  fn state(self, state: impl Into<crate::ui::Name>, on: bool) -> Self {
    Self::state(self, state, on)
  }
}

/// Every select has the one recipe and renderer, whatever its values are.
impl<V: 'static> Themed for Select<V> {
  const NAME: &'static str = "select";
  /// Its id is on the trigger: the box that is pressed.
  const ROOT: &'static str = "trigger";
  type Renderer = SelectRenderer;

  fn parts() -> crate::ui::Declared {
    SelectPart::declared()
  }

  fn states() -> crate::ui::Declared {
    SelectState::declared()
  }

  /// The design's select (`pencil/lurq-design-system.pen`, Select, and Menu
  /// and Menu Item for its list): a trigger shaped like an input, 260 by 36,
  /// the background's fill in an edge of 1 in the color of an input's, 6
  /// round, its text 12 from its edge in the body's type, its placeholder and
  /// its chevron of 16 in the muted foreground. Under the pointer its edge is
  /// the ring's color; focused from the keyboard and open it has a line of 2
  /// inside the box in that color; the destructive, warning and success
  /// colors for what is chosen, as an input has them. A read only one is on the muted fill in an edge of the border's
  /// color, as a read only input is. A disabled one is half opaque on the muted fill. Small is 32 with
  /// text of 13, large 40 with text of 15. The list is 4 under it on the popover's
  /// fill, in an edge of 1 in the border's color, 8 round, with the theme's
  /// middle shadow and 4 of padding; an item is 32 high, 8 from the edge of
  /// the list, 4 round, in the body's type, on the highlight under the
  /// pointer or the keys; the chosen one has a check of 16 at its end, and a
  /// disabled one is dimmed. A hint at an item's end is 11 in the mono type,
  /// muted; a group's label is 11 and semibold, muted, over its items; a
  /// line of the border's color is between groups, 4 from each.
  fn recipe() -> Recipe {
    use PaletteColor::{
      Background, Destructive, Foreground, Highlight, Muted, MutedForeground, Popover, Ring, Success,
    };
    use SelectPart::{Check, Chevron, Detail, Group, Hint, Item, Panel, Placeholder, Separator, Trigger};
    let edge = |color: PaletteColor| Border::inside(BorderSize::Sm, color);
    // The focus is a line of 2 inside the box, over its edge, as an input's is: an outline, so that the edge under
    // the pointer does not take it away.
    let ring = |color: PaletteColor| Outline::new(2.0, color).offset(-2.0);
    let recipe = Recipe::new()
      .base(Trigger, |s| {
        s.width(260.0)
          .max_width(Length::Fill)
          .height(ControlSize::Md)
          .padding_x(12.0)
          .gap(8.0)
          .radius(RadiusSize::Md)
          .background(Background)
          .border(edge(PaletteColor::Input))
          .foreground(Foreground)
          .typography(TypographyStyle::Body)
          .cursor(CursorIcon::Pointer)
      })
      .base(Placeholder, |s| s.foreground(MutedForeground))
      .base(Chevron, |s| s.square(IconSize::Md).foreground(MutedForeground))
      .base(Panel, |s| {
        s.gap(4.0)
          .padding_x(4.0)
          .padding_y(4.0)
          .radius(RadiusSize::Lg)
          .background(Popover)
          .border(edge(PaletteColor::Border))
          .shadow(ShadowStyle::Md)
      })
      .base(Item, |s| {
        s.height(32.0)
          .padding_x(8.0)
          .gap(8.0)
          .radius(RadiusSize::Sm)
          .foreground(Foreground)
          .typography(TypographyStyle::Body)
      })
      // A row of the density's height: 32 at the default, 24, 40 and 44 at the others. The box keeps its size.
      .row_density(Selector::new(), Item, 32.0)
      .base(Check, |s| s.square(IconSize::Md).foreground(Foreground))
      .base(Detail, |s| {
        s.typography(TypographyStyle::Caption).foreground(MutedForeground)
      })
      .base(Hint, |s| {
        s.font_family(FontFamily::Mono)
          .font_size(11.0)
          .foreground(MutedForeground)
      })
      // The board pads a group's label 6 over and 4 under.
      .base(Group, |s| {
        s.padding_x(8.0)
          .padding_top(6.0)
          .padding_bottom(4.0)
          .typography(TypographyStyle::Caption)
          .font_size(11.0)
          .weight(FontWeight::SemiBold)
          .foreground(MutedForeground)
      })
      .base(Separator, |s| {
        s.height(1.0)
          .padding_y(4.0)
          .background(PaletteColor::Border)
      })
      .hovered(Trigger, |s| s.border(edge(Ring)))
      .focus_visible(Trigger, |s| s.outline(ring(Ring)))
      .state(SelectState::Open, Trigger, |s| s.outline(ring(Ring)))
      .hovered(Item, |s| s.background(Highlight))
      // An item that cannot be chosen: its own state, resolved on its part.
      .state(SelectState::Disabled, Item, |s| s.opacity(0.45))
      .size(SelectSize::Sm, Trigger, |s| s.height(ControlSize::Sm).font_size(13.0))
      .size(SelectSize::Lg, Trigger, |s| s.height(ControlSize::Lg).font_size(15.0))
      .state(SelectState::FullWidth, Trigger, |s| s.width(Length::Fill))
      .state(SelectState::ReadOnly, Trigger, |s| {
        s.background(Muted)
          .border(edge(PaletteColor::Border))
          .cursor(CursorIcon::Default)
      })
      .state(SelectState::Disabled, Trigger, |s| {
        s.background(Muted)
          .opacity(OpacityLevel::Disabled)
          .cursor(CursorIcon::Default)
      });
    // A judgment keeps its color under the pointer, in the focus and while open.
    let recipe = [
      (SelectState::Invalid, Destructive),
      (SelectState::Warning, PaletteColor::Warning),
      (SelectState::Valid, Success),
    ]
    .into_iter()
    .fold(recipe, |recipe, (state, color)| {
      recipe
        .state(state, Trigger, |s| s.border(edge(color.clone())))
        .when(Selector::new().state(state).hovered(), Trigger, |s| {
          s.border(edge(color.clone()))
        })
        .when(Selector::new().state(state).focus_visible(), Trigger, |s| {
          s.outline(ring(color.clone()))
        })
        .when(Selector::new().state(state).state(SelectState::Open), Trigger, |s| {
          s.outline(ring(color))
        })
    });
    // In a fluid field the trigger is the fluid box, what is chosen on the line under the field's label; its line
    // says what its edge says.
    let fluid = || Selector::new().state(SelectState::Fluid);
    let recipe = recipe
      .state(SelectState::Fluid, Trigger, |s| {
        text_box::fluid_box(s)
          .width(Length::Fill)
          .height(text_box::FLUID_HEIGHT)
      })
      .when(fluid().hovered(), Trigger, |s| text_box::fluid_line(s, Ring))
      .when(fluid().state(SelectState::ReadOnly), Trigger, |s| {
        text_box::fluid_line(s, PaletteColor::Border)
      })
      .when(fluid().state(SelectState::ReadOnly).hovered(), Trigger, |s| {
        text_box::fluid_line(s, PaletteColor::Border)
      });
    [
      (SelectState::Invalid, Destructive),
      (SelectState::Warning, PaletteColor::Warning),
      (SelectState::Valid, Success),
    ]
    .into_iter()
    .fold(recipe, |recipe, (state, color)| {
      recipe
        .when(fluid().state(state), Trigger, |s| {
          text_box::fluid_line(s, color.clone())
        })
        .when(fluid().state(state).hovered(), Trigger, |s| {
          text_box::fluid_line(s, color)
        })
    })
  }
}

impl<V: 'static> super::FieldControl for Select<V> {
  /// Called by the field's label unless it has a name of its own, judged as the field is unless the app judged the
  /// select itself, and disabled where the field is.
  fn in_field(mut self, field: &super::InField) -> Element {
    if self.label.is_none() && !field.label.is_empty() {
      self.label = Some(field.label.clone());
    }
    if !(self.invalid || self.warning || self.valid) {
      self.invalid = field.judgment == Some(super::Judgment::Invalid);
      self.warning = field.judgment == Some(super::Judgment::Warning);
      self.valid = field.judgment == Some(super::Judgment::Valid);
    }
    self.disabled |= field.disabled;
    if field.fluid {
      self = self.state(SelectState::Fluid, true);
    }
    self.into()
  }

  fn has_fluid_form(&self) -> bool {
    true
  }
}
