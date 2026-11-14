//! A radio: one of a few choices that exclude each other, and the group that is the choice.

use std::sync::Arc;

use super::{
  names,
  part_style::PartTransition,
  recipe::{Case, Interaction, Name, Recipe, ResolvedPart, ResolvedRecipe},
  shared::{Choice, Chosen, fading, pressable},
  theme::{Themed, current_theme},
};
use crate::{
  animation::Transition,
  app::theme::{BorderSize, MotionDuration, OpacityLevel, PaletteColor, RadiusSize, Theme, TypographyStyle},
  components::{self, Column, Row},
  core::{Signal, SignalValue},
  layout::Alignment,
  node::{CursorIcon, Element, FocusGroup, border::Border},
};

/// The parts of a radio a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RadioPart {
  /// The radio and its label, side by side.
  Root,
  /// What is pressed: the round mark with its edge.
  Mark,
  /// What is in the mark while the radio is chosen.
  Dot,
  /// Its text, beside the mark.
  Label,
}

names!(RadioPart {
  Root => "root",
  Mark => "mark",
  Dot => "dot",
  Label => "label",
});

/// The states of a radio a recipe styles, besides what the pointer and the
/// keyboard do to it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RadioState {
  /// It is the choice made.
  Chosen,
  /// What it is part of was answered wrongly, or not at all.
  Invalid,
  /// It takes no input.
  Disabled,
}

names!(RadioState {
  Chosen => "chosen",
  Invalid => "invalid",
  Disabled => "disabled",
});

/// What builds a radio's nodes: a theme's in place of the default one.
pub type RadioRenderer = Arc<dyn Fn(RadioView) -> Element + Send + Sync>;

/// A radio: one of a few choices of which exactly one is made. The density
/// of a list, the plan of an account, the size of a print.
///
/// ```
/// use lurq::ui::Radio;
///
/// let compact = Radio::new("Compact").chosen(true);
/// let spacious = Radio::new("Spacious").on_choose(|| println!("spacious"));
/// ```
///
/// A radio does not keep whether it is chosen: the app says so with
/// [`Radio::chosen`], and is told with [`Radio::on_choose`] that it was
/// pressed while it was not. A press of one that is chosen already does
/// nothing, and a press of its label is a press of the radio. Radios that are
/// one choice are a [`RadioGroup`].
pub struct Radio {
  label: Arc<str>,
  label_hidden: bool,
  chosen: bool,
  invalid: bool,
  disabled: bool,
  states: Vec<Name>,
  rules: Recipe,
  forced: Option<Interaction>,
  on_choose: Vec<Arc<dyn Fn() + Send + Sync>>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl Radio {
  /// A radio called `label`, with the label beside it.
  pub fn new(label: &str) -> Self {
    Self {
      label: Arc::from(label),
      label_hidden: false,
      chosen: false,
      invalid: false,
      disabled: false,
      states: Vec::new(),
      rules: Recipe::new(),
      forced: None,
      on_choose: Vec::new(),
      id: None,
      key: None,
    }
  }

  /// Whether the radio is its mark alone. Its label stays what it is called.
  pub fn label_hidden(mut self, hidden: bool) -> Self {
    self.label_hidden = hidden;
    self
  }

  /// Whether the radio is the choice made.
  pub fn chosen(mut self, chosen: bool) -> Self {
    self.chosen = chosen;
    self
  }

  /// Whether what the radio is part of was answered wrongly, or not at all.
  pub fn invalid(mut self, invalid: bool) -> Self {
    self.invalid = invalid;
    self
  }

  /// Whether the radio takes no input.
  pub fn disabled(mut self, disabled: bool) -> Self {
    self.disabled = disabled;
    self
  }

  /// Puts the radio in a state the theme's recipe names, or takes it out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// Called on a press of the radio while it is not chosen: it is to be.
  pub fn on_choose(mut self, handler: impl Fn() + Send + Sync + 'static) -> Self {
    self.on_choose.push(Arc::new(handler));
    self
  }

  /// Styles one part of this radio over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(super::PartStyle) -> super::PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for this radio: what the component
  /// it is in says of the radio in this place.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// Draws the radio as it is under the pointer, held down or focused from
  /// the keyboard, with nothing pointing at it: for a gallery or a test.
  pub fn force(mut self, interaction: Option<Interaction>) -> Self {
    self.forced = interaction;
    self
  }

  /// The id of the mark: what is pressed.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// Builds the radio in `theme`. Converting a radio into an element does
  /// this in the theme of what is being rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let case = Case {
      variant: None,
      size: None,
      density: None,
      breakpoint: super::current_breakpoint(),
      states: self.states.clone(),
    }
    .state(RadioState::Chosen, self.chosen)
    .state(RadioState::Invalid, self.invalid)
    .state(RadioState::Disabled, self.disabled);
    let view = RadioView {
      label: self.label,
      label_hidden: self.label_hidden,
      chosen: self.chosen,
      invalid: self.invalid,
      disabled: self.disabled,
      recipe: theme.resolve::<Radio>(&case, Some(&self.rules)),
      forced: self.forced,
      on_choose: self.on_choose,
      id: self.id,
      key: self.key,
    };
    match theme.renderer::<Radio>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }
}

impl From<Radio> for Element {
  fn from(radio: Radio) -> Self {
    radio.build(&current_theme())
  }
}

/// A radio as a renderer sees it: what it was given, the style of its parts,
/// and its behavior ready to be dressed.
pub struct RadioView {
  pub label: Arc<str>,
  pub label_hidden: bool,
  pub chosen: bool,
  pub invalid: bool,
  pub disabled: bool,
  /// The style of each part of this radio.
  pub recipe: ResolvedRecipe,
  forced: Option<Interaction>,
  on_choose: Vec<Arc<dyn Fn() + Send + Sync>>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl RadioView {
  /// The style of a part, as this radio is to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part).forced(self.forced)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// What a press does: every handler is told, unless the radio is chosen already.
  fn press(&self) -> Option<impl Fn() + Send + Sync + Clone + 'static> {
    let handlers = self.on_choose.clone();
    (!handlers.is_empty() && !self.disabled && !self.chosen).then(|| {
      move || {
        for handler in &handlers {
          handler();
        }
      }
    })
  }

  /// The radio's behavior with the style of its mark: the element that is
  /// pressed, focused and disabled, with nothing in it yet. It is a radio to
  /// what reads the tree, called by its label, and says whether it is chosen.
  pub fn mark(&self) -> components::Button {
    let mut mark = pressable(&self.part(RadioPart::Mark), self.disabled)
      .align_items(Alignment::Center)
      .describe("a11y_role", "radio")
      .describe("a11y_name", self.label.clone())
      .describe("a11y_checked", if self.chosen { "true" } else { "false" })
      // In a group, which Tab stops at once, the chosen radio is where Tab enters.
      .focus_entry(self.chosen);
    if let Some(press) = self.press() {
      mark = mark.on_click(move |_| press());
    }
    if let Some(id) = &self.id {
      mark = mark.id(id.clone());
    }
    if let Some(key) = &self.key {
      mark = mark.key(key.clone());
    }
    mark
  }

  /// The dot in the style of its part.
  pub fn dot(&self) -> Row {
    Row::new().part(&self.part(RadioPart::Dot))
  }

  /// How the default renderer builds a radio: the mark, with the dot in it
  /// while the radio is chosen, and the label beside it.
  pub fn render(self) -> Element {
    let mut mark = self.mark();
    if self.chosen {
      mark = mark.child(self.dot());
    }
    let mut root = Row::new()
      .align_items(Alignment::Center)
      .part(&self.part(RadioPart::Root))
      .child(mark);
    if !self.label_hidden {
      let part = self.part(RadioPart::Label);
      let mut label = Row::new().part(&part).child(part.text(&self.label).nowrap());
      if let Some(press) = self.press() {
        label = label.on_click(move |_| press());
      }
      root = root.child(label);
    }
    root.into()
  }
}

impl Themed for Radio {
  const NAME: &'static str = "radio";
  type Renderer = RadioRenderer;

  /// The design's radio (`pencil/lurq-design-system.pen`, Radio): a round
  /// mark of 16 in the background's color, with an edge of 1 inside it in the
  /// color of an input's edge; chosen, the edge is in the primary color and a
  /// dot of 8 in it is too. The edge of an invalid one is in the destructive
  /// color. Its label is 8 beside it, in the body's type.
  fn recipe() -> Recipe {
    // Not every role: the palette has a `Border` too.
    use PaletteColor::{Background, Destructive, Foreground, Input, Primary};
    use RadioPart::{Dot, Label, Mark, Root};
    Recipe::new()
      .base(Root, |s| s.gap(8.0).foreground(Foreground))
      .base(Mark, |s| {
        fading(
          s.square(16.0)
            .radius(RadiusSize::Full)
            .background(Background)
            .border(Border::inside(BorderSize::Sm, Input))
            .cursor(CursorIcon::Pointer),
        )
        .transition(PartTransition::new(Transition::border_color(), MotionDuration::Fast))
      })
      .base(Dot, |s| s.square(8.0).radius(RadiusSize::Full).background(Primary))
      .base(Label, |s| {
        s.typography(TypographyStyle::Body).cursor(CursorIcon::Pointer)
      })
      .focus_visible(Mark, |s| s.focus_ring())
      .state(RadioState::Chosen, Mark, |s| {
        s.border(Border::inside(BorderSize::Sm, Primary))
      })
      .state(RadioState::Invalid, Mark, |s| {
        s.border(Border::inside(BorderSize::Sm, Destructive))
      })
      .state(RadioState::Disabled, Root, |s| s.opacity(OpacityLevel::Disabled))
      .state(RadioState::Disabled, Mark, |s| s.cursor(CursorIcon::Default))
      .state(RadioState::Disabled, Label, |s| s.cursor(CursorIcon::Default))
  }
}

/// The parts of a group of radios a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RadioGroupPart {
  /// The group: the radios one under the other.
  Root,
  /// What the group says of each radio in it: laid over the theme's recipe for a radio's root.
  Item,
}

names!(RadioGroupPart {
  Root => "root",
  Item => "item",
});

/// The states of a group of radios a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RadioGroupState {
  /// Its radios stand side by side.
  Horizontal,
}

names!(RadioGroupState {
  Horizontal => "horizontal",
});

/// One radio of a group, with what a press of it does.
struct Item {
  label: Arc<str>,
  disabled: bool,
  chosen: bool,
  pick: Arc<dyn Fn() + Send + Sync>,
}

/// Radios that are one choice, one under the other: the one whose value the
/// app holds is chosen, and a press of another chooses it.
///
/// ```
/// use lurq::{core::Signal, ui::{Choice, RadioGroup}};
///
/// # fn groups(density: Signal<u8>, plan: Signal<&'static str>) {
/// let density = RadioGroup::new(density, [(0, "Comfortable"), (1, "Compact"), (2, "Spacious")]);
/// // A choice that cannot be made is drawn, dimmed, and takes nothing.
/// let plans = RadioGroup::new(plan, [
///   Choice::new("free", "Free"),
///   Choice::new("team", "Team"),
///   Choice::new("enterprise", "Enterprise").disabled(true),
/// ]);
/// # }
/// ```
///
/// A choice's icon is not drawn: a radio is its mark and its label.
///
/// The group is one stop of Tab, at the chosen radio or the first; the arrow
/// keys move the focus to the next radio and choose it, Home and End the
/// first and the last, and a disabled one is passed by.
pub struct RadioGroup {
  items: Vec<Item>,
  label: Option<Arc<str>>,
  horizontal: bool,
  invalid: bool,
  disabled: bool,
  rules: Recipe,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl RadioGroup {
  fn of<T: Clone + PartialEq + Send + Sync + 'static>(
    chosen: Chosen<T>,
    choices: impl IntoIterator<Item = impl Into<Choice<T>>>,
  ) -> Self {
    let items = choices
      .into_iter()
      .map(Into::into)
      .map(|choice: Choice<T>| Item {
        chosen: chosen.has(&choice.value),
        pick: Arc::new(chosen.picker(choice.value)),
        label: choice.label,
        disabled: choice.disabled,
      })
      .collect();
    Self {
      items,
      label: None,
      horizontal: false,
      invalid: false,
      disabled: false,
      rules: Recipe::new(),
      id: None,
      key: None,
    }
  }

  /// A group of which the radio whose value the signal has is chosen. A
  /// press of another sets the signal.
  pub fn new<T>(value: Signal<T>, choices: impl IntoIterator<Item = impl Into<Choice<T>>>) -> Self
  where
    T: Clone + PartialEq + Send + Sync + SignalValue + 'static,
  {
    Self::of(Chosen::one(value), choices)
  }

  /// A group whose choice the app holds: the radio of `chosen` is chosen,
  /// none if it is `None`, and `choose` is told the value of each other one
  /// pressed.
  pub fn controlled<T: Clone + PartialEq + Send + Sync + 'static>(
    chosen: Option<T>,
    choices: impl IntoIterator<Item = impl Into<Choice<T>>>,
    choose: impl Fn(T) + Send + Sync + 'static,
  ) -> Self {
    Self::of(Chosen::held(chosen.into_iter().collect(), choose), choices)
  }

  /// Whether the radios stand side by side, going on to the next line where
  /// there is no room, in place of one under the other.
  pub fn horizontal(mut self, horizontal: bool) -> Self {
    self.horizontal = horizontal;
    self
  }

  /// What the group is called, to what reads the tree. It draws nothing: a field gives it its own label.
  pub fn label(mut self, label: &str) -> Self {
    self.label = Some(Arc::from(label));
    self
  }

  /// Whether the choice was made wrongly, or not at all: every radio's edge says so.
  pub fn invalid(mut self, invalid: bool) -> Self {
    self.invalid = invalid;
    self
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
    let case = Case::new()
      .breakpoint(super::current_breakpoint())
      .state(RadioGroupState::Horizontal, self.horizontal);
    let recipe = theme.resolve::<RadioGroup>(&case, Some(&self.rules));
    let item = recipe.rules_for(RadioGroupPart::Item, RadioPart::Root);
    let radios: Vec<Element> = self
      .items
      .into_iter()
      .map(|choice| {
        let pick = choice.pick;
        Radio::new(&choice.label)
          .chosen(choice.chosen)
          .invalid(self.invalid)
          .disabled(self.disabled || choice.disabled)
          .rules(&item)
          .on_choose(move || pick())
          .build(theme)
      })
      .collect();
    // One choice: Tab stops in it once, and an arrow key moves the focus and the choice with it.
    let keys = FocusGroup::both().activate(true).one_stop(true);
    let root = recipe.part(RadioGroupPart::Root);
    let (id, key, label) = (self.id, self.key, self.label);
    macro_rules! group {
      ($holder:expr) => {{
        let mut holder = $holder
          .part(&root)
          .focus_group(keys)
          .describe("a11y_role", "radiogroup");
        if let Some(label) = label {
          holder = holder.describe("a11y_name", label);
        }
        for radio in radios {
          holder = holder.child(radio);
        }
        if let Some(id) = id {
          holder = holder.id(id);
        }
        if let Some(key) = key {
          holder = holder.key(key);
        }
        holder.into()
      }};
    }
    if self.horizontal {
      group!(Row::new().wrap().align_items(Alignment::Center))
    } else {
      group!(Column::new())
    }
  }
}

impl From<RadioGroup> for Element {
  fn from(group: RadioGroup) -> Self {
    group.build(&current_theme())
  }
}

impl Themed for RadioGroup {
  const NAME: &'static str = "radio_group";
  type Renderer = Arc<dyn Fn(RadioGroup) -> Element + Send + Sync>;

  /// The design's radios (`pencil/lurq-design-system.pen`, Toggles): one
  /// under the other, 12 apart.
  fn recipe() -> Recipe {
    Recipe::new()
      .base(RadioGroupPart::Root, |s| s.gap(12.0))
      // Side by side they are further apart than one under the other, so that a label is read with its own mark.
      .state(RadioGroupState::Horizontal, RadioGroupPart::Root, |s| s.gap(24.0))
  }
}

impl super::FieldControl for Radio {
  /// Called by the field's label where its own is hidden, since the label a person reads is then the field's;
  /// invalid where the field is, and disabled where the field is.
  fn in_field(mut self, field: &super::InField) -> Element {
    if self.label_hidden {
      self.label = field.label.clone();
    }
    self.invalid |= field.judgment == Some(super::Judgment::Invalid);
    self.disabled |= field.disabled;
    self.into()
  }
}

impl super::FieldControl for RadioGroup {
  /// Called by the field's label unless it has a name of its own, every radio invalid where the field is, and
  /// disabled where the field is.
  fn in_field(mut self, field: &super::InField) -> Element {
    if self.label.is_none() {
      self.label = Some(field.label.clone());
    }
    self.invalid |= field.judgment == Some(super::Judgment::Invalid);
    self.disabled |= field.disabled;
    self.into()
  }
}
