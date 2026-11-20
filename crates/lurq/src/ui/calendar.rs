//! A calendar: a month of days a person chooses a date from, or a range of dates.

use std::{collections::HashMap, marker::PhantomData, sync::Arc};

use parking_lot::Mutex;

use super::{
  button::{Button, ButtonPart, ButtonSize, ButtonVariant},
  date_picker::Picked,
  icons, names,
  part_style::PartStyle,
  recipe::{Case, Interaction, Name, Recipe, ResolvedPart, ResolvedRecipe, Selector},
  shared::fading,
  theme::{Themed, current_date_locale, current_theme},
};
use crate::{
  app::{
    component::Component,
    ctx::Ctx,
    events::{KeyboardEvent, MouseEvent},
    theme::{BorderSize, OpacityLevel, PaletteColor, RadiusSize, Theme, TypographyStyle},
  },
  components::{self, Column, Row},
  core::{Date, DateRange, DateText, Signal, Weekday, YearMonth, shared_date_text},
  layout::{Alignment, layout_kind::Justify, text_style::FontWeight},
  node::{CursorIcon, Element, FocusGroup, RadiusValue, border::Border, color::Color, dimension::Dimension},
};

/// The parts of a calendar a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CalendarPart {
  /// The panel: its fill, its edge, its corners, its padding, and the room between the headings and the days.
  Root,
  /// The row of the months' headings, and the row of their days: the room between the months side by side.
  Months,
  /// The row of a month's heading: the button before, the title, the button after.
  Header,
  /// The month and the year in the heading.
  Title,
  /// What the calendar says of a button that shows the month before or after: rules over the button's root.
  Nav,
  /// What the calendar says of the glyph of those buttons: rules over the button's icon.
  NavIcon,
  /// The names of the days over the weeks, and the weeks, one under another: its gap is the room between them.
  Grid,
  /// The row of the names of the days.
  Weekdays,
  /// The name of a day of the week, at the head of its column.
  Weekday,
  /// A row of seven days.
  Week,
  /// The number of a week, before it, where the calendar shows them.
  WeekNumber,
  /// A day: what is pressed, focused and chosen.
  Day,
  /// What the app puts under the months, across the panel: Today and Clear.
  Footer,
}

names!(CalendarPart {
  Root => "root",
  Months => "months",
  Header => "header",
  Title => "title",
  Nav => "nav",
  NavIcon => "nav_icon",
  Grid => "grid",
  Weekdays => "weekdays",
  Weekday => "weekday",
  Week => "week",
  WeekNumber => "week_number",
  Day => "day",
  Footer => "footer",
});

/// The states of a calendar a recipe styles, besides what the pointer and the keyboard do to it. The calendar is in
/// `Disabled` and `Bare`; a day is in the others that are its own.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CalendarState {
  /// The calendar takes no input.
  Disabled,
  /// The calendar is in a panel that draws its fill and its edge, a date picker's: it draws none of its own.
  Bare,
  /// The day is the chosen one, or an end of the chosen range.
  Selected,
  /// The day is today: the current one, as `a11y_current` says it.
  Current,
  /// The day is of the month before or after the one it is shown in.
  Outside,
  /// The day cannot be chosen: before the least date, after the greatest, or one the app says is not to be.
  Unavailable,
  /// The day is the first of the range, or of the range the pointer shows.
  RangeStart,
  /// The day is the last of the range, or where the pointer shows the range would end.
  RangeEnd,
  /// The day is between the ends of the range.
  InRange,
  /// The day is the other end of a range the calendar's date is one end of, which this calendar does not choose: the
  /// end a range's other picker holds ([`Calendar::other_end`]).
  OtherEnd,
  /// The day is the one the keys are on in a calendar whose panel reads them ([`CalendarKeys::Panel`]): what Enter
  /// chooses.
  Cursor,
  /// The calendar's headings have the month first and both buttons at the end ([`CalendarHeading::MonthThenNav`]).
  NavAtEnd,
}

names!(CalendarState {
  Disabled => "disabled",
  Bare => "bare",
  Selected => "selected",
  Current => "current",
  Outside => "outside",
  Unavailable => "unavailable",
  RangeStart => "range_start",
  RangeEnd => "range_end",
  InRange => "in_range",
  OtherEnd => "other_end",
  Cursor => "cursor",
  NavAtEnd => "nav_at_end",
});

/// How a calendar's heading is laid out.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum CalendarHeading {
  /// The month between the button before and the button after, as the design draws it.
  #[default]
  Centered,
  /// The month first, at the heading's start, and both buttons after it at the end: of several months, at the end of
  /// the last month's heading. The calendar is in the state [`CalendarState::NavAtEnd`].
  MonthThenNav,
}

/// Where a calendar's keys are read.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum CalendarKeys {
  /// The days are one stop of Tab, and the keys move the focus from day to day.
  #[default]
  Days,
  /// The days take no focus, and the panel reads the keys wherever the app keeps the focus (the panel, a field over
  /// it) while the calendar is shown: a day is the one the keys are on (state [`CalendarState::Cursor`]), which the
  /// arrows and PageUp and PageDown move and Enter chooses.
  Panel,
}

/// What builds a calendar's nodes: a theme's in place of the default one.
pub type CalendarRenderer = Arc<dyn Fn(CalendarView) -> Element + Send + Sync>;

/// What a calendar chooses: a date, or a range of them.
#[derive(Clone)]
pub(crate) enum Selection {
  Single(Signal<Option<Date>>),
  Range(Signal<Option<DateRange>>),
}

/// The month a calendar shows: the app's, or its own, which is `start` until it is set.
#[derive(Clone)]
pub(crate) enum MonthSource {
  App(Signal<YearMonth>),
  Own(Signal<Option<YearMonth>>, YearMonth),
}

impl MonthSource {
  /// The month, read so that the render reading it is run again when it changes.
  pub fn get(&self) -> YearMonth {
    match self {
      Self::App(month) => month.get(),
      Self::Own(month, start) => month.get().unwrap_or(*start),
    }
  }

  pub fn get_untracked(&self) -> YearMonth {
    match self {
      Self::App(month) => month.get_untracked(),
      Self::Own(month, start) => month.get_untracked().unwrap_or(*start),
    }
  }

  /// Shows `month`, where it is not shown.
  pub fn set(&self, month: YearMonth) {
    if self.get_untracked() == month {
      return;
    }
    match self {
      Self::App(signal) => signal.set(month),
      Self::Own(signal, _) => signal.set(Some(month)),
    }
  }
}

/// What a component of dates keeps from one render to the next, kept by the app: a field of the component that renders
/// the calendar, the picker or the field, made once, as the [`ElementRef`](crate::core::ElementRef) a popover hangs
/// from is. The month or the years a calendar or a month or year picker shows, the first press of a range and where the
/// pointer or the keys are while the second is to come; for a picker whether its panel is open and the field it opens
/// under; for a date or time field what is typed in its segments while it is not a whole value. A mounted component
/// keeps one of its own; one in a [`Field`](super::Field), which is given the builder, keeps the app's.
///
/// ```
/// use lurq::{
///   app::{component::Component, ctx::Ctx},
///   core::{Date, Signal},
///   node::Element,
///   ui::{DateMemory, DatePicker, Field},
/// };
///
/// struct Booking {
///   day: Signal<Option<Date>>,
///   picker: DateMemory,
/// }
///
/// impl Component for Booking {
///   type Props = ();
///
///   fn create(_ctx: &mut Ctx) -> Self {
///     Self { day: Signal::new(None), picker: DateMemory::new() }
///   }
///
///   fn render(&self, _ctx: &mut Ctx) -> impl Into<Element> {
///     // A field names and judges the picker; the memory keeps it open between renders.
///     Field::new("Day", DatePicker::new(self.day.clone()).memory(&self.picker))
///   }
/// }
/// ```
#[derive(Clone)]
pub struct DateMemory {
  pub(crate) month: Signal<Option<YearMonth>>,
  pub(crate) draft: Signal<Option<Date>>,
  pub(crate) hover: Signal<Option<Date>>,
  /// The day the keys are on, in a calendar whose panel reads them: `None` for the day Tab would enter at.
  pub(crate) cursor: Signal<Option<Date>>,
  pub(crate) open: Signal<bool>,
  pub(crate) anchor: crate::core::ElementRef,
  pub(crate) last: Arc<Mutex<Option<CalendarChoice>>>,
  /// What a field of segments has typed while it is not a whole value: `None` where it shows the value.
  pub(crate) typed: Signal<Option<super::date_field::Typed>>,
  /// Which segment of a field has the focus.
  pub(crate) focused: Signal<Option<usize>>,
  /// The digits typed in the focused segment so far: the segment, how many, and the number they make.
  pub(crate) digits: Arc<Mutex<Option<(usize, usize, u32)>>>,
  /// The value a field of segments was last built with, written as its `Debug`: one set from outside resets what is
  /// typed.
  pub(crate) seen: Arc<Mutex<Option<String>>>,
}

impl DateMemory {
  pub fn new() -> Self {
    Self {
      month: Signal::new(None),
      draft: Signal::new(None),
      hover: Signal::new(None),
      cursor: Signal::new(None),
      open: Signal::new(false),
      anchor: crate::core::ElementRef::new(),
      last: Arc::new(Mutex::new(None)),
      typed: Signal::new(None),
      focused: Signal::new(None),
      digits: Arc::new(Mutex::new(None)),
      seen: Arc::new(Mutex::new(None)),
    }
  }

  /// Whether the calendar of the picker it keeps is open.
  pub fn is_open(&self) -> bool {
    self.open.get()
  }

  /// Opens or closes the calendar of the picker it keeps.
  pub fn set_open(&self, open: bool) {
    if self.open.get_untracked() != open {
      self.open.set(open);
    }
  }
}

impl Default for DateMemory {
  fn default() -> Self {
    Self::new()
  }
}

/// What a calendar keeps from one render to the next, as a build reads it: the month it shows, and the first press of
/// a range and where the pointer or the keys are while the second is to come. The month is read by the render, so
/// that a change renders it again; the other two are where a memory keeps them (`tracked`), and are else made anew in
/// each build and read by nothing, so that a press sets them without a render that would make them anew.
#[derive(Clone)]
pub(crate) struct Kept {
  pub month: MonthSource,
  pub draft: Signal<Option<Date>>,
  pub hover: Signal<Option<Date>>,
  pub cursor: Signal<Option<Date>>,
  pub tracked: bool,
}

/// A calendar: the days of a month in weeks, which a person chooses a date from, or a range of dates. A due date, a
/// day to book, the week of a report.
///
/// ```
/// use lurq::{
///   core::{Date, DateRange, Signal},
///   ui::Calendar,
/// };
///
/// // In a component's render: the calendar is mounted, so that it keeps the month it shows.
/// # fn render(ctx: &mut lurq::app::ctx::Ctx) {
/// let due = Signal::new(Date::new(2026, 11, 12));
/// let calendar = Calendar::new(due.clone()).min(Date::new(2026, 1, 1).unwrap()).mount(ctx);
///
/// // A range, two months side by side, weekends not to be chosen.
/// let stay = Signal::new(None::<DateRange>);
/// let booking = Calendar::new(stay.clone())
///   .months(2)
///   .disabled_days(|date| date.weekday().days_from_monday() >= 5)
///   .mount(ctx);
///
/// // The app holds the date and answers a choice: here it takes only dates after the 1st.
/// let held = due.get();
/// let answer = due.clone();
/// let asked = Calendar::controlled(held, move |date: Date| {
///   if date.day() > 1 {
///     answer.set(Some(date));
///   }
/// });
/// # }
/// ```
///
/// The calendar does not keep what is chosen: the app's signal does, which a press of a day sets. The month it shows
/// is not what is chosen: it is the app's where it gives one ([`Calendar::month`]), else the calendar keeps it,
/// starting at the month of what is chosen, or of today. It keeps it, and the first press of a range, where it is
/// mounted ([`Calendar::mount`]) or given a [`DateMemory`] the app keeps ([`Calendar::memory`]). Converted into an
/// element with neither, it keeps nothing: its buttons show no other month unless the app gives it the month, and a
/// range is shown only once it is chosen.
///
/// Bound to a `Signal<Option<DateRange>>` it is a `Calendar<DateRange>`, and a range is chosen with two presses: the
/// first starts it, the pointer and the keys show where it would end, and the second ends it, before or after the
/// first.
///
/// To the keyboard the days are one stop of Tab, at the chosen day, today or the first of the month. Left and Right
/// go a day, Up and Down a week, Home and End to the ends of the week, PageUp and PageDown a month and, with Shift, a
/// year: into the month before or after, which is then shown. A day that cannot be chosen takes the focus too, and
/// says it cannot be chosen: the keys pass no day by. Enter and Space choose the focused day.
///
/// To tooling the days of each month are a `grid` called by its month, of `gridcell`s that say whether they are
/// chosen (`a11y_selected`), today (`a11y_current`) and not to be chosen (`a11y_disabled`).
#[derive(Clone)]
pub struct Calendar<V: Picked = Date> {
  pub(crate) selection: Selection,
  pub(crate) month: Option<Signal<YearMonth>>,
  pub(crate) min: Option<Date>,
  pub(crate) max: Option<Date>,
  pub(crate) unavailable: Option<Arc<dyn Fn(Date) -> bool + Send + Sync>>,
  pub(crate) first_day: Option<Weekday>,
  pub(crate) locale: Option<Arc<dyn DateText>>,
  pub(crate) months: usize,
  pub(crate) week_numbers: bool,
  pub(crate) outside_days: bool,
  pub(crate) today: Option<Date>,
  pub(crate) other_end: Option<Date>,
  pub(crate) disabled: bool,
  on_change: Vec<Arc<dyn Fn(CalendarChoice) + Send + Sync>>,
  /// Told of every choice, of what is chosen already too: a date picker's, which closes its panel.
  pub(crate) on_choose: Vec<Arc<dyn Fn(CalendarChoice) + Send + Sync>>,
  /// Whether a choice is only told, and the app answers with the value: `controlled`, and a date picker's.
  pub(crate) controlled: bool,
  label: Option<Arc<str>>,
  previous_label: Option<Arc<str>>,
  next_label: Option<Arc<str>>,
  forced: Option<Interaction>,
  /// Where a date picker keeps what the calendar keeps.
  pub(crate) kept: Option<Kept>,
  pub(crate) memory: Option<DateMemory>,
  /// Whether the day Tab would enter at takes the focus as the panel it is in opens.
  pub(crate) autofocus: bool,
  pub(crate) bare: bool,
  heading: CalendarHeading,
  keys: CalendarKeys,
  footer: Option<Element>,
  states: Vec<Name>,
  rules: Recipe,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
  value: PhantomData<fn() -> V>,
}

impl<V: Picked> Calendar<V> {
  pub(crate) fn with(selection: Selection) -> Self {
    Self {
      selection,
      month: None,
      min: None,
      max: None,
      unavailable: None,
      first_day: None,
      locale: None,
      months: 1,
      week_numbers: false,
      outside_days: true,
      today: None,
      other_end: None,
      disabled: false,
      on_change: Vec::new(),
      on_choose: Vec::new(),
      controlled: false,
      label: None,
      previous_label: None,
      next_label: None,
      forced: None,
      kept: None,
      memory: None,
      autofocus: false,
      bare: false,
      heading: CalendarHeading::Centered,
      keys: CalendarKeys::Days,
      footer: None,
      states: Vec::new(),
      rules: Recipe::new(),
      id: None,
      key: None,
      value: PhantomData,
    }
  }

  /// A calendar bound to `value`, which a choice sets: of a `Signal<Option<Date>>` a date is chosen with one press,
  /// of a `Signal<Option<DateRange>>` a range with two. None is chosen to begin with where it holds none.
  pub fn new(value: Signal<Option<V>>) -> Self {
    V::calendar(value)
  }

  /// A calendar that shows `value` and tells `on_change` of a choice, which it does not set: the app answers with the
  /// value it renders it with next, and what was pressed is not shown chosen until it does.
  pub fn controlled(value: Option<V>, on_change: impl Fn(V) + Send + Sync + 'static) -> Self {
    let mut calendar = Self::new(Signal::new(value));
    calendar.controlled = true;
    calendar.on_change(on_change)
  }

  /// The month the calendar shows, the app's: the first of them where it shows several. The buttons of its heading
  /// and the keys that leave it set it.
  pub fn month(mut self, month: Signal<YearMonth>) -> Self {
    self.month = Some(month);
    self
  }

  /// The least date that can be chosen: the days before it cannot, and the months before its month are not shown.
  pub fn min(mut self, min: Date) -> Self {
    self.min = Some(min);
    self
  }

  /// The greatest date that can be chosen.
  pub fn max(mut self, max: Date) -> Self {
    self.max = Some(max);
    self
  }

  /// Which days cannot be chosen besides those out of the range: weekends, days that are booked.
  pub fn disabled_days(mut self, unavailable: impl Fn(Date) -> bool + Send + Sync + 'static) -> Self {
    self.unavailable = Some(Arc::new(unavailable));
    self
  }

  /// The day the weeks start on, in place of the locale's.
  pub fn first_day(mut self, first: Weekday) -> Self {
    self.first_day = Some(first);
    self
  }

  /// What the calendar names its months, its days and its buttons in and starts its weeks by, in place of the app's
  /// ([`App::set_date_locale`](crate::app::App::set_date_locale), English where it set none): a
  /// [`DateLocale`](crate::core::DateLocale), the app's own [`DateText`], or an `Arc` of it.
  pub fn locale(mut self, locale: impl DateText + 'static) -> Self {
    self.locale = Some(shared_date_text(locale));
    self
  }

  /// How many months the calendar shows side by side: one, or two for a range. Each has its heading.
  pub fn months(mut self, months: usize) -> Self {
    self.months = months.clamp(1, 12);
    self
  }

  /// Whether the number of each week (ISO 8601) is before it.
  pub fn week_numbers(mut self, week_numbers: bool) -> Self {
    self.week_numbers = week_numbers;
    self
  }

  /// Whether the days of the months before and after fill the first and the last week, as the design draws them. A
  /// calendar of several months shows them in none: they are in the month beside.
  pub fn outside_days(mut self, shown: bool) -> Self {
    self.outside_days = shown;
    self
  }

  /// The day the calendar marks as today, in place of the system's.
  pub fn today(mut self, today: Date) -> Self {
    self.today = Some(today);
    self
  }

  /// Whether the calendar takes no input.
  pub fn disabled(mut self, disabled: bool) -> Self {
    self.disabled = disabled;
    self
  }

  /// What stands under the months, across the panel, in the part `Footer`: the app's own buttons (Today, Clear), a
  /// line of what the days mean. What is in it is the app's, and is not in the focus group of the days: a button
  /// in it is a stop of Tab of its own. A second footer takes the place of the first.
  pub fn footer(mut self, footer: impl Into<Element>) -> Self {
    self.footer = Some(footer.into());
    self
  }

  /// How the headings are laid out: the month between the buttons, by default, or the month first and both buttons at
  /// the end ([`CalendarHeading::MonthThenNav`]).
  pub fn heading(mut self, heading: CalendarHeading) -> Self {
    self.heading = heading;
    self
  }

  /// Where the keys are read: by the days, which take the focus, by default; or by the panel, wherever the app keeps
  /// the focus while the calendar is shown ([`CalendarKeys::Panel`]), in a popover that leaves the focus in the app's
  /// field or panel ([`Popover::keeps_focus`](super::Popover::keeps_focus)). There the days take no focus, and one is
  /// the one the keys are on (state [`CalendarState::Cursor`], `cursor` to tooling): the chosen day, else today, else
  /// the first of the month to begin with. Left and Right move it a day, Up and Down a week, PageUp and PageDown a
  /// month and, with Shift, a year, showing its month; Enter chooses it, which tells [`on_choose`](Self::on_choose).
  /// A key a handler before took (`prevent_default`), a key with Ctrl, Alt or Cmd, and Enter where the focus is on
  /// what takes Enter itself (a button of the footer) are left alone; Home and End are the field's. The day the keys
  /// are on is kept where the month is ([`mount`](Self::mount), [`memory`](Self::memory)); it goes back to the chosen
  /// one after a choice, and to the day Tab would enter at where its month is no longer shown.
  pub fn keys(mut self, keys: CalendarKeys) -> Self {
    self.keys = keys;
    self
  }

  /// Called when a person chooses, with what is chosen: a press or Enter on a day, or for a range the second press,
  /// which ends it, whether or not it is what was chosen already, as [`on_change`](Self::on_change) is not. What closes
  /// a picker of the app's that holds the calendar. A second handler is called after the first.
  pub fn on_choose(mut self, handler: impl Fn(V) + Send + Sync + 'static) -> Self {
    self.on_choose.push(Arc::new(move |choice: CalendarChoice| {
      if let Some(value) = V::from_choice(choice) {
        handler(value);
      }
    }));
    self
  }

  /// Called when a person chooses another value, with it: a press or Enter on a day, or for a range the second
  /// press, which ends it. Bound to a signal, the value is set already. Not called for what is chosen already, nor
  /// when the app sets the value. A second handler is called after the first.
  pub fn on_change(mut self, handler: impl Fn(V) + Send + Sync + 'static) -> Self {
    self.on_change.push(Arc::new(move |choice: CalendarChoice| {
      if let Some(value) = V::from_choice(choice) {
        handler(value);
      }
    }));
    self
  }

  /// What the calendar is called to what reads the tree: its panel, which is then a `group`. Each month is a `grid`
  /// called by its month whatever this says.
  pub fn label(mut self, label: &str) -> Self {
    self.label = Some(Arc::from(label));
    self
  }

  /// What the button that shows the month before is called to what reads the tree: "Previous month" in English.
  pub fn previous_label(mut self, label: &str) -> Self {
    self.previous_label = Some(Arc::from(label));
    self
  }

  /// What the button that shows the month after is called to what reads the tree: "Next month" in English.
  pub fn next_label(mut self, label: &str) -> Self {
    self.next_label = Some(Arc::from(label));
    self
  }

  /// Draws the day Tab would enter at (the chosen one, today, or the first of the month) as it is under the pointer,
  /// held down or focused from the keyboard, with nothing pointing at it: for a gallery or a test. The other days and
  /// the buttons are drawn as they are.
  pub fn force(mut self, interaction: Option<Interaction>) -> Self {
    self.forced = interaction;
    self
  }

  /// Puts the calendar in a state the theme's recipe names, or takes it out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// Styles one part of this calendar over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(PartStyle) -> PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for this calendar.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// The id of its panel. Each day has it with `-day-` and the date after it: `due-day-2026-11-12`; each weekday's
  /// name with `-weekday-` and its English short name (`due-weekday-mon`); the buttons have it with `-nav-previous`
  /// and `-nav-next`; its other parts with `-` and the part's name (`due-title`).
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// What keeps the month the calendar shows, the first press of a range and where the range would end, from one
  /// render to the next: the app's, made once. A calendar with one follows the value to its month when the app sets
  /// it to a date that is not shown.
  pub fn memory(mut self, memory: &DateMemory) -> Self {
    self.memory = Some(memory.clone());
    self
  }

  /// The calendar mounted where it is declared: it keeps the month it shows where the app gives none, follows the
  /// value to its month when the app sets it to a date that is not shown, keeps the first press of a range and shows
  /// where the range would end, and names its months and days as the app says (`App::set_date_locale`).
  pub fn mount(self, ctx: &mut Ctx) -> Element {
    let key = self.key.clone();
    let mut element = ctx.mount::<CalendarHost<V>>(CalendarProps { calendar: self });
    if let Some(key) = key {
      element = element.key(key);
    }
    element
  }

  /// The value's date, or the end of its range, read so that the render reading it is run again when it changes.
  fn chosen(&self) -> CalendarChoice {
    match &self.selection {
      Selection::Single(value) => CalendarChoice::Date(value.get()),
      Selection::Range(value) => CalendarChoice::Range(value.get()),
    }
  }

  /// Whether a range is chosen with it, and not a date.
  pub(crate) fn is_range(&self) -> bool {
    matches!(self.selection, Selection::Range(_))
  }

  /// What is chosen, read by nothing.
  pub(crate) fn chosen_untracked(&self) -> CalendarChoice {
    match &self.selection {
      Selection::Single(value) => CalendarChoice::Date(value.get_untracked()),
      Selection::Range(value) => CalendarChoice::Range(value.get_untracked()),
    }
  }

  /// The month a calendar starts at: of what is chosen, else of today, kept in the range.
  pub(crate) fn start_month_untracked(&self, today: Date) -> YearMonth {
    let date = match self.chosen_untracked() {
      CalendarChoice::Date(Some(date)) => date,
      CalendarChoice::Range(Some(range)) => range.start(),
      _ => today,
    };
    YearMonth::of(date.clamp_to(self.min, self.max))
  }

  /// Builds the calendar in `theme`, with what its memory keeps where it has one. Converting a calendar into an
  /// element does this in the theme of what is being rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let today = self.today.unwrap_or_else(Date::today);
    let kept = self
      .kept
      .clone()
      .unwrap_or_else(|| self.kept_by(self.memory.as_ref(), today));
    let locale = self.locale.clone().unwrap_or_else(current_date_locale);
    self.build_with(theme, kept, locale, today)
  }

  /// What the calendar keeps in `memory`, or, with none, what it keeps for one build.
  pub(crate) fn kept_by(&self, memory: Option<&DateMemory>, today: Date) -> Kept {
    let start = self.start_month_untracked(today);
    let Some(memory) = memory else {
      return Kept {
        month: match &self.month {
          Some(month) => MonthSource::App(month.clone()),
          None => MonthSource::Own(Signal::new(None), start),
        },
        draft: Signal::new(None),
        hover: Signal::new(None),
        cursor: Signal::new(None),
        tracked: false,
      };
    };
    let month = match &self.month {
      Some(month) => MonthSource::App(month.clone()),
      None => MonthSource::Own(memory.month.clone(), start),
    };
    follow(&memory.last, self, &month);
    Kept {
      month,
      draft: memory.draft.clone(),
      hover: memory.hover.clone(),
      cursor: memory.cursor.clone(),
      tracked: true,
    }
  }

  pub(crate) fn build_with(self, theme: &Theme, kept: Kept, locale: Arc<dyn DateText>, today: Date) -> Element {
    let view = self.view(theme, kept, locale, today);
    match theme.renderer::<Calendar>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }

  fn view(self, theme: &Theme, kept: Kept, locale: Arc<dyn DateText>, today: Date) -> CalendarView {
    let first_day = self.first_day.unwrap_or_else(|| locale.first_day());
    let shown = kept.month.get();
    let (draft, hover) = if kept.tracked {
      (kept.draft.get(), kept.hover.get())
    } else {
      (None, None)
    };
    // The months shown start where the least date allows.
    let first = match self.min {
      Some(min) if shown < YearMonth::of(min) => YearMonth::of(min),
      _ => shown,
    };
    let chosen = self.chosen();
    let case = Case {
      variant: None,
      size: None,
      density: None,
      breakpoint: super::current_breakpoint(),
      states: self.states.clone(),
    }
    .state(CalendarState::Disabled, self.disabled)
    .state(CalendarState::Bare, self.bare)
    .state(CalendarState::NavAtEnd, self.heading == CalendarHeading::MonthThenNav);
    let cursor = match (self.keys, kept.tracked) {
      (CalendarKeys::Panel, true) => kept.cursor.get(),
      _ => None,
    };
    CalendarView {
      months: (0..self.months).map(|index| first.add_months(index as i32)).collect(),
      first_day,
      today,
      chosen,
      draft,
      hover,
      other_end: self.other_end,
      heading: self.heading,
      keys: self.keys,
      cursor,
      week_numbers: self.week_numbers,
      outside_days: self.outside_days && self.months == 1,
      disabled: self.disabled,
      recipe: theme
        .resolve::<Calendar>(&case, Some(&self.rules))
        .owned_by(self.id.as_ref()),
      days: Mutex::new(HashMap::new()),
      case,
      model: Model {
        selection: self.selection,
        month: kept.month,
        draft: kept.draft,
        hover: kept.hover,
        cursor: kept.cursor,
        shown: self.months,
        min: self.min,
        max: self.max,
        unavailable: self.unavailable,
        first_day,
        on_change: self.on_change,
        on_choose: self.on_choose,
        controlled: self.controlled,
      },
      locale,
      autofocus: self.autofocus,
      footer: self.footer,
      label: self.label,
      previous_label: self.previous_label,
      next_label: self.next_label,
      forced: self.forced,
      rules: self.rules,
      id: self.id,
      key: self.key,
    }
  }
}

impl Calendar<Date> {
  /// The other end of a range this calendar's date is one end of: a day the calendar marks, in the state
  /// [`CalendarState::OtherEnd`], tinted as the days inside a range are, while a person chooses this end. A range
  /// chosen in two pickers of one date each (a start and an end) gives each the date of the other; with
  /// [`min`](Self::min) or [`max`](Self::max) at it, the days past it cannot be chosen. `None` marks none. The day
  /// that is chosen is drawn chosen, whatever this says. Tooling reads it on the day as `range_other_end`.
  pub fn other_end(mut self, other_end: impl Into<Option<Date>>) -> Self {
    self.other_end = other_end.into();
    self
  }
}

impl<V: Picked> From<Calendar<V>> for Element {
  fn from(calendar: Calendar<V>) -> Self {
    calendar.build(&current_theme())
  }
}

/// What is chosen, as the calendar was built with it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CalendarChoice {
  Date(Option<Date>),
  Range(Option<DateRange>),
}

/// What the handlers of a calendar know of it: what it chooses, what it keeps, and what can be chosen.
#[derive(Clone)]
struct Model {
  selection: Selection,
  month: MonthSource,
  draft: Signal<Option<Date>>,
  hover: Signal<Option<Date>>,
  /// The day the keys are on, in a calendar whose panel reads them.
  cursor: Signal<Option<Date>>,
  /// How many months are shown.
  shown: usize,
  min: Option<Date>,
  max: Option<Date>,
  unavailable: Option<Arc<dyn Fn(Date) -> bool + Send + Sync>>,
  first_day: Weekday,
  on_change: Vec<Arc<dyn Fn(CalendarChoice) + Send + Sync>>,
  on_choose: Vec<Arc<dyn Fn(CalendarChoice) + Send + Sync>>,
  controlled: bool,
}

impl Model {
  /// Whether `date` can be chosen.
  fn available(&self, date: Date) -> bool {
    self.min.is_none_or(|min| date >= min)
      && self.max.is_none_or(|max| date <= max)
      && self.unavailable.as_ref().is_none_or(|unavailable| !unavailable(date))
  }

  /// Chooses `date`: the date, or an end of the range. What is chosen already is chosen again for a picker, which
  /// closes, and changes nothing.
  fn choose(&self, date: Date) {
    if !self.available(date) {
      return;
    }
    let (choice, changed) = match &self.selection {
      Selection::Single(value) => {
        let changed = value.get_untracked() != Some(date);
        if changed && !self.controlled {
          value.set(Some(date));
        }
        (CalendarChoice::Date(Some(date)), changed)
      }
      Selection::Range(value) => match self.draft.get_untracked() {
        None => {
          self.draft.set(Some(date));
          self.hover.set(Some(date));
          return;
        }
        Some(start) => {
          let range = DateRange::new(start, date);
          self.draft.set(None);
          self.hover.set(None);
          let changed = value.get_untracked() != Some(range);
          if changed && !self.controlled {
            value.set(Some(range));
          }
          (CalendarChoice::Range(Some(range)), changed)
        }
      },
    };
    // The keys go back to what is chosen.
    if self.cursor.get_untracked().is_some() {
      self.cursor.set(None);
    }
    for handler in &self.on_choose {
      handler(choice);
    }
    if changed {
      for handler in &self.on_change {
        handler(choice);
      }
    }
  }

  /// Shows the month `date` is in, where it is not shown: the first of the months where it is before them, the last
  /// where it is after them.
  fn show(&self, date: Date) {
    let first = self.month.get_untracked();
    let last = first.add_months(self.shown as i32 - 1);
    let month = YearMonth::of(date);
    let next = if month < first {
      month
    } else if month > last {
      month.add_months(-(self.shown as i32 - 1))
    } else {
      return;
    };
    self.month.set(next);
  }

  /// Whether `date` is in one of the months shown.
  fn is_shown(&self, date: Date) -> bool {
    let first = self.month.get_untracked();
    let month = YearMonth::of(date);
    month >= first && month <= first.add_months(self.shown as i32 - 1)
  }

  /// Where the pointer or the keys are while a range is half chosen: the range shown runs to it.
  fn point(&self, date: Date) {
    if self.draft.get_untracked().is_some() && self.hover.get_untracked() != Some(date) {
      self.hover.set(Some(date));
    }
  }

  /// The date a key takes the focus to from `date`, where the key is one of the grid's.
  fn step(&self, date: Date, event: &KeyboardEvent) -> Option<Date> {
    Some(step_date(date, event, self.first_day)?.clamp_to(self.min, self.max))
  }
}

/// The date a key of a grid of days takes the focus to from `date`, in weeks that start on `first_day`: Left and
/// Right a day, Up and Down a week, Home and End the ends of the week, PageUp and PageDown a month and, with Shift,
/// a year. `None` for a key that is not the days'. The keys of a [`Calendar`] and of an
/// [`EventCalendar`](super::EventCalendar), from one place.
pub(crate) fn step_date(date: Date, event: &KeyboardEvent, first_day: Weekday) -> Option<Date> {
  Some(match event.key.as_str() {
    "ArrowLeft" => date.add_days(-1),
    "ArrowRight" => date.add_days(1),
    "ArrowUp" => date.add_days(-7),
    "ArrowDown" => date.add_days(7),
    "Home" => date.start_of_week(first_day),
    "End" => date.end_of_week(first_day),
    "PageUp" if event.shift => date.add_years(-1),
    "PageDown" if event.shift => date.add_years(1),
    "PageUp" => date.add_months(-1),
    "PageDown" => date.add_months(1),
    _ => return None,
  })
}

/// The key of a weekday in the id of its name: its English short name in lower case.
pub(crate) fn weekday_key(day: Weekday) -> &'static str {
  match day {
    Weekday::Monday => "mon",
    Weekday::Tuesday => "tue",
    Weekday::Wednesday => "wed",
    Weekday::Thursday => "thu",
    Weekday::Friday => "fri",
    Weekday::Saturday => "sat",
    Weekday::Sunday => "sun",
  }
}

/// The key of a day of the month it is shown in: what the keys move the focus to.
fn day_key(date: Date) -> String {
  format!("day-{date}")
}

/// A calendar as a renderer sees it: the months it shows, what is chosen, the style of its parts, and its behavior
/// ready to be dressed.
pub struct CalendarView {
  /// The months shown, side by side.
  pub months: Vec<YearMonth>,
  pub first_day: Weekday,
  pub today: Date,
  pub chosen: CalendarChoice,
  /// The first press of a range, while the second is to come.
  pub draft: Option<Date>,
  /// Where the pointer or the keys are while the second press of a range is to come.
  pub hover: Option<Date>,
  /// The other end of a range the date chosen is one end of, which the calendar marks: [`Calendar::other_end`].
  pub other_end: Option<Date>,
  /// How the headings are laid out.
  pub heading: CalendarHeading,
  /// Where the keys are read.
  pub keys: CalendarKeys,
  /// The day the keys were last put on, in a calendar whose panel reads them: see [`Self::cursor_day`].
  pub cursor: Option<Date>,
  pub week_numbers: bool,
  /// Whether the first and the last week are filled with the days of the months before and after.
  pub outside_days: bool,
  pub disabled: bool,
  /// What the calendar's text is: its own `.locale(..)`, else the app's, else English.
  pub locale: Arc<dyn DateText>,
  /// The style of each part of this calendar.
  pub recipe: ResolvedRecipe,
  /// What the app put under the months.
  pub footer: Option<Element>,
  /// The recipe of a day in each set of its states, resolved once for each.
  days: Mutex<HashMap<Vec<Name>, ResolvedRecipe>>,
  case: Case,
  model: Model,
  autofocus: bool,
  label: Option<Arc<str>>,
  previous_label: Option<Arc<str>>,
  next_label: Option<Arc<str>>,
  forced: Option<Interaction>,
  rules: Recipe,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl CalendarView {
  /// The style of a part, as this calendar is to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// The range shown: the one chosen, or the one the first press and the pointer make while the second is to come.
  pub fn shown_range(&self) -> Option<DateRange> {
    match (self.draft, self.chosen) {
      (Some(start), _) => Some(DateRange::new(start, self.hover.unwrap_or(start))),
      (None, CalendarChoice::Range(range)) => range,
      (None, CalendarChoice::Date(_)) => None,
    }
  }

  /// Whether `date` can be chosen.
  pub fn available(&self, date: Date) -> bool {
    self.model.available(date)
  }

  /// The states of a day, `outside` where it is of the month before or after the one it is shown in.
  pub fn day_states(&self, date: Date, outside: bool) -> Vec<CalendarState> {
    let mut states = Vec::new();
    if outside {
      states.push(CalendarState::Outside);
    }
    if date == self.today {
      states.push(CalendarState::Current);
    }
    if !self.available(date) {
      states.push(CalendarState::Unavailable);
    }
    match (self.chosen, self.shown_range()) {
      (CalendarChoice::Date(Some(chosen)), _) if chosen == date => states.push(CalendarState::Selected),
      (_, Some(range)) if range.contains(date) => {
        if date == range.start() || date == range.end() {
          states.push(CalendarState::Selected);
        }
        if date == range.start() {
          states.push(CalendarState::RangeStart);
        }
        if date == range.end() {
          states.push(CalendarState::RangeEnd);
        }
        if date != range.start() && date != range.end() {
          states.push(CalendarState::InRange);
        }
      }
      _ => {}
    }
    if self.other_end == Some(date) && !states.contains(&CalendarState::Selected) {
      states.push(CalendarState::OtherEnd);
    }
    states
  }

  /// The recipe a day in `states` is drawn by.
  pub fn day_recipe(&self, states: &[CalendarState]) -> ResolvedRecipe {
    let names: Vec<Name> = states.iter().map(|state| (*state).into()).collect();
    self
      .days
      .lock()
      .entry(names.clone())
      .or_insert_with(|| {
        let case = names
          .iter()
          .fold(self.case.clone(), |case, state| case.state(*state, true));
        self
          .theme()
          .resolve::<Calendar>(&case, Some(&self.rules))
          .owned_like(&self.recipe)
      })
      .clone()
  }

  /// The day Tab enters the days at: the chosen one where it is shown, else today where it is, else the first of the
  /// first month.
  pub fn entry(&self) -> Date {
    let shown = |date: Date| self.months.iter().any(|month| month.contains(date));
    let chosen = match self.chosen {
      CalendarChoice::Date(date) => date,
      CalendarChoice::Range(range) => range.map(DateRange::start),
    };
    chosen
      .filter(|date| shown(*date))
      .or_else(|| shown(self.today).then_some(self.today))
      .unwrap_or_else(|| self.months[0].first())
  }

  /// The day the keys are on in a calendar whose panel reads them: the one they were put on where its month is shown,
  /// else the day Tab would enter at ([`Self::entry`]).
  pub fn cursor_day(&self) -> Date {
    self
      .cursor
      .filter(|date| self.months.iter().any(|month| month.contains(*date)))
      .unwrap_or_else(|| self.entry())
  }

  /// Whether the panel reads the keys, and the days take no focus.
  fn panel_keys(&self) -> bool {
    self.keys == CalendarKeys::Panel
  }

  /// A button that shows the month before (`by` -1) or after (1), in the design's dress: an outline button of an
  /// icon, called by what it does. Disabled where that month is out of the range.
  pub fn nav(&self, by: i32) -> Element {
    // The app's name for the button, else its locale's.
    let (label, icon, suffix) = if by < 0 {
      (
        self
          .previous_label
          .as_deref()
          .map_or_else(|| self.locale.previous_month_label(), str::to_owned),
        icons::CHEVRON_LEFT,
        "previous",
      )
    } else {
      (
        self
          .next_label
          .as_deref()
          .map_or_else(|| self.locale.next_month_label(), str::to_owned),
        icons::CHEVRON_RIGHT,
        "next",
      )
    };
    let first = self.months[0];
    let last = self.months[self.months.len() - 1];
    let out = if by < 0 {
      self.model.min.is_some_and(|min| first.add_months(-1).last() < min)
    } else {
      self.model.max.is_some_and(|max| last.add_months(1).first() > max)
    };
    let rules = self
      .recipe
      .rules_for(CalendarPart::Nav, ButtonPart::Root)
      .extend(&self.recipe.rules_for(CalendarPart::NavIcon, ButtonPart::Icon));
    let month = self.model.month.clone();
    let mut button = Button::new(&label)
      .variant(ButtonVariant::Outline)
      .size(ButtonSize::Xs)
      .icon(icon)
      .icon_only(true)
      .disabled(self.disabled || out)
      .rules(&rules)
      .on_click(move |_: MouseEvent| month.set(month.get_untracked().add_months(by)));
    // Named by its side, whether the calendar's id is its own or one it was lent.
    if let Some(id) = self.recipe.keyed_id(CalendarPart::Nav, suffix) {
      button = button.id(id);
    }
    button.build(self.theme())
  }

  /// The heading of the `index`th month: its title between the button before, on the first month, and the button
  /// after, on the last. Where a month has no button, a room as wide keeps its title in the middle. Laid out
  /// [`CalendarHeading::MonthThenNav`], the title is at the start and both buttons are at the end of the last month's.
  pub fn header(&self, index: usize) -> Row {
    let month = self.months[index];
    let title = self.part(CalendarPart::Title);
    if self.heading == CalendarHeading::MonthThenNav {
      let mut row = Row::new()
        .align_items(Alignment::Center)
        .part(&self.part(CalendarPart::Header))
        .child(
          Row::new()
            .flex_full(1.0, 1.0, None)
            .min_width(0.0)
            .part(&title)
            .child(title.text(&self.locale.month_title(month)).nowrap()),
        );
      if index + 1 == self.months.len() {
        row = row.child(self.nav(-1)).child(self.nav(1));
      }
      return row;
    }
    let nav_width = self
      .part(CalendarPart::Nav)
      .rest()
      .width
      .and_then(|width| width.resolve(self.theme()));
    let room = || {
      let mut room = Row::new();
      if let Some(width) = nav_width {
        room = room.width(width);
      }
      room
    };
    let before: Element = if index == 0 { self.nav(-1) } else { room().into() };
    let after: Element = if index + 1 == self.months.len() {
      self.nav(1)
    } else {
      room().into()
    };
    Row::new()
      .align_items(Alignment::Center)
      .justify(Justify::SpaceBetween)
      .part(&self.part(CalendarPart::Header))
      .child(before)
      .child(
        Row::new()
          .part(&title)
          .child(title.text(&self.locale.month_title(month)).nowrap()),
      )
      .child(after)
  }

  /// The row of the names of the days, from the first day, after a room for the week numbers where they are shown.
  pub fn weekdays(&self) -> Row {
    let mut row = Row::new()
      .part(&self.part(CalendarPart::Weekdays))
      .describe("a11y_role", "row");
    if self.week_numbers {
      row = row.child(Row::new().part(&self.part(CalendarPart::WeekNumber)));
    }
    for index in 0..7 {
      let day = Weekday::from_monday(self.first_day as i64 + index);
      // Named by the day's English short name, an identifier and not a word of the locale: `x-weekday-mon`.
      let part = self.part(CalendarPart::Weekday).keyed(weekday_key(day));
      row = row.child(
        Row::new()
          .align_items(Alignment::Center)
          .justify(Justify::Center)
          .part(&part)
          .describe("a11y_role", "columnheader")
          .describe("a11y_name", self.locale.weekday_name(day))
          .child(part.text(&self.locale.weekday_short(day)).nowrap()),
      );
    }
    row
  }

  /// A day of `month`: pressed, focused and chosen, in the style of its states. A day of the month before or after
  /// is drawn where the calendar shows them, is pressed to choose it and show its month, and is no stop of the keys;
  /// elsewhere it is an empty cell.
  pub fn day(&self, date: Date, month: YearMonth) -> Element {
    let outside = !month.contains(date);
    if outside && !self.outside_days {
      return Row::new().part(&self.part(CalendarPart::Day)).into();
    }
    let mut states = self.day_states(date, outside);
    let cursor = self.panel_keys() && !outside && date == self.cursor_day();
    if cursor {
      states.push(CalendarState::Cursor);
    }
    let recipe = self.day_recipe(&states);
    let entry = !outside && date == self.entry();
    // A day is named by its date, not by its place: `x-day-2026-11-20`.
    let part = recipe
      .part(CalendarPart::Day)
      .keyed(date)
      .forced(self.forced.filter(|_| entry));
    let available = self.available(date);
    let selected = states.contains(&CalendarState::Selected) || states.contains(&CalendarState::InRange);
    let mut day = components::Button::empty()
      .justify(Justify::Center)
      .align_items(Alignment::Center)
      .disabled(self.disabled)
      .part(&part)
      .describe("a11y_role", "gridcell")
      .describe("a11y_name", self.locale.day_label(date))
      .describe("a11y_selected", if selected { "true" } else { "false" })
      .child(part.text(&date.day().to_string()).nowrap());
    if date == self.today {
      day = day.describe("a11y_current", "date");
    }
    if !available {
      day = day.describe("a11y_disabled", "true");
    }
    if states.contains(&CalendarState::OtherEnd) {
      day = day.describe("range_other_end", "true");
    }
    if self.panel_keys() {
      // The panel reads the keys: a day takes no focus, and a press of it leaves the focus where the app keeps it.
      day = day.focusable(false).key(if outside {
        format!("outside-{date}")
      } else {
        day_key(date)
      });
      if cursor {
        day = day.describe("cursor", "true");
      }
    } else if outside {
      // Of the month beside: the pointer chooses it, the keys reach it in its own month.
      day = day.tab_index(-1).key(format!("outside-{date}"));
    } else {
      let model = self.model.clone();
      day = day
        .tab_index(0)
        .key(day_key(date))
        .focus_entry(date == self.entry())
        .on_key_down(move |event: KeyboardEvent| {
          if !event.focus_within || event.ctrl || event.alt || event.meta || event.composing {
            return;
          }
          let Some(target) = model.step(date, &event) else {
            return;
          };
          model.show(target);
          model.point(target);
          event.focus_key(day_key(target));
        });
      if self.autofocus && date == self.entry() {
        day = day.autofocus(true);
      }
    }
    if available && !self.disabled {
      let (press, enter, focus) = (self.model.clone(), self.model.clone(), self.model.clone());
      day = day
        .on_click(move |event: MouseEvent| {
          press.choose(date);
          if outside {
            // The day pressed is gone with the month it was beside: the focus goes to the day in its own month.
            press.show(date);
            event.focus_key(day_key(date));
          }
        })
        .on_mouse_enter(move || enter.point(date))
        .on_focus(move || focus.point(date));
    }
    day.into()
  }

  /// A week of `month`: its number, where the calendar shows them, and its seven days.
  pub fn week(&self, days: &[Date; 7], month: YearMonth) -> Row {
    let mut row = Row::new()
      .part(&self.part(CalendarPart::Week))
      .describe("a11y_role", "row");
    if self.week_numbers {
      // The week's number is of its Thursday by ISO 8601, wherever the week starts.
      let thursday = days
        .iter()
        .find(|day| day.weekday() == Weekday::Thursday)
        .copied()
        .unwrap_or(days[0]);
      let part = self.part(CalendarPart::WeekNumber);
      row = row.child(
        Row::new()
          .align_items(Alignment::Center)
          .justify(Justify::Center)
          .part(&part)
          .describe("a11y_role", "rowheader")
          .child(part.text(&thursday.iso_week().1.to_string()).nowrap()),
      );
    }
    for date in days {
      row = row.child(self.day(*date, month));
    }
    row
  }

  /// The days of the `index`th month: the names of the days over its six weeks. A `grid` to what reads the tree,
  /// called by its month.
  pub fn grid(&self, index: usize) -> Column {
    let month = self.months[index];
    let mut grid = Column::new()
      .part(&self.part(CalendarPart::Grid))
      .describe("a11y_role", "grid")
      .describe("a11y_name", self.locale.month_title(month))
      .child(self.weekdays());
    for week in month.weeks(self.first_day) {
      grid = grid.child(self.week(&week, month));
    }
    grid
  }

  /// How wide the days of a month are: seven days, after a week's number where the calendar shows them. `None` where
  /// a theme's rules give a day no width.
  fn days_width(&self) -> Option<f32> {
    let width = |part: CalendarPart| match self.part(part).rest().width?.resolve(self.theme())? {
      Dimension::Px(width) => Some(width),
      _ => None,
    };
    let days = width(CalendarPart::Day)? * 7.0;
    if self.week_numbers {
      Some(days + width(CalendarPart::WeekNumber)?)
    } else {
      Some(days)
    }
  }

  /// The headings of the months, side by side, each as wide as the days of its month under it. Where a day has no
  /// width the headings share the row, which then takes the room it is given.
  pub fn headers(&self) -> Row {
    let mut row = Row::new()
      .align_items(Alignment::Center)
      .part(&self.part(CalendarPart::Months));
    let width = self.days_width();
    for index in 0..self.months.len() {
      let header = self.header(index);
      row = row.child(match width {
        Some(width) => header.width(width),
        None => header.flex(1.0),
      });
    }
    row
  }

  /// The days of the months, side by side: one focus group, which Tab stops in once at the day [`Self::entry`] says,
  /// and whose keys are the days' own. The buttons of the headings are out of it, stops of their own.
  ///
  /// Where the panel reads the keys ([`CalendarKeys::Panel`]) the days are no group of the focus.
  pub fn grids(&self) -> Row {
    let mut row = Row::new()
      .align_items(Alignment::Start)
      .part(&self.part(CalendarPart::Months));
    if !self.panel_keys() {
      row = row.focus_group(FocusGroup::both().one_stop(true).wrap(false));
    }
    for index in 0..self.months.len() {
      row = row.child(self.grid(index));
    }
    row
  }

  /// The panel with nothing in it yet: what holds the headings over the days. Where it reads the keys
  /// ([`CalendarKeys::Panel`]) it says the day they are on to tooling (`cursor`), and takes them.
  pub fn panel(&self) -> Column {
    let mut panel = Column::new()
      .align_items(Alignment::Stretch)
      .part(&self.part(CalendarPart::Root));
    if self.panel_keys() {
      panel = panel.describe("cursor", self.cursor_day().to_string());
      if !self.disabled {
        panel = panel.on_key_down(self.panel_key_handler());
      }
    }
    if let Some(label) = &self.label {
      panel = panel
        .describe("a11y_role", "group")
        .describe("a11y_name", label.clone());
    }
    if let Some(id) = &self.id {
      panel = panel.id(id.clone());
    }
    if let Some(key) = &self.key {
      panel = panel.key(key.clone());
    }
    panel
  }

  /// What a key does at a panel that reads the keys: the arrows and PageUp and PageDown move the day the keys are on,
  /// showing its month, and Enter chooses it.
  fn panel_key_handler(&self) -> impl Fn(KeyboardEvent) + Send + Sync + 'static {
    let (model, at) = (self.model.clone(), self.cursor_day());
    move |event: KeyboardEvent| {
      if event.default_prevented() || event.ctrl || event.alt || event.meta || event.composing {
        return;
      }
      let at = model
        .cursor
        .get_untracked()
        .filter(|date| model.is_shown(*date))
        .unwrap_or(at);
      match event.key.as_str() {
        "Enter" if !event.focus_takes_enter => {
          event.prevent_default();
          model.choose(at);
        }
        "Home" | "End" => {}
        _ => {
          let Some(target) = model.step(at, &event) else {
            return;
          };
          event.prevent_default();
          model.show(target);
          model.point(target);
          if model.cursor.get_untracked() != Some(target) {
            model.cursor.set(Some(target));
          }
        }
      }
    }
  }

  /// The app's footer in the style of its part, across the panel: `None` where the calendar has none. It takes the
  /// footer out of the view.
  pub fn take_footer(&mut self) -> Option<Element> {
    let footer = self.footer.take()?;
    Some(
      Row::new()
        .align_items(Alignment::Center)
        .part(&self.part(CalendarPart::Footer))
        .child(footer)
        .into(),
    )
  }

  /// How the default renderer builds a calendar: the headings of its months over their days, in the panel, and the
  /// footer under them where it has one.
  pub fn render(mut self) -> Element {
    let footer = self.take_footer();
    self
      .panel()
      .child(self.headers())
      .child(self.grids())
      .with_children(footer)
      .into()
  }
}

impl<V: Picked> crate::ui::Styled for Calendar<V> {
  type Component = Self;

  fn rules(self, rules: &crate::ui::Recipe) -> Self {
    Self::rules(self, rules)
  }

  fn state(self, state: impl Into<crate::ui::Name>, on: bool) -> Self {
    Self::state(self, state, on)
  }
}

impl<V: Picked> Themed for Calendar<V> {
  const NAME: &'static str = "calendar";
  type Renderer = CalendarRenderer;

  fn parts() -> crate::ui::Declared {
    CalendarPart::declared()
  }

  fn states() -> crate::ui::Declared {
    CalendarState::declared()
  }

  /// The design's calendar (`pencil/lurq-design-system.pen`, Calendar): a panel in the popover's colors with an edge
  /// of 1 in the border's color and the large radius, 12 of padding, its months 16 apart. The headings are 8 over the
  /// days: a heading 32 high, the month and the year in the body's type at medium weight between two
  /// outline buttons of 28 in the color of an input's edge, the medium radius and no fill, their chevrons 16 in the
  /// muted foreground. The grid's rows are 8 apart: the names of the days in 12 and the muted foreground over six
  /// weeks of days 32 square with the medium radius, their numbers in 13. Under the pointer a day is on the highlight
  /// (the board's accent), and today is too; the chosen one is on the primary fill, its number in the primary's
  /// foreground at medium weight. A day of the month beside is in the muted foreground at half opacity, and one that
  /// cannot be chosen is the same, with no fill under the pointer. A range's ends are chosen days, and the days
  /// between them are on the highlight with no radius, so that they make one band (Date Range Picker); the other end
  /// a calendar of one date marks is on the highlight with its radius. A footer the
  /// app gives is under the months, 8 under a line of 1 in the border's color across the panel.
  fn recipe() -> Recipe {
    use CalendarPart::{Day, Footer, Grid, Header, Months, Nav, NavIcon, Root, Title, WeekNumber, Weekday};
    use CalendarState::{Bare, Current, Cursor, Disabled, InRange, NavAtEnd, OtherEnd, Outside, Selected, Unavailable};
    use PaletteColor::{
      Border as Edge, Foreground, Highlight, HighlightForeground, Input, MutedForeground, Popover, PopoverForeground,
      Primary, PrimaryForeground,
    };
    let none = Color::new(0, 0, 0, 0);
    let cell = |s: PartStyle| s.square(32.0);
    Recipe::new()
      .base(Root, |s| {
        s.padding_x(12.0)
          .padding_y(12.0)
          .gap(8.0)
          .radius(RadiusSize::Lg)
          .background(Popover)
          .foreground(PopoverForeground)
          .border(Border::inside(BorderSize::Sm, Edge))
      })
      .base(Months, |s| s.gap(16.0))
      .base(Footer, |s| {
        s.padding_top(8.0).top_line(Border::inside(BorderSize::Sm, Edge))
      })
      .base(Header, |s| s.height(32.0))
      .base(Title, |s| {
        s.typography(TypographyStyle::Body)
          .weight(FontWeight::Medium)
          .foreground(Foreground)
      })
      .base(Nav, |s| {
        s.square(28.0)
          .radius(RadiusSize::Md)
          .background(none)
          .border(Border::inside(BorderSize::Sm, Input))
      })
      .base(NavIcon, |s| s.square(16.0).foreground(MutedForeground))
      .base(Grid, |s| s.gap(8.0))
      .base(Weekday, |s| {
        cell(s)
          .typography(TypographyStyle::Body)
          .font_size(12.0)
          .foreground(MutedForeground)
      })
      .base(WeekNumber, |s| {
        cell(s)
          .typography(TypographyStyle::Body)
          .font_size(12.0)
          .foreground(MutedForeground)
      })
      .base(Day, |s| {
        fading(
          cell(s)
            .radius(RadiusSize::Md)
            .typography(TypographyStyle::Body)
            .font_size(13.0)
            .foreground(Foreground)
            .cursor(CursorIcon::Pointer),
        )
      })
      .hovered(Day, |s| s.background(Highlight).foreground(HighlightForeground))
      .focus_visible(Day, |s| s.focus_ring())
      .state(Current, Day, |s| {
        s.background(Highlight).foreground(HighlightForeground)
      })
      .state(Outside, Day, |s| s.foreground(MutedForeground).opacity(0.5))
      .state(Unavailable, Day, |s| {
        s.foreground(MutedForeground).opacity(0.5).cursor(CursorIcon::Default)
      })
      .when(Selector::new().state(Unavailable).hovered(), Day, |s| {
        s.background(none).foreground(MutedForeground)
      })
      .state(InRange, Day, |s| {
        s.background(Highlight)
          .foreground(HighlightForeground)
          .radius(RadiusValue::Px(0.0))
      })
      .state(OtherEnd, Day, |s| {
        s.background(Highlight).foreground(HighlightForeground)
      })
      .state(Selected, Day, |s| {
        s.background(Primary)
          .foreground(PrimaryForeground)
          .weight(FontWeight::Medium)
          .opacity(1.0)
      })
      .when(Selector::new().state(Selected).hovered(), Day, |s| {
        s.background(Primary).foreground(PrimaryForeground)
      })
      .state(Bare, Root, |s| {
        s.background(none)
          .border(Border::inside(0.0, Edge))
          .radius(RadiusValue::Px(0.0))
      })
      .state(Disabled, Root, |s| s.opacity(OpacityLevel::Disabled))
      .state(Disabled, Day, |s| s.cursor(CursorIcon::Default))
      // lurq's, the board has none: the day the keys are on is ringed as a focused day is, and the buttons at the end
      // of a heading are 4 apart.
      .state(Cursor, Day, |s| s.focus_ring())
      .state(NavAtEnd, Header, |s| s.gap(4.0))
  }
}

/// A calendar.
#[derive(Clone)]
struct CalendarProps<V: Picked> {
  calendar: Calendar<V>,
}

/// The calendar is built anew by every render of what holds it, so props are never the same as before.
impl<V: Picked> PartialEq for CalendarProps<V> {
  fn eq(&self, _: &Self) -> bool {
    false
  }
}

impl<V: Picked> crate::app::component::DevtoolsInspectable for CalendarProps<V> {}

/// What a mounted calendar keeps between renders.
struct CalendarHost<V: Picked> {
  memory: DateMemory,
  value: PhantomData<fn() -> V>,
}

impl<V: Picked> Component for CalendarHost<V> {
  type Props = CalendarProps<V>;

  fn create(_ctx: &mut Ctx) -> Self {
    Self {
      memory: DateMemory::new(),
      value: PhantomData,
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let mut calendar = ctx.props::<CalendarProps<V>>().calendar.clone();
    if calendar.memory.is_none() {
      calendar.memory = Some(self.memory.clone());
    }
    // The app's date text is read in the build, which is in this render: a change of it renders this again.
    calendar.build(&current_theme())
  }
}

/// Shows the month of what is chosen where it changed from outside since the last render, `last`, and is not shown.
pub(crate) fn follow<V: Picked>(last: &Mutex<Option<CalendarChoice>>, calendar: &Calendar<V>, month: &MonthSource) {
  let chosen = calendar.chosen_untracked();
  let changed = {
    let mut last = last.lock();
    let changed = last.is_some_and(|last| last != chosen);
    *last = Some(chosen);
    changed
  };
  let date = match chosen {
    CalendarChoice::Date(date) => date,
    CalendarChoice::Range(range) => range.map(DateRange::start),
  };
  if let (true, Some(date)) = (changed, date) {
    let first = month.get_untracked();
    let shown = YearMonth::of(date);
    if shown < first || shown > first.add_months(calendar.months as i32 - 1) {
      month.set(shown);
    }
  }
}
