//! A chip: a tag that is pressed, a filter or a choice that is on or off;
//! and a row of them that is one choice.

use std::sync::Arc;

use super::{
  icons, names,
  recipe::{Case, Interaction, Name, Recipe, ResolvedPart, ResolvedRecipe, Selector},
  shared::{Choice, Chosen, fading, glyph, pressable},
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
  /// It has an icon before its text: the check, or its own.
  Leading,
  /// It takes no input.
  Disabled,
}

names!(ChipState {
  Selected => "selected",
  Leading => "leading",
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
    .state(ChipState::Leading, self.selected || self.icon.is_some())
    .state(ChipState::Disabled, self.disabled);
    let view = ChipView {
      label: self.label,
      icon: self.icon,
      selected: self.selected,
      disabled: self.disabled,
      recipe: theme.resolve::<Chip>(&case, Some(&self.rules)),
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
  pub fn leading(&self) -> Option<Name> {
    if self.selected {
      Some(Name::from(icons::CHECK))
    } else {
      self.icon
    }
  }

  /// How the default renderer builds a chip: the root, with its icon and its text.
  pub fn render(self) -> Element {
    let mut root = self.root();
    if let Some(name) = self.leading() {
      root = root.child(glyph(&self.part(ChipPart::Icon), name, false));
    }
    let label = self.part(ChipPart::Label);
    root.child(label.text(&self.label).nowrap().part(&label)).into()
  }
}

impl Themed for Chip {
  const NAME: &'static str = "chip";
  type Renderer = ChipRenderer;

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
      .state(ChipState::Leading, Root, |s| s.padding_start(8.0))
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

/// One chip of a group, with what a press of it does.
struct Item {
  label: Arc<str>,
  icon: Option<Name>,
  disabled: bool,
  selected: bool,
  pick: Arc<dyn Fn() + Send + Sync>,
}

/// Chips that are one choice: the filters of a list, of which any number are
/// on, or the kinds of a thing, of which one is.
///
/// ```
/// use lurq::{core::Signal, ui::ChipGroup};
///
/// # fn groups(languages: Signal<Vec<&'static str>>, kind: Signal<u8>) {
/// // Any number are chosen: the signal's values.
/// let filters = ChipGroup::multiple(languages, [("rust", "Rust"), ("swift", "Swift"), ("kotlin", "Kotlin")]);
/// // One of them is: the signal's value.
/// let kinds = ChipGroup::new(kind, [(0, "All"), (1, "Issues"), (2, "Pull requests")]);
/// # }
/// ```
///
/// Tab stops in the group once, the arrow keys move between its chips, and
/// Space or Enter turns the one the focus is on.
pub struct ChipGroup {
  items: Vec<Item>,
  disabled: bool,
  rules: Recipe,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl ChipGroup {
  fn of<T: Clone + PartialEq + Send + Sync + 'static>(
    chosen: Chosen<T>,
    choices: impl IntoIterator<Item = impl Into<Choice<T>>>,
  ) -> Self {
    let items = choices
      .into_iter()
      .map(Into::into)
      .map(|choice: Choice<T>| Item {
        selected: chosen.has(&choice.value),
        pick: Arc::new(chosen.picker(choice.value)),
        label: choice.label,
        icon: choice.icon,
        disabled: choice.disabled,
      })
      .collect();
    Self {
      items,
      disabled: false,
      rules: Recipe::new(),
      id: None,
      key: None,
    }
  }

  /// A group of which exactly one chip is chosen: the one whose value the
  /// signal has. A press of another sets the signal.
  pub fn new<T>(value: Signal<T>, choices: impl IntoIterator<Item = impl Into<Choice<T>>>) -> Self
  where
    T: Clone + PartialEq + Send + Sync + SignalValue + 'static,
  {
    Self::of(Chosen::one(value), choices)
  }

  /// A group of which any number of chips are chosen: those whose values the
  /// signal has. A press adds a value, or takes it out.
  pub fn multiple<T>(values: Signal<Vec<T>>, choices: impl IntoIterator<Item = impl Into<Choice<T>>>) -> Self
  where
    T: Clone + PartialEq + Send + Sync + 'static,
    Vec<T>: SignalValue,
  {
    Self::of(Chosen::many(values), choices)
  }

  /// A group whose state the app holds: the chips of `selected` are chosen,
  /// and `press` is told the value of each one pressed.
  pub fn controlled<T: Clone + PartialEq + Send + Sync + 'static>(
    selected: impl IntoIterator<Item = T>,
    choices: impl IntoIterator<Item = impl Into<Choice<T>>>,
    press: impl Fn(T) + Send + Sync + 'static,
  ) -> Self {
    Self::of(Chosen::held(selected.into_iter().collect(), press), choices)
  }

  /// Whether the whole group takes no input.
  pub fn disabled(mut self, disabled: bool) -> Self {
    self.disabled = disabled;
    self
  }

  /// Styles one part of this group over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(super::PartStyle) -> super::PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
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
    let case = Case::new().breakpoint(super::current_breakpoint());
    let recipe = theme.resolve::<ChipGroup>(&case, Some(&self.rules));
    let item = recipe.rules_for(ChipGroupPart::Item, ChipPart::Root);
    let mut root = Row::new()
      .wrap()
      .align_items(Alignment::Center)
      // The ring of a chip's focus is drawn outside it.
      .overflow_visible()
      .part(&recipe.part(ChipGroupPart::Root))
      .focus_group(FocusGroup::horizontal().one_stop(true));
    for choice in self.items {
      let pick = choice.pick;
      let mut chip = Chip::new(&choice.label)
        .selected(choice.selected)
        .disabled(self.disabled || choice.disabled)
        .rules(&item)
        .on_change(move |_| pick());
      if let Some(name) = choice.icon {
        chip = chip.icon(name);
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

impl From<ChipGroup> for Element {
  fn from(group: ChipGroup) -> Self {
    group.build(&current_theme())
  }
}

impl Themed for ChipGroup {
  const NAME: &'static str = "chip_group";
  type Renderer = Arc<dyn Fn(ChipGroup) -> Element + Send + Sync>;

  /// The design's group of chips (`pencil/lurq-design-system.pen`, Chip group): chips 6 apart, with nothing
  /// around them.
  fn recipe() -> Recipe {
    Recipe::new().base(ChipGroupPart::Root, |s| s.gap(6.0))
  }
}

impl super::FieldControl for ChipGroup {
  /// Disabled where the field is. Each chip is called by its own text, and the group has no judgment.
  fn in_field(mut self, field: &super::InField) -> Element {
    self.disabled |= field.disabled;
    self.into()
  }
}
