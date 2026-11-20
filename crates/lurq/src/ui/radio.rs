//! A radio: one of a few choices that exclude each other, and the group that is the choice.

use std::sync::Arc;

use super::{
  names,
  part_style::PartTransition,
  recipe::{Case, Interaction, Name, Recipe, ResolvedPart, ResolvedRecipe, Selector},
  shared::{Choice, Orientation, fading, pressable},
  theme::{Themed, current_theme},
};
use crate::{
  animation::Transition,
  app::events::MouseEvent,
  app::theme::{BorderSize, MotionDuration, OpacityLevel, PaletteColor, RadiusSize, Theme, TypographyStyle},
  components::{self, Column, Row},
  core::{ElementRef, Signal, SignalValue},
  layout::{Alignment, layout_kind::Justify},
  node::{CursorIcon, Element, FocusGroup, border::Border},
};

/// The parts of a radio a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RadioPart {
  /// The radio and its label, side by side.
  Root,
  /// What is pressed: the round mark with its edge.
  Mark,
  /// What is in the mark while the radio is selected.
  Dot,
  /// Its text, beside the mark.
  Label,
  /// The number after the label of a radio that has one: how many things the choice holds.
  Count,
}

names!(RadioPart {
  Root => "root",
  Mark => "mark",
  Dot => "dot",
  Label => "label",
  Count => "count",
});

/// The states of a radio a recipe styles, besides what the pointer and the
/// keyboard do to it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RadioState {
  /// It is the one of several that is selected.
  Selected,
  /// What it is part of was answered wrongly, or not at all.
  Invalid,
  /// It takes no input.
  Disabled,
  /// It is pressed as a whole: its root is what is pressed and focused, with the mark and the label in it.
  Whole,
}

names!(RadioState {
  Selected => "selected",
  Invalid => "invalid",
  Disabled => "disabled",
  Whole => "whole",
});

/// What builds a radio's nodes: a theme's in place of the default one.
pub type RadioRenderer = Arc<dyn Fn(RadioView) -> Element + Send + Sync>;

/// A radio: one of a few choices of which exactly one is made. The density
/// of a list, the plan of an account, the size of a print.
///
/// ```
/// use lurq::ui::Radio;
///
/// let compact = Radio::new("Compact").selected(true);
/// let spacious = Radio::new("Spacious").on_change(|selected| println!("spacious: {selected}"));
/// ```
///
/// A radio does not keep whether it is selected: the app says so with
/// [`Radio::selected`], and is told with [`Radio::on_change`], given `true`,
/// that it was pressed while it was not. A press of one that is selected
/// already does nothing: a radio is not unselected by a press. A press of its
/// label is a press of the radio, and a press of either is the radio's alone:
/// a row around it that is pressed is not. [`Radio::whole`] makes the mark and
/// its label one control, its id on the whole. Radios that are one choice are
/// a [`RadioGroup`].
pub struct Radio {
  label: Arc<str>,
  label_hidden: bool,
  whole: bool,
  count: Option<Arc<str>>,
  element: Option<ElementRef>,
  selected: bool,
  invalid: bool,
  disabled: bool,
  states: Vec<Name>,
  rules: Recipe,
  forced: Option<Interaction>,
  on_change: Vec<Arc<dyn Fn(bool) + Send + Sync>>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl Radio {
  /// A radio called `label`, with the label beside it.
  pub fn new(label: &str) -> Self {
    Self {
      label: Arc::from(label),
      label_hidden: false,
      whole: false,
      count: None,
      element: None,
      selected: false,
      invalid: false,
      disabled: false,
      states: Vec::new(),
      rules: Recipe::new(),
      forced: None,
      on_change: Vec::new(),
      id: None,
      key: None,
    }
  }

  /// Whether the radio is its mark alone. Its label stays what it is called.
  pub fn label_hidden(mut self, hidden: bool) -> Self {
    self.label_hidden = hidden;
    self
  }

  /// Whether the mark and its label are one control: what is pressed,
  /// focused and ringed is the whole of them, one stop of Tab, and its id is
  /// on the whole, the mark, the dot, the label and the count named after it
  /// (`x-mark`, `x-dot`, `x-label`, `x-count`), the element ref referring to
  /// the whole. Not unless told: the mark is what is pressed, and has the id.
  pub fn whole(mut self, whole: bool) -> Self {
    self.whole = whole;
    self
  }

  /// A number after the label: how many things the choice holds. "24", "99+".
  pub fn count(mut self, count: impl std::fmt::Display) -> Self {
    self.count = Some(Arc::from(count.to_string()));
    self
  }

  /// Refers to the mark, what is pressed, once it is laid out: what a popup the radio opens hangs from.
  pub fn element_ref(mut self, element: ElementRef) -> Self {
    self.element = Some(element);
    self
  }

  /// Whether the radio is the one of several that is selected.
  pub fn selected(mut self, selected: bool) -> Self {
    self.selected = selected;
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

  /// Called on a press of the radio while it is not selected, with what it is to be: selected, `true`. A press of
  /// the one selected calls nothing. A second handler is called after the first.
  pub fn on_change(mut self, handler: impl Fn(bool) + Send + Sync + 'static) -> Self {
    self.on_change.push(Arc::new(handler));
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

  /// The id of the mark: what is pressed. The id of the whole where it is
  /// [`whole`](Self::whole).
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  /// What tells the radio apart from those beside it, where two have one label.
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
    .state(RadioState::Selected, self.selected)
    .state(RadioState::Invalid, self.invalid)
    .state(RadioState::Disabled, self.disabled)
    .state(RadioState::Whole, self.whole);
    let view = RadioView {
      label: self.label,
      label_hidden: self.label_hidden,
      whole: self.whole,
      count: self.count,
      element: self.element,
      selected: self.selected,
      invalid: self.invalid,
      disabled: self.disabled,
      recipe: theme
        .resolve::<Radio>(&case, Some(&self.rules))
        .owned_by(self.id.as_ref()),
      forced: self.forced,
      on_change: self.on_change,
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
  /// Whether the mark and its label are one control, its id on the whole.
  pub whole: bool,
  /// The number after the label, where the radio has one.
  pub count: Option<Arc<str>>,
  element: Option<ElementRef>,
  pub selected: bool,
  pub invalid: bool,
  pub disabled: bool,
  /// The style of each part of this radio.
  pub recipe: ResolvedRecipe,
  forced: Option<Interaction>,
  on_change: Vec<Arc<dyn Fn(bool) + Send + Sync>>,
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

  /// What a press does: every handler is told the radio is to be selected, unless it is already.
  fn press(&self) -> Option<impl Fn() + Send + Sync + Clone + 'static> {
    let handlers = self.on_change.clone();
    (!handlers.is_empty() && !self.disabled && !self.selected).then(|| {
      move || {
        for handler in &handlers {
          handler(true);
        }
      }
    })
  }

  /// The radio's behavior with the style of its mark: the element that is
  /// pressed, focused and disabled, with nothing in it yet. It is a radio to
  /// what reads the tree, called by its label, and says whether it is selected.
  pub fn mark(&self) -> components::Button {
    self.control(RadioPart::Mark)
  }

  /// The behavior of a radio pressed as a whole, with the style of its root:
  /// what is pressed, focused and ringed, with nothing in it yet, the mark and
  /// the label to go in it. What [`mark`](Self::mark) is otherwise.
  pub fn whole_root(&self) -> components::Button {
    self.control(RadioPart::Root).justify(Justify::Start)
  }

  /// The radio's behavior with the style of `part`, its id, its key and its
  /// element ref. A press of it is its own: it does not reach what is around it.
  fn control(&self, part: RadioPart) -> components::Button {
    let mut control = pressable(&self.part(part), self.disabled)
      .align_items(Alignment::Center)
      .describe("a11y_role", "radio")
      .describe("a11y_name", self.label.clone())
      .describe("a11y_checked", if self.selected { "true" } else { "false" })
      // In a group, which Tab stops at once, the selected radio is where Tab enters.
      .focus_entry(self.selected)
      .on_click(own_press(self.press()));
    if let Some(id) = &self.id {
      control = control.id(id.clone());
    }
    if let Some(key) = &self.key {
      control = control.key(key.clone());
    }
    if let Some(element) = &self.element {
      control = control.ref_element(element.clone());
    }
    control
  }

  /// The label in the style of its part: `None` where it is hidden.
  fn label(&self) -> Option<Row> {
    if self.label_hidden {
      return None;
    }
    let part = self.part(RadioPart::Label);
    Some(Row::new().part(&part).child(part.text(&self.label).nowrap()))
  }

  /// The count after the label, in the style of its part: `None` where it has none.
  fn count(&self) -> Option<Row> {
    let count = self.count.as_ref()?;
    let part = self.part(RadioPart::Count);
    Some(Row::new().part(&part).child(part.text(count).nowrap()))
  }

  /// The dot in the style of its part.
  pub fn dot(&self) -> Row {
    Row::new().part(&self.part(RadioPart::Dot))
  }

  /// How the default renderer builds a radio: the mark, with the dot in it
  /// while the radio is selected, and the label beside it; pressed as a whole,
  /// all of them in what is pressed.
  pub fn render(self) -> Element {
    let dot = self.selected.then(|| self.dot());
    if self.whole {
      let mark = Row::new()
        .align_items(Alignment::Center)
        .justify(Justify::Center)
        .part(&self.part(RadioPart::Mark))
        .with_children(dot);
      return self
        .whole_root()
        .child(mark)
        .with_children(self.label())
        .with_children(self.count())
        .into();
    }
    let mark = self.mark().with_children(dot);
    let mut root = Row::new()
      .align_items(Alignment::Center)
      .part(&self.part(RadioPart::Root))
      .child(mark);
    if let Some(label) = self.label() {
      root = root.child(label.on_click(own_press(self.press())));
    }
    root.with_children(self.count()).into()
  }
}

/// What a press of a radio or its label does: `press`, where it has one, and nothing more. It is its own, and does
/// not reach what is around it (a row that is pressed).
fn own_press(press: Option<impl Fn() + Send + Sync + 'static>) -> impl Fn(MouseEvent) + Send + Sync + 'static {
  move |event: MouseEvent| {
    event.stop_propagation();
    if let Some(press) = &press {
      press();
    }
  }
}

impl crate::ui::Styled for Radio {
  type Component = Self;

  fn rules(self, rules: &crate::ui::Recipe) -> Self {
    Self::rules(self, rules)
  }

  fn state(self, state: impl Into<crate::ui::Name>, on: bool) -> Self {
    Self::state(self, state, on)
  }
}

impl Themed for Radio {
  const NAME: &'static str = "radio";
  type Renderer = RadioRenderer;

  fn parts() -> crate::ui::Declared {
    RadioPart::declared()
  }

  fn states() -> crate::ui::Declared {
    RadioState::declared()
  }

  /// The design's radio (`pencil/lurq-design-system.pen`, Radio): a round
  /// mark of 16 in the background's color, with an edge of 1 inside it in the
  /// color of an input's edge; selected, the edge is in the primary color and a
  /// dot of 8 in it is too. The edge of an invalid one is in the destructive
  /// color. Its label is 8 beside it, in the body's type.
  fn recipe() -> Recipe {
    // Not every role: the palette has a `Border` too.
    use PaletteColor::{Background, Destructive, Foreground, Input, MutedForeground, Primary};
    use RadioPart::{Count, Dot, Label, Mark, Root};
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
      // A count is quieter than the label it follows.
      .base(Count, |s| s.typography(TypographyStyle::Caption).foreground(MutedForeground))
      .focus_visible(Mark, |s| s.focus_ring())
      .state(RadioState::Selected, Mark, |s| {
        s.border(Border::inside(BorderSize::Sm, Primary))
      })
      .state(RadioState::Invalid, Mark, |s| {
        s.border(Border::inside(BorderSize::Sm, Destructive))
      })
      .state(RadioState::Disabled, Root, |s| s.opacity(OpacityLevel::Disabled))
      .state(RadioState::Disabled, Mark, |s| s.cursor(CursorIcon::Default))
      .state(RadioState::Disabled, Label, |s| s.cursor(CursorIcon::Default))
      // Pressed as a whole: the root is what the pointer presses and the ring goes round, the mark only drawn.
      .state(RadioState::Whole, Root, |s| s.radius(RadiusSize::Sm).cursor(CursorIcon::Pointer))
      .when(Selector::new().state(RadioState::Whole).focus_visible(), Root, |s| s.focus_ring())
      .when(Selector::new().state(RadioState::Whole).state(RadioState::Disabled), Root, |s| {
        s.cursor(CursorIcon::Default)
      })
  }
}

/// The parts of a group of radios a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RadioGroupPart {
  /// The group: the radios one under the other, or side by side.
  Root,
  /// What the group says of each radio in it: laid over the theme's recipe for a radio's root.
  Item,
}

names!(RadioGroupPart {
  Root => "root",
  Item => "item",
});

/// The states of a group of radios a recipe styles. One of `Horizontal` and `Vertical` is on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RadioGroupState {
  /// Its radios stand side by side.
  Horizontal,
  /// Its radios stand one under the other.
  Vertical,
}

names!(RadioGroupState {
  Horizontal => "horizontal",
  Vertical => "vertical",
});

/// What a group of radios or of radio cards is bound to: a `Signal<T>` of the value selected, or a
/// `Signal<Option<T>>` where that nothing is selected is a state it has (a question of a form before it is
/// answered). A press of another sets the signal.
pub struct RadioSelection<T> {
  pub(crate) selected: Option<T>,
  pub(crate) set: Arc<dyn Fn(T) + Send + Sync>,
}

impl<T: Clone + Send + Sync + SignalValue + 'static> From<Signal<T>> for RadioSelection<T> {
  fn from(signal: Signal<T>) -> Self {
    Self {
      selected: Some(signal.get()),
      set: Arc::new(move |value| signal.set(value)),
    }
  }
}

impl<T: Clone + Send + Sync + 'static> From<Signal<Option<T>>> for RadioSelection<T>
where
  Option<T>: SignalValue,
{
  fn from(signal: Signal<Option<T>>) -> Self {
    Self {
      selected: signal.get(),
      set: Arc::new(move |value| signal.set(Some(value))),
    }
  }
}

/// What a press of a choice of a group of one does, where it is not the one selected: the signal is set where the
/// group is bound to one, and then every handler is told the value.
pub(crate) fn selecting<T: Clone + Send + Sync + 'static>(
  value: T,
  set: Option<Arc<dyn Fn(T) + Send + Sync>>,
  handlers: Vec<Arc<dyn Fn(&T) + Send + Sync>>,
) -> impl Fn() + Send + Sync + 'static {
  move || {
    if let Some(set) = &set {
      set(value.clone());
    }
    for handler in &handlers {
      handler(&value);
    }
  }
}

/// What builds a group of radios' nodes: a theme's in place of the default one.
pub type RadioGroupRenderer = Arc<dyn Fn(RadioGroupView) -> Element + Send + Sync>;

/// Radios that are one choice, one under the other: the one whose value the
/// app holds is selected, and a press of another selects it.
///
/// ```
/// use lurq::{core::Signal, ui::{Choice, Orientation, RadioGroup}};
///
/// # fn groups(density: Signal<u8>, plan: Signal<Option<&'static str>>) {
/// let density = RadioGroup::new(density, [(0, "Comfortable"), (1, "Compact"), (2, "Spacious")])
///   .orientation(Orientation::Horizontal);
/// // Nothing is selected until a plan is: a signal of an `Option`. A choice that cannot be made is drawn, dimmed,
/// // and takes nothing.
/// let plans = RadioGroup::new(plan, [
///   Choice::new("free", "Free"),
///   Choice::new("team", "Team"),
///   Choice::new("enterprise", "Enterprise").disabled(true),
/// ])
/// .on_change(|plan| println!("{plan}"));
/// // The app holds it, and answers a press.
/// let held = RadioGroup::controlled(Some(1), [(0, "Day"), (1, "Week")], |period| println!("{period}"));
/// # }
/// ```
///
/// A choice's icon is not drawn: a radio is its mark and its label, and the count after it where the choice has one.
/// A choice's id is its radio's (its mark's), after which the radio's parts are named; its states are the radio's,
/// and its element ref refers to the mark.
///
/// The group is one stop of Tab, at the selected radio or the first; the arrow
/// keys move the focus to the next radio and select it, Home and End the
/// first and the last, and a disabled one is passed by.
pub struct RadioGroup<T = ()> {
  choices: Vec<Choice<T>>,
  selected: Option<T>,
  set: Option<Arc<dyn Fn(T) + Send + Sync>>,
  on_change: Vec<Arc<dyn Fn(&T) + Send + Sync>>,
  label: Option<Arc<str>>,
  orientation: Orientation,
  invalid: bool,
  disabled: bool,
  whole: bool,
  states: Vec<Name>,
  rules: Recipe,
  forced: Option<Interaction>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl<T: Clone + PartialEq + Send + Sync + 'static> RadioGroup<T> {
  fn of(
    selected: Option<T>,
    set: Option<Arc<dyn Fn(T) + Send + Sync>>,
    choices: impl IntoIterator<Item = impl Into<Choice<T>>>,
  ) -> Self {
    Self {
      choices: choices.into_iter().map(Into::into).collect(),
      selected,
      set,
      on_change: Vec::new(),
      label: None,
      orientation: Orientation::Vertical,
      invalid: false,
      disabled: false,
      whole: false,
      states: Vec::new(),
      rules: Recipe::new(),
      forced: None,
      id: None,
      key: None,
    }
  }

  /// A group of which the radio whose value the signal has is selected: a `Signal<T>`, or a `Signal<Option<T>>`
  /// of which `None` selects none. A press of another sets the signal.
  pub fn new(selected: impl Into<RadioSelection<T>>, choices: impl IntoIterator<Item = impl Into<Choice<T>>>) -> Self {
    let selected = selected.into();
    Self::of(selected.selected, Some(selected.set), choices)
  }

  /// A group whose choice the app holds: the radio of `selected` is selected,
  /// none if it is `None`, and `on_change` is told the value of each other one
  /// pressed. What is drawn is what the app says next.
  pub fn controlled(
    selected: Option<T>,
    choices: impl IntoIterator<Item = impl Into<Choice<T>>>,
    on_change: impl Fn(&T) + Send + Sync + 'static,
  ) -> Self {
    Self::of(selected, None, choices).on_change(on_change)
  }

  /// Called on a press of a radio that is not the one selected, with its value, after the signal the group is
  /// bound to is set. A press of the one selected calls nothing; that the app sets the signal calls nothing. A
  /// second handler is called after the first.
  pub fn on_change(mut self, handler: impl Fn(&T) + Send + Sync + 'static) -> Self {
    self.on_change.push(Arc::new(handler));
    self
  }

  /// Which way the radios stand: one under the other, unless told. Side by side they go on to the next line where
  /// there is no room.
  pub fn orientation(mut self, orientation: Orientation) -> Self {
    self.orientation = orientation;
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

  /// Whether each radio is pressed as a whole, its mark and its label one control with a choice's id on it
  /// ([`Radio::whole`]). The group is one stop of Tab still, and the arrow keys move between the wholes.
  pub fn whole(mut self, whole: bool) -> Self {
    self.whole = whole;
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

  /// Lays rules over the theme's recipe for this group: what the component it is in says of it in this place.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// Draws the radio Tab would stop on (the selected one, else the first that is not disabled) as it is under the
  /// pointer, held down or focused from the keyboard, with nothing pointing at it: for a gallery or a test. The
  /// other radios are drawn as they are.
  pub fn force(mut self, interaction: Option<Interaction>) -> Self {
    self.forced = interaction;
    self
  }

  /// The id of the group.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  /// What tells the group apart from those beside it.
  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// Builds the group in `theme`. Converting a group into an element does
  /// this in the theme of what is being rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let case = Case {
      variant: None,
      size: None,
      density: None,
      breakpoint: super::current_breakpoint(),
      states: self.states.clone(),
    }
    .state(RadioGroupState::Horizontal, !self.orientation.is_vertical())
    .state(RadioGroupState::Vertical, self.orientation.is_vertical());
    let recipe = theme
      .resolve::<Self>(&case, Some(&self.rules))
      .owned_by(self.id.as_ref());
    let item = recipe.rules_for(RadioGroupPart::Item, RadioPart::Root);
    let takes = |choice: &Choice<T>| !self.disabled && !choice.disabled;
    // What Tab stops on: the selected radio, else the first that takes a press.
    let entry = self
      .choices
      .iter()
      .position(|choice| self.selected.as_ref() == Some(&choice.value) && takes(choice))
      .or_else(|| self.choices.iter().position(takes));
    let radios = self
      .choices
      .iter()
      .enumerate()
      .map(|(index, choice)| {
        let selected = self.selected.as_ref() == Some(&choice.value);
        let mut radio = Radio::new(&choice.label)
          .selected(selected)
          .invalid(self.invalid)
          .disabled(self.disabled || choice.disabled)
          .whole(self.whole)
          .rules(&item);
        if let Some(id) = &choice.id {
          radio = radio.id(id.clone());
        }
        if let Some(count) = &choice.count {
          radio = radio.count(count);
        }
        if let Some(element) = &choice.element {
          radio = radio.element_ref(element.clone());
        }
        for state in &choice.states {
          radio = radio.state(*state, true);
        }
        if Some(index) == entry {
          radio = radio.force(self.forced);
        }
        if !selected {
          let press = selecting(choice.value.clone(), self.set.clone(), self.on_change.clone());
          radio = radio.on_change(move |_| press());
        }
        radio.build(theme)
      })
      .collect();
    let view = RadioGroupView {
      label: self.label,
      orientation: self.orientation,
      invalid: self.invalid,
      disabled: self.disabled,
      recipe,
      radios,
      id: self.id,
      key: self.key,
    };
    match theme.renderer::<Self>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }
}

impl<T: Clone + PartialEq + Send + Sync + 'static> From<RadioGroup<T>> for Element {
  fn from(group: RadioGroup<T>) -> Self {
    group.build(&current_theme())
  }
}

/// A group of radios as a renderer sees it: what it was given, the style of its parts, and its radios, built with
/// what the group says of each and what a press of each does.
pub struct RadioGroupView {
  pub label: Option<Arc<str>>,
  pub orientation: Orientation,
  pub invalid: bool,
  pub disabled: bool,
  /// The style of each part of this group.
  pub recipe: ResolvedRecipe,
  /// Its radios, in order.
  pub radios: Vec<Element>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl RadioGroupView {
  /// The style of a part, as this group is to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// How the default renderer builds a group: its radios one under the other, or side by side and going on to the
  /// next line, as one stop of Tab whose arrow keys move the focus and the choice with it. It is a radio group to
  /// what reads the tree, called by its label.
  pub fn render(self) -> Element {
    // One choice: Tab stops in it once, and an arrow key moves the focus and the choice with it.
    let keys = FocusGroup::both().activate(true).one_stop(true);
    let root = self.part(RadioGroupPart::Root);
    let (id, key, label, radios) = (self.id, self.key, self.label, self.radios);
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
    if self.orientation.is_vertical() {
      group!(Column::new())
    } else {
      group!(Row::new().wrap().align_items(Alignment::Center))
    }
  }
}

impl<T: Clone + PartialEq + Send + Sync + 'static> crate::ui::Styled for RadioGroup<T> {
  type Component = Self;

  fn rules(self, rules: &crate::ui::Recipe) -> Self {
    Self::rules(self, rules)
  }

  fn state(self, state: impl Into<crate::ui::Name>, on: bool) -> Self {
    Self::state(self, state, on)
  }
}

impl<T: Send + Sync + 'static> Themed for RadioGroup<T> {
  const NAME: &'static str = "radio_group";
  type Renderer = RadioGroupRenderer;

  fn parts() -> crate::ui::Declared {
    RadioGroupPart::declared()
  }

  fn states() -> crate::ui::Declared {
    RadioGroupState::declared()
  }

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
    if self.label_hidden && !field.label.is_empty() {
      self.label = field.label.clone();
    }
    self.invalid |= field.judgment == Some(super::Judgment::Invalid);
    self.disabled |= field.disabled;
    self.into()
  }
}

impl<T: Clone + PartialEq + Send + Sync + 'static> super::FieldControl for RadioGroup<T> {
  /// Called by the field's label unless it has a name of its own, every radio invalid where the field is, and
  /// disabled where the field is.
  fn in_field(mut self, field: &super::InField) -> Element {
    if self.label.is_none() && !field.label.is_empty() {
      self.label = Some(field.label.clone());
    }
    self.invalid |= field.judgment == Some(super::Judgment::Invalid);
    self.disabled |= field.disabled;
    self.into()
  }
}
