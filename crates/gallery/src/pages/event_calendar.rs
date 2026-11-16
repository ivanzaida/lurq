//! The page of the event calendar: its playground, the board's week, a month with a crowded day, one day, and what
//! a press of an event tells.

use std::sync::Arc;

use lurq::{
  app::{component::Component, ctx::Ctx},
  components::Column,
  core::{Date, Signal},
  node::Element,
  ui::{CalendarEvent, CalendarEventVariant, EventCalendar, EventCalendarPeriod},
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, props_table, quiet, segmented_fill},
  playground::{Playground, Shown, Spec, flag, prop},
};

/// A day of November 2026, the month of the board's week.
fn day(day: u8) -> Date {
  Date::new(2026, 11, day).expect("a day of November")
}

/// The board's today: Thursday the 12th.
fn today() -> Date {
  day(12)
}

/// The board's events (`eh5zV`), each with the name the app knows it by.
fn board_events() -> Vec<CalendarEvent<&'static str>> {
  vec![
    CalendarEvent::new("standup", day(9), "Standup"),
    CalendarEvent::new("release-0.60", day(10), "Release 0.60").variant(CalendarEventVariant::Success),
    CalendarEvent::new("retro", day(10), "Retro"),
    CalendarEvent::new("design-review", day(12), "Design review").variant(CalendarEventVariant::Warning),
    CalendarEvent::new("deploy-freeze", day(13), "Deploy freeze").variant(CalendarEventVariant::Destructive),
  ]
}

/// The month around the board's week: a crowded day, a weekend, an event of no kind.
fn month_events() -> Vec<CalendarEvent<&'static str>> {
  let mut events = board_events();
  events.extend([
    CalendarEvent::new("planning", day(2), "Sprint planning"),
    CalendarEvent::new("offsite", day(7), "Offsite").variant(CalendarEventVariant::Neutral),
    CalendarEvent::new("standup-17", day(17), "Standup"),
    CalendarEvent::new("interviews", day(17), "Interviews").variant(CalendarEventVariant::Warning),
    CalendarEvent::new("release-0.61", day(17), "Release 0.61").variant(CalendarEventVariant::Success),
    CalendarEvent::new("retro-17", day(17), "Retro"),
    CalendarEvent::new("on-call", day(17), "On-call handover").variant(CalendarEventVariant::Neutral),
    CalendarEvent::new("audit", day(24), "Security audit").variant(CalendarEventVariant::Destructive),
    CalendarEvent::new("demo", day(27), "Demo day").variant(CalendarEventVariant::Success),
  ]);
  events
}

/// What the chosen day is, in words.
fn chosen(value: &Signal<Option<Date>>) -> String {
  match value.get() {
    Some(date) => format!("the signal has: {date}"),
    None => "the signal has: None".to_owned(),
  }
}

pub struct EventCalendarPage {
  frame: Frame,
  playground: Playground,
  value: Signal<Option<Date>>,
  period: Signal<EventCalendarPeriod>,
  weekends: Signal<bool>,
  header: Signal<bool>,
  crowded: Signal<bool>,
  disabled: Signal<bool>,
  /// The chosen days of the sections.
  board: Signal<Option<Date>>,
  month: Signal<Option<Date>>,
  one: Signal<Option<Date>>,
  /// The date whose day "A day" shows: the app's.
  one_shown: Signal<Date>,
  /// The chosen day of "Events".
  events_day: Signal<Option<Date>>,
  /// The event last pressed in "Events", by the app's name for it.
  pressed: Signal<String>,
  told: Signal<Option<Date>>,
}

impl EventCalendarPage {
  /// The playground: the calendar as the panel's props make it.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let (period, weekends, header) = (self.period.get(), self.weekends.get(), self.header.get());
    let (crowded, disabled) = (self.crowded.get(), self.disabled.get());
    let forced = self.playground.forced();
    let record = self.playground.recorder();
    let value = self.value.clone();
    let build = move || -> EventCalendar<&'static str> {
      EventCalendar::new(value.clone(), if crowded { month_events() } else { board_events() })
        .today(today())
        .period(period)
        .weekends(weekends)
        .header(header)
        .disabled(disabled)
    };
    let make = {
      let build = build.clone();
      move || -> Element { build().into() }
    };
    self.playground.window_shows({
      let build = build.clone();
      move |ctx| build().mount(ctx)
    });
    let mut code = vec![
      "// day: Signal<Option<Date>>; events: Vec<CalendarEvent<&str>>".to_owned(),
      "EventCalendar::new(day.clone(), events)".to_owned(),
    ];
    match period {
      EventCalendarPeriod::Week => {}
      EventCalendarPeriod::Month => code.push("    .period(EventCalendarPeriod::Month)".to_owned()),
      EventCalendarPeriod::Day => code.push("    .period(EventCalendarPeriod::Day)".to_owned()),
    }
    if !weekends {
      code.push("    .weekends(false)".to_owned());
    }
    if !header {
      code.push("    .header(false)".to_owned());
    }
    if disabled {
      code.push("    .disabled(true)".to_owned());
    }
    code.push("    .on_select(|event: &&str| ..)".to_owned());
    code.push("    .mount(ctx)".to_owned());
    let mut tree = vec![format!(
      "root       {}   Background · edge Border · radius 8",
      self.playground.measured()
    )];
    if header {
      tree.push("├─ header  44 · previous, next · title · Today".to_owned());
    }
    if period == EventCalendarPeriod::Month {
      tree.push("├─ weekdays  32 · the days' names, 12 medium MutedForeground".to_owned());
      tree.push("└─ week ×6  days at least 115 high · a line under each but the last".to_owned());
    } else {
      tree.push("└─ week    days 148 high · a line after each but the last".to_owned());
    }
    tree.push("   └─ day   label 32 · events 6 in, 4 apart · today on Highlight".to_owned());
    let reset = {
      let (value, period, weekends) = (self.value.clone(), self.period.clone(), self.weekends.clone());
      let (header, crowded, disabled) = (self.header.clone(), self.crowded.clone(), self.disabled.clone());
      Arc::new(move || {
        value.set(None);
        period.set(EventCalendarPeriod::Week);
        weekends.set(true);
        header.set(true);
        crowded.set(false);
        disabled.set(false);
      })
    };
    let shown = {
      let (build, playground) = (build.clone(), &self.playground);
      Shown::built(move |ctx| {
        let (select, more) = (record.clone(), record.clone());
        let calendar = build()
          .force(forced)
          .on_select(move |event: &&str| select(format!("on_select({event:?})")))
          .on_more(move |date| more(format!("on_more({date})")))
          .mount(ctx);
        // The calendar takes the form its room gives it: its own sideways scroll, one day, or dots.
        Column::new().width(FILL).child(playground.measure(calendar))
      })
      .measured()
    };
    let choose = self.period.clone();
    self.playground.card(
      ctx,
      Spec {
        title: "Event Calendar",
        make: Arc::new(make),
        shown,
        fill: true,
        forceable: true,
        facts: match period {
          EventCalendarPeriod::Month => "a month".to_owned(),
          EventCalendarPeriod::Week => "a week".to_owned(),
          EventCalendarPeriod::Day => "a day".to_owned(),
        },
        event: "on_change",
        code,
        tree,
        controls: vec![
          prop(
            "period",
            "EventCalendarPeriod",
            segmented_fill(
              &[
                ("Month", EventCalendarPeriod::Month),
                ("Week", EventCalendarPeriod::Week),
                ("Day", EventCalendarPeriod::Day),
              ],
              &period,
              move |period| choose.set(period),
            ),
          ),
          flag("weekends", &self.weekends),
          flag("header", &self.header),
          flag("more events", &self.crowded),
          flag("disabled", &self.disabled),
        ],
        changed: [
          period != EventCalendarPeriod::Week,
          !weekends,
          !header,
          crowded,
          disabled,
        ]
        .into_iter()
        .filter(|changed| *changed)
        .count(),
        reset,
      },
    )
  }
}

impl Component for EventCalendarPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    let playground = Playground::new(ctx, 0);
    let value: Signal<Option<Date>> = ctx.signal(None);
    playground.watch(ctx, &value, |value| match value {
      Some(date) => format!("on_change({date})"),
      None => "on_change(None)".to_owned(),
    });
    Self {
      frame: Frame::new(ctx),
      playground,
      value,
      period: ctx.signal(EventCalendarPeriod::Week),
      weekends: ctx.signal(true),
      header: ctx.signal(true),
      crowded: ctx.signal(false),
      disabled: ctx.signal(false),
      board: ctx.signal(None),
      month: ctx.signal(Some(day(17))),
      one: ctx.signal(Some(day(10))),
      one_shown: ctx.signal(day(17)),
      events_day: ctx.signal(None),
      pressed: ctx.signal(String::new()),
      told: ctx.signal(None),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    // The board's week: five days, no header, today the Thursday.
    let board = Column::new()
      .width(FILL)
      .max_width(560.0)
      .spacing(10.0)
      .child(
        EventCalendar::new(self.board.clone(), board_events())
          .today(today())
          .weekends(false)
          .header(false)
          .mount(ctx),
      )
      .child(quiet(&format!("the board's week · {}", chosen(&self.board)), 12.0));
    // A month: the 17th has five events, of which it shows two and "+3 more".
    let month = Column::new()
      .width(FILL)
      .spacing(10.0)
      .child({
        let told = self.told.clone();
        EventCalendar::new(self.month.clone(), month_events())
          .today(today())
          .period(EventCalendarPeriod::Month)
          .on_more(move |date| told.set(Some(date)))
          .mount(ctx)
      })
      .child(quiet(
        &match self.told.get() {
          Some(date) => format!("{} · on_more({date})", chosen(&self.month)),
          None => format!("{} · press \"+3 more\" on the 17th", chosen(&self.month)),
        },
        12.0,
      ));
    // One day, which shows all that is on it.
    let one = Column::new()
      .width(FILL)
      .max_width(320.0)
      .spacing(10.0)
      .child(
        EventCalendar::new(self.one.clone(), month_events())
          .today(today())
          .shown(self.one_shown.clone())
          .period(EventCalendarPeriod::Day)
          .mount(ctx),
      )
      .child(quiet("period(Day) · the 17th, all five of its events", 12.0));
    // What a press of an event tells.
    let events = Column::new()
      .width(FILL)
      .max_width(560.0)
      .spacing(10.0)
      .child({
        let pressed = self.pressed.clone();
        EventCalendar::new(
          self.events_day.clone(),
          board_events()
            .into_iter()
            .chain([CalendarEvent::new("offsite", day(11), "Offsite (held)").disabled(true)]),
        )
        .today(today())
        .weekends(false)
        .on_select(move |event: &&str| pressed.set((*event).to_owned()))
        .mount(ctx)
      })
      .child(quiet(
        &match self.pressed.get().as_str() {
          "" => "press an event".to_owned(),
          event => format!("on_select({event:?})"),
        },
        12.0,
      ));
    let rows = [
      (
        "new",
        "Signal<Option<Date>>, events",
        "",
        "The chosen day, which a press of a day sets, and the events. controlled(value, events, on_change) tells \
         and does not set.",
      ),
      (
        "CalendarEvent::new",
        "T, Date, &str",
        "",
        "What the event is to the app (on_select is told it), its day and its title. CalendarEvent::on(date, \
         title) for one that is only pressed.",
      ),
      (
        "variant",
        "CalendarEventVariant",
        "Info",
        "An event's kind: Neutral, Info, Success, Warning, Destructive. Its ink on that kind's muted fill.",
      ),
      (
        "period",
        "EventCalendarPeriod",
        "Week",
        "What is shown at once: a Week, a Month, or one Day.",
      ),
      (
        "shown",
        "Signal<Date>",
        "",
        "The date whose period is shown, the app's. Without it a mounted calendar keeps it, from the chosen day \
         or today.",
      ),
      ("weekends", "bool", "true", "Whether Saturdays and Sundays are shown."),
      (
        "header",
        "bool",
        "true",
        "The buttons that show the period before and after, what the period is, and Today.",
      ),
      (
        "max_shown",
        "usize",
        "4 · 3",
        "How many events of a day are shown at most: 4 in a week, 3 in a month, all in a day. With more, one \
         fewer and \"+N more\".",
      ),
      (
        "on_change",
        "Fn(Date)",
        "",
        "A person chose another day: a press, Enter, or the day's \"+N more\".",
      ),
      (
        "on_select",
        "Fn(&T)",
        "",
        "An event was pressed, with what it is to the app. It does not choose its day.",
      ),
      (
        "on_more",
        "Fn(Date)",
        "",
        "The \"+N more\" of a day was pressed, after the day is chosen.",
      ),
      (
        "first_day / locale / today",
        "Weekday / impl DateText / Date",
        "",
        "The day weeks start on, the words of dates, and the day marked as today, in place of the app's and \
         the system's.",
      ),
      (
        "previous_label / next_label / today_label / more_text",
        "&str / Fn(usize) -> String",
        "",
        "The names of its buttons and \"+N more\", in place of the locale's and English.",
      ),
      ("disabled", "bool", "false", "It takes no input."),
      (
        "mount",
        "&mut Ctx",
        "",
        "The calendar where it is declared, keeping the period it shows.",
      ),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Data", "Event Calendar"],
        title: "Event Calendar",
        description: "A calendar that shows events in its days: the days of a week side by side, the weeks of a \
                      month, or one day. A press chooses a day, an event is pressed on its own, and the keys are a \
                      calendar's.",
        import: Some("use lurq::ui::{CalendarEvent, EventCalendar, EventCalendarPeriod};"),
        status: Some("Beta"),
        since: Some("0.63"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop of EventCalendar is a control on the right. Press a day, an event, the buttons of the \
           header; Tab into the days and move with the arrows.",
          playground,
        ),
        Section::new(
          "Event Calendar",
          "The board's week: five days side by side, each with its label over its events, today on the \
           highlight, an event in the color of its kind.",
          self.frame.demo(
            0,
            board,
            "EventCalendar::new(..)",
            &[
              "// day: Signal<Option<Date>>",
              "EventCalendar::new(day.clone(), vec![",
              "  CalendarEvent::new(\"standup\", monday, \"Standup\"),",
              "  CalendarEvent::new(\"release-0.60\", tuesday, \"Release 0.60\").variant(CalendarEventVariant::Success),",
              "  CalendarEvent::new(\"retro\", tuesday, \"Retro\"),",
              "  CalendarEvent::new(\"design-review\", thursday, \"Design review\").variant(CalendarEventVariant::Warning),",
              "  CalendarEvent::new(\"deploy-freeze\", friday, \"Deploy freeze\").variant(CalendarEventVariant::Destructive),",
              "])",
              ".weekends(false)",
              ".header(false)",
              ".mount(ctx)",
            ],
          ),
        ),
        Section::new(
          "A month",
          "The weeks of a month are a calendar's grid, under the names of the days. A day shows three events at \
           most: with more, two and what stands for the rest, which chooses the day and tells the app.",
          self.frame.demo(
            1,
            month,
            ".period(EventCalendarPeriod::Month)",
            &[
              "EventCalendar::new(day.clone(), events)",
              "    .period(EventCalendarPeriod::Month)",
              "    .on_more(|date| ..)",
              "    .mount(ctx)",
            ],
          ),
        ),
        Section::new(
          "A day",
          "One day shows all that is on it. The date whose period is shown can be the app's.",
          self.frame.demo(
            2,
            one,
            ".period(EventCalendarPeriod::Day)",
            &[
              "EventCalendar::new(day.clone(), events)",
              "    .period(EventCalendarPeriod::Day)",
              "    .shown(shown.clone())",
              "    .mount(ctx)",
            ],
          ),
        ),
        Section::new(
          "Events",
          "A press of an event tells the app which, by what the app gave the event, and does not choose its day. \
           Tab goes from a day into its events; a disabled one takes nothing.",
          self.frame.demo(
            3,
            events,
            ".on_select(|event| ..)",
            &[
              "EventCalendar::new(day.clone(), events)",
              "    .event(CalendarEvent::new(\"offsite\", wednesday, \"Offsite (held)\").disabled(true))",
              "    .on_select(|event: &&str| open(event))",
              "    .mount(ctx)",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What an event calendar and its events are given.",
          props_table(&rows, &["new", "CalendarEvent::new"], self.frame.room()),
        ),
      ],
    )
  }
}
