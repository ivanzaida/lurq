//! An event calendar: the days of a week, a month or one day, each with the events of that day in it.

use std::{collections::HashMap, sync::Arc};

use parking_lot::Mutex;

use super::{
  button::{Button, ButtonPart, ButtonSize, ButtonVariant},
  calendar::step_date,
  icons, names,
  part_style::{Length, PartStyle},
  recipe::{Case, Interaction, Name, Recipe, ResolvedPart, ResolvedRecipe},
  shared::{fading, pressable},
  theme::{Themed, current_breakpoint, current_date_locale, current_theme},
};
use crate::{
  app::{
    component::Component,
    ctx::Ctx,
    events::{KeyboardEvent, MouseEvent},
    theme::{BorderSize, OpacityLevel, PaletteColor, RadiusSize, Theme, TypographyStyle},
  },
  components::{self, Column, Row, TextOverflow},
  core::{Date, DateRange, DateText, ElementRef, Signal, Weekday, YearMonth, shared_date_text},
  layout::{Alignment, layout_kind::Justify, text_style::FontWeight},
  node::{CursorIcon, Element, FocusGroup, Outline, border::Border, color::Color, dimension::Dimension},
};

/// The parts of an event calendar a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EventCalendarPart {
  /// The calendar: its fill, its edge and its corners.
  Root,
  /// The row at the top: the buttons that show the period before and after, the title, and the button that shows
  /// today.
  Header,
  /// What the header says of the period shown: the month, the week's days, the day.
  Title,
  /// What the calendar says of a button that shows the period before or after: rules over the button's root.
  Nav,
  /// What the calendar says of the glyph of those buttons: rules over the button's icon.
  NavIcon,
  /// The names of the days over the weeks, and the weeks, one under another.
  Grid,
  /// The row of the names of the days, over the weeks of a month.
  Weekdays,
  /// The name of a day of the week, at the head of its column.
  Weekday,
  /// A row of days.
  Week,
  /// A day: what is pressed, focused and chosen, with its label and its events.
  Day,
  /// The row at the top of a day that says which day it is: its number, after the name of its day where the
  /// calendar shows a week or a day.
  Label,
  /// The events of a day, one under another.
  Events,
  /// An event: what is pressed, in the fill and the ink of its kind.
  Event,
  /// What stands for the events of a day that are not shown: "+2 more".
  More,
  /// The row of dots under a day's number in a month narrower than seven days need ([`EventCalendarForm::Dots`]).
  Dots,
  /// A dot: one event, in the ink of its kind.
  Dot,
}

names!(EventCalendarPart {
  Root => "root",
  Header => "header",
  Title => "title",
  Nav => "nav",
  NavIcon => "nav_icon",
  Grid => "grid",
  Weekdays => "weekdays",
  Weekday => "weekday",
  Week => "week",
  Day => "day",
  Label => "label",
  Events => "events",
  Event => "event",
  More => "more",
  Dots => "dots",
  Dot => "dot",
});

/// The states of an event calendar a recipe styles, besides what the pointer and the keyboard do to it. The
/// calendar is in the period it shows, `Header` and `Disabled`; a day or a week is in the others that are its own.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EventCalendarState {
  /// It shows a month: weeks of days under the names of the days.
  Month,
  /// It shows a week: one row of days, as the design draws it.
  Week,
  /// It shows one day.
  Day,
  /// It has its header.
  Header,
  /// It takes no input.
  Disabled,
  /// The day is the chosen one.
  Selected,
  /// The day is today.
  Current,
  /// The day is of the month before or after the one shown.
  Outside,
  /// The day is the last of its row, or the week the last of the calendar: it has no line after it.
  Last,
  /// The event takes no input.
  Unavailable,
  /// It is a month too narrow for seven days with their events: each day shows its events as dots under its
  /// number ([`EventCalendarForm::Dots`]).
  Compact,
  /// It is a week too narrow for its days side by side, which scroll sideways under the header
  /// ([`EventCalendarForm::Scrolling`]).
  Scrolling,
}

names!(EventCalendarState {
  Month => "month",
  Week => "week",
  Day => "day",
  Header => "header",
  Disabled => "disabled",
  Selected => "selected",
  Current => "current",
  Outside => "outside",
  Last => "last",
  Unavailable => "unavailable",
  Compact => "compact",
  Scrolling => "scrolling",
});

/// What form a mounted event calendar takes in the room it is laid out in, measured before the frame is painted.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum EventCalendarForm {
  /// The days side by side, each with its events: the room holds them all.
  #[default]
  Full,
  /// A week whose days, each [`EventCalendar::WEEK_DAY_LEAST`] wide, are more than the room: they scroll sideways
  /// inside the calendar, under its header, which stands still.
  Scrolling,
  /// A week in a room under [`EventCalendar::DAY_VIEW_BELOW`]: the calendar shows one day, the chosen one or the
  /// first of the week, with its buttons going a day back and on.
  OneDay,
  /// A month in a room under seven days of [`EventCalendar::MONTH_DAY_LEAST`]: each day shows its number and a dot
  /// for each of its events, and a press of it tells `on_more`.
  Dots,
}

impl EventCalendarForm {
  /// The form of a calendar of `period`, `days` to a week, in a room `width` wide.
  pub fn for_room(period: EventCalendarPeriod, days: usize, width: f32) -> Self {
    match period {
      EventCalendarPeriod::Month if width < 7.0 * EventCalendar::<()>::MONTH_DAY_LEAST => Self::Dots,
      EventCalendarPeriod::Week if width < EventCalendar::<()>::DAY_VIEW_BELOW => Self::OneDay,
      EventCalendarPeriod::Week if width < days as f32 * EventCalendar::<()>::WEEK_DAY_LEAST => Self::Scrolling,
      _ => Self::Full,
    }
  }
}

/// What an event calendar shows at once.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum EventCalendarPeriod {
  /// The weeks of a month, under the names of the days.
  Month,
  /// The days of a week, side by side: the design's.
  #[default]
  Week,
  /// One day.
  Day,
}

names!(EventCalendarPeriod {
  Month => "month",
  Week => "week",
  Day => "day",
});

/// The kinds of an event, each a tone of the theme's.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum CalendarEventVariant {
  /// On the muted fill, in the foreground: an event of no kind.
  Neutral,
  /// A meeting, a call: what the design draws most.
  #[default]
  Info,
  /// A release, a thing that went well.
  Success,
  /// A review, a thing to look at.
  Warning,
  /// A freeze, a deadline.
  Destructive,
}

names!(CalendarEventVariant {
  Neutral => "neutral",
  Info => "info",
  Success => "success",
  Warning => "warning",
  Destructive => "destructive",
});

/// What builds an event calendar's nodes: a theme's in place of the default one.
pub type EventCalendarRenderer = Arc<dyn Fn(EventCalendarView) -> Element + Send + Sync>;

/// An event of an [`EventCalendar`]: what it is to the app, the day it is on, and what it is called.
///
/// ```
/// use lurq::{core::Date, ui::{CalendarEvent, CalendarEventVariant}};
///
/// let day = Date::new(2026, 11, 10).unwrap();
/// let release = CalendarEvent::new("release-0.60", day, "Release 0.60").variant(CalendarEventVariant::Success);
/// // An event that is nothing to the app but its press.
/// let retro = CalendarEvent::on(day, "Retro").on_select(|| println!("open the retro"));
/// ```
#[derive(Clone)]
pub struct CalendarEvent<T = ()> {
  value: T,
  date: Date,
  title: Arc<str>,
  variant: Name,
  disabled: bool,
  on_select: Vec<Arc<dyn Fn() + Send + Sync>>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl CalendarEvent<()> {
  /// An event called `title` on `date` that is no value to the app: its own [`on_select`](Self::on_select) hears
  /// its press.
  pub fn on(date: Date, title: &str) -> Self {
    Self::new((), date, title)
  }
}

impl<T> CalendarEvent<T> {
  /// An event called `title` on `date`. `value` is what the calendar's [`EventCalendar::on_select`] is told when it
  /// is pressed: its id, the record it is of.
  pub fn new(value: T, date: Date, title: &str) -> Self {
    Self {
      value,
      date,
      title: Arc::from(title),
      variant: CalendarEventVariant::default().into(),
      disabled: false,
      on_select: Vec::new(),
      id: None,
      key: None,
    }
  }

  /// Its kind: a [`CalendarEventVariant`], or a variant the theme's recipe names.
  pub fn variant(mut self, variant: impl Into<Name>) -> Self {
    self.variant = variant.into();
    self
  }

  /// Whether the event takes no input.
  pub fn disabled(mut self, disabled: bool) -> Self {
    self.disabled = disabled;
    self
  }

  /// Called when the event is pressed, before the calendar's own `on_select`. A second handler is called after the
  /// first.
  pub fn on_select(mut self, handler: impl Fn() + Send + Sync + 'static) -> Self {
    self.on_select.push(Arc::new(handler));
    self
  }

  /// The id of the event's element.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// The day it is on.
  pub fn date(&self) -> Date {
    self.date
  }

  /// What it is called.
  pub fn title(&self) -> &str {
    &self.title
  }
}

/// The date whose period a calendar shows: the app's, or its own, which is `start` until it is set.
#[derive(Clone)]
enum ShownSource {
  App(Signal<Date>),
  Own(Signal<Option<Date>>, Date),
}

impl ShownSource {
  /// The date, read so that the render reading it is run again when it changes.
  fn get(&self) -> Date {
    match self {
      Self::App(shown) => shown.get(),
      Self::Own(shown, start) => shown.get().unwrap_or(*start),
    }
  }

  fn get_untracked(&self) -> Date {
    match self {
      Self::App(shown) => shown.get_untracked(),
      Self::Own(shown, start) => shown.get_untracked().unwrap_or(*start),
    }
  }

  fn set(&self, date: Date) {
    if self.get_untracked() == date {
      return;
    }
    match self {
      Self::App(shown) => shown.set(date),
      Self::Own(shown, _) => shown.set(Some(date)),
    }
  }
}

/// A calendar that shows events in its days: the days of a week side by side, the weeks of a month, or one day,
/// each day with what is on it. The meetings of a team, the releases of a project, what is due.
///
/// ```
/// use lurq::{
///   core::{Date, Signal},
///   ui::{CalendarEvent, CalendarEventVariant, EventCalendar, EventCalendarPeriod},
/// };
///
/// # fn render(ctx: &mut lurq::app::ctx::Ctx) {
/// let day = |day: u8| Date::new(2026, 11, day).unwrap();
/// let chosen = Signal::new(Some(day(12)));
/// let events = vec![
///   CalendarEvent::new("standup", day(9), "Standup"),
///   CalendarEvent::new("release", day(10), "Release 0.60").variant(CalendarEventVariant::Success),
///   CalendarEvent::new("freeze", day(13), "Deploy freeze").variant(CalendarEventVariant::Destructive),
/// ];
/// // Mounted, it keeps the week it shows; a press of an event tells which.
/// let week = EventCalendar::new(chosen.clone(), events.clone())
///   .weekends(false)
///   .on_select(|event: &&str| println!("open {event}"))
///   .mount(ctx);
/// // A month, of which a day shows three events at most and "+2 more".
/// let month = EventCalendar::new(chosen, events)
///   .period(EventCalendarPeriod::Month)
///   .max_shown(3)
///   .mount(ctx);
/// # }
/// ```
///
/// The calendar does not keep what is chosen: the app's signal does, which a press of a day sets. The period it
/// shows is not what is chosen: it is the period of the app's date where it gives one ([`EventCalendar::shown`]),
/// else the calendar keeps it where it is mounted ([`EventCalendar::mount`]), starting at the chosen day, or today.
/// Converted into an element on its own it keeps nothing: its buttons show no other period unless the app gives it
/// the date.
///
/// Its days are the days of a [`Calendar`](super::Calendar): the weeks of a month are the same grid
/// (`YearMonth::weeks`), the keys the same (Left and Right a day, Up and Down a week, Home and End the ends of the
/// week, PageUp and PageDown a month and, with Shift, a year, into a period that is then shown), and the names of
/// the days and the months are the app's date locale's. The days are one stop of Tab, at the chosen day, today or
/// the first day shown; Enter and Space choose the focused day, and Tab goes on from a day into its events.
///
/// To tooling the days are a `grid` called by the period shown, of `gridcell`s that say whether they are chosen
/// (`a11y_selected`) and today (`a11y_current`); an event is a `button` called by its title.
#[derive(Clone)]
pub struct EventCalendar<T = ()> {
  value: Signal<Option<Date>>,
  controlled: bool,
  events: Vec<CalendarEvent<T>>,
  period: EventCalendarPeriod,
  shown: Option<Signal<Date>>,
  /// Where a mounted calendar keeps the date whose period it shows.
  kept: Option<Signal<Option<Date>>>,
  weekends: bool,
  header: bool,
  max_shown: Option<usize>,
  first_day: Option<Weekday>,
  locale: Option<Arc<dyn DateText>>,
  today: Option<Date>,
  disabled: bool,
  on_change: Vec<Arc<dyn Fn(Date) + Send + Sync>>,
  on_select: Vec<Arc<dyn Fn(&T) + Send + Sync>>,
  on_more: Vec<Arc<dyn Fn(Date) + Send + Sync>>,
  label: Option<Arc<str>>,
  previous_label: Option<Arc<str>>,
  next_label: Option<Arc<str>>,
  today_label: Option<Arc<str>>,
  more_text: Arc<dyn Fn(usize) -> String + Send + Sync>,
  forced: Option<Interaction>,
  states: Vec<Name>,
  rules: Recipe,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
  /// The form the room it was measured in gives it, and the ref it is measured by: a mounted calendar's.
  form: EventCalendarForm,
  room: Option<ElementRef>,
}

impl EventCalendar<()> {
  /// The least width of a day of a week: under seven (or five) of them the week scrolls sideways.
  pub const WEEK_DAY_LEAST: f32 = 110.0;
  /// The room under which a week shows one day: a scroll that holds fewer than three days at a time is no week.
  pub const DAY_VIEW_BELOW: f32 = 3.0 * Self::WEEK_DAY_LEAST;
  /// The least width of a day of a month with its events: under seven of them each day shows dots.
  pub const MONTH_DAY_LEAST: f32 = 80.0;
}

impl<T: Clone + Send + Sync + 'static> EventCalendar<T> {
  /// A calendar of `events` bound to `value`, the chosen day, which a press of a day sets. None is chosen to begin
  /// with where it holds none.
  pub fn new(value: Signal<Option<Date>>, events: impl IntoIterator<Item = CalendarEvent<T>>) -> Self {
    Self {
      value,
      controlled: false,
      events: events.into_iter().collect(),
      period: EventCalendarPeriod::default(),
      shown: None,
      kept: None,
      weekends: true,
      header: true,
      max_shown: None,
      first_day: None,
      locale: None,
      today: None,
      disabled: false,
      on_change: Vec::new(),
      on_select: Vec::new(),
      on_more: Vec::new(),
      label: None,
      previous_label: None,
      next_label: None,
      today_label: None,
      more_text: Arc::new(|count| format!("+{count} more")),
      forced: None,
      states: Vec::new(),
      rules: Recipe::new(),
      id: None,
      key: None,
      form: EventCalendarForm::Full,
      room: None,
    }
  }

  /// A calendar that shows `value` chosen and tells `on_change` of a press of a day, which it does not set: the app
  /// answers with the value it renders it with next.
  pub fn controlled(
    value: Option<Date>,
    events: impl IntoIterator<Item = CalendarEvent<T>>,
    on_change: impl Fn(Date) + Send + Sync + 'static,
  ) -> Self {
    let mut calendar = Self::new(Signal::new(value), events);
    calendar.controlled = true;
    calendar.on_change(on_change)
  }

  /// Adds an event.
  pub fn event(mut self, event: CalendarEvent<T>) -> Self {
    self.events.push(event);
    self
  }

  /// Adds events.
  pub fn events(mut self, events: impl IntoIterator<Item = CalendarEvent<T>>) -> Self {
    self.events.extend(events);
    self
  }

  /// What the calendar shows at once: a week (the default, the design's), a month, or one day.
  pub fn period(mut self, period: EventCalendarPeriod) -> Self {
    self.period = period;
    self
  }

  /// The date whose week, month or day the calendar shows, the app's. The buttons of its header and the keys that
  /// leave the period set it.
  pub fn shown(mut self, shown: Signal<Date>) -> Self {
    self.shown = Some(shown);
    self
  }

  /// Whether Saturdays and Sundays are shown: they are unless told. Without them a week is the five days the
  /// design draws.
  pub fn weekends(mut self, weekends: bool) -> Self {
    self.weekends = weekends;
    self
  }

  /// Whether the calendar has its header: the buttons that show the period before and after, what the period is,
  /// and the button that shows today. It has unless told; the design draws the days alone.
  pub fn header(mut self, header: bool) -> Self {
    self.header = header;
    self
  }

  /// How many events of a day are shown at most: with more, one fewer are shown, and "+N more" in the place of the
  /// last. Four in a week and three in a month unless told, as many as their days are high; a day alone shows all.
  pub fn max_shown(mut self, max: usize) -> Self {
    self.max_shown = Some(max.max(1));
    self
  }

  /// The day the weeks start on, in place of the locale's.
  pub fn first_day(mut self, first: Weekday) -> Self {
    self.first_day = Some(first);
    self
  }

  /// What the calendar names its months, its days and its buttons in and starts its weeks by, in place of the
  /// app's ([`App::set_date_locale`](crate::app::App::set_date_locale), English where it set none).
  pub fn locale(mut self, locale: impl DateText + 'static) -> Self {
    self.locale = Some(shared_date_text(locale));
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

  /// Called when a person chooses another day, with it: a press or Enter on a day, or a press of its "+N more".
  /// Bound to a signal, the value is set already. Not called for the day chosen already, nor when the app sets the
  /// value. A second handler is called after the first.
  pub fn on_change(mut self, handler: impl Fn(Date) + Send + Sync + 'static) -> Self {
    self.on_change.push(Arc::new(handler));
    self
  }

  /// Called when an event is pressed, with what the event is to the app, after the event's own handlers. A press
  /// of an event does not choose its day. A second handler is called after the first.
  pub fn on_select(mut self, handler: impl Fn(&T) + Send + Sync + 'static) -> Self {
    self.on_select.push(Arc::new(handler));
    self
  }

  /// Called when the "+N more" of a day is pressed, with the day, after the day is chosen: an app shows the day
  /// alone, or a list of what is on it.
  pub fn on_more(mut self, handler: impl Fn(Date) + Send + Sync + 'static) -> Self {
    self.on_more.push(Arc::new(handler));
    self
  }

  /// What the calendar is called to what reads the tree: its box, which is then a `group`. Its days are a `grid`
  /// called by the period shown whatever this says.
  pub fn label(mut self, label: &str) -> Self {
    self.label = Some(Arc::from(label));
    self
  }

  /// What the button that shows the period before is called to what reads the tree, in place of the locale's
  /// "Previous month", "Previous week" or "Previous day".
  pub fn previous_label(mut self, label: &str) -> Self {
    self.previous_label = Some(Arc::from(label));
    self
  }

  /// What the button that shows the period after is called, in place of the locale's.
  pub fn next_label(mut self, label: &str) -> Self {
    self.next_label = Some(Arc::from(label));
    self
  }

  /// What the button that shows today says, in place of the locale's "Today".
  pub fn today_label(mut self, label: &str) -> Self {
    self.today_label = Some(Arc::from(label));
    self
  }

  /// What stands for the events of a day that are not shown, given how many: "+2 more" unless told.
  pub fn more_text(mut self, text: impl Fn(usize) -> String + Send + Sync + 'static) -> Self {
    self.more_text = Arc::new(text);
    self
  }

  /// Draws the day Tab would enter at (the chosen one, today, or the first shown) as it is under the pointer, held
  /// down or focused from the keyboard, with nothing pointing at it: for a gallery or a test.
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

  /// The id of its box. Each day has it with `-` and the date after it: `team-2026-11-12`; the buttons have it with
  /// `-previous`, `-next` and `-today`, and the "+N more" of a day with `-more-` and the date.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// The calendar mounted where it is declared: it keeps the period it shows where the app gives no date for it,
  /// follows the chosen day to its period when the app sets it to a day that is not shown, and names its months and
  /// days as the app says.
  pub fn mount(self, ctx: &mut Ctx) -> Element {
    let key = self.key.clone();
    let mut element = ctx.mount::<Host<T>>(Props { calendar: self });
    if let Some(key) = key {
      element = element.key(key);
    }
    element
  }

  /// Builds the calendar in `theme`. Converting a calendar into an element does this in the theme of what is
  /// being rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let view = self.view(theme);
    match theme.renderer::<EventCalendar>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }

  fn view(self, theme: &Theme) -> EventCalendarView {
    let locale = self.locale.clone().unwrap_or_else(current_date_locale);
    let today = self.today.unwrap_or_else(Date::today);
    let first_day = self.first_day.unwrap_or_else(|| locale.first_day());
    let chosen = self.value.get();
    // A week in a room under a few days' shows one day of it.
    let period = match (self.period, self.form) {
      (EventCalendarPeriod::Week, EventCalendarForm::OneDay) => EventCalendarPeriod::Day,
      (period, _) => period,
    };
    let source = match (&self.shown, &self.kept) {
      (Some(shown), _) => ShownSource::App(shown.clone()),
      (None, Some(kept)) => ShownSource::Own(kept.clone(), chosen.unwrap_or(today)),
      (None, None) => ShownSource::Own(Signal::new(None), chosen.unwrap_or(today)),
    };
    let model = Model {
      value: self.value,
      controlled: self.controlled,
      shown: source,
      period,
      first_day,
      weekends: self.weekends,
      on_change: self.on_change,
      on_more: self.on_more,
    };
    let anchor = model.settled(model.shown.get());
    let case = self
      .states
      .iter()
      .fold(Case::new().breakpoint(current_breakpoint()), |case, state| {
        case.state(*state, true)
      })
      .state(EventCalendarState::Month, period == EventCalendarPeriod::Month)
      .state(EventCalendarState::Week, period == EventCalendarPeriod::Week)
      .state(EventCalendarState::Day, period == EventCalendarPeriod::Day)
      .state(EventCalendarState::Compact, self.form == EventCalendarForm::Dots)
      .state(EventCalendarState::Scrolling, self.form == EventCalendarForm::Scrolling)
      .state(EventCalendarState::Header, self.header)
      .state(EventCalendarState::Disabled, self.disabled);
    // What an event is to the app is out of sight of the view: its press tells the handlers its value.
    let on_select = self.on_select;
    let events = self
      .events
      .into_iter()
      .map(|event| {
        let (own, told, value) = (event.on_select, on_select.clone(), event.value);
        let press: Arc<dyn Fn() + Send + Sync> = Arc::new(move || {
          for handler in &own {
            handler();
          }
          for handler in &told {
            handler(&value);
          }
        });
        ShownEvent {
          date: event.date,
          title: event.title,
          variant: event.variant,
          disabled: event.disabled,
          press,
          id: event.id,
          key: event.key,
        }
      })
      .collect();
    EventCalendarView {
      period,
      form: self.form,
      anchor,
      first_day,
      today,
      chosen,
      weekends: self.weekends,
      header: self.header,
      max_shown: self.max_shown.or(match period {
        EventCalendarPeriod::Month => Some(3),
        EventCalendarPeriod::Week => Some(4),
        EventCalendarPeriod::Day => None,
      }),
      disabled: self.disabled,
      events,
      recipe: theme.resolve::<EventCalendar>(&case, Some(&self.rules)),
      recipes: Mutex::new(HashMap::new()),
      case,
      model,
      locale,
      label: self.label,
      previous_label: self.previous_label,
      next_label: self.next_label,
      today_label: self.today_label,
      more_text: self.more_text,
      forced: self.forced,
      rules: self.rules,
      id: self.id,
      key: self.key,
      room: self.room,
    }
  }
}

impl<T: Clone + Send + Sync + 'static> From<EventCalendar<T>> for Element {
  fn from(calendar: EventCalendar<T>) -> Self {
    calendar.build(&current_theme())
  }
}

/// An event as the view shows it: what it is to the app is in what its press does.
pub struct ShownEvent {
  /// The day it is on.
  pub date: Date,
  pub title: Arc<str>,
  /// Its kind.
  pub variant: Name,
  pub disabled: bool,
  press: Arc<dyn Fn() + Send + Sync>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

/// What the handlers of an event calendar know of it: what it chooses, what it shows, and how its days run.
#[derive(Clone)]
struct Model {
  value: Signal<Option<Date>>,
  controlled: bool,
  shown: ShownSource,
  period: EventCalendarPeriod,
  first_day: Weekday,
  weekends: bool,
  on_change: Vec<Arc<dyn Fn(Date) + Send + Sync>>,
  on_more: Vec<Arc<dyn Fn(Date) + Send + Sync>>,
}

impl Model {
  /// Whether `date` is a day the calendar shows a column for.
  fn has(&self, date: Date) -> bool {
    self.weekends || date.weekday().days_from_monday() < 5
  }

  /// `date`, or where the calendar hides weekends and it is on one, the Monday after it (`back`: the Friday
  /// before).
  fn off_weekend(&self, date: Date, back: bool) -> Date {
    let mut date = date;
    while !self.has(date) {
      date = date.add_days(if back { -1 } else { 1 });
    }
    date
  }

  /// The date a period is shown by: `date`, off a weekend the calendar hides where it shows one day.
  fn settled(&self, date: Date) -> Date {
    match self.period {
      EventCalendarPeriod::Day => self.off_weekend(date, false),
      _ => date,
    }
  }

  /// The first and the last day of the period `anchor` is in.
  fn bounds(&self, anchor: Date) -> (Date, Date) {
    match self.period {
      EventCalendarPeriod::Month => {
        let month = YearMonth::of(anchor);
        (month.first(), month.last())
      }
      EventCalendarPeriod::Week => (anchor.start_of_week(self.first_day), anchor.end_of_week(self.first_day)),
      EventCalendarPeriod::Day => (anchor, anchor),
    }
  }

  /// Chooses `date`, where it is not the chosen one.
  fn choose(&self, date: Date) {
    if self.value.get_untracked() == Some(date) {
      return;
    }
    if !self.controlled {
      self.value.set(Some(date));
    }
    for handler in &self.on_change {
      handler(date);
    }
  }

  /// Shows the period `date` is in, where it is not shown.
  fn show(&self, date: Date) {
    let (first, last) = self.bounds(self.settled(self.shown.get_untracked()));
    if date < first || date > last {
      self.shown.set(date);
    }
  }

  /// Shows the period before (`by` -1) or after (1) the one shown.
  fn turn(&self, by: i32) {
    let anchor = self.settled(self.shown.get_untracked());
    self.shown.set(match self.period {
      EventCalendarPeriod::Month => anchor.add_months(by),
      EventCalendarPeriod::Week => anchor.add_days(7 * by as i64),
      EventCalendarPeriod::Day => self.off_weekend(anchor.add_days(by as i64), by < 0),
    });
  }

  /// The date a key takes the focus to from `date`, where the key is one of the days': a calendar's, past the
  /// weekends where they are hidden.
  fn step(&self, date: Date, event: &KeyboardEvent) -> Option<Date> {
    let target = step_date(date, event, self.first_day)?;
    // Off a hidden weekend the way the key went: End to the Friday before it, Home to the Monday after.
    let back = match event.key.as_str() {
      "End" => true,
      "Home" => false,
      _ => target < date,
    };
    Some(self.off_weekend(target, back))
  }
}

/// The key of a day of the period shown: what the keys move the focus to.
fn day_key(date: Date) -> String {
  format!("day-{date}")
}

/// An event calendar as a renderer sees it: the period it shows, its events, the style of its parts, and its
/// behavior ready to be dressed.
pub struct EventCalendarView {
  /// The period shown: the app's, or a day where a week has too little room for more.
  pub period: EventCalendarPeriod,
  /// The form its room gives it.
  pub form: EventCalendarForm,
  /// The date whose period is shown.
  pub anchor: Date,
  pub first_day: Weekday,
  pub today: Date,
  /// The chosen day.
  pub chosen: Option<Date>,
  /// Whether Saturdays and Sundays are shown.
  pub weekends: bool,
  /// Whether it has its header.
  pub header: bool,
  /// How many events of a day are shown at most: all with `None`.
  pub max_shown: Option<usize>,
  pub disabled: bool,
  /// Its events, in the order the app gave them.
  pub events: Vec<ShownEvent>,
  /// What the calendar's text is: its own `.locale(..)`, else the app's, else English.
  pub locale: Arc<dyn DateText>,
  /// The style of each part of this calendar.
  pub recipe: ResolvedRecipe,
  /// The recipe of a day, a week or an event in each set of its states and its variant, resolved once for each.
  recipes: Mutex<HashMap<(Option<Name>, Vec<Name>), ResolvedRecipe>>,
  case: Case,
  model: Model,
  label: Option<Arc<str>>,
  previous_label: Option<Arc<str>>,
  next_label: Option<Arc<str>>,
  today_label: Option<Arc<str>>,
  more_text: Arc<dyn Fn(usize) -> String + Send + Sync>,
  forced: Option<Interaction>,
  rules: Recipe,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
  room: Option<ElementRef>,
}

impl EventCalendarView {
  /// The style of a part, as this calendar is to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// The recipe of what is in `states` too, and of the kind `variant`: a day's, a week's, an event's.
  pub fn recipe_in(&self, variant: Option<Name>, states: &[EventCalendarState]) -> ResolvedRecipe {
    let names: Vec<Name> = states.iter().map(|state| (*state).into()).collect();
    self
      .recipes
      .lock()
      .entry((variant, names.clone()))
      .or_insert_with(|| {
        let mut case = names
          .iter()
          .fold(self.case.clone(), |case, state| case.state(*state, true));
        case.variant = variant;
        self.theme().resolve::<EventCalendar>(&case, Some(&self.rules))
      })
      .clone()
  }

  /// The first and the last day of the period shown: of the month, the week, or the day itself.
  pub fn bounds(&self) -> (Date, Date) {
    self.model.bounds(self.anchor)
  }

  /// The rows of days shown: the weeks of the month, the week, or the day, without the weekends where the calendar
  /// hides them.
  pub fn weeks(&self) -> Vec<Vec<Date>> {
    let shown = |days: &[Date]| -> Vec<Date> { days.iter().copied().filter(|date| self.model.has(*date)).collect() };
    match self.period {
      // The grid of a calendar's month: its weeks, filled with the days of the months beside it.
      EventCalendarPeriod::Month => YearMonth::of(self.anchor)
        .weeks(self.first_day)
        .iter()
        .map(|week| shown(week))
        .collect(),
      EventCalendarPeriod::Week => {
        let first = self.anchor.start_of_week(self.first_day);
        let week: Vec<Date> = (0..7).map(|day| first.add_days(day)).collect();
        vec![shown(&week)]
      }
      EventCalendarPeriod::Day => vec![vec![self.anchor]],
    }
  }

  /// Whether `date` is of the period shown: in a month, not a day of the month before or after.
  pub fn within(&self, date: Date) -> bool {
    let (first, last) = self.bounds();
    date >= first && date <= last
  }

  /// The day Tab enters the days at: the chosen one where it is shown, else today where it is, else the first day
  /// shown of the period.
  pub fn entry(&self) -> Date {
    let shown = |date: &Date| self.within(*date) && self.model.has(*date);
    self
      .chosen
      .filter(shown)
      .or_else(|| Some(self.today).filter(shown))
      .unwrap_or_else(|| self.model.off_weekend(self.bounds().0, false))
  }

  /// What the header says of the period shown: the month and the year, the first and the last day shown of the
  /// week, or the day.
  pub fn title_text(&self) -> String {
    match self.period {
      EventCalendarPeriod::Month => self.locale.month_title(YearMonth::of(self.anchor)),
      EventCalendarPeriod::Week => {
        let week = self.weeks().remove(0);
        match (week.first(), week.last()) {
          (Some(first), Some(last)) => self.locale.format_range(DateRange::new(*first, *last)),
          _ => String::new(),
        }
      }
      EventCalendarPeriod::Day => self.locale.day_label(self.anchor),
    }
  }

  /// The id of an element of the calendar: its own id with `suffix` after it.
  fn id_of(&self, suffix: &str) -> Option<String> {
    self.id.as_ref().map(|id| format!("{id}-{suffix}"))
  }

  /// A button that shows the period before (`by` -1) or after (1): an outline button of an icon, as a calendar's,
  /// called by what it does.
  pub fn nav(&self, by: i32) -> Element {
    let (given, icon, suffix) = if by < 0 {
      (&self.previous_label, icons::CHEVRON_LEFT, "previous")
    } else {
      (&self.next_label, icons::CHEVRON_RIGHT, "next")
    };
    let label = given.as_deref().map_or_else(
      || match (self.period, by < 0) {
        (EventCalendarPeriod::Month, true) => self.locale.previous_month_label(),
        (EventCalendarPeriod::Month, false) => self.locale.next_month_label(),
        (EventCalendarPeriod::Week, true) => self.locale.previous_week_label(),
        (EventCalendarPeriod::Week, false) => self.locale.next_week_label(),
        (EventCalendarPeriod::Day, true) => self.locale.previous_day_label(),
        (EventCalendarPeriod::Day, false) => self.locale.next_day_label(),
      },
      str::to_owned,
    );
    let rules = self
      .recipe
      .rules_for(EventCalendarPart::Nav, ButtonPart::Root)
      .extend(&self.recipe.rules_for(EventCalendarPart::NavIcon, ButtonPart::Icon));
    let model = self.model.clone();
    let mut button = Button::new(&label)
      .variant(ButtonVariant::Outline)
      .size(ButtonSize::Xs)
      .icon(icon)
      .icon_only(true)
      .disabled(self.disabled)
      .rules(&rules)
      .on_click(move |_: MouseEvent| model.turn(by));
    if let Some(id) = self.id_of(suffix) {
      button = button.id(id);
    }
    button.build(self.theme())
  }

  /// The button that shows the period today is in: an outline button that says "Today" in the locale's word, or
  /// the app's.
  pub fn today_button(&self) -> Element {
    let label = self
      .today_label
      .as_deref()
      .map_or_else(|| self.locale.today_label(), str::to_owned);
    let (model, today) = (self.model.clone(), self.today);
    let mut button = Button::new(&label)
      .variant(ButtonVariant::Outline)
      .size(ButtonSize::Xs)
      .disabled(self.disabled)
      .on_click(move |_: MouseEvent| model.shown.set(model.settled(today)));
    if let Some(id) = self.id_of("today") {
      button = button.id(id);
    }
    button.build(self.theme())
  }

  /// The header: the buttons that show the period before and after, what the period is, and the button that shows
  /// today. A title longer than its room is cut with an ellipsis: the buttons keep theirs.
  pub fn header(&self) -> Row {
    let title = self.part(EventCalendarPart::Title);
    Row::new()
      .align_items(Alignment::Center)
      .part(&self.part(EventCalendarPart::Header))
      .child(self.nav(-1))
      .child(self.nav(1))
      .child(
        Row::new().flex(1.0).min_width(Dimension::Px(0.0)).part(&title).child(
          title
            .text(&self.title_text())
            .nowrap()
            .text_overflow(TextOverflow::Elipsis),
        ),
      )
      .child(self.today_button())
  }

  /// The row of the names of the days over the weeks of a month, each as wide as its column.
  pub fn weekdays(&self) -> Row {
    let part = self.part(EventCalendarPart::Weekday);
    let mut row = Row::new()
      .align_items(Alignment::Stretch)
      .part(&self.part(EventCalendarPart::Weekdays))
      .describe("a11y_role", "row");
    for index in 0..7 {
      let day = Weekday::from_monday(self.first_day as i64 + index);
      if !self.weekends && day.days_from_monday() >= 5 {
        continue;
      }
      row = row.child(
        Row::new()
          .flex(1.0)
          .min_width(Dimension::Px(0.0))
          .align_items(Alignment::Center)
          .justify(if self.form == EventCalendarForm::Dots {
            Justify::Center
          } else {
            Justify::Start
          })
          .part(&part)
          .describe("a11y_role", "columnheader")
          .describe("a11y_name", self.locale.weekday_name(day))
          .child(
            part
              .text(&self.locale.weekday_medium(day))
              .nowrap()
              .text_overflow(TextOverflow::Elipsis),
          ),
      );
    }
    row
  }

  /// What the label of a day says: its number, after the name of its day where the calendar shows a week or a
  /// day ("Thu 12").
  pub fn day_text(&self, date: Date) -> String {
    match self.period {
      EventCalendarPeriod::Month => date.day().to_string(),
      _ => format!("{} {}", self.locale.weekday_medium(date.weekday()), date.day()),
    }
  }

  /// The states of a day, `last` where it is the last of its row.
  pub fn day_states(&self, date: Date, last: bool) -> Vec<EventCalendarState> {
    let mut states = Vec::new();
    if !self.within(date) {
      states.push(EventCalendarState::Outside);
    }
    if date == self.today {
      states.push(EventCalendarState::Current);
    }
    if self.chosen == Some(date) {
      states.push(EventCalendarState::Selected);
    }
    if last {
      states.push(EventCalendarState::Last);
    }
    states
  }

  /// An event: what is pressed, in the fill and the ink of its kind, its title on one line cut with an ellipsis in
  /// a week and a month, and wrapping in the day view, where the room is the day's. A `button` to what reads the
  /// tree. Its press is its own: it does not choose the day it is on.
  pub fn event(&self, event: &ShownEvent) -> components::Button {
    let states: &[EventCalendarState] = if event.disabled {
      &[EventCalendarState::Unavailable]
    } else {
      &[]
    };
    let part = self
      .recipe_in(Some(event.variant), states)
      .part(EventCalendarPart::Event);
    let mut pill = pressable(&part, self.disabled || event.disabled)
      .justify(Justify::Start)
      .align_items(Alignment::Center)
      .describe("a11y_role", "button")
      .describe("a11y_name", event.title.clone())
      .child(if self.period == EventCalendarPeriod::Day {
        // A text wraps in a column of the room it has, not in a row.
        Column::new()
          .width(Dimension::Pct(100.0))
          .min_width(Dimension::Px(0.0))
          .child(part.text(&event.title).width(Dimension::Pct(100.0)))
          .into()
      } else {
        Element::from(
          Row::new()
            .flex(1.0)
            .min_width(Dimension::Px(0.0))
            .child(part.text(&event.title).nowrap().text_overflow(TextOverflow::Elipsis)),
        )
      });
    let press = event.press.clone();
    pill = pill.on_click(move |click: MouseEvent| {
      click.stop_propagation();
      press();
    });
    if let Some(id) = &event.id {
      pill = pill.id(id.clone());
    }
    if let Some(key) = &event.key {
      pill = pill.key(key.clone());
    }
    pill
  }

  /// What stands for the `count` events of `date` that are not shown: pressed, it chooses the day and tells the
  /// app.
  pub fn more(&self, date: Date, count: usize) -> components::Button {
    let part = self.part(EventCalendarPart::More);
    let text = (self.more_text)(count);
    let model = self.model.clone();
    let mut more = pressable(&part, self.disabled)
      .justify(Justify::Start)
      .align_items(Alignment::Center)
      .describe("a11y_role", "button")
      .describe("a11y_name", Arc::<str>::from(text.as_str()))
      .child(
        Row::new()
          .flex(1.0)
          .min_width(Dimension::Px(0.0))
          .child(part.text(&text).nowrap().text_overflow(TextOverflow::Elipsis)),
      )
      .on_click(move |click: MouseEvent| {
        click.stop_propagation();
        model.choose(date);
        for handler in &model.on_more {
          handler(date);
        }
      });
    if let Some(id) = self.id_of(&format!("more-{date}")) {
      more = more.id(id);
    }
    more
  }

  /// The events of `date`, one under another: all of them, or one fewer than the most a day shows and what stands
  /// for the rest.
  pub fn events(&self, date: Date) -> Column {
    let on: Vec<&ShownEvent> = self.events.iter().filter(|event| event.date == date).collect();
    let shown = match self.max_shown {
      Some(max) if on.len() > max => max - 1,
      _ => on.len(),
    };
    let mut events = Column::new()
      .align_items(Alignment::Stretch)
      .part(&self.part(EventCalendarPart::Events));
    for event in &on[..shown] {
      events = events.child(self.event(event));
    }
    if shown < on.len() {
      events = events.child(self.more(date, on.len() - shown));
    }
    events
  }

  /// The events of `date` as dots in a row, each in the ink of its kind, where a month is too narrow for them
  /// ([`EventCalendarForm::Dots`]): what they are is told by a press of the day, to `on_more`.
  pub fn dots(&self, date: Date) -> Row {
    let mut dots = Row::new()
      .align_items(Alignment::Center)
      .justify(Justify::Center)
      .wrap()
      .part(&self.part(EventCalendarPart::Dots));
    for event in self.events.iter().filter(|event| event.date == date) {
      let part = self.recipe_in(Some(event.variant), &[]).part(EventCalendarPart::Dot);
      dots = dots.child(Row::new().part(&part));
    }
    dots
  }

  /// A day: what is pressed, focused and chosen, with its label over its events. A `gridcell` to what reads the
  /// tree. Tab goes on from it into its events. A day of the month beside is pressed to choose it and show its
  /// month, and is no stop of the keys.
  pub fn day(&self, date: Date, last: bool) -> Element {
    let states = self.day_states(date, last);
    let recipe = self.recipe_in(None, &states);
    let outside = states.contains(&EventCalendarState::Outside);
    let entry = !outside && date == self.entry();
    let part = recipe
      .part(EventCalendarPart::Day)
      .forced(self.forced.filter(|_| entry));
    let label = recipe.part(EventCalendarPart::Label);
    let selected = states.contains(&EventCalendarState::Selected);
    // In a week that scrolls, a day keeps its least width.
    let least = match self.form {
      EventCalendarForm::Scrolling => EventCalendar::<()>::WEEK_DAY_LEAST,
      _ => 0.0,
    };
    let dotted = self.form == EventCalendarForm::Dots;
    let mut day = components::Button::empty()
      .flex(1.0)
      .min_width(Dimension::Px(least))
      .justify(Justify::Start)
      .align_items(Alignment::Stretch)
      .disabled(self.disabled)
      // Its events are stops of Tab of their own, after the day.
      .stops_inside(true)
      .part(&part)
      .describe("a11y_role", "gridcell")
      .describe("a11y_name", self.locale.day_label(date))
      .describe("a11y_selected", if selected { "true" } else { "false" });
    if date == self.today {
      day = day.describe("a11y_current", "date");
    }
    if outside {
      day = day.tab_index(-1).key(format!("outside-{date}"));
    } else {
      let model = self.model.clone();
      day = day
        .tab_index(0)
        .key(day_key(date))
        .focus_entry(entry)
        .on_key_down(move |event: KeyboardEvent| {
          if !event.focus_within || event.ctrl || event.alt || event.meta || event.composing {
            return;
          }
          let Some(target) = model.step(date, &event) else {
            return;
          };
          model.show(target);
          event.focus_key(day_key(target));
        });
    }
    if !self.disabled {
      let press = self.model.clone();
      day = day.on_click(move |event: MouseEvent| {
        press.choose(date);
        if dotted {
          // Its events are dots: the app is told to show them.
          for handler in &press.on_more {
            handler(date);
          }
        }
        if outside {
          // The day pressed is gone with the month it was beside: the focus goes to the day in its own month.
          press.show(date);
          event.focus_key(day_key(date));
        }
      });
    }
    if let Some(id) = self.id_of(&date.to_string()) {
      day = day.id(id);
    }
    // What is pressed is a row: the label and the events are in a column as wide as it.
    day
      .child(
        Column::new()
          .flex(1.0)
          .min_width(Dimension::Px(0.0))
          .align_items(Alignment::Stretch)
          .child(
            Row::new()
              .align_items(Alignment::Center)
              .justify(if dotted { Justify::Center } else { Justify::Start })
              .part(&label)
              .child(
                label
                  .text(&self.day_text(date))
                  .nowrap()
                  .text_overflow(TextOverflow::Elipsis),
              ),
          )
          .child(if dotted {
            Element::from(self.dots(date))
          } else {
            self.events(date).into()
          }),
      )
      .into()
  }

  /// A row of days, as high as the highest of them, `last` where it is the last of the calendar.
  pub fn week(&self, days: &[Date], last: bool) -> Row {
    let states: &[EventCalendarState] = if last { &[EventCalendarState::Last] } else { &[] };
    let mut row = Row::new()
      .align_items(Alignment::Stretch)
      .part(&self.recipe_in(None, states).part(EventCalendarPart::Week))
      .describe("a11y_role", "row");
    for (index, date) in days.iter().enumerate() {
      row = row.child(self.day(*date, index + 1 == days.len()));
    }
    row
  }

  /// The days: the names of the days over the weeks of a month, or the one row of a week or a day. One focus group,
  /// which Tab stops in once at the day [`Self::entry`] says, and a `grid` to what reads the tree, called by the
  /// period shown.
  pub fn grid(&self) -> Column {
    let weeks = self.weeks();
    let mut grid = Column::new()
      .align_items(Alignment::Stretch)
      .part(&self.part(EventCalendarPart::Grid))
      .focus_group(FocusGroup::both().one_stop(true).wrap(false))
      .describe("a11y_role", "grid")
      .describe("a11y_name", self.title_text());
    if self.period == EventCalendarPeriod::Month {
      grid = grid.child(self.weekdays());
    }
    for (index, week) in weeks.iter().enumerate() {
      grid = grid.child(self.week(week, index + 1 == weeks.len()));
    }
    grid
  }

  /// The days as the room has them: the grid, or in a week that scrolls the grid in a sideways scroll as wide as
  /// the calendar, under the header that stands still.
  pub fn days(&self) -> Element {
    match self.form {
      EventCalendarForm::Scrolling => components::ScrollHorizontal::new(self.grid())
        .fill_width(true)
        .key("days")
        .into(),
      _ => self.grid().into(),
    }
  }

  /// The box with nothing in it yet: what holds the header over the days.
  pub fn root(&self) -> Column {
    let mut root = Column::new()
      .align_items(Alignment::Stretch)
      .clip()
      .part(&self.part(EventCalendarPart::Root));
    if let Some(label) = &self.label {
      root = root.describe("a11y_role", "group").describe("a11y_name", label.clone());
    }
    if let Some(id) = &self.id {
      root = root.id(id.clone());
    }
    if let Some(key) = &self.key {
      root = root.key(key.clone());
    }
    if let Some(room) = &self.room {
      root = root.ref_element(room.clone());
    }
    root
  }

  /// How the default renderer builds an event calendar: its header, where it has one, over its days.
  pub fn render(self) -> Element {
    let mut root = self.root();
    if self.header {
      root = root.child(self.header());
    }
    root.child(self.days()).into()
  }
}

impl<T: 'static> Themed for EventCalendar<T> {
  const NAME: &'static str = "event_calendar";
  type Renderer = EventCalendarRenderer;

  /// The design's event calendar (`pencil/lurq-design-system.pen`, Event Calendar `eh5zV`): a box on `Background`
  /// in an edge of 1 in `Border` with the large radius, its days side by side, each with a line of 1 in `Border`
  /// after it but the last, 150 high. A day's label is a row of 32 with a line under it, padded 10, its text
  /// ("Mon 9") 12 at medium weight in `MutedForeground`; today's is on the highlight (the board's accent), in
  /// `Foreground` at 600. The events are 6 from the day's sides and 4 apart, each a pill with the small radius
  /// padded 4 and 6, its title 11 at medium weight in the color of its kind on that kind's muted fill.
  ///
  /// Where the board is silent: the header is a row of 44 with a line under it, its title in the body's type at
  /// medium weight between two outline buttons of 28, as a calendar's, and an outline button that says "Today"; a
  /// month's days are at least 115 high (their label and three events) under a row of the days' names of 32, each
  /// week with a line under it but the last, and a day of the month beside has its label at half its opacity; the
  /// chosen day's label is on `Primary` in `PrimaryForeground`; "+N more" has an event's box, in `MutedForeground`;
  /// the focus of a day and of an event is a line of 2 in the ring's color inside it, an event is veiled under the
  /// pointer, and an event of no kind is on `Muted` in `Foreground`.
  fn recipe() -> Recipe {
    use CalendarEventVariant::{Destructive, Info, Neutral, Success, Warning};
    use EventCalendarPart::{
      Day, Dot, Dots, Event, Events, Header, Label, More, Nav, NavIcon, Root, Title, Week, Weekday, Weekdays,
    };
    use EventCalendarState::{Compact, Current, Disabled, Last, Month, Outside, Selected, Unavailable};
    use PaletteColor as Role;
    let none = Color::new(0, 0, 0, 0);
    let line = || Border::inside(BorderSize::Sm, Role::Border);
    let no_line = || Border::inside(0.0, Role::Border);
    let ring = || Outline::new(2.0, Role::Ring).offset(-2.0);
    let pill = |s: PartStyle| {
      s.padding_x(6.0)
        .padding_y(4.0)
        .radius(RadiusSize::Sm)
        .typography(TypographyStyle::Body)
        .font_size(11.0)
        .weight(FontWeight::Medium)
    };
    let recipe = Recipe::new()
      .base(Root, |s| {
        s.width(Length::Fill)
          .radius(RadiusSize::Lg)
          .background(Role::Background)
          .foreground(Role::Foreground)
          .border(line())
      })
      .base(Header, |s| s.height(44.0).padding_x(10.0).gap(8.0).underline(line()))
      .base(Title, |s| {
        s.typography(TypographyStyle::Body)
          .weight(FontWeight::Medium)
          .foreground(Role::Foreground)
      })
      .base(Nav, |s| {
        s.square(28.0)
          .radius(RadiusSize::Md)
          .background(none)
          .border(Border::inside(BorderSize::Sm, Role::Input))
      })
      .base(NavIcon, |s| s.square(16.0).foreground(Role::MutedForeground))
      .base(Weekdays, |s| s.height(32.0).underline(line()))
      .base(Weekday, |s| {
        s.padding_x(10.0)
          .typography(TypographyStyle::Body)
          .font_size(12.0)
          .weight(FontWeight::Medium)
          .foreground(Role::MutedForeground)
      })
      .base(Week, |s| s.underline(line()))
      .state(Last, Week, |s| s.underline(no_line()))
      .base(Day, |s| {
        fading(s.min_height(150.0).end_line(line()).cursor(CursorIcon::Pointer))
      })
      .state(Month, Day, |s| s.min_height(115.0))
      .state(Last, Day, |s| s.end_line(no_line()))
      .focus_visible(Day, |s| s.outline(ring()))
      .state(Disabled, Day, |s| s.cursor(CursorIcon::Default))
      .base(Label, |s| {
        s.height(32.0)
          .padding_x(10.0)
          .underline(line())
          .typography(TypographyStyle::Body)
          .font_size(12.0)
          .weight(FontWeight::Medium)
          .foreground(Role::MutedForeground)
      })
      .state(Outside, Label, |s| s.opacity(0.5))
      .state(Current, Label, |s| {
        s.background(Role::Highlight)
          .foreground(Role::Foreground)
          .weight(FontWeight::SemiBold)
      })
      .state(Selected, Label, |s| {
        s.background(Role::Primary)
          .foreground(Role::PrimaryForeground)
          .weight(FontWeight::SemiBold)
          .opacity(1.0)
      })
      .base(Events, |s| s.padding_x(6.0).padding_y(6.0).gap(4.0))
      .base(Event, |s| fading(pill(s)).cursor(CursorIcon::Pointer))
      .hovered(Event, |s| s.veil(Role::StateHover))
      .focus_visible(Event, |s| s.outline(ring()))
      .state(Unavailable, Event, |s| {
        s.opacity(OpacityLevel::Disabled).cursor(CursorIcon::Default)
      })
      .base(More, |s| {
        fading(pill(s))
          .foreground(Role::MutedForeground)
          .cursor(CursorIcon::Pointer)
      })
      .hovered(More, |s| s.foreground(Role::Foreground))
      .focus_visible(More, |s| s.outline(ring()))
      .state(Disabled, Root, |s| s.opacity(OpacityLevel::Disabled))
      // A month too narrow for its events: the names of the days and the numbers in the middle of their columns,
      // and a dot of 6 for each event under the number.
      .state(Compact, Weekday, |s| s.padding_x(2.0))
      .state(Compact, Day, |s| s.min_height(56.0))
      .state(Compact, Label, |s| s.height(28.0).padding_x(2.0))
      .base(Dots, |s| s.padding_x(4.0).padding_y(6.0).gap(3.0))
      .base(Dot, |s| s.square(6.0).radius(RadiusSize::Full).background(Role::Foreground));
    [
      (Neutral, Role::Muted, Role::Foreground),
      (Info, Role::InfoMuted, Role::Info),
      (Success, Role::SuccessMuted, Role::Success),
      (Warning, Role::WarningMuted, Role::Warning),
      (Destructive, Role::DestructiveMuted, Role::Destructive),
    ]
    .into_iter()
    .fold(recipe, |recipe, (variant, fill, ink)| {
      let dot = ink.clone();
      recipe
        .variant(variant, Event, |s| s.background(fill).foreground(ink))
        .variant(variant, Dot, |s| s.background(dot))
    })
  }
}

/// An event calendar.
#[derive(Clone)]
struct Props<T> {
  calendar: EventCalendar<T>,
}

/// The calendar is built anew by every render of what holds it, so props are never the same as before.
impl<T> PartialEq for Props<T> {
  fn eq(&self, _: &Self) -> bool {
    false
  }
}

#[cfg(feature = "devtools")]
impl<T> crate::app::component::DevtoolsInspectable for Props<T> {}

/// What a mounted event calendar keeps between renders.
struct Host<T> {
  /// The date whose period is shown, where the app gave none: none until a button or a key sets it.
  shown: Signal<Option<Date>>,
  /// The chosen day as the last render had it: one set from outside since is followed to its period.
  last: Arc<Mutex<Option<Option<Date>>>>,
  /// What the calendar is laid out in, measured before paint, and the form that room gives it.
  room: ElementRef,
  form: Signal<EventCalendarForm>,
  value: std::marker::PhantomData<fn() -> T>,
}

impl<T: Clone + Send + Sync + 'static> Component for Host<T> {
  type Props = Props<T>;

  fn create(_ctx: &mut Ctx) -> Self {
    Self {
      shown: Signal::new(None),
      last: Arc::new(Mutex::new(None)),
      room: ElementRef::new(),
      form: Signal::new(EventCalendarForm::Full),
      value: std::marker::PhantomData,
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let mut calendar = ctx.props::<Props<T>>().calendar.clone();
    // The chosen day set from outside since the last render is shown, where the calendar keeps what it shows.
    let chosen = calendar.value.get_untracked();
    let changed = {
      let mut last = self.last.lock();
      let changed = last.is_some_and(|last| last != chosen);
      *last = Some(chosen);
      changed
    };
    if changed
      && calendar.shown.is_none()
      && let Some(date) = chosen
    {
      self.shown.set(Some(date));
    }
    calendar.kept = Some(self.shown.clone());
    // The form follows the room the calendar is laid out in, from its first frame: the observer changes it only
    // where the width crosses one of the widths it is chosen by.
    let (period, form) = (calendar.period, self.form.clone());
    let days = if calendar.weekends { 7 } else { 5 };
    self.room.observe_rect_before_paint(move |rect| {
      let fits = EventCalendarForm::for_room(period, days, rect.width);
      if form.get_untracked() != fits {
        form.set(fits);
      }
    });
    calendar.form = self.form.get();
    calendar.room = Some(self.room.clone());
    // The app's date text is read in the build, which is in this render: a change of it renders this again.
    calendar.build(&current_theme())
  }
}
