//! A chip: a tag that is pressed, a filter or a choice that is on or off;
//! and a row of them that is one choice.

use std::sync::Arc;

use super::{
  icons, names,
  recipe::{Case, Interaction, Name, Recipe, ResolvedPart, ResolvedRecipe, Selector},
  shared::{ChangeHandler, Choice, Chosen, fading, glyph, pressable},
  theme::{Themed, current_theme},
};
use crate::{
  app::theme::{BorderSize, IconSize, OpacityLevel, PaletteColor, RadiusSize, Theme, TypographyStyle},
  components::{self, Row},
  core::{Signal, SignalValue},
  layout::Alignment,
  node::{CursorIcon, Element, FocusGroup, border::Border as Line},
};

/// The parts of a chip a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ChipPart {
  /// The chip itself: what is pressed.
  Root,
  /// The icon before its text: the check while it is chosen, else its own if it has one.
  Icon,
  /// Its text.
  Label,
}

names!(ChipPart {
  Root => "root",
  Icon => "icon",
  Label => "label",
});

/// The states of a chip a recipe styles, besides what the pointer and the
/// keyboard do to it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ChipState {
  /// It is chosen.
  Selected,
  /// It has an icon before its text, the part `Icon`: the check, or its own.
  Icon,
  /// It takes no input.
  Disabled,
}

names!(ChipState {
  Selected => "selected",
  Icon => "icon",
  Disabled => "disabled",
});

/// What builds a chip's nodes: a theme's in place of the default one.
pub type ChipRenderer = Arc<dyn Fn(ChipView) -> Element + Send + Sync>;

/// A chip: a tag that is pressed, a filter or a choice that is on or off.
/// Open issues only, a language among others, a kind of place to eat.
///
/// ```
/// use lurq::{core::Signal, ui::Chip};
///
/// # fn chips(open: Signal<bool>) {
/// let rust = Chip::new("Rust").selected(true).on_change(|on| println!("rust: {on}"));
/// // Bound to the app's signal: a press turns it.
/// let only_open = Chip::new("Open").bind(open);
/// # }
/// ```
///
/// A chip does not keep whether it is chosen: the app says so with
/// [`Chip::selected`], and is told of a press with [`Chip::on_change`], or
/// gives it a `Signal<bool>` with [`Chip::bind`]. A chosen chip has a check
/// before its text. Several that are one choice are a [`ChipGroup`].
pub struct Chip {
  label: Arc<str>,
  icon: Option<Name>,
  selected: bool,
  disabled: bool,
  states: Vec<Name>,
  rules: Recipe,
  forced: Option<Interaction>,
  on_change: Vec<Arc<dyn Fn(bool) + Send + Sync>>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl Chip {
  pub fn new(label: &str) -> Self {
    Self {
      label: Arc::from(label),
      icon: None,
      selected: false,
      disabled: false,
      states: Vec::new(),
      rules: Recipe::new(),
      forced: None,
      on_change: Vec::new(),
      id: None,
      key: None,
    }
  }

  /// An icon before the text while the chip is not chosen, by its name in
  /// the theme's set. While it is chosen the check is in its place.
  pub fn icon(mut self, name: impl Into<Name>) -> Self {
    self.icon = Some(name.into());
    self
  }

  /// Whether the chip is chosen.
  pub fn selected(mut self, selected: bool) -> Self {
    self.selected = selected;
    self
  }

  /// Chosen while `selected` is true, which a press turns.
  pub fn bind(self, selected: Signal<bool>) -> Self {
    let set = selected.clone();
    self.selected(selected.get()).on_change(move |next| set.set(next))
  }

  /// Whether the chip takes no input.
  pub fn disabled(mut self, disabled: bool) -> Self {
    self.disabled = disabled;
    self
  }

  /// Puts the chip in a state the theme's recipe names, or takes it out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// Called on a press, with what the chip is to be: chosen if it was not.
  pub fn on_change(mut self, handler: impl Fn(bool) + Send + Sync + 'static) -> Self {
    self.on_change.push(Arc::new(handler));
    self
  }

  /// Styles one part of this chip over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(super::PartStyle) -> super::PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for this chip: what the component
  /// it is in says of the chip in this place.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// Draws the chip as it is under the pointer, held down or focused from
  /// the keyboard, with nothing pointing at it: for a gallery or a test.
  pub fn force(mut self, interaction: Option<Interaction>) -> Self {
    self.forced = interaction;
    self
  }

  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// Builds the chip in `theme`. Converting a chip into an element does this
  /// in the theme of what is being rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let case = Case {
      variant: None,
      size: None,
      density: None,
      breakpoint: super::current_breakpoint(),
      states: self.states.clone(),
    }
    .state(ChipState::Selected, self.selected)
    .state(ChipState::Icon, self.selected || self.icon.is_some())
    .state(ChipState::Disabled, self.disabled);
    let view = ChipView {
      label: self.label,
      icon: self.icon,
      selected: self.selected,
      disabled: self.disabled,
      recipe: theme
        .resolve::<Chip>(&case, Some(&self.rules))
        .owned_by(self.id.as_ref()),
      forced: self.forced,
      on_change: self.on_change,
      id: self.id,
      key: self.key,
    };
    match theme.renderer::<Chip>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }
}

impl From<Chip> for Element {
  fn from(chip: Chip) -> Self {
    chip.build(&current_theme())
  }
}

/// A chip as a renderer sees it: what it was given, the style of its parts,
/// and its behavior ready to be dressed.
pub struct ChipView {
  pub label: Arc<str>,
  /// The name of its own icon, if it has one.
  pub icon: Option<Name>,
  pub selected: bool,
  pub disabled: bool,
  /// The style of each part of this chip.
  pub recipe: ResolvedRecipe,
  forced: Option<Interaction>,
  on_change: Vec<Arc<dyn Fn(bool) + Send + Sync>>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl ChipView {
  /// The style of a part, as this chip is to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part).forced(self.forced)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// The chip's behavior with the style of its root: the element that is
  /// pressed, focused and disabled, with nothing in it yet. It is a button to
  /// what reads the tree, called by its text, that says whether it is pressed.
  pub fn root(&self) -> components::Button {
    let mut root = pressable(&self.part(ChipPart::Root), self.disabled)
      .describe("a11y_role", "button")
      .describe("a11y_name", self.label.clone())
      .describe("a11y_pressed", if self.selected { "true" } else { "false" });
    let (handlers, next) = (self.on_change.clone(), !self.selected);
    if !handlers.is_empty() && !self.disabled {
      root = root.on_click(move |_| {
        for handler in &handlers {
          handler(next);
        }
      });
    }
    if let Some(id) = &self.id {
      root = root.id(id.clone());
    }
    if let Some(key) = &self.key {
      root = root.key(key.clone());
    }
    root
  }

  /// The icon before the text: the check while the chip is chosen, its own icon while it is not.
  pub fn shown_icon(&self) -> Option<Name> {
    if self.selected {
      Some(Name::from(icons::CHECK))
    } else {
      self.icon
    }
  }

  /// How the default renderer builds a chip: the root, with its icon and its text.
  pub fn render(self) -> Element {
    let mut root = self.root();
    if let Some(name) = self.shown_icon() {
      root = root.child(glyph(&self.part(ChipPart::Icon), name, false));
    }
    let label = self.part(ChipPart::Label);
    root.child(label.text(&self.label).nowrap().part(&label)).into()
  }
}

impl crate::ui::Styled for Chip {
  type Component = Self;

  fn rules(self, rules: &crate::ui::Recipe) -> Self {
    Self::rules(self, rules)
  }

  fn state(self, state: impl Into<crate::ui::Name>, on: bool) -> Self {
    Self::state(self, state, on)
  }
}

impl Themed for Chip {
  const NAME: &'static str = "chip";
  type Renderer = ChipRenderer;

  fn parts() -> crate::ui::Declared {
    ChipPart::declared()
  }

  fn states() -> crate::ui::Declared {
    ChipState::declared()
  }

  /// The design's chip (`pencil/lurq-design-system.pen`, Chip): 28 high and
  /// round, padded 12, its text 13 and medium. One not chosen is in the
  /// background's fill with a line of an input's edge; a chosen one is in the
  /// primary color, line and fill, with a check of 14 before its text, 6 from
  /// it, and padded 8 at its start.
  fn recipe() -> Recipe {
    use ChipPart::{Icon, Label, Root};
    use PaletteColor::*;
    let line = |color: PaletteColor| Line::inside(BorderSize::Sm, color);
    Recipe::new()
      .base(Root, |s| {
        fading(
          s.height(28.0)
            .padding_x(12.0)
            .gap(6.0)
            .radius(RadiusSize::Full)
            .background(Background)
            .border(line(Input))
            .foreground(Foreground)
            .cursor(CursorIcon::Pointer),
        )
      })
      .base(Label, |s| s.typography(TypographyStyle::Label))
      .base(Icon, |s| s.square(IconSize::Sm))
      .hovered(Root, |s| s.background(Highlight).foreground(HighlightForeground))
      .pressed(Root, |s| s.veil(StatePressed))
      .focus_visible(Root, |s| s.focus_ring())
      .state(ChipState::Icon, Root, |s| s.padding_start(8.0))
      .state(ChipState::Selected, Root, |s| {
        s.background(Primary).border(line(Primary)).foreground(PrimaryForeground)
      })
      // A chosen chip has its fill already: under the pointer it is the primary color's hover.
      .when(Selector::new().state(ChipState::Selected).hovered(), Root, |s| {
        s.background(PrimaryHover)
          .border(line(PrimaryHover))
          .foreground(PrimaryForeground)
      })
      .state(ChipState::Disabled, Root, |s| {
        s.opacity(OpacityLevel::Disabled).cursor(CursorIcon::Default)
      })
  }
}

/// The parts of a group of chips a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ChipGroupPart {
  /// The row the chips are in, which goes on to the next line where it has no room.
  Root,
  /// What the group says of each chip in it: laid over the theme's recipe for a chip's root.
  Item,
}

names!(ChipGroupPart {
  Root => "root",
  Item => "item",
});

/// The states of a group of chips a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ChipGroupState {
  /// The whole group takes no input.
  Disabled,
  /// What is chosen is wrong: each chip has a line in the destructive color.
  Invalid,
}

names!(ChipGroupState {
  Disabled => "disabled",
  Invalid => "invalid",
});

/// What builds a group of chips' nodes: a theme's in place of the default one.
pub type ChipGroupRenderer = Arc<dyn Fn(ChipGroup) -> Element + Send + Sync>;

/// One chip of a group, with what a press of it does.
struct Item {
  label: Arc<str>,
  icon: Option<Name>,
  disabled: bool,
  selected: bool,
  pick: Arc<dyn Fn() + Send + Sync>,
  id: Option<Arc<str>>,
  states: Vec<Name>,
}

/// Chips that are one choice: the filters of a list, of which any number are
/// on, or the kinds of a thing, of which one is.
///
/// ```
/// use lurq::{core::Signal, ui::ChipGroup};
///
/// # fn groups(languages: Signal<Vec<&'static str>>, kind: Signal<u8>) {
/// // Any number are chosen: the signal's values.
/// let filters = ChipGroup::multiple(languages, [("rust", "Rust"), ("swift", "Swift"), ("kotlin", "Kotlin")])
///   .label("Languages")
///   .on_change(|languages| println!("{languages:?}"));
/// // One of them is: the signal's value.
/// let kinds = ChipGroup::new(kind, [(0, "All"), (1, "Issues"), (2, "Pull requests")]);
/// # }
/// ```
///
/// Tab stops in the group once, the arrow keys move between its chips, and
/// Space or Enter turns the one the focus is on. Bound to a signal
/// ([`new`](Self::new), [`multiple`](Self::multiple)) the group sets it, or
/// the app holds the value and is told what it is to be
/// ([`controlled`](Self::controlled), [`controlled_multiple`](Self::controlled_multiple)).
/// [`on_change`](Self::on_change) is told the new value as a person changes
/// it: `&T` for a group of one, a `ChipGroup<T>`, and the whole new set
/// `&[T]` for a group of any number, a `ChipGroup<T, [T]>`. The compiler
/// checks what a handler is told. `T` is `()` where nothing names it: the
/// theme's recipe is `ChipGroup`'s, whatever its values.
pub struct ChipGroup<T = (), V: ?Sized = T> {
  choices: Vec<Choice<T>>,
  chosen: Chosen<T, V>,
  on_change: Vec<ChangeHandler<V>>,
  label: Option<Arc<str>>,
  disabled: bool,
  invalid: bool,
  states: Vec<Name>,
  rules: Recipe,
  forced: Option<Interaction>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl<T: Clone + PartialEq + Send + Sync + 'static, V: ?Sized + 'static> ChipGroup<T, V> {
  fn of(chosen: Chosen<T, V>, choices: impl IntoIterator<Item = impl Into<Choice<T>>>) -> Self {
    Self {
      choices: choices.into_iter().map(Into::into).collect(),
      chosen,
      on_change: Vec::new(),
      label: None,
      disabled: false,
      invalid: false,
      states: Vec::new(),
      rules: Recipe::new(),
      forced: None,
      id: None,
      key: None,
    }
  }
}

impl<T: Clone + PartialEq + Send + Sync + 'static> ChipGroup<T, T> {
  /// A group of which exactly one chip is chosen: the one whose value the
  /// signal has. A press of another sets the signal.
  pub fn new(value: Signal<T>, choices: impl IntoIterator<Item = impl Into<Choice<T>>>) -> Self
  where
    T: SignalValue,
  {
    Self::of(Chosen::one(value), choices)
  }

  /// A group of one whose value the app holds: the chip of `value` is
  /// chosen, and a press of another tells `on_change` the value it is to be.
  /// A press of the chosen one tells nothing.
  pub fn controlled(
    value: T,
    choices: impl IntoIterator<Item = impl Into<Choice<T>>>,
    on_change: impl Fn(&T) + Send + Sync + 'static,
  ) -> Self {
    Self::of(Chosen::held_one(value), choices).on_change(on_change)
  }
}

impl<T: Clone + PartialEq + Send + Sync + 'static> ChipGroup<T, [T]> {
  /// A group of which any number of chips are chosen: those whose values the
  /// signal has. A press adds a value, or takes it out.
  pub fn multiple(values: Signal<Vec<T>>, choices: impl IntoIterator<Item = impl Into<Choice<T>>>) -> Self
  where
    Vec<T>: SignalValue,
  {
    Self::of(Chosen::many(values), choices)
  }

  /// A group of any number whose values the app holds: the chips of
  /// `values` are chosen, and a press tells `on_change` the whole set it is
  /// to be, with the value pressed added or taken out.
  pub fn controlled_multiple(
    values: impl IntoIterator<Item = T>,
    choices: impl IntoIterator<Item = impl Into<Choice<T>>>,
    on_change: impl Fn(&[T]) + Send + Sync + 'static,
  ) -> Self {
    Self::of(Chosen::held_many(values.into_iter().collect()), choices).on_change(on_change)
  }
}

impl<T: Clone + PartialEq + Send + Sync + 'static, V: ?Sized + 'static> ChipGroup<T, V> {
  /// Called as a person changes what is chosen, after the handlers there
  /// are, with the new value: `&T` for a group of one, the whole new set
  /// `&[T]` for a group of any number. Not called where the app sets the
  /// signal, nor for a press of what is already so.
  pub fn on_change(mut self, handler: impl Fn(&V) + Send + Sync + 'static) -> Self {
    self.on_change.push(Arc::new(handler));
    self
  }

  /// What the group is called to what reads the tree: what the chips are a choice of.
  pub fn label(mut self, label: &str) -> Self {
    self.label = Some(Arc::from(label));
    self
  }

  /// Whether the whole group takes no input.
  pub fn disabled(mut self, disabled: bool) -> Self {
    self.disabled = disabled;
    self
  }

  /// Whether what is chosen is wrong: each chip has a line in the destructive color.
  pub fn invalid(mut self, invalid: bool) -> Self {
    self.invalid = invalid;
    self
  }

  /// Puts the group in a state the theme's recipe names, or takes it out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// Styles one part of this group over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(super::PartStyle) -> super::PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for this group: what the component
  /// it is in says of it in this place.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// Draws the chip where Tab enters the group as it is under the pointer,
  /// held down or focused from the keyboard, with nothing pointing at it:
  /// for a gallery or a test.
  pub fn force(mut self, interaction: Option<Interaction>) -> Self {
    self.forced = interaction;
    self
  }

  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// Builds the group in `theme`. Converting a group into an element does
  /// this in the theme of what is being rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let mut case = Case::new()
      .breakpoint(super::current_breakpoint())
      .state(ChipGroupState::Disabled, self.disabled)
      .state(ChipGroupState::Invalid, self.invalid);
    for state in &self.states {
      case = case.state(state.clone(), true);
    }
    let recipe = theme
      .resolve::<ChipGroup>(&case, Some(&self.rules))
      .owned_by(self.id.as_ref());
    let item = recipe.rules_for(ChipGroupPart::Item, ChipPart::Root);
    let mut root = Row::new()
      .wrap()
      .align_items(Alignment::Center)
      // The ring of a chip's focus is drawn outside it.
      .overflow_visible()
      .part(&recipe.part(ChipGroupPart::Root))
      .focus_group(FocusGroup::horizontal().one_stop(true))
      .describe("a11y_role", "group");
    if let Some(label) = &self.label {
      root = root.describe("a11y_name", label.clone());
    }
    if self.invalid {
      root = root.describe("a11y_invalid", "true");
    }
    // Each chip with what a press of it does, of no type of the app's.
    let handlers: Arc<[ChangeHandler<V>]> = self.on_change.into();
    let items: Vec<Item> = self
      .choices
      .into_iter()
      .map(|choice| Item {
        selected: self.chosen.has(&choice.value),
        pick: self.chosen.picker(choice.value, handlers.clone()),
        label: choice.label,
        icon: choice.icon,
        disabled: choice.disabled,
        id: choice.id,
        states: choice.states,
      })
      .collect();
    // Where Tab enters the group: the interaction forced is drawn there alone, as it can only be.
    let entry = items
      .iter()
      .position(|item| item.selected && !item.disabled)
      .or_else(|| items.iter().position(|item| !item.disabled));
    for (index, choice) in items.into_iter().enumerate() {
      let pick = choice.pick;
      let mut chip = Chip::new(&choice.label)
        .selected(choice.selected)
        .disabled(self.disabled || choice.disabled)
        .rules(&item)
        .force(self.forced.filter(|_| Some(index) == entry))
        .on_change(move |_| pick());
      if let Some(name) = choice.icon {
        chip = chip.icon(name);
      }
      if let Some(id) = choice.id {
        chip = chip.id(id);
      }
      for state in choice.states {
        chip = chip.state(state, true);
      }
      root = root.child(chip.build(theme));
    }
    if let Some(id) = self.id {
      root = root.id(id);
    }
    if let Some(key) = self.key {
      root = root.key(key);
    }
    root.into()
  }
}

impl<T: Clone + PartialEq + Send + Sync + 'static, V: ?Sized + 'static> From<ChipGroup<T, V>> for Element {
  fn from(group: ChipGroup<T, V>) -> Self {
    group.build(&current_theme())
  }
}

impl<T: Clone + PartialEq + Send + Sync + 'static, V: ?Sized + 'static> crate::ui::Styled for ChipGroup<T, V> {
  type Component = Self;

  fn rules(self, rules: &crate::ui::Recipe) -> Self {
    Self::rules(self, rules)
  }

  fn state(self, state: impl Into<crate::ui::Name>, on: bool) -> Self {
    Self::state(self, state, on)
  }
}

impl<T: Send + Sync + 'static, V: ?Sized + 'static> Themed for ChipGroup<T, V> {
  const NAME: &'static str = "chip_group";
  type Renderer = ChipGroupRenderer;

  fn parts() -> crate::ui::Declared {
    ChipGroupPart::declared()
  }

  fn states() -> crate::ui::Declared {
    ChipGroupState::declared()
  }

  /// The design's group of chips (`pencil/lurq-design-system.pen`, Chip group): chips 6 apart, with nothing
  /// around them.
  fn recipe() -> Recipe {
    Recipe::new()
      .base(ChipGroupPart::Root, |s| s.gap(6.0))
      .state(ChipGroupState::Invalid, ChipGroupPart::Item, |s| {
        s.border(Line::inside(BorderSize::Sm, PaletteColor::Destructive))
      })
  }
}

impl<T: Clone + PartialEq + Send + Sync + 'static, V: ?Sized + 'static> super::FieldControl for ChipGroup<T, V> {
  /// Called by the field's label unless it has a name of its own, invalid where the field is, and disabled
  /// where the field is. Each chip is called by its own text.
  fn in_field(mut self, field: &super::InField) -> Element {
    if self.label.is_none() {
      self.label = Some(field.label.clone());
    }
    self.invalid |= field.judgment == Some(super::Judgment::Invalid);
    self.disabled |= field.disabled;
    self.into()
  }
}
