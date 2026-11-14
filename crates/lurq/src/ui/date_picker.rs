//! A date picker: a field that holds a date, typed or chosen from a calendar that opens under it.

use std::sync::Arc;

use super::{
  button::{Button, ButtonPart, ButtonSize, ButtonVariant},
  calendar::{Calendar, CalendarChoice, CalendarMemory, Kept, MonthSource, Selection, app_locale},
  field::{FieldControl, InField},
  icon::Icon,
  icons,
  input::{Input, InputPart, InputView},
  names,
  part_style::{Length, PartStyle},
  popover::{Popover, PopoverPart},
  recipe::{Case, Interaction, Name, Recipe, ResolvedPart, ResolvedRecipe},
  separator::{Separator, SeparatorOrientation, SeparatorPart},
  shared::fading,
  text_box::{BoxStates, Judgment, TextBoxSize},
  theme::{Themed, current_theme},
};
use crate::{
  app::{
    component::Component,
    ctx::{CollisionStrategy, Ctx, Placement},
    events::{KeyboardEvent, MouseEvent, TextInputEvent},
    theme::{IconSize, PaletteColor, RadiusSize, SpacingSize, Theme},
  },
  components::{self, Column, Row, Stack},
  core::{Date, DateLocale, DateRange, ElementRef, Signal, Weekday, YearMonth},
  layout::{Alignment, StackAlignment, layout_kind::Justify},
  node::{CursorIcon, Element, HitTestBehavior, SpacingValue, dimension::Dimension},
};

/// The parts of a date picker, and of a date range picker, a recipe styles. The field is an [`Input`] in the theme's
/// dress for one; the panel a [`Popover`]; the days a [`Calendar`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DatePickerPart {
  /// What the picker says of the box that is typed in: rules over the input's control.
  Control,
  /// What is pressed to open the calendar, at the start of the field: its size, its corners, its fill under the
  /// pointer.
  Trigger,
  /// The calendar's glyph in it.
  TriggerIcon,
  /// What is pressed to empty the field, at its end, where the picker is clearable.
  Clear,
  /// The glyph in it.
  ClearIcon,
  /// What the picker says of the popover's panel the calendar is in: rules over its `Panel` part.
  Panel,
  /// The column of a range picker's presets, beside the calendar.
  Presets,
  /// What the picker says of a preset's button: rules over its root.
  Preset,
  /// What the picker says of the line between the presets and the calendar: rules over the separator's line.
  Separator,
}

names!(DatePickerPart {
  Control => "control",
  Trigger => "trigger",
  TriggerIcon => "trigger_icon",
  Clear => "clear",
  ClearIcon => "clear_icon",
  Panel => "panel",
  Presets => "presets",
  Preset => "preset",
  Separator => "separator",
});

/// The states of a date picker a recipe styles. It is in the field's size too, by its name.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DatePickerState {
  /// Its calendar is open.
  Open,
  /// What it holds is wrong.
  Invalid,
  /// What it holds is allowed with a caveat.
  Warning,
  /// What it holds was checked and is right.
  Valid,
  /// It can be read and not changed.
  ReadOnly,
  /// It takes no input.
  Disabled,
  /// It is as wide as what holds it.
  FullWidth,
}

names!(DatePickerState {
  Open => "open",
  Invalid => "invalid",
  Warning => "warning",
  Valid => "valid",
  ReadOnly => "read_only",
  Disabled => "disabled",
  FullWidth => "full_width",
});

/// What a picker holds: a date, or a range of them.
pub trait Picked:
  Copy + PartialEq + Send + Sync + std::fmt::Debug + crate::app::component::DevtoolsInspectable + 'static
{
  /// The component of the design system that picks it, whose recipe and renderer it is drawn by.
  #[doc(hidden)]
  type Picker: Themed<Renderer = PickerRenderer<Self>>;

  /// The short form of it in `locale`.
  fn format_in(self, locale: &DateLocale) -> String;

  /// What `text` holds, as `locale` reads it.
  fn parse_in(text: &str, locale: &DateLocale) -> Option<Self>;

  /// The day it starts at.
  fn start(self) -> Date;

  /// The day it ends at.
  fn end(self) -> Date;

  #[doc(hidden)]
  fn calendar(value: Signal<Option<Self>>) -> Calendar<Self>;

  /// The date a calendar of one date chose, as a value of this kind.
  #[doc(hidden)]
  fn from_date(date: Date) -> Option<Self>;

  /// The range a calendar of ranges chose, as a value of this kind.
  #[doc(hidden)]
  fn from_range(range: DateRange) -> Option<Self>;

  /// What a calendar chose, as a value of this kind.
  #[doc(hidden)]
  fn from_choice(choice: CalendarChoice) -> Option<Self> {
    match choice {
      CalendarChoice::Date(Some(date)) => Self::from_date(date),
      CalendarChoice::Range(Some(range)) => Self::from_range(range),
      _ => None,
    }
  }
}

impl Picked for Date {
  type Picker = DatePicker;

  fn format_in(self, locale: &DateLocale) -> String {
    locale.format(self)
  }

  fn parse_in(text: &str, locale: &DateLocale) -> Option<Self> {
    locale.parse(text)
  }

  fn start(self) -> Date {
    self
  }

  fn end(self) -> Date {
    self
  }

  fn calendar(value: Signal<Option<Self>>) -> Calendar<Self> {
    Calendar::with(Selection::Single(value))
  }

  fn from_date(date: Date) -> Option<Self> {
    Some(date)
  }

  fn from_range(_: DateRange) -> Option<Self> {
    None
  }
}

impl Picked for DateRange {
  type Picker = super::DateRangePicker;

  fn format_in(self, locale: &DateLocale) -> String {
    locale.format_range(self)
  }

  fn parse_in(text: &str, locale: &DateLocale) -> Option<Self> {
    locale.parse_range(text)
  }

  fn start(self) -> Date {
    DateRange::start(self)
  }

  fn end(self) -> Date {
    DateRange::end(self)
  }

  fn calendar(value: Signal<Option<Self>>) -> Calendar<Self> {
    Calendar::with(Selection::Range(value))
  }

  fn from_date(_: Date) -> Option<Self> {
    None
  }

  fn from_range(range: DateRange) -> Option<Self> {
    Some(range)
  }
}

/// What builds a picker's nodes: a theme's in place of the default one.
pub type PickerRenderer<V> = Arc<dyn Fn(PickerView<V>) -> Element + Send + Sync>;

/// What builds a date picker's nodes: a theme's in place of the default one.
pub type DatePickerRenderer = PickerRenderer<Date>;

/// A date picker as a renderer sees it.
pub type DatePickerView = PickerView<Date>;

/// The sizes of a date picker and of a date range picker: an input's, the heights of the theme's controls.
pub type DatePickerSize = TextBoxSize;

type Format<V> = Arc<dyn Fn(V) -> String + Send + Sync>;
type Parse<V> = Arc<dyn Fn(&str) -> Option<V> + Send + Sync>;

/// What a date picker and a date range picker are built from: the field, the calendar, and what they keep.
#[derive(Clone)]
pub(crate) struct Picker<V: Picked> {
  pub value: Signal<Option<V>>,
  pub placeholder: Option<Arc<str>>,
  pub boxed: BoxStates,
  pub full_width: bool,
  pub clearable: bool,
  pub format: Option<Format<V>>,
  pub parse: Option<Parse<V>>,
  pub calendar: Calendar<V>,
  /// Whether a change is only told, and the app answers with the value.
  pub controlled: bool,
  pub open: Option<Signal<bool>>,
  pub memory: Option<CalendarMemory>,
  pub on_change: Vec<Arc<dyn Fn(Option<V>) + Send + Sync>>,
  pub on_input: Vec<Arc<dyn Fn(&TextInputEvent) + Send + Sync>>,
  pub on_submit: Vec<Arc<dyn Fn(Option<V>) + Send + Sync>>,
  pub on_open_change: Vec<Arc<dyn Fn(bool) + Send + Sync>>,
  pub on_dismiss: Vec<Arc<dyn Fn() + Send + Sync>>,
  pub placement: Placement,
  pub offset: Option<SpacingValue>,
  pub collision: Option<CollisionStrategy>,
  pub inline: bool,
  pub trigger_label: Option<Arc<str>>,
  pub clear_label: Option<Arc<str>>,
  pub panel_label: Option<Arc<str>>,
  pub presets: Vec<(Arc<str>, V)>,
  // Read only where the engine has forms.
  #[cfg_attr(not(feature = "form"), allow(dead_code))]
  pub name: Option<Arc<str>>,
  pub states: Vec<Name>,
  pub rules: Recipe,
  pub forced: Option<Interaction>,
  pub id: Option<Arc<str>>,
  pub key: Option<Arc<str>>,
}

impl<V: Picked> Picker<V> {
  pub fn new(value: Signal<Option<V>>) -> Self {
    Self {
      calendar: V::calendar(value.clone()),
      value,
      controlled: false,
      placeholder: None,
      boxed: BoxStates::default(),
      full_width: false,
      clearable: false,
      format: None,
      parse: None,
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
      clear_label: None,
      panel_label: None,
      presets: Vec::new(),
      name: None,
      states: Vec::new(),
      rules: Recipe::new(),
      forced: None,
      id: None,
      key: None,
    }
  }

  /// Builds the picker in `theme` with what it keeps: its own where it is mounted, else made anew.
  pub fn build_with(self, theme: &Theme, kept: PickerKept, locale: DateLocale, today: Date) -> Element {
    let view = self.view(theme, kept, locale, today);
    match theme.renderer::<V::Picker>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }

  /// Builds the picker in `theme`, with what its memory keeps where it has one.
  pub fn build(self, theme: &Theme) -> Element {
    let today = self.calendar.today.unwrap_or_else(Date::today);
    let memory = self.memory.clone();
    let kept = PickerKept {
      open: self
        .open
        .clone()
        .or_else(|| memory.as_ref().map(|memory| memory.open.clone()))
        .unwrap_or_else(|| Signal::new(false)),
      anchor: memory.as_ref().map(|memory| memory.anchor.clone()).unwrap_or_default(),
      calendar: self.calendar.kept_by(memory.as_ref(), today),
    };
    let locale = self.calendar.locale.clone().unwrap_or_default();
    self.build_with(theme, kept, locale, today)
  }

  fn view(self, theme: &Theme, kept: PickerKept, locale: DateLocale, today: Date) -> PickerView<V> {
    // Read so that what renders the picker renders it again when the app's value changes: the text is made from it.
    let value = self.value.get();
    let open = kept.open.get();
    let format: Format<V> = self.format.clone().unwrap_or_else(|| {
      let locale = locale.clone();
      Arc::new(move |value: V| value.format_in(&locale))
    });
    let parse: Parse<V> = self.parse.clone().unwrap_or_else(|| {
      let locale = locale.clone();
      Arc::new(move |text: &str| V::parse_in(text, &locale))
    });
    let disabled = self.boxed.disabled;
    let case = self
      .boxed
      .case(self.states.clone())
      .state(DatePickerState::Open, open)
      .state(DatePickerState::Disabled, disabled)
      .state(DatePickerState::FullWidth, self.full_width);
    let recipe = theme.resolve::<V::Picker>(&case, Some(&self.rules));
    let calendar_rules = &self.calendar;
    let model = Model {
      value: self.value.clone(),
      open: kept.open.clone(),
      month: kept.calendar.month.clone(),
      draft: kept.calendar.draft.clone(),
      hover: kept.calendar.hover.clone(),
      format: format.clone(),
      parse,
      min: calendar_rules.min,
      max: calendar_rules.max,
      unavailable: calendar_rules.unavailable.clone(),
      today,
      controlled: self.controlled,
      on_change: self.on_change.clone(),
      on_submit: self.on_submit.clone(),
      on_open_change: self.on_open_change.clone(),
      on_dismiss: self.on_dismiss.clone(),
      read_only: self.boxed.read_only,
      disabled,
    };
    let text = Signal::new(value.map(|value| format(value)).unwrap_or_default());
    let mut input = Input::new(text.clone())
      .icon(icons::CALENDAR)
      .full_width(self.full_width)
      .force(self.forced)
      .rules(&recipe.rules_for(DatePickerPart::Control, InputPart::Control));
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
    // The app's handlers hear an edit first, and may refuse it: what is refused is not read.
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
    PickerView {
      text,
      value,
      open,
      placeholder: self.placeholder,
      label: self.boxed.label.clone(),
      invalid: self.boxed.judgment() == Some(Judgment::Invalid),
      read_only: self.boxed.read_only,
      disabled,
      presets: self.presets,
      clearable: self.clearable,
      full_width: self.full_width,
      locale,
      recipe,
      input: input.view(theme),
      calendar: self.calendar,
      placement: self.placement,
      offset: self.offset,
      collision: self.collision,
      inline: self.inline,
      trigger_label: self.trigger_label,
      clear_label: self.clear_label,
      panel_label: self.panel_label,
      kept,
      today,
      model,
      id: self.id,
      key: self.key,
    }
  }
}

/// What a picker keeps from one render to the next: whether its calendar is open, the field it opens under, and what
/// the calendar keeps.
#[derive(Clone)]
pub(crate) struct PickerKept {
  pub open: Signal<bool>,
  pub anchor: ElementRef,
  pub calendar: Kept,
}

/// What the handlers of a picker know of it.
#[derive(Clone)]
struct Model<V: Picked> {
  value: Signal<Option<V>>,
  open: Signal<bool>,
  month: MonthSource,
  draft: Signal<Option<Date>>,
  hover: Signal<Option<Date>>,
  format: Format<V>,
  parse: Parse<V>,
  min: Option<Date>,
  max: Option<Date>,
  unavailable: Option<Arc<dyn Fn(Date) -> bool + Send + Sync>>,
  today: Date,
  controlled: bool,
  on_change: Vec<Arc<dyn Fn(Option<V>) + Send + Sync>>,
  on_submit: Vec<Arc<dyn Fn(Option<V>) + Send + Sync>>,
  on_open_change: Vec<Arc<dyn Fn(bool) + Send + Sync>>,
  on_dismiss: Vec<Arc<dyn Fn() + Send + Sync>>,
  read_only: bool,
  disabled: bool,
}

impl<V: Picked> Model<V> {
  /// Whether `value` can be chosen: in the range, and neither end a day the app says is not to be.
  fn allowed(&self, value: V) -> bool {
    let day = |date: Date| {
      self.min.is_none_or(|min| date >= min)
        && self.max.is_none_or(|max| date <= max)
        && self.unavailable.as_ref().is_none_or(|unavailable| !unavailable(date))
    };
    day(value.start()) && day(value.end())
  }

  /// Sets the value where it changed, and tells the handlers; a controlled picker only tells them.
  fn write(&self, value: Option<V>) {
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

  /// Opens or closes the calendar, and tells the handlers where it changed.
  fn set_open(&self, open: bool) {
    if self.open.get_untracked() == open {
      return;
    }
    self.open.set(open);
    for handler in &self.on_open_change {
      handler(open);
    }
  }

  /// What Escape or a press outside does after the popover closed: forgets a range half chosen and tells the
  /// handlers.
  fn dismissed(&self) {
    self.draft.set(None);
    self.hover.set(None);
    for handler in &self.on_open_change {
      handler(false);
    }
    for handler in &self.on_dismiss {
      handler();
    }
  }

  /// What a person typed: the value where it reads as one that can be chosen, none where the field is empty, and
  /// nothing changed while it is being typed. The calendar shows the month of a date typed.
  fn typed(&self, text: &str) {
    if text.trim().is_empty() {
      self.write(None);
      return;
    }
    if let Some(value) = (self.parse)(text).filter(|value| self.allowed(*value)) {
      self.write(Some(value));
      self.month.set(YearMonth::of(value.start()));
    }
  }

  /// Puts what `text` holds right: the value it reads as, written as the picker writes it; an empty field none; and
  /// anything else the value it had. A controlled picker writes what the app holds, until it answers. Gives the value
  /// it holds now.
  fn commit(&self, text: &Signal<String>) -> Option<V> {
    let held = text.get_untracked();
    let next = if held.trim().is_empty() {
      self.write(None);
      None
    } else {
      match (self.parse)(&held).filter(|value| self.allowed(*value)) {
        Some(value) => {
          self.write(Some(value));
          Some(value)
        }
        None => self.value.get_untracked(),
      }
    };
    let shown = if self.controlled {
      self.value.get_untracked()
    } else {
      next
    };
    let written = shown.map(|value| (self.format)(value)).unwrap_or_default();
    if held != written {
      text.set(written);
    }
    next
  }

  /// Opens the calendar at the month of the value, or of today, in the range.
  fn open(&self) {
    if self.read_only || self.disabled {
      return;
    }
    let date = self
      .value
      .get_untracked()
      .map(Picked::start)
      .unwrap_or(self.today)
      .clamp_to(self.min, self.max);
    self.month.set(YearMonth::of(date));
    self.draft.set(None);
    self.hover.set(None);
    self.set_open(true);
  }

  /// Opens the calendar where it is closed, and closes it where it is open.
  fn toggle(&self) {
    if self.open.get_untracked() {
      self.set_open(false);
    } else {
      self.open();
    }
  }

  /// What a choice in the calendar or of a preset does: closes the calendar, then sets the value and tells the
  /// handlers where it is another, so that a handler that opens a dialog keeps its focus.
  fn chose(&self, value: V) {
    self.draft.set(None);
    self.hover.set(None);
    self.set_open(false);
    self.write(Some(value));
  }
}

/// A picker as a renderer sees it: what it was given, the style of its parts, and its behavior ready to be dressed.
pub struct PickerView<V: Picked> {
  /// What the field shows, made anew in each build from the value: the field keeps what is half typed in it while
  /// it has the focus.
  pub text: Signal<String>,
  /// The value as the picker was built with it.
  pub value: Option<V>,
  /// Whether the calendar is open.
  pub open: bool,
  pub placeholder: Option<Arc<str>>,
  /// What it is called to what reads the tree: its own label, or its field's.
  pub label: Option<Arc<str>>,
  /// Whether what it holds is wrong, as the field shows it.
  pub invalid: bool,
  pub read_only: bool,
  pub disabled: bool,
  /// The ranges a range picker offers beside its calendar, each with its label.
  pub presets: Vec<(Arc<str>, V)>,
  /// Whether it has a button that empties it while it holds a value.
  pub clearable: bool,
  pub full_width: bool,
  pub locale: DateLocale,
  /// The style of each part of this picker.
  pub recipe: ResolvedRecipe,
  input: InputView,
  id: Option<Arc<str>>,
  calendar: Calendar<V>,
  placement: Placement,
  offset: Option<SpacingValue>,
  collision: Option<CollisionStrategy>,
  inline: bool,
  trigger_label: Option<Arc<str>>,
  clear_label: Option<Arc<str>>,
  panel_label: Option<Arc<str>>,
  kept: PickerKept,
  today: Date,
  model: Model<V>,
  key: Option<Arc<str>>,
}

impl<V: Picked> PickerView<V> {
  /// The style of a part, as this picker is to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// The field's own padding at its start and its end, in pixels.
  fn padding(&self) -> (f32, f32) {
    let [_, end, _, start] = self.input.part(InputPart::Control).rest().paddings();
    let spacing = self.theme().spacing();
    let px = |padding: Option<crate::node::SpacingValue>| padding.map_or(0.0, |padding| padding.resolve(&spacing, 0.0));
    (px(start), px(end))
  }

  /// A length of a part's style in pixels: 0 where it has none in pixels.
  fn px(&self, length: Option<Length>) -> f32 {
    match length.and_then(|length| length.resolve(self.theme())) {
      Some(Dimension::Px(px)) => px,
      _ => 0.0,
    }
  }

  /// The field's behavior: the input's box, typed in and focused, a `combobox` to what reads the tree that says it
  /// opens a dialog and whether it is open. Down opens the calendar; Enter and the focus leaving put the text right.
  pub fn control(&self) -> components::TextInput {
    let model = self.model.clone();
    let (text, keys) = (self.text.clone(), model.clone());
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
          "ArrowDown" if !keys.open.get_untracked() => {
            event.prevent_default();
            keys.open();
          }
          // The text is put right and Enter goes on to what it does besides: it sends the form.
          "Enter" => {
            let value = keys.commit(&text);
            for handler in &keys.on_submit {
              handler(value);
            }
          }
          _ => {}
        }
      });
    if self.shows_clear() {
      let icon = self.px(self.part(DatePickerPart::ClearIcon).rest().width);
      control = control.padding_right(self.input.end_room(icon));
    }
    if !self.read_only {
      let (blur_text, blur) = (self.text.clone(), model);
      control = control.on_blur(move || {
        blur.commit(&blur_text);
      });
    }
    control
  }

  /// What the button that opens the calendar is called: the app's, or "Choose date" ("Choose dates" for a range).
  fn trigger_name(&self) -> Arc<str> {
    self.trigger_label.clone().unwrap_or_else(|| {
      Arc::from(if self.calendar.is_range() {
        "Choose dates"
      } else {
        "Choose date"
      })
    })
  }

  /// What the panel is called: the app's, or "Choose date" ("Choose dates" for a range).
  fn panel_name(&self) -> Arc<str> {
    self.panel_label.clone().unwrap_or_else(|| {
      Arc::from(if self.calendar.is_range() {
        "Choose dates"
      } else {
        "Choose date"
      })
    })
  }

  /// Whether the field has its button that empties it: where the picker is clearable and holds a value it can change.
  pub fn shows_clear(&self) -> bool {
    self.clearable && self.value.is_some() && !self.disabled && !self.read_only
  }

  /// A glyph of the theme's set in the style of `part`.
  fn glyph(&self, part: DatePickerPart, name: &'static str) -> Element {
    let part = self.part(part);
    let side = part.rest().width.unwrap_or(Length::Icon(IconSize::Md));
    Row::new()
      .part(&part)
      .child(Icon::new(name).size(side).build(self.theme()))
      .into()
  }

  /// What opens and closes the calendar, at the start of the field: pressed by the pointer, and taking no focus, since
  /// Down in the field opens it. Called by what it does to what reads the tree.
  pub fn trigger(&self) -> components::Button {
    let icon = if self.calendar.is_range() {
      icons::CALENDAR_RANGE
    } else {
      icons::CALENDAR
    };
    let model = self.model.clone();
    let mut trigger = components::Button::empty()
      .justify(Justify::Center)
      .align_items(Alignment::Center)
      .part(&self.part(DatePickerPart::Trigger))
      // No stop of Tab, in a form either, and a press leaves the focus where it is: Down in the field opens the
      // calendar, and a choice gives the focus back to the field, the first thing in the anchor that takes it.
      .focusable(false)
      .disabled(self.disabled || self.read_only)
      .describe("a11y_name", self.trigger_name())
      .describe("a11y_haspopup", "dialog")
      .describe("a11y_expanded", if self.open { "true" } else { "false" })
      .child(self.glyph(DatePickerPart::TriggerIcon, icon))
      .on_click(move |_: MouseEvent| model.toggle());
    if let Some(id) = &self.id {
      trigger = trigger.id(format!("{id}-trigger"));
    }
    trigger
  }

  /// What empties the field, at its end, where it shows one: pressed by the pointer, and taking no focus, since the
  /// field is emptied by deleting what it holds.
  pub fn clear(&self) -> Option<components::Button> {
    if !self.shows_clear() {
      return None;
    }
    let (model, text) = (self.model.clone(), self.text.clone());
    let mut clear = components::Button::empty()
      .justify(Justify::Center)
      .align_items(Alignment::Center)
      .part(&self.part(DatePickerPart::Clear))
      .focusable(false)
      .describe("a11y_name", self.clear_label.as_deref().unwrap_or("Clear"))
      .child(self.glyph(DatePickerPart::ClearIcon, icons::CLOSE))
      .on_click(move |_: MouseEvent| {
        model.write(None);
        text.set(String::new());
      });
    if let Some(id) = &self.id {
      clear = clear.id(format!("{id}-clear"));
    }
    Some(clear)
  }

  /// The field: the input's box, the trigger over its start where the input draws its icon, and the button that
  /// empties it over its end. It is what the calendar opens under.
  pub fn field(&self) -> Stack {
    let (start, end) = self.padding();
    let inset = |button: DatePickerPart, icon: DatePickerPart| {
      // A button is larger than its glyph, which stays where the input draws its icon.
      ((self.px(self.part(button).rest().width) - self.px(self.part(icon).rest().width)) / 2.0).max(0.0)
    };
    let mut field = Stack::new()
      .stack_align(StackAlignment::CenterStart)
      .part(&self.input.part(InputPart::Root))
      .ref_element(self.kept.anchor.clone())
      .child(self.control())
      .child(
        Row::new()
          .padding_left((start - inset(DatePickerPart::Trigger, DatePickerPart::TriggerIcon)).max(0.0))
          .align_items(Alignment::Center)
          .hit_test(HitTestBehavior::ContentOnly)
          .child(self.trigger()),
      );
    if let Some(clear) = self.clear() {
      field = field.child(
        Row::new()
          .width(Dimension::Pct(100.0))
          .padding_right((end - inset(DatePickerPart::Clear, DatePickerPart::ClearIcon)).max(0.0))
          .align_items(Alignment::Center)
          .justify(Justify::End)
          .hit_test(HitTestBehavior::ContentOnly)
          .child(clear),
      );
    }
    field
  }

  /// The calendar of the panel: the picker's value, its range and its days, drawn bare in the panel, the day Tab
  /// would enter at taking the focus as it opens. A choice closes it.
  pub fn calendar(&self) -> Element {
    let mut calendar = self.calendar.clone();
    calendar.kept = Some(self.kept.calendar.clone());
    calendar.bare = true;
    // Drawn in place, it takes no focus as the page shows it.
    calendar.autofocus = !self.inline;
    calendar.disabled |= self.disabled;
    // The picker sets the value: after it closed the calendar, and only where it is another.
    calendar.controlled = true;
    let model = self.model.clone();
    calendar.on_choose.push(Arc::new(move |choice: CalendarChoice| {
      if let Some(value) = V::from_choice(choice) {
        model.chose(value);
      }
    }));
    calendar.build_with(
      self.theme(),
      self.kept.calendar.clone(),
      self.locale.clone(),
      self.today,
    )
  }

  /// The presets of a range picker, one under another: each a small ghost button that chooses its range and closes
  /// the calendar; the one that is chosen is on the secondary fill. `None` where there are none.
  pub fn presets(&self) -> Option<Column> {
    if self.presets.is_empty() {
      return None;
    }
    let rules = self.recipe.rules_for(DatePickerPart::Preset, ButtonPart::Root);
    let mut column = Column::new()
      .align_items(Alignment::Stretch)
      .part(&self.part(DatePickerPart::Presets));
    for (label, value) in &self.presets {
      let (model, value) = (self.model.clone(), *value);
      let chosen = self.value == Some(value);
      let allowed = model.allowed(value);
      let mut button = Button::new(label)
        .variant(if chosen {
          ButtonVariant::Secondary
        } else {
          ButtonVariant::Ghost
        })
        .size(ButtonSize::Sm)
        .full_width(true)
        .disabled(self.disabled || !allowed)
        .rules(&rules)
        .on_click(move |_: MouseEvent| model.chose(value));
      if let Some(id) = &self.id {
        let slug: String = label
          .chars()
          .map(|c| {
            if c.is_alphanumeric() {
              c.to_ascii_lowercase()
            } else {
              '-'
            }
          })
          .collect();
        button = button.id(format!("{id}-preset-{slug}"));
      }
      column = column.child(button.build(self.theme()));
    }
    Some(column)
  }

  /// What the panel holds: the presets, where there are any, and a line, beside the calendar.
  fn content(&self) -> Element {
    match self.presets() {
      Some(presets) => Row::new()
        .align_items(Alignment::Stretch)
        .child(presets)
        .child(
          Separator::new()
            .orientation(SeparatorOrientation::Vertical)
            .rules(&self.recipe.rules_for(DatePickerPart::Separator, SeparatorPart::Line))
            .build(self.theme()),
        )
        .child(self.calendar())
        .into(),
      None => self.calendar(),
    }
  }

  /// The panel: a popover under the field (where [`DatePicker::placement`] says), holding the presets, where there
  /// are any, beside the calendar. Escape and a press outside close it, and with the focus in it give the focus back
  /// to the field. Inline, it is the popover's panel drawn in place under the field, open or not.
  pub fn panel(&self) -> Element {
    let rules = self.recipe.rules_for(DatePickerPart::Panel, PopoverPart::Panel);
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
        .describe("a11y_name", self.panel_name())
        .child(
          Column::new()
            .align_items(Alignment::Stretch)
            .part(&recipe.part(PopoverPart::Body))
            .child(self.content()),
        );
      if let Some(id) = &self.id {
        panel = panel.id(format!("{id}-panel"));
      }
      return panel.into();
    }
    let model = self.model.clone();
    let mut popover = Popover::new(self.kept.open.clone(), self.kept.anchor.clone())
      .placement(self.placement)
      .label(&self.panel_name())
      .rules(&rules)
      .on_dismiss(move || model.dismissed())
      .child(self.content());
    if let Some(offset) = self.offset {
      let _ = offset; // TEMP until Popover::offset is a distance again
    }
    if let Some(collision) = self.collision {
      popover = popover.collision(collision);
    }
    if let Some(id) = &self.id {
      popover = popover.id(format!("{id}-panel"));
    }
    popover.into()
  }

  /// How the default renderer builds a picker: the field, and the panel that opens under it.
  pub fn render(self) -> Element {
    if self.inline {
      // The panel in place under the field, as far from it as the popover would be.
      let gap = self
        .offset
        .unwrap_or_else(|| SpacingSize::Xs.into())
        .resolve(&self.theme().spacing(), 0.0);
      let mut root = Column::new().spacing(gap).child(self.field()).child(self.panel());
      if self.full_width {
        root = root.width(Dimension::Pct(100.0));
      }
      if let Some(key) = &self.key {
        root = root.key(key.clone());
      }
      return root.into();
    }
    let mut root = Row::new().child(self.field()).child(self.panel());
    if self.full_width {
      root = root.width(Dimension::Pct(100.0));
    }
    if let Some(key) = &self.key {
      root = root.key(key.clone());
    }
    root.into()
  }
}

/// A date picker: a field that holds a date, typed in it or chosen from a calendar that opens under it. A due date, a
/// date of birth, the day of a booking.
///
/// ```
/// use lurq::{
///   core::{Date, Signal},
///   ui::{DatePicker, Field},
/// };
///
/// # fn render(ctx: &mut lurq::app::ctx::Ctx) {
/// let due = Signal::new(Date::new(2026, 11, 12));
/// // Mounted, so that it keeps whether its calendar is open and the month it shows.
/// let picker = DatePicker::new(due.clone()).placeholder("Pick a date").clearable(true).mount(ctx);
/// // In a field, which names it and judges it, with what it keeps kept by the app (made once, in `create`).
/// let memory = lurq::ui::CalendarMemory::new();
/// let named = Field::new("Due date", DatePicker::new(due.clone()).min(Date::new(2026, 1, 1).unwrap()).memory(&memory));
/// # }
/// ```
///
/// The picker does not keep its date: the app's signal does. What is typed is read as it is typed, in the locale's
/// short form (`Nov 12, 2026`), in its numbers (`11/12/2026`) or as ISO 8601: a date that can be chosen is the value
/// at once, and the field keeps what is half typed while it has the focus. When the focus leaves it, or on Enter, the
/// text is written again as the picker writes the value; an emptied field is no date, and anything else takes back
/// the date it had. [`DatePicker::format`] and [`DatePicker::parse`] replace how it is written and read.
///
/// The calendar opens in a popover under the field from the button at its start, or from Down in the field, with the
/// focus on the chosen day; a choice closes it and the focus goes back to the field, as Escape and a press outside do.
/// To tooling the field is a `combobox` that opens a `dialog`, as the date picker of the ARIA patterns is, and its
/// button takes no focus.
///
/// The picker keeps whether its calendar is open, the field it opens under and the month its calendar shows where it
/// is mounted ([`DatePicker::mount`]) or given a [`CalendarMemory`](super::CalendarMemory) the app keeps
/// ([`DatePicker::memory`]), as a picker in a [`Field`](super::Field) is. Converted into an element with neither, it
/// opens only where the app gives it whether it is open ([`DatePicker::open`]).
#[derive(Clone)]
pub struct DatePicker {
  pub(crate) core: Picker<Date>,
}

/// The builder methods a date picker and a date range picker share.
macro_rules! picker_methods {
  ($value:ty) => {
    /// What the field says while it is empty.
    pub fn placeholder(mut self, placeholder: &str) -> Self {
      self.core.placeholder = Some(Arc::from(placeholder));
      self
    }

    /// A [`DatePickerSize`](crate::ui::DatePickerSize) (an input's), or a size the theme's recipe names: the field's.
    pub fn size(mut self, size: impl Into<Name>) -> Self {
      self.core.boxed.size = size.into();
      self
    }

    /// Whether what the picker holds is wrong. Of invalid, warning and valid, the first that is set is the one shown.
    pub fn invalid(mut self, invalid: bool) -> Self {
      self.core.boxed.invalid = invalid;
      self
    }

    /// Whether what the picker holds is allowed with a caveat.
    pub fn warning(mut self, warning: bool) -> Self {
      self.core.boxed.warning = warning;
      self
    }

    /// Whether what the picker holds was checked and is right.
    pub fn valid(mut self, valid: bool) -> Self {
      self.core.boxed.valid = valid;
      self
    }

    /// Whether the field can be read and not changed: it opens no calendar.
    pub fn read_only(mut self, read_only: bool) -> Self {
      self.core.boxed.read_only = read_only;
      self
    }

    /// Whether the picker takes no input.
    pub fn disabled(mut self, disabled: bool) -> Self {
      self.core.boxed.disabled = disabled;
      self
    }

    /// What the field is called, to what reads the tree. A [`Field`](crate::ui::Field) around it gives it its label.
    pub fn label(mut self, label: &str) -> Self {
      self.core.boxed.label = Some(Arc::from(label));
      self
    }

    /// Whether the field is as wide as what holds it, in place of the design's 260.
    pub fn full_width(mut self, full_width: bool) -> Self {
      self.core.full_width = full_width;
      self
    }

    /// Whether a button at the end of the field empties it while it holds something.
    pub fn clearable(mut self, clearable: bool) -> Self {
      self.core.clearable = clearable;
      self
    }

    /// How the field writes the value, in place of the locale's short form.
    pub fn format(mut self, format: impl Fn($value) -> String + Send + Sync + 'static) -> Self {
      self.core.format = Some(Arc::new(format));
      self
    }

    /// How the field reads what is typed, in place of the locale's: `None` for what is no value, or not yet one.
    pub fn parse(mut self, parse: impl Fn(&str) -> Option<$value> + Send + Sync + 'static) -> Self {
      self.core.parse = Some(Arc::new(parse));
      self
    }

    /// Whether the calendar is open, the app's: in place of what a mounted picker keeps.
    pub fn open(mut self, open: Signal<bool>) -> Self {
      self.core.open = Some(open);
      self
    }

    /// Called when the calendar opens or closes, with whether it is open: from its button, Down, a choice, Escape or
    /// a press outside. Not when the app sets [`Self::open`]'s signal. A second handler is called after the first.
    pub fn on_open_change(mut self, handler: impl Fn(bool) + Send + Sync + 'static) -> Self {
      self.core.on_open_change.push(Arc::new(handler));
      self
    }

    /// Called when a person closed the calendar with Escape or a press outside it, not by a choice. A second handler
    /// is called after the first.
    pub fn on_dismiss(mut self, handler: impl Fn() + Send + Sync + 'static) -> Self {
      self.core.on_dismiss.push(Arc::new(handler));
      self
    }

    /// Where the calendar opens beside the field: under its start by default.
    pub fn placement(mut self, placement: crate::app::ctx::Placement) -> Self {
      self.core.placement = placement;
      self
    }

    /// How far the calendar is from the field: the theme's smallest spacing by default.
    pub fn offset(mut self, offset: impl Into<crate::node::SpacingValue>) -> Self {
      self.core.offset = Some(offset.into());
      self
    }

    /// What the calendar does where there is no room for it on its side: goes to the other side, then is moved into
    /// the window, by default.
    pub fn collision(mut self, collision: crate::app::ctx::CollisionStrategy) -> Self {
      self.core.collision = Some(collision);
      self
    }

    /// Draws the panel in place under the field, open or not, and not over the window: for a gallery, a test, or a
    /// calendar that is part of a page. A choice still sets the value, and the calendar takes no focus as it is shown.
    pub fn inline(mut self, inline: bool) -> Self {
      self.core.inline = inline;
      self
    }

    /// What the button that opens the calendar is called to what reads the tree: "Choose date" in English, "Choose
    /// dates" for a range.
    pub fn trigger_label(mut self, label: &str) -> Self {
      self.core.trigger_label = Some(Arc::from(label));
      self
    }

    /// What the button that empties the field is called to what reads the tree: "Clear" in English.
    pub fn clear_label(mut self, label: &str) -> Self {
      self.core.clear_label = Some(Arc::from(label));
      self
    }

    /// What the panel of the calendar, a dialog, is called to what reads the tree: "Choose date" in English, "Choose
    /// dates" for a range.
    pub fn panel_label(mut self, label: &str) -> Self {
      self.core.panel_label = Some(Arc::from(label));
      self
    }

    /// What the calendar's button that shows the month before is called: "Previous month" in English.
    pub fn previous_label(mut self, label: &str) -> Self {
      self.core.calendar = self.core.calendar.previous_label(label);
      self
    }

    /// What the calendar's button that shows the month after is called: "Next month" in English.
    pub fn next_label(mut self, label: &str) -> Self {
      self.core.calendar = self.core.calendar.next_label(label);
      self
    }

    /// What keeps whether the calendar is open, the field it opens under and what the calendar keeps, from one
    /// render to the next: the app's, made once. A picker in a [`Field`](crate::ui::Field) keeps them so.
    pub fn memory(mut self, memory: &crate::ui::CalendarMemory) -> Self {
      self.core.memory = Some(memory.clone());
      self
    }

    /// The month the calendar shows, the app's: in place of what a mounted picker keeps.
    pub fn month(mut self, month: Signal<YearMonth>) -> Self {
      self.core.calendar = self.core.calendar.month(month);
      self
    }

    /// The least date that can be chosen or typed.
    pub fn min(mut self, min: Date) -> Self {
      self.core.calendar = self.core.calendar.min(min);
      self
    }

    /// The greatest date that can be chosen or typed.
    pub fn max(mut self, max: Date) -> Self {
      self.core.calendar = self.core.calendar.max(max);
      self
    }

    /// Which days cannot be chosen or typed besides those out of the range.
    pub fn disabled_days(mut self, unavailable: impl Fn(Date) -> bool + Send + Sync + 'static) -> Self {
      self.core.calendar = self.core.calendar.disabled_days(unavailable);
      self
    }

    /// The day the calendar's weeks start on, in place of the locale's.
    pub fn first_day(mut self, first: Weekday) -> Self {
      self.core.calendar = self.core.calendar.first_day(first);
      self
    }

    /// The locale the field writes and reads dates in and the calendar names its months and days in, in place of the
    /// app's.
    pub fn locale(mut self, locale: DateLocale) -> Self {
      self.core.calendar = self.core.calendar.locale(locale);
      self
    }

    /// Whether the calendar shows the number of each week before it.
    pub fn week_numbers(mut self, week_numbers: bool) -> Self {
      self.core.calendar = self.core.calendar.week_numbers(week_numbers);
      self
    }

    /// The day the calendar marks as today and opens at where nothing is chosen, in place of the system's.
    pub fn today(mut self, today: Date) -> Self {
      self.core.calendar = self.core.calendar.today(today);
      self
    }

    /// Called when a person changes the value, by typing, a choice or the button that empties it, with what it is
    /// now: `None` for an emptied field. Bound to a signal, the value is set already. Not called for what is chosen
    /// already, nor when the app sets the value. A choice closes the calendar before it is called. A second handler
    /// is called after the first.
    pub fn on_change(mut self, handler: impl Fn(Option<$value>) + Send + Sync + 'static) -> Self {
      self.core.on_change.push(Arc::new(handler));
      self
    }

    /// Called before an edit of the field's text, with what it will hold: a handler may refuse it
    /// ([`TextInputEvent::prevent_default`](crate::app::events::TextInputEvent::prevent_default)), and what is refused
    /// is not read as a value. A second handler is called after the first.
    pub fn on_input(mut self, handler: impl Fn(&crate::app::events::TextInputEvent) + Send + Sync + 'static) -> Self {
      self.core.on_input.push(Arc::new(handler));
      self
    }

    /// Called on Enter in the field, once its text is put right, with the value it holds then. Enter goes on to send
    /// the form the picker is in. A second handler is called after the first.
    pub fn on_submit(mut self, handler: impl Fn(Option<$value>) + Send + Sync + 'static) -> Self {
      self.core.on_submit.push(Arc::new(handler));
      self
    }

    /// The name the value has in the form it is in: the field's text is what the form holds.
    #[cfg(feature = "form")]
    pub fn name(mut self, name: impl Into<Arc<str>>) -> Self {
      self.core.name = Some(name.into());
      self
    }

    /// Puts the picker in a state the theme's recipe names, or takes it out.
    pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
      let state = state.into();
      self.core.states.retain(|held| *held != state);
      if on {
        self.core.states.push(state);
      }
      self
    }

    /// Styles one part of this picker over what the theme gives it.
    pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(PartStyle) -> PartStyle) -> Self {
      self.core.rules = self.core.rules.base(part, style);
      self
    }

    /// Lays rules over the theme's recipe for this picker.
    pub fn rules(mut self, rules: &Recipe) -> Self {
      self.core.rules = self.core.rules.extend(rules);
      self
    }

    /// Draws the field as it is under the pointer or focused, with nothing pointing at it: for a gallery or a test.
    pub fn force(mut self, interaction: Option<Interaction>) -> Self {
      self.core.forced = interaction;
      self
    }

    /// The id of the field: what is typed in and focused. The button at its start has it with `-trigger` after it,
    /// the one at its end `-clear`, the panel `-panel`, and the calendar `-calendar`, whose days have that with `-`
    /// and the date after it.
    pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
      let id = id.into();
      self.core.calendar = self.core.calendar.id(format!("{id}-calendar"));
      self.core.id = Some(id);
      self
    }

    pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
      self.core.key = Some(key.into());
      self
    }
  };
}
pub(crate) use picker_methods;

impl DatePicker {
  /// A picker bound to `value`: it shows the signal's date, and what is typed or chosen is set on it.
  pub fn new(value: Signal<Option<Date>>) -> Self {
    Self {
      core: Picker::new(value),
    }
  }

  /// A picker that shows `value` and tells `on_change` of what is typed, chosen or emptied, which it does not set:
  /// the app answers with the value it renders it with next, and the field shows what the app holds until it does.
  pub fn controlled(value: Option<Date>, on_change: impl Fn(Option<Date>) + Send + Sync + 'static) -> Self {
    let mut picker = Self::new(Signal::new(value)).on_change(on_change);
    picker.core.controlled = true;
    picker
  }

  picker_methods!(Date);

  /// Builds the picker in `theme`, with nothing kept but what the app gives it. Converting a picker into an element
  /// does this in the theme of what is being rendered.
  pub fn build(self, theme: &Theme) -> Element {
    self.core.build(theme)
  }

  /// The picker mounted where it is declared: it keeps whether its calendar is open, the field it opens under and the
  /// month the calendar shows, and writes dates in the app's locale.
  pub fn mount(self, ctx: &mut Ctx) -> Element {
    mount_picker(ctx, self.core)
  }
}

impl From<DatePicker> for Element {
  fn from(picker: DatePicker) -> Self {
    picker.build(&current_theme())
  }
}

impl FieldControl for DatePicker {
  /// Called by the field's label unless it has a name of its own, judged as the field is unless the app judged the
  /// picker itself, and disabled where the field is.
  fn in_field(mut self, field: &InField) -> Element {
    self.core.boxed.in_field(field);
    self.into()
  }
}

impl Themed for DatePicker {
  const NAME: &'static str = "date_picker";
  type Renderer = DatePickerRenderer;

  /// The design's date picker (`pencil/lurq-design-system.pen`, Date Picker): an input, 260 wide and 36 high, with a
  /// calendar's glyph of 16 in the muted foreground 12 from its start and its date 8 after it in the body's type (the
  /// input's own recipe). The glyph is in a button of 24 with the small radius, on the highlight under the pointer.
  /// The calendar opens 4 under the field in the popover's panel, which has no padding of its own and is as wide as
  /// the calendar (Open date picker). A clearable one has a cross of 16 in the same dress at its end.
  fn recipe() -> Recipe {
    picker_recipe()
  }
}

/// The recipe of a date picker and of a date range picker. A range picker's presets are a column of small ghost
/// buttons 2 apart, 8 from the panel's edge and the line beside the calendar, 12 from its top and its bottom.
pub(crate) fn picker_recipe() -> Recipe {
  // `Control` (rules over the input's box) and `Separator` (over the line beside the presets) add nothing here: the
  // input and the separator draw them in their own recipes.
  use DatePickerPart::{Clear, ClearIcon, Panel, Preset, Presets, Trigger, TriggerIcon};
  use PaletteColor::{Highlight, MutedForeground};
  let button = |s: PartStyle| {
    fading(
      s.square(24.0)
        .radius(RadiusSize::Sm)
        .foreground(MutedForeground)
        .cursor(CursorIcon::Pointer),
    )
  };
  Recipe::new()
    .base(Trigger, button)
    .base(Clear, button)
    .hovered(Trigger, |s| s.background(Highlight))
    .hovered(Clear, |s| s.background(Highlight))
    .base(TriggerIcon, |s| s.square(IconSize::Md))
    .base(ClearIcon, |s| s.square(IconSize::Md))
    .base(Panel, |s| s.width(Length::Auto).padding_x(0.0).padding_y(0.0).gap(0.0))
    .base(Presets, |s| s.gap(2.0).padding_x(8.0).padding_y(12.0).min_width(140.0))
    .base(Preset, |s| s.padding_x(10.0))
    .state(DatePickerState::Disabled, Trigger, |s| s.cursor(CursorIcon::Default))
}

#[derive(Clone)]
struct PickerProps<V: Picked> {
  picker: Picker<V>,
}

/// The picker is built anew by every render of what holds it, so props are never the same as before.
impl<V: Picked> PartialEq for PickerProps<V> {
  fn eq(&self, _: &Self) -> bool {
    false
  }
}

#[cfg(feature = "devtools")]
impl<V: Picked> crate::app::component::DevtoolsInspectable for PickerProps<V> {}

/// What a mounted picker keeps between renders.
struct PickerHost<V: Picked> {
  memory: CalendarMemory,
  _value: std::marker::PhantomData<fn() -> V>,
}

impl<V: Picked> Component for PickerHost<V> {
  type Props = PickerProps<V>;

  fn create(_ctx: &mut Ctx) -> Self {
    Self {
      memory: CalendarMemory::new(),
      _value: std::marker::PhantomData,
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let mut picker = ctx.props::<PickerProps<V>>().picker.clone();
    if picker.memory.is_none() {
      picker.memory = Some(self.memory.clone());
    }
    if picker.calendar.locale.is_none() {
      picker.calendar.locale = Some(app_locale(ctx));
    }
    picker.build(&current_theme())
  }
}

/// Mounts a picker in the tree of `ctx`'s component.
pub(crate) fn mount_picker<V: Picked>(ctx: &mut Ctx, picker: Picker<V>) -> Element {
  let key = picker.key.clone();
  let mut element = ctx.mount::<PickerHost<V>>(PickerProps { picker });
  if let Some(key) = key {
    element = element.key(key);
  }
  element
}
