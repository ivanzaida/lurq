//! A time picker: a field that holds a time, typed in it or chosen from columns of hours and minutes that open under
//! it.

use std::{marker::PhantomData, sync::Arc};

use super::{
  calendar::DateMemory,
  date_field::FieldValue,
  field::{FieldControl, InField},
  icon::Icon,
  icons,
  input::{Input, InputPart, InputView},
  names,
  part_style::{Length, PartStyle},
  popover::{Popover, PopoverPart},
  recipe::{Case, Interaction, Name, Recipe, ResolvedPart, ResolvedRecipe},
  shared::{fading, pressable},
  text_box::{BoxStates, Judgment, TextBoxSize},
  theme::{Themed, current_date_locale, current_theme},
};
use crate::{
  app::{
    component::Component,
    ctx::{CollisionStrategy, Ctx, Placement},
    events::{KeyboardEvent, MouseEvent, TextInputEvent},
    theme::{IconSize, PaletteColor, RadiusSize, Theme, TypographyStyle},
  },
  components::{self, Column, Row, ScrollVertical, Stack},
  core::{DateText, DayPeriod, ElementRef, HourCycle, Signal, Time},
  layout::{Alignment, StackAlignment, layout_kind::Justify, text_style::FontWeight},
  node::{CursorIcon, Element, FocusGroup, HitTestBehavior, SpacingValue, dimension::Dimension},
};

/// The parts of a time picker a recipe styles. The field is an [`Input`] in the theme's dress; the panel a
/// [`Popover`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TimePickerPart {
  /// What the picker says of the box that is typed in: rules over the input's control.
  Control,
  /// What is pressed to open the panel, at the start of the field.
  Trigger,
  /// The timer's glyph in it.
  TriggerIcon,
  /// What the picker says of the popover's panel the columns are in: rules over its root.
  Panel,
  /// A column: the hours, the minutes, the halves of the day. Its height is what is seen of it; it scrolls.
  Column,
  /// An hour, a minute or a half of the day: what is pressed and chosen.
  Item,
}

names!(TimePickerPart {
  Control => "control",
  Trigger => "trigger",
  TriggerIcon => "trigger_icon",
  Panel => "panel",
  Column => "column",
  Item => "item",
});

/// The states of a time picker a recipe styles. The picker is in the field's states; an item in `Selected`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TimePickerState {
  /// Its panel is open.
  Open,
  Invalid,
  Warning,
  Valid,
  ReadOnly,
  Disabled,
  FullWidth,
  /// The item is the hour, the minute or the half of the day of the time held.
  Selected,
  /// It is in a fluid field ([`Field::fluid`](super::Field::fluid)): the field's label is inside its box, over
  /// the time.
  Fluid,
}

names!(TimePickerState {
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

/// The sizes of a time picker: an input's.
pub type TimePickerSize = TextBoxSize;

/// What builds a time picker's nodes: a theme's in place of the default one.
pub type TimePickerRenderer = Arc<dyn Fn(TimePickerView) -> Element + Send + Sync>;

/// A time picker: a field that holds a time, typed in it or chosen from columns of hours and minutes that open under
/// it. The time of a meeting, a reminder, a delivery slot.
///
/// ```
/// use lurq::{
///   core::{Signal, Time},
///   ui::TimePicker,
/// };
///
/// # fn render(ctx: &mut lurq::app::ctx::Ctx) {
/// let at = Signal::new(Time::new(9, 30, 0).unwrap());
/// // Mounted, so that it keeps whether its panel is open; the minutes are offered by the quarter of an hour.
/// let picker = TimePicker::new(at.clone()).step(15).mount(ctx);
/// # }
/// ```
///
/// The picker does not keep its time: the app's signal does. What is typed is read as it is typed (`9:30`, `14:05`,
/// `2:05 pm`) and is the value as soon as it is a time in the range; the focus leaving and Enter write it again as
/// the locale writes times. The panel opens from the button at the start of the field and from Down in it: a column
/// of the hours, one of the minutes [`TimePicker::step`] apart, and on a clock of twelve hours one of the halves of
/// the day. A press of an hour sets the hour; a press of a minute sets the minute and closes the panel, the focus back
/// in the field. To tooling the field is a `combobox` that opens a `dialog` of `listbox`es.
#[derive(Clone)]
pub struct TimePicker<V: FieldValue<Time> = Time> {
  value: Signal<V>,
  controlled: bool,
  boxed: BoxStates,
  full_width: bool,
  placeholder: Option<Arc<str>>,
  min: Option<Time>,
  max: Option<Time>,
  step: u32,
  hour_cycle: Option<HourCycle>,
  locale: Option<Arc<dyn DateText>>,
  open: Option<Signal<bool>>,
  memory: Option<DateMemory>,
  on_change: Vec<Arc<dyn Fn(V) + Send + Sync>>,
  on_input: Vec<Arc<dyn Fn(&TextInputEvent) + Send + Sync>>,
  on_submit: Vec<Arc<dyn Fn(V) + Send + Sync>>,
  on_open_change: Vec<Arc<dyn Fn(bool) + Send + Sync>>,
  on_dismiss: Vec<Arc<dyn Fn() + Send + Sync>>,
  placement: Placement,
  offset: Option<SpacingValue>,
  collision: Option<CollisionStrategy>,
  inline: bool,
  trigger_label: Option<Arc<str>>,
  panel_label: Option<Arc<str>>,
  // Read only where the engine has forms.
  #[cfg_attr(not(feature = "form"), allow(dead_code))]
  name: Option<Arc<str>>,
  states: Vec<Name>,
  rules: Recipe,
  forced: Option<Interaction>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl TimePicker<Time> {
  /// A picker bound to `value`: it shows the signal's time, and what is typed or chosen is set on it.
  pub fn new(value: Signal<Time>) -> Self {
    Self::with(value, false)
  }

  /// A picker that shows `value` and tells `on_change` of what is typed or chosen, which it does not set.
  pub fn controlled(value: Time, on_change: impl Fn(Time) + Send + Sync + 'static) -> Self {
    Self::with(Signal::new(value), true).on_change(on_change)
  }
}

impl TimePicker<Option<Time>> {
  /// A picker bound to `value`, which may hold no time: an emptied field holds none.
  pub fn optional(value: Signal<Option<Time>>) -> Self {
    Self::with(value, false)
  }

  /// A picker that shows `value`, which may be none, and tells `on_change` of what is typed or chosen.
  pub fn controlled_optional(value: Option<Time>, on_change: impl Fn(Option<Time>) + Send + Sync + 'static) -> Self {
    Self::with(Signal::new(value), true).on_change(on_change)
  }
}

impl<V: FieldValue<Time>> TimePicker<V> {
  fn with(value: Signal<V>, controlled: bool) -> Self {
    Self {
      value,
      controlled,
      boxed: BoxStates::default(),
      full_width: false,
      placeholder: None,
      min: None,
      max: None,
      step: 15,
      hour_cycle: None,
      locale: None,
      open: None,
      memory: None,
      on_change: Vec::new(),
      on_input: Vec::new(),
      on_submit: Vec::new(),
      on_open_change: Vec::new(),
      on_dismiss: Vec::new(),
      placement: Placement::BottomStart,
      offset: None,
      collision: None,
      inline: false,
      trigger_label: None,
      panel_label: None,
      name: None,
      states: Vec::new(),
      rules: Recipe::new(),
      forced: None,
      id: None,
      key: None,
    }
  }

  /// What the field says while it is empty.
  pub fn placeholder(mut self, placeholder: &str) -> Self {
    self.placeholder = Some(Arc::from(placeholder));
    self
  }

  /// A [`TimePickerSize`](crate::ui::TimePickerSize) (an input's), or a size the theme's recipe names.
  pub fn size(mut self, size: impl Into<Name>) -> Self {
    self.boxed.size = size.into();
    self
  }

  /// Whether what the picker holds is wrong. Of invalid, warning and valid, the first that is set is the one shown.
  pub fn invalid(mut self, invalid: bool) -> Self {
    self.boxed.invalid = invalid;
    self
  }

  pub fn warning(mut self, warning: bool) -> Self {
    self.boxed.warning = warning;
    self
  }

  pub fn valid(mut self, valid: bool) -> Self {
    self.boxed.valid = valid;
    self
  }

  /// Whether the field can be read and not changed: it opens no panel.
  pub fn read_only(mut self, read_only: bool) -> Self {
    self.boxed.read_only = read_only;
    self
  }

  pub fn disabled(mut self, disabled: bool) -> Self {
    self.boxed.disabled = disabled;
    self
  }

  /// What the field is called, to what reads the tree. A [`Field`](crate::ui::Field) around it gives it its label.
  pub fn label(mut self, label: &str) -> Self {
    self.boxed.label = Some(Arc::from(label));
    self
  }

  /// Whether the field is as wide as what holds it, in place of the design's 160.
  pub fn full_width(mut self, full_width: bool) -> Self {
    self.full_width = full_width;
    self
  }

  /// The least time that can be typed or chosen: the hours and minutes before it are not offered.
  pub fn min(mut self, min: Time) -> Self {
    self.min = Some(min);
    self
  }

  /// The greatest time that can be typed or chosen.
  pub fn max(mut self, max: Time) -> Self {
    self.max = Some(max);
    self
  }

  /// The minutes the panel offers apart: 15 as the design draws them (00, 15, 30, 45), 5, 1. What is typed may be any
  /// minute.
  pub fn step(mut self, minutes: impl Into<f64>) -> Self {
    self.step = minutes.into().round().clamp(1.0, 60.0) as u32;
    self
  }

  /// How the hours are counted, in place of the locale's: from 0 to 23, or from 1 to 12 with a column of the halves
  /// of the day.
  pub fn hour_cycle(mut self, cycle: HourCycle) -> Self {
    self.hour_cycle = Some(cycle);
    self
  }

  /// How times are written and read and what the controls are called, in place of the app's
  /// ([`App::set_date_locale`](crate::app::App::set_date_locale), English where it set none).
  pub fn locale(mut self, locale: impl DateText + 'static) -> Self {
    self.locale = Some(crate::core::shared_date_text(locale));
    self
  }

  /// Whether the panel is open, the app's: in place of what a mounted picker keeps.
  pub fn open(mut self, open: Signal<bool>) -> Self {
    self.open = Some(open);
    self
  }

  /// What keeps whether the panel is open and the field it opens under, from one render to the next: the app's, made
  /// once. A picker in a [`Field`](crate::ui::Field) keeps them so; a mounted one keeps its own.
  pub fn memory(mut self, memory: &DateMemory) -> Self {
    self.memory = Some(memory.clone());
    self
  }

  /// Called when a person changes the value, by typing or a choice, with what it holds now. Bound to a signal, the
  /// value is set already. Not called for what it holds already, nor when the app sets the value. A choice of a
  /// minute closes the panel before it is called. A second handler is called after the first.
  pub fn on_change(mut self, handler: impl Fn(V) + Send + Sync + 'static) -> Self {
    self.on_change.push(Arc::new(handler));
    self
  }

  /// Called before an edit of the field's text, which it may refuse: what is refused is not read. A second handler is
  /// called after the first.
  pub fn on_input(mut self, handler: impl Fn(&TextInputEvent) + Send + Sync + 'static) -> Self {
    self.on_input.push(Arc::new(handler));
    self
  }

  /// Called on Enter in the field, once its text is put right, with what it holds then.
  pub fn on_submit(mut self, handler: impl Fn(V) + Send + Sync + 'static) -> Self {
    self.on_submit.push(Arc::new(handler));
    self
  }

  /// Called when the panel opens or closes, with whether it is open. Not when the app sets [`Self::open`]'s signal.
  pub fn on_open_change(mut self, handler: impl Fn(bool) + Send + Sync + 'static) -> Self {
    self.on_open_change.push(Arc::new(handler));
    self
  }

  /// Called when a person closed the panel with Escape or a press outside it, not by a choice.
  pub fn on_dismiss(mut self, handler: impl Fn() + Send + Sync + 'static) -> Self {
    self.on_dismiss.push(Arc::new(handler));
    self
  }

  /// Where the panel opens beside the field: under its start by default.
  pub fn placement(mut self, placement: Placement) -> Self {
    self.placement = placement;
    self
  }

  /// How far the panel is from the field: the theme's smallest spacing by default.
  pub fn offset(mut self, offset: impl Into<SpacingValue>) -> Self {
    self.offset = Some(offset.into());
    self
  }

  /// What the panel does where there is no room for it on its side.
  pub fn collision(mut self, collision: CollisionStrategy) -> Self {
    self.collision = Some(collision);
    self
  }

  /// Draws the panel in place under the field, open or not, and not over the window: for a gallery or a test.
  pub fn inline(mut self, inline: bool) -> Self {
    self.inline = inline;
    self
  }

  /// What the button that opens the panel is called to what reads the tree: "Choose time" in English.
  pub fn trigger_label(mut self, label: &str) -> Self {
    self.trigger_label = Some(Arc::from(label));
    self
  }

  /// What the panel, a dialog, is called to what reads the tree: "Choose time" in English.
  pub fn panel_label(mut self, label: &str) -> Self {
    self.panel_label = Some(Arc::from(label));
    self
  }

  /// The name the value has in the form it is in: the field's text is what the form holds.
  #[cfg(feature = "form")]
  pub fn name(mut self, name: impl Into<Arc<str>>) -> Self {
    self.name = Some(name.into());
    self
  }

  /// Puts the picker in a state the theme's recipe names, or takes it out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// Styles one part of this picker over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(PartStyle) -> PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for this picker.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// Draws the field as it is under the pointer or focused, with nothing pointing at it: for a gallery or a test.
  pub fn force(mut self, interaction: Option<Interaction>) -> Self {
    self.forced = interaction;
    self
  }

  /// The id of the field: what is typed in and focused. The button has it with `-trigger` after it, the panel
  /// `-panel`, and each item `-hour-9`, `-minute-30`, `-period-pm`.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// Builds the picker in `theme`, with what its memory keeps where it has one. Converting a picker into an element
  /// does this in the theme of what is being rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let view = self.view(theme);
    match theme.renderer::<TimePicker>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }

  /// The picker mounted where it is declared: it keeps whether its panel is open and the field it opens under.
  pub fn mount(self, ctx: &mut Ctx) -> Element {
    let key = self.key.clone();
    let mut element = ctx.mount::<PickerHost<V>>(PickerProps { picker: self });
    if let Some(key) = key {
      element = element.key(key);
    }
    element
  }

  fn view(self, theme: &Theme) -> TimePickerView {
    let text = self.locale.clone().unwrap_or_else(current_date_locale);
    let cycle = self.hour_cycle.unwrap_or_else(|| text.hour_cycle());
    let (open, anchor) = match &self.memory {
      Some(memory) => (
        self.open.clone().unwrap_or_else(|| memory.open.clone()),
        memory.anchor.clone(),
      ),
      None => (
        self.open.clone().unwrap_or_else(|| Signal::new(false)),
        ElementRef::new(),
      ),
    };
    // Read so that what renders the picker renders it again when the app's value changes: the text is made from it.
    let value = self.value.get();
    let is_open = open.get();
    let held = value.get();
    let disabled = self.boxed.disabled;
    let fluid = self.boxed.fluid;
    let case = self
      .boxed
      .case(self.states.clone())
      .state(TimePickerState::Open, is_open)
      .state(TimePickerState::FullWidth, self.full_width);
    let recipe = theme.resolve::<TimePicker>(&case, Some(&self.rules));
    let resolve: Resolve = {
      let (theme, rules) = (theme.clone(), self.rules.clone());
      Arc::new(move |case: &Case| theme.resolve::<TimePicker>(case, Some(&rules)))
    };
    let model = Arc::new(Model {
      value: self.value.clone(),
      controlled: self.controlled,
      open: open.clone(),
      text: text.clone(),
      min: self.min,
      max: self.max,
      on_change: self.on_change.clone(),
      on_submit: self.on_submit.clone(),
      on_open_change: self.on_open_change.clone(),
      on_dismiss: self.on_dismiss.clone(),
      read_only: self.boxed.read_only,
      disabled,
    });
    let shown = Signal::new(held.map(|time| text.format_time(time, false)).unwrap_or_default());
    let mut input = Input::new(shown.clone())
      .icon(icons::TIMER)
      .full_width(self.full_width)
      .force(self.forced)
      .rules(&recipe.rules_for(TimePickerPart::Control, InputPart::Control))
      // The root is as wide as the box, which the picker's recipe sizes: as wide as the field in a fluid one.
      .rules(&Recipe::new().base(InputPart::Root, |s| {
        s.width(if fluid { Length::Fill } else { Length::Auto })
      }));
    input.boxed = self.boxed.clone();
    // The box says what it is to what reads the tree: a combobox that opens a dialog.
    input.boxed.role = Some("combobox");
    if let Some(placeholder) = &self.placeholder {
      input = input.placeholder(placeholder);
    }
    #[cfg(feature = "form")]
    if let Some(name) = &self.name {
      input = input.name(name.clone());
    }
    if let Some(id) = &self.id {
      input = input.id(id.clone());
    }
    for handler in &self.on_input {
      let handler = handler.clone();
      input = input.on_input(move |event: &TextInputEvent| handler(event));
    }
    let typed = model.clone();
    input = input.on_input(move |event: &TextInputEvent| {
      if !event.default_prevented() {
        typed.typed(event.new_value());
      }
    });
    let hours: Vec<u32> = match cycle {
      HourCycle::H23 => (0..24).collect(),
      HourCycle::H12 => (1..=12).collect(),
    };
    let minutes: Vec<u32> = (0..60).step_by(self.step.max(1) as usize).collect();
    TimePickerView {
      text: shown,
      value: held,
      open: is_open,
      cycle,
      hours,
      minutes,
      label: self.boxed.label.clone(),
      invalid: self.boxed.judgment() == Some(Judgment::Invalid),
      read_only: self.boxed.read_only,
      disabled,
      full_width: self.full_width,
      locale: text,
      recipe,
      case,
      resolve,
      input: input.view(theme),
      anchor,
      open_signal: open,
      placement: self.placement,
      offset: self.offset,
      collision: self.collision,
      inline: self.inline,
      trigger_label: self.trigger_label,
      panel_label: self.panel_label,
      model,
      id: self.id,
      key: self.key,
    }
  }
}

impl<V: FieldValue<Time>> From<TimePicker<V>> for Element {
  fn from(picker: TimePicker<V>) -> Self {
    picker.build(&current_theme())
  }
}

impl<V: FieldValue<Time>> FieldControl for TimePicker<V> {
  /// Called by the field's label unless it has a name of its own, judged as the field is unless the app judged the
  /// picker itself, and disabled where the field is.
  fn in_field(mut self, field: &InField) -> Element {
    self.boxed.in_field(field);
    self.into()
  }

  /// Told what the field says, as in `in_field`, and mounted: the same control as outside a field.
  fn mount_in_field(mut self, field: &InField, ctx: &mut Ctx) -> Element {
    self.boxed.in_field(field);
    self.mount(ctx)
  }

  fn has_fluid_form(&self) -> bool {
    true
  }
}

/// How the picker's recipe is resolved in a case of its own: an item's.
type Resolve = Arc<dyn Fn(&Case) -> ResolvedRecipe + Send + Sync>;

/// What the handlers of a time picker do.
trait TimeModel: Send + Sync {
  /// What a person typed: the value where it reads as a time in the range, none where the field is emptied.
  fn typed(&self, text: &str);
  /// Puts what `text` holds right, and gives whether it holds a time; on Enter the handlers of a submit are told.
  fn commit(&self, text: &Signal<String>, submit: bool);
  fn set_open(&self, open: bool);
  fn toggle(&self);
  fn dismissed(&self);
  /// A press of an hour (`Hour`), a minute (`Minute`) or a half of the day (`Period`) with its number.
  fn choose(&self, column: ColumnKind, number: u32, cycle: HourCycle);
  fn allowed(&self, time: Time) -> bool;
  fn held(&self) -> Option<Time>;
}

/// The columns of the panel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColumnKind {
  Hour,
  Minute,
  Period,
}

struct Model<V: FieldValue<Time>> {
  value: Signal<V>,
  controlled: bool,
  open: Signal<bool>,
  text: Arc<dyn DateText>,
  min: Option<Time>,
  max: Option<Time>,
  on_change: Vec<Arc<dyn Fn(V) + Send + Sync>>,
  on_submit: Vec<Arc<dyn Fn(V) + Send + Sync>>,
  on_open_change: Vec<Arc<dyn Fn(bool) + Send + Sync>>,
  on_dismiss: Vec<Arc<dyn Fn() + Send + Sync>>,
  read_only: bool,
  disabled: bool,
}

impl<V: FieldValue<Time>> Model<V> {
  fn write(&self, time: Option<Time>) {
    let Some(value) = V::of(time) else {
      return;
    };
    if self.value.get_untracked() == value {
      return;
    }
    if !self.controlled {
      self.value.set(value);
    }
    for handler in &self.on_change {
      handler(value);
    }
  }
}

impl<V: FieldValue<Time>> TimeModel for Model<V> {
  fn allowed(&self, time: Time) -> bool {
    self.min.is_none_or(|min| time >= min) && self.max.is_none_or(|max| time <= max)
  }

  fn held(&self) -> Option<Time> {
    self.value.get_untracked().get()
  }

  fn typed(&self, text: &str) {
    if text.trim().is_empty() {
      self.write(None);
      return;
    }
    if let Some(time) = self.text.parse_time(text).filter(|time| self.allowed(*time)) {
      self.write(Some(time));
    }
  }

  fn commit(&self, text: &Signal<String>, submit: bool) {
    let held = text.get_untracked();
    if held.trim().is_empty() {
      self.write(None);
    } else if let Some(time) = self.text.parse_time(&held).filter(|time| self.allowed(*time)) {
      self.write(Some(time));
    }
    let written = self
      .held()
      .map(|time| self.text.format_time(time, false))
      .unwrap_or_default();
    if held != written {
      text.set(written);
    }
    if submit {
      let value = self.value.get_untracked();
      for handler in &self.on_submit {
        handler(value);
      }
    }
  }

  fn set_open(&self, open: bool) {
    if open && (self.read_only || self.disabled) {
      return;
    }
    if self.open.get_untracked() == open {
      return;
    }
    self.open.set(open);
    for handler in &self.on_open_change {
      handler(open);
    }
  }

  fn toggle(&self) {
    self.set_open(!self.open.get_untracked());
  }

  fn dismissed(&self) {
    for handler in &self.on_open_change {
      handler(false);
    }
    for handler in &self.on_dismiss {
      handler();
    }
  }

  fn choose(&self, column: ColumnKind, number: u32, cycle: HourCycle) {
    let base = self.held().unwrap_or(Time::MIDNIGHT);
    let time = match (column, cycle) {
      (ColumnKind::Hour, HourCycle::H23) => Time::new(number as u8, base.minute(), 0),
      (ColumnKind::Hour, HourCycle::H12) => Time::from_hour12(number as u8, base.period(), base.minute(), 0),
      (ColumnKind::Minute, _) => Time::new(base.hour(), number as u8, 0),
      (ColumnKind::Period, _) => {
        let period = if number == 0 { DayPeriod::Am } else { DayPeriod::Pm };
        Time::from_hour12(base.hour12().0, period, base.minute(), 0)
      }
    };
    let Some(time) = time.filter(|time| self.allowed(*time)) else {
      return;
    };
    // A minute ends the choice: the panel closes first, then the handlers are told.
    if column == ColumnKind::Minute {
      self.set_open(false);
    }
    self.write(Some(time));
  }
}

/// A time picker as a renderer sees it: what it was given, the style of its parts, and its behavior ready to be
/// dressed.
pub struct TimePickerView {
  /// What the field shows, made anew in each build from the value.
  pub text: Signal<String>,
  /// The time held, where there is one.
  pub value: Option<Time>,
  /// Whether the panel is open.
  pub open: bool,
  pub cycle: HourCycle,
  /// The hours of the panel's first column, and the minutes of its second.
  pub hours: Vec<u32>,
  pub minutes: Vec<u32>,
  pub label: Option<Arc<str>>,
  pub invalid: bool,
  pub read_only: bool,
  pub disabled: bool,
  pub full_width: bool,
  /// What the picker's text is: its own `.locale(..)`, else the app's, else English.
  pub locale: Arc<dyn DateText>,
  /// The style of each part of this picker.
  pub recipe: ResolvedRecipe,
  case: Case,
  resolve: Resolve,
  input: InputView,
  anchor: ElementRef,
  open_signal: Signal<bool>,
  placement: Placement,
  offset: Option<SpacingValue>,
  collision: Option<CollisionStrategy>,
  inline: bool,
  trigger_label: Option<Arc<str>>,
  panel_label: Option<Arc<str>>,
  model: Arc<dyn TimeModel>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl TimePickerView {
  /// The style of a part, as this picker is to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  fn id_of(&self, suffix: &str) -> Option<String> {
    self.id.as_ref().map(|id| format!("{id}-{suffix}"))
  }

  fn choose_name(&self) -> Arc<str> {
    Arc::from(self.locale.choose_time_label())
  }

  /// The gap of a part's style in pixels.
  fn gap(&self, part: TimePickerPart) -> f32 {
    let spacing = self.theme().spacing();
    self.part(part).rest().gap.map_or(0.0, |gap| gap.resolve(&spacing, 0.0))
  }

  /// A length of a part's style in pixels: 0 where it has none in pixels.
  fn px(&self, length: Option<Length>) -> f32 {
    match length.and_then(|length| length.resolve(self.theme())) {
      Some(Dimension::Px(px)) => px,
      _ => 0.0,
    }
  }

  /// The field's behavior: the input's box, typed in and focused, a `combobox` to what reads the tree. Down opens the
  /// panel; Enter and the focus leaving put the text right.
  pub fn control(&self) -> components::TextInput {
    let (text, keys, blur) = (self.text.clone(), self.model.clone(), self.model.clone());
    let blur_text = self.text.clone();
    let mut control = self
      .input
      .control()
      // Each build makes the text anew: the field keeps what is half typed while it has the focus.
      .keep_text_while_focused(true)
      .describe("a11y_haspopup", "dialog")
      .describe("a11y_expanded", if self.open { "true" } else { "false" })
      .on_key_down(move |event: KeyboardEvent| {
        if !event.focus_within || event.ctrl || event.meta || event.composing {
          return;
        }
        match event.key.as_str() {
          "ArrowDown" => {
            event.prevent_default();
            keys.set_open(true);
          }
          "Enter" => keys.commit(&text, true),
          _ => {}
        }
      });
    if !self.read_only {
      control = control.on_blur(move || blur.commit(&blur_text, false));
    }
    control
  }

  /// What opens and closes the panel, at the start of the field: pressed by the pointer, and taking no focus.
  pub fn trigger(&self) -> components::Button {
    let model = self.model.clone();
    let part = self.part(TimePickerPart::Trigger);
    let glyph = self.part(TimePickerPart::TriggerIcon);
    let side = glyph.rest().width.unwrap_or(Length::Icon(IconSize::Md));
    let mut trigger = components::Button::empty()
      .justify(Justify::Center)
      .align_items(Alignment::Center)
      .part(&part)
      .focusable(false)
      .disabled(self.disabled || self.read_only)
      .describe(
        "a11y_name",
        self.trigger_label.clone().unwrap_or_else(|| self.choose_name()),
      )
      .describe("a11y_haspopup", "dialog")
      .describe("a11y_expanded", if self.open { "true" } else { "false" })
      .child(
        Row::new()
          .part(&glyph)
          .child(Icon::new(icons::TIMER).size(side).build(self.theme())),
      )
      .on_click(move |_: MouseEvent| model.toggle());
    if let Some(id) = self.id_of("trigger") {
      trigger = trigger.id(id);
    }
    trigger
  }

  /// The field: the input's box, and the button over its start where the input draws its icon. What the panel opens
  /// under. In a fluid field the button is on the line of the time, under the field's label.
  pub fn field(&self) -> Stack {
    let [_, _, _, start] = self.input.part(InputPart::Control).rest().paddings();
    let spacing = self.theme().spacing();
    let start = start.map_or(0.0, |padding| padding.resolve(&spacing, 0.0));
    let inset = ((self.px(self.part(TimePickerPart::Trigger).rest().width)
      - self.px(self.part(TimePickerPart::TriggerIcon).rest().width))
      / 2.0)
      .max(0.0);
    let mut field = Stack::new()
      .stack_align(StackAlignment::CenterStart)
      .part(&self.input.part(InputPart::Root))
      .ref_element(self.anchor.clone())
      .child(self.control())
      .child(
        Row::new()
          .padding_left((start - inset).max(0.0))
          .padding_top(self.input.value_line())
          .align_items(Alignment::Center)
          .hit_test(HitTestBehavior::ContentOnly)
          .child(self.trigger()),
      );
    if self.full_width || self.input.fluid() {
      field = field.width(Dimension::Pct(100.0));
    }
    field
  }

  /// The style of an item, in its states.
  fn item_part(&self, selected: bool) -> ResolvedPart {
    let case = self.case.clone().state(TimePickerState::Selected, selected);
    (self.resolve)(&case).part(TimePickerPart::Item)
  }

  /// An item of a column: pressed to choose it, a stop of the column's keys.
  fn item(&self, column: ColumnKind, number: u32, label: String, selected: bool, allowed: bool) -> Element {
    let part = self.item_part(selected);
    let model = self.model.clone();
    let cycle = self.cycle;
    let suffix = match column {
      ColumnKind::Hour => format!("hour-{number}"),
      ColumnKind::Minute => format!("minute-{number}"),
      ColumnKind::Period => format!("period-{}", if number == 0 { "am" } else { "pm" }),
    };
    let mut item = pressable(&part, self.disabled || !allowed)
      .align_items(Alignment::Center)
      .key(suffix.clone())
      .focus_entry(selected)
      .autofocus(selected && column == ColumnKind::Hour && !self.inline)
      .describe("a11y_role", "option")
      .describe("a11y_name", label.clone())
      .describe("a11y_selected", if selected { "true" } else { "false" })
      .child(part.text(&label).nowrap())
      .on_click(move |_: MouseEvent| model.choose(column, number, cycle));
    if let Some(id) = self.id_of(&suffix) {
      item = item.id(id);
    }
    item.into()
  }

  /// A column of items: a `listbox` called by what it holds, which scrolls, one stop of Tab whose Up and Down go from
  /// one item to the next. It opens with the item at `held` second from its top, the one before it above it.
  fn column(&self, column: ColumnKind, items: Vec<Element>, held: Option<usize>) -> Element {
    let part = self.part(TimePickerPart::Column);
    let label = self.locale.segment_label(match column {
      ColumnKind::Hour => crate::core::DateSegment::Hour,
      ColumnKind::Minute => crate::core::DateSegment::Minute,
      ColumnKind::Period => crate::core::DateSegment::Period,
    });
    let list = Column::new()
      .align_items(Alignment::Stretch)
      .spacing(self.gap(TimePickerPart::Column))
      .focus_group(FocusGroup::vertical().one_stop(true).wrap(false))
      .describe("a11y_role", "listbox")
      .describe("a11y_name", label)
      .with_children(items);
    let item = match self
      .part(TimePickerPart::Item)
      .rest()
      .height
      .and_then(|height| height.resolve(self.theme()))
    {
      Some(Dimension::Px(height)) => height,
      _ => 0.0,
    };
    let state = crate::layout::layout_kind::ScrollState::new();
    if let Some(index) = held {
      let offset = index.saturating_sub(1) as f32 * (item + self.gap(TimePickerPart::Column));
      state.set_scroll_pending(0.0, offset.max(0.0));
    }
    // The design draws no bar: one shows while the pointer is on the column, which says it scrolls.
    let mut scroll = ScrollVertical::new(list)
      .with_scroll_state(state)
      .scrollbar(crate::layout::scrollbar::ScrollBarStyle::on_hover());
    if let Some(Dimension::Px(width)) = part.rest().width.and_then(|width| width.resolve(self.theme())) {
      scroll = scroll.width(width);
    }
    if let Some(Dimension::Px(height)) = part.rest().height.and_then(|height| height.resolve(self.theme())) {
      scroll = scroll.height(height);
    }
    scroll.into()
  }

  /// The columns of the panel: the hours, the minutes, and on a clock of twelve hours the halves of the day.
  pub fn columns(&self) -> Row {
    let held = self.value;
    let model = &self.model;
    let hour_of = |time: Time| match self.cycle {
      HourCycle::H23 => time.hour() as u32,
      HourCycle::H12 => time.hour12().0 as u32,
    };
    let hours = self
      .hours
      .iter()
      .map(|hour| {
        let selected = held.is_some_and(|time| hour_of(time) == *hour);
        let allowed = (0..60).any(|minute| {
          let base = held.unwrap_or(Time::MIDNIGHT);
          let time = match self.cycle {
            HourCycle::H23 => Time::new(*hour as u8, minute, 0),
            HourCycle::H12 => Time::from_hour12(*hour as u8, base.period(), minute, 0),
          };
          time.is_some_and(|time| model.allowed(time))
        });
        self.item(ColumnKind::Hour, *hour, format!("{hour:02}"), selected, allowed)
      })
      .collect();
    let minutes = self
      .minutes
      .iter()
      .map(|minute| {
        let selected = held.is_some_and(|time| time.minute() as u32 == *minute);
        let base = held.unwrap_or(Time::MIDNIGHT);
        let allowed = Time::new(base.hour(), *minute as u8, 0).is_some_and(|time| model.allowed(time));
        self.item(ColumnKind::Minute, *minute, format!("{minute:02}"), selected, allowed)
      })
      .collect();
    let mut row = Row::new()
      .align_items(Alignment::Start)
      .spacing(self.gap(TimePickerPart::Panel))
      .child(self.column(
        ColumnKind::Hour,
        hours,
        held.and_then(|time| self.hours.iter().position(|hour| *hour == hour_of(time))),
      ))
      .child(self.column(
        ColumnKind::Minute,
        minutes,
        held.and_then(|time| self.minutes.iter().position(|minute| *minute == time.minute() as u32)),
      ));
    if self.cycle == HourCycle::H12 {
      let periods = [DayPeriod::Am, DayPeriod::Pm]
        .into_iter()
        .map(|period| {
          let selected = held.is_some_and(|time| time.period() == period);
          self.item(
            ColumnKind::Period,
            period as u32,
            self.locale.day_period_name(period),
            selected,
            true,
          )
        })
        .collect();
      row = row.child(self.column(ColumnKind::Period, periods, None));
    }
    row
  }

  /// The panel: a popover under the field (where the picker's placement says) holding the columns. Escape and a press
  /// outside close it and give the focus back to the field. Inline, it is the popover's panel drawn in place, open or
  /// not.
  pub fn panel(&self) -> Element {
    let rules = self.recipe.rules_for(TimePickerPart::Panel, PopoverPart::Panel);
    let name = self.panel_label.clone().unwrap_or_else(|| self.choose_name());
    if self.inline {
      let case = Case {
        breakpoint: super::current_breakpoint(),
        ..Case::default()
      };
      let recipe = self.theme().resolve::<Popover>(&case, Some(&rules));
      let mut panel = Column::new()
        .align_items(Alignment::Stretch)
        .part(&recipe.part(PopoverPart::Panel))
        .describe("a11y_role", "dialog")
        .describe("a11y_name", name)
        .child(
          Column::new()
            .align_items(Alignment::Stretch)
            .part(&recipe.part(PopoverPart::Body))
            .child(self.columns()),
        );
      if let Some(id) = self.id_of("panel") {
        panel = panel.id(id);
      }
      return panel.into();
    }
    let model = self.model.clone();
    let mut popover = Popover::new(self.open_signal.clone(), self.anchor.clone())
      .placement(self.placement)
      .label(&name)
      .rules(&rules)
      .on_dismiss(move || model.dismissed())
      .child(self.columns());
    if let Some(offset) = self.offset {
      popover = popover.offset(offset);
    }
    if let Some(collision) = self.collision {
      popover = popover.collision(collision);
    }
    if let Some(id) = self.id_of("panel") {
      popover = popover.id(id);
    }
    popover.into()
  }

  /// How the default renderer builds a picker: the field, and the panel that opens under it.
  pub fn render(self) -> Element {
    if self.inline {
      let gap = self
        .offset
        .unwrap_or_else(|| crate::app::theme::SpacingSize::Xs.into())
        .resolve(&self.theme().spacing(), 0.0);
      let mut root = Column::new()
        .max_width(Dimension::Pct(100.0))
        .spacing(gap)
        .child(self.field())
        .child(self.panel());
      if self.input.fluid() {
        root = root.width(Dimension::Pct(100.0));
      }
      if let Some(key) = &self.key {
        root = root.key(key.clone());
      }
      return root.into();
    }
    // No wider than the room it has: the field in it is as wide as its recipe says at most.
    let mut root = Row::new()
      .max_width(Dimension::Pct(100.0))
      .child(self.field())
      .child(self.panel());
    if self.full_width || self.input.fluid() {
      root = root.width(Dimension::Pct(100.0));
    }
    if let Some(key) = &self.key {
      root = root.key(key.clone());
    }
    root.into()
  }
}

impl<V: FieldValue<Time>> crate::ui::Styled for TimePicker<V> {
  type Component = Self;

  fn rules(self, rules: &crate::ui::Recipe) -> Self {
    Self::rules(self, rules)
  }

  fn state(self, state: impl Into<crate::ui::Name>, on: bool) -> Self {
    Self::state(self, state, on)
  }
}

impl<V: FieldValue<Time>> Themed for TimePicker<V> {
  const NAME: &'static str = "time_picker";
  type Renderer = TimePickerRenderer;

  fn parts() -> crate::ui::Declared {
    TimePickerPart::declared()
  }

  fn states() -> crate::ui::Declared {
    TimePickerState::declared()
  }

  /// The design's time picker (`pencil/lurq-design-system.pen`, Time Picker): an input 160 by 36 with a timer's glyph
  /// of 16 in the muted foreground at its start, 12 from its edge, and the time 8 after it in the mono type at 14. The
  /// panel opens 4 under it, 160 wide in the popover's colors, the large radius, 4 of padding and 4 between its
  /// columns: the hours and the minutes, 15 apart, items 30 high and 2 apart with the small radius, their numbers in
  /// the mono type at 13; the held one on the highlight at medium weight. Four items are seen of a column, which
  /// scrolls. In a fluid field the box is the input's fluid one, as wide as the field, with the button on the line
  /// of the time.
  fn recipe() -> Recipe {
    use PaletteColor::{Foreground, Highlight, HighlightForeground, MutedForeground};
    use TimePickerPart::{Column as Lane, Control, Item, Panel, Trigger, TriggerIcon};
    let none = crate::node::color::Color::new(0, 0, 0, 0);
    Recipe::new()
      .base(Control, |s| {
        s.width(160.0).typography(TypographyStyle::Mono).font_size(14.0)
      })
      .state(TimePickerState::FullWidth, Control, |s| s.width(Length::Fill))
      // A line of the mono type is higher than one of the body's: the box is 56, and not as high as what it holds.
      .state(TimePickerState::Fluid, Control, |s| {
        s.width(Length::Fill).height(super::text_box::FLUID_HEIGHT)
      })
      .base(Trigger, |s| {
        fading(
          s.square(24.0)
            .radius(RadiusSize::Sm)
            .foreground(MutedForeground)
            .cursor(CursorIcon::Pointer),
        )
      })
      .hovered(Trigger, |s| s.background(Highlight))
      .base(TriggerIcon, |s| s.square(IconSize::Md))
      .base(Panel, |s| s.width(Length::Auto).padding_x(4.0).padding_y(4.0).gap(4.0))
      .base(Lane, |s| s.width(74.0).height(126.0).gap(2.0))
      .base(Item, |s| {
        fading(
          s.height(30.0)
            .radius(RadiusSize::Sm)
            .typography(TypographyStyle::Mono)
            .font_size(13.0)
            .weight(FontWeight::Normal)
            .foreground(Foreground)
            .background(none)
            .cursor(CursorIcon::Pointer),
        )
      })
      .hovered(Item, |s| s.background(Highlight).foreground(HighlightForeground))
      .focus_visible(Item, |s| s.focus_ring())
      .state(TimePickerState::Selected, Item, |s| {
        s.background(Highlight)
          .foreground(HighlightForeground)
          .weight(FontWeight::Medium)
      })
      .state(TimePickerState::Disabled, Trigger, |s| s.cursor(CursorIcon::Default))
  }
}

#[derive(Clone)]
struct PickerProps<V: FieldValue<Time>> {
  picker: TimePicker<V>,
}

/// The picker is built anew by every render of what holds it, so props are never the same as before.
impl<V: FieldValue<Time>> PartialEq for PickerProps<V> {
  fn eq(&self, _: &Self) -> bool {
    false
  }
}

impl<V: FieldValue<Time>> crate::app::component::DevtoolsInspectable for PickerProps<V> {}

/// What a mounted picker keeps between renders.
struct PickerHost<V: FieldValue<Time>> {
  memory: DateMemory,
  value: PhantomData<fn() -> V>,
}

impl<V: FieldValue<Time>> Component for PickerHost<V> {
  type Props = PickerProps<V>;

  fn create(_ctx: &mut Ctx) -> Self {
    Self {
      memory: DateMemory::new(),
      value: PhantomData,
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let mut picker = ctx.props::<PickerProps<V>>().picker.clone();
    if picker.memory.is_none() {
      picker.memory = Some(self.memory.clone());
    }
    picker.build(&current_theme())
  }
}
