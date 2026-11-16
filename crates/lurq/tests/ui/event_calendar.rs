//! `ui::EventCalendar`: the design's week of days with their events (`eh5zV`), a month and a day, what a press and
//! the keys do, and what it keeps.

use std::sync::{Arc, Mutex};

use lurq::{
  app::{
    App,
    theme::{PaletteColor, ThemePalette},
  },
  components::Column,
  core::{Date, DateLocale, Signal, Weekday},
  layout::{
    quad::{Quad, QuadContent},
    text_style::FontWeight,
  },
  node::{Element, color::Color},
  ui::{CalendarEvent, CalendarEventVariant, EventCalendar, EventCalendarPart, EventCalendarPeriod, Interaction},
};

use super::stage::Stage;

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

fn day(day: u8) -> Date {
  Date::new(2026, 11, day).unwrap()
}

/// The board's week: Monday the 9th to Friday the 13th of November 2026, today the Thursday.
fn board_events() -> Vec<CalendarEvent<&'static str>> {
  vec![
    CalendarEvent::new("standup", day(9), "Standup").id("standup"),
    CalendarEvent::new("release", day(10), "Release 0.60")
      .variant(CalendarEventVariant::Success)
      .id("release"),
    CalendarEvent::new("retro", day(10), "Retro").id("retro"),
    CalendarEvent::new("review", day(12), "Design review")
      .variant(CalendarEventVariant::Warning)
      .id("review"),
    CalendarEvent::new("freeze", day(13), "Deploy freeze")
      .variant(CalendarEventVariant::Destructive)
      .id("freeze"),
  ]
}

fn light_app() -> App {
  let app = App::new();
  app
    .theme()
    .set_preferred_appearance(Some(lurq::app::window::Appearance::Light));
  app
}

/// A window of 560 by 900, which a month has room in, with the calendar `scene` makes in it, mounted, at its top.
fn staged(scene: impl Fn() -> EventCalendar<&'static str> + Send + Sync + 'static) -> Stage {
  let mut stage = Stage::first_frame_in(light_app(), 560.0, 900.0, move |ctx| {
    Column::new().size(560.0, 900.0).child(scene().mount(ctx)).into()
  });
  stage.pass();
  stage
}

/// The board's calendar: the week of the 12th without its weekend and with no header, nothing chosen.
fn board(value: &Signal<Option<Date>>) -> EventCalendar<&'static str> {
  EventCalendar::new(value.clone(), board_events())
    .today(day(12))
    .weekends(false)
    .header(false)
    .id("team")
}

fn text_of(stage: &Stage, content: &str) -> Quad {
  stage
    .text_quad(content)
    .unwrap_or_else(|| panic!("{content:?} is painted"))
}

fn weight_of(quad: &Quad) -> FontWeight {
  match &quad.content {
    QuadContent::Text { style, .. } => style.weight,
    _ => unreachable!(),
  }
}

fn focused(stage: &Stage) -> Option<String> {
  stage
    .tree
    .focused_element()
    .and_then(|element| element.id().map(str::to_owned))
}

/// A key with Shift held.
fn shift_key(stage: &mut Stage, key: &str) {
  stage.tree.key_down(key.to_owned(), key.to_owned(), true, false, false);
  stage.tree.key_up(key.to_owned(), key.to_owned(), true, false, false);
  stage.pass();
}

/// The fill painted at exactly `(x, y, width, height)`.
fn fill_at(stage: &Stage, rect: (f32, f32, f32, f32)) -> Option<Color> {
  stage.quads().into_iter().find_map(|quad| match quad.content {
    QuadContent::Rect { color, .. }
      if color.a() > 0
        && (quad.x - rect.0).abs() < 0.01
        && (quad.y - rect.1).abs() < 0.01
        && (quad.width - rect.2).abs() < 0.01
        && (quad.height - rect.3).abs() < 0.01 =>
    {
      Some(color)
    }
    _ => None,
  })
}

#[test]
fn the_week_is_the_boards() {
  let value = Signal::new(None);
  let bound = value.clone();
  let mut stage = staged(move || board(&bound));
  let (x, y, width, height) = stage.bounds("team");
  // Five days of 112 by 150 with the edge drawn over them: 560 by 150, as the board's.
  assert_eq!((x, y, width), (0.0, 0.0, 560.0));
  assert!((height - 150.0).abs() < 0.01, "{height}");
  assert_eq!(stage.fill_color("team"), Some(light(PaletteColor::Background)));
  // Its edge: the quad at its box with a line on top (the row of days in it has the same box, and none).
  let edge = stage
    .quads()
    .into_iter()
    .find(|quad| {
      (quad.x, quad.y, quad.width, quad.height) == (x, y, width, height)
        && quad
          .border
          .is_some_and(|border| border.top.is_some_and(|side| side.width > 0.0))
    })
    .expect("its edge");
  assert_eq!(
    edge
      .border
      .and_then(|border| border.top)
      .map(|side| (side.width, side.color)),
    Some((1.0, light(PaletteColor::Border)))
  );
  assert_eq!(edge.border_radius.map(|radius| radius.top_left), Some(8.0));
  let mut last_end = 0.0;
  for (index, (number, name)) in [
    (9, "Mon 9"),
    (10, "Tue 10"),
    (11, "Wed 11"),
    (12, "Thu 12"),
    (13, "Fri 13"),
  ]
  .into_iter()
  .enumerate()
  {
    let id = format!("team-2026-11-{number:02}");
    let (day_x, day_y, day_width, day_height) = stage.bounds(&id);
    assert!(
      (day_x - last_end).abs() < 0.01,
      "{name} starts where the day before it ends"
    );
    assert_eq!(day_width, 112.0, "{name}: a fifth of the room");
    assert_eq!((day_y, day_height), (0.0, 150.0), "{name}");
    last_end = day_x + day_width;
    // A line of 1 after each day but the last.
    let after = stage
      .border(&id)
      .and_then(|quad| quad.border)
      .and_then(|border| border.right)
      .map_or(0.0, |side| side.width);
    assert_eq!(after, if index == 4 { 0.0 } else { 1.0 }, "{name}");
    // The label: a row of 32 with a line under it, its text 12 in MutedForeground at medium weight, 10 in.
    let label = text_of(&stage, name);
    assert!((label.x - (day_x + 10.0)).abs() < 0.01, "{name}");
    assert!((label.y + label.height / 2.0 - (day_y + 16.0)).abs() < 0.51, "{name}");
    let (color, size) = stage.text(name);
    assert_eq!(size, 12.0);
    if number == 12 {
      // Today: on the highlight, in Foreground at 600.
      assert_eq!(
        (color, weight_of(&label)),
        (light(PaletteColor::Foreground), FontWeight::SemiBold)
      );
      assert_eq!(
        fill_at(&stage, (day_x, day_y, day_width, 32.0)),
        Some(light(PaletteColor::Highlight))
      );
    } else {
      assert_eq!(
        (color, weight_of(&label)),
        (light(PaletteColor::MutedForeground), FontWeight::Medium)
      );
    }
  }
  // The events: 6 from the day's sides and from its label, 4 apart, 21 high, with the small radius.
  let (monday_x, ..) = stage.bounds("team-2026-11-09");
  let (x, y, width, height) = stage.bounds("standup");
  assert!((x - (monday_x + 6.0)).abs() < 0.01);
  assert_eq!(y, 32.0 + 6.0);
  assert_eq!(width, 100.0, "the day less 6 at each side");
  assert!((height - 21.0).abs() < 0.6, "{height}");
  let (_, release_y, _, release_height) = stage.bounds("release");
  let (_, retro_y, ..) = stage.bounds("retro");
  assert!((retro_y - (release_y + release_height + 4.0)).abs() < 0.01);
  assert_eq!(
    stage
      .fill("standup")
      .and_then(|quad| quad.border_radius)
      .map(|radius| radius.top_left),
    Some(4.0)
  );
  // Each in the color of its kind on that kind's muted fill, 11 at medium weight, 6 in.
  for (id, title, fill, ink) in [
    ("standup", "Standup", PaletteColor::InfoMuted, PaletteColor::Info),
    (
      "release",
      "Release 0.60",
      PaletteColor::SuccessMuted,
      PaletteColor::Success,
    ),
    (
      "review",
      "Design review",
      PaletteColor::WarningMuted,
      PaletteColor::Warning,
    ),
    (
      "freeze",
      "Deploy freeze",
      PaletteColor::DestructiveMuted,
      PaletteColor::Destructive,
    ),
  ] {
    assert_eq!(stage.fill_color(id), Some(light(fill)), "{title}");
    assert_eq!(stage.text(title), (light(ink), 11.0), "{title}");
    let (text, pill) = (text_of(&stage, title), stage.bounds(id));
    assert_eq!(weight_of(&text), FontWeight::Medium);
    assert!((text.x - (pill.0 + 6.0)).abs() < 0.01);
  }
}

#[test]
fn a_press_chooses_a_day_and_an_events_press_is_its_own() {
  let value = Signal::new(None);
  let (changed, selected) = (Arc::new(Mutex::new(Vec::new())), Arc::new(Mutex::new(Vec::new())));
  let (bound, change, select) = (value.clone(), changed.clone(), selected.clone());
  let mut stage = staged(move || {
    let (change, select) = (change.clone(), select.clone());
    board(&bound)
      .on_change(move |date| change.lock().unwrap().push(date))
      .on_select(move |event: &&str| select.lock().unwrap().push(*event))
  });
  // A press in the room under a day's events chooses the day.
  let (x, y, _, height) = stage.bounds("team-2026-11-10");
  stage.tree.mouse_move(x + 20.0, y + height - 10.0);
  stage
    .tree
    .mouse_down(x + 20.0, y + height - 10.0, lurq::app::events::MouseButton::Left);
  stage
    .tree
    .mouse_up(x + 20.0, y + height - 10.0, lurq::app::events::MouseButton::Left);
  stage.pass();
  assert_eq!(value.get(), Some(day(10)));
  assert_eq!(*changed.lock().unwrap(), [day(10)]);
  // The chosen day's label is on Primary, in PrimaryForeground.
  assert_eq!(stage.text("Tue 10").0, light(PaletteColor::PrimaryForeground));
  // A press of an event tells which, and does not choose its day.
  stage.click("review");
  assert_eq!(*selected.lock().unwrap(), ["review"]);
  assert_eq!(value.get(), Some(day(10)));
  // A press of the chosen day tells nothing.
  stage.click("team-2026-11-10");
  assert_eq!(changed.lock().unwrap().len(), 1);
  // The app sets the value: it is shown chosen, and nothing is told.
  value.set(Some(day(13)));
  stage.pass();
  assert_eq!(stage.text("Fri 13").0, light(PaletteColor::PrimaryForeground));
  assert_eq!(changed.lock().unwrap().len(), 1);
}

#[test]
fn a_controlled_calendar_tells_and_does_not_choose() {
  let told = Arc::new(Mutex::new(Vec::new()));
  let tell = told.clone();
  let mut stage = staged(move || {
    let tell = tell.clone();
    EventCalendar::controlled(Some(day(9)), board_events(), move |date| {
      tell.lock().unwrap().push(date)
    })
    .today(day(12))
    .header(false)
    .id("team")
  });
  stage.click("team-2026-11-11");
  assert_eq!(*told.lock().unwrap(), [day(11)]);
  assert_eq!(
    stage.text("Mon 9").0,
    light(PaletteColor::PrimaryForeground),
    "the app has not answered: the day it holds is the chosen one"
  );
  assert_eq!(stage.text("Wed 11").0, light(PaletteColor::MutedForeground));
}

#[test]
fn the_keys_are_a_calendars_and_show_the_week_they_leave_for() {
  let value = Signal::new(None);
  let bound = value.clone();
  let mut stage = staged(move || board(&bound));
  stage.key("Tab");
  assert_eq!(
    focused(&stage).as_deref(),
    Some("team-2026-11-12"),
    "Tab enters at today"
  );
  assert!(stage.has_ring(light(PaletteColor::Ring)));
  stage.key("ArrowRight");
  assert_eq!(focused(&stage).as_deref(), Some("team-2026-11-13"));
  // Past the weekend the calendar hides, into the week after, which is then shown.
  stage.key("ArrowRight");
  assert_eq!(focused(&stage).as_deref(), Some("team-2026-11-16"));
  assert!(stage.paints("Mon 16") && !stage.paints("Fri 13"));
  stage.key("ArrowLeft");
  assert_eq!(focused(&stage).as_deref(), Some("team-2026-11-13"));
  stage.key("Home");
  assert_eq!(focused(&stage).as_deref(), Some("team-2026-11-09"));
  // End is the Sunday, which is hidden: the Friday before it.
  stage.key("End");
  assert_eq!(focused(&stage).as_deref(), Some("team-2026-11-13"));
  stage.key("ArrowUp");
  assert_eq!(focused(&stage).as_deref(), Some("team-2026-11-06"));
  stage.key("PageDown");
  assert_eq!(
    focused(&stage).as_deref(),
    Some("team-2026-12-07"),
    "a month on, off the Sunday"
  );
  shift_key(&mut stage, "PageUp");
  assert_eq!(
    focused(&stage).as_deref(),
    Some("team-2025-12-05"),
    "a year back, off the Sunday"
  );
  // Enter chooses the focused day.
  stage.key("Enter");
  assert_eq!(value.get(), Date::new(2025, 12, 5));
}

#[test]
fn tab_goes_from_a_day_into_its_events_and_then_out() {
  let value = Signal::new(Some(day(10)));
  let selected = Arc::new(Mutex::new(Vec::new()));
  let (bound, select) = (value.clone(), selected.clone());
  let mut stage = staged(move || {
    let select = select.clone();
    board(&bound).on_select(move |event: &&str| select.lock().unwrap().push(*event))
  });
  stage.key("Tab");
  assert_eq!(
    focused(&stage).as_deref(),
    Some("team-2026-11-10"),
    "Tab enters at the chosen day"
  );
  stage.key("Tab");
  assert_eq!(focused(&stage).as_deref(), Some("release"));
  stage.key("Enter");
  assert_eq!(*selected.lock().unwrap(), ["release"]);
  assert_eq!(value.get(), Some(day(10)), "an event's Enter is its own");
  stage.key("Tab");
  assert_eq!(focused(&stage).as_deref(), Some("retro"));
  stage.key("Tab");
  assert_ne!(
    focused(&stage).as_deref(),
    Some("review"),
    "Tab leaves the days after the day's events"
  );
}

#[test]
fn the_header_names_the_period_and_its_buttons_show_another() {
  let value = Signal::new(None);
  let bound = value.clone();
  let mut stage = staged(move || board(&bound).header(true));
  // A header of 44 over the days.
  let (_, monday_y, ..) = stage.bounds("team-2026-11-09");
  assert_eq!(monday_y, 44.0);
  assert!(stage.paints("Nov 9 – Nov 13, 2026"), "the days of the week shown");
  stage.click("team-next");
  assert!(stage.paints("Mon 16") && stage.paints("Nov 16 – Nov 20, 2026"));
  stage.click("team-previous");
  stage.click("team-previous");
  assert!(stage.paints("Mon 2"));
  assert_eq!(value.get(), None, "showing another week chooses nothing");
  // Today brings the week today is in back.
  assert!(stage.paints("Today"));
  stage.click("team-today");
  assert!(stage.paints("Thu 12"));
  #[cfg(any(feature = "mcp", feature = "devtools"))]
  {
    let name = stage
      .tree
      .get_element_by_id_mut("team-next")
      .and_then(|element| element.description("a11y_name"));
    assert_eq!(name.as_deref(), Some("Next week"));
  }
}

#[test]
fn the_apps_date_is_the_period_shown_and_the_chosen_day_is_followed() {
  // The app's date: the buttons set it.
  let (value, shown) = (Signal::new(None), Signal::new(day(12)));
  let (bound, seen) = (value.clone(), shown.clone());
  let mut stage = staged(move || board(&bound).header(true).shown(seen.clone()));
  stage.click("team-next");
  assert_eq!(shown.get(), day(19));
  shown.set(day(2));
  stage.pass();
  assert!(stage.paints("Mon 2"));
  // The calendar's own: a chosen day the app sets outside the week shown is followed to its week.
  let value = Signal::new(Some(day(12)));
  let bound = value.clone();
  let mut stage = staged(move || board(&bound));
  assert!(stage.paints("Thu 12"));
  value.set(Some(day(24)));
  stage.pass();
  assert!(stage.paints("Tue 24") && !stage.paints("Thu 12"));
  assert_eq!(stage.text("Tue 24").0, light(PaletteColor::PrimaryForeground));
}

#[test]
fn a_month_is_a_calendars_grid_of_days_with_their_events() {
  let value = Signal::new(None);
  let bound = value.clone();
  let mut stage = staged(move || {
    board(&bound)
      .weekends(true)
      .header(true)
      .period(EventCalendarPeriod::Month)
  });
  assert!(stage.paints("November 2026"));
  // The names of the days over the weeks, from the locale's first day.
  let (monday, sunday) = (text_of(&stage, "Mon"), text_of(&stage, "Sun"));
  assert!(monday.x < sunday.x);
  // Six weeks of seven days: the grid of a calendar's month, from Monday the 26th of October.
  let (first_x, first_y, first_width, first_height) = stage.bounds("team-2026-10-26");
  assert_eq!((first_x, first_y), (0.0, 44.0 + 32.0));
  assert_eq!(first_width, 80.0);
  assert!(first_height >= 115.0);
  let (_, last_y, ..) = stage.bounds("team-2026-12-06");
  assert!((last_y - (first_y + 5.0 * first_height)).abs() < 0.6, "the sixth week");
  // A day says its number; one of the month beside is dimmed, and a press of it shows its month.
  assert!(
    (monday.x - (first_x + 10.0)).abs() < 0.01,
    "a name stands over its column"
  );
  assert_eq!(stage.text("12").0, light(PaletteColor::Foreground), "today");
  // In a day of 80 a longer title is cut.
  assert!(stage.paints("Standup") && stage.cut_text_quad("Deploy freeze").is_some());
  stage.click("team-2026-12-03");
  assert_eq!(value.get(), Date::new(2026, 12, 3));
  assert!(stage.paints("December 2026"));
  assert_eq!(focused(&stage).as_deref(), Some("team-2026-12-03"));
  // The buttons and the keys go a month.
  stage.click("team-previous");
  assert!(stage.paints("November 2026"));
  #[cfg(any(feature = "mcp", feature = "devtools"))]
  {
    let name = stage
      .tree
      .get_element_by_id_mut("team-previous")
      .and_then(|element| element.description("a11y_name"));
    assert_eq!(name.as_deref(), Some("Previous month"));
  }
}

#[test]
fn a_day_with_more_events_than_it_shows_says_how_many_more() {
  let value = Signal::new(None);
  let more = Arc::new(Mutex::new(Vec::new()));
  let (bound, told) = (value.clone(), more.clone());
  let mut stage = staged(move || {
    let told = told.clone();
    let crowd = ["Standup", "Planning", "Lunch", "Review", "Retro"]
      .into_iter()
      .map(|title| CalendarEvent::new(title, day(10), title));
    EventCalendar::new(bound.clone(), crowd)
      .today(day(12))
      .period(EventCalendarPeriod::Month)
      .on_more(move |date| told.lock().unwrap().push(date))
      .id("team")
  });
  // Three at most in a month's day: two events, and what stands for the other three.
  assert!(stage.paints("Standup") && stage.paints("Planning"));
  assert!(!stage.paints("Lunch") && !stage.paints("Retro"));
  assert_eq!(stage.text("+3 more"), (light(PaletteColor::MutedForeground), 11.0));
  // It is as high as an event, and does not make its day higher than the days beside it.
  let (_, _, _, crowded) = stage.bounds("team-2026-11-10");
  let (_, _, _, empty) = stage.bounds("team-2026-11-11");
  assert_eq!(crowded, empty);
  // Pressed, it chooses the day and tells the app.
  stage.click("team-more-2026-11-10");
  assert_eq!(value.get(), Some(day(10)));
  assert_eq!(*more.lock().unwrap(), [day(10)]);
  // The app says how many a day shows, and how the rest is said; a day alone shows all.
  let value = Signal::new(None);
  let bound = value.clone();
  let stage = staged(move || {
    let crowd = ["Standup", "Planning", "Lunch", "Review", "Retro"]
      .into_iter()
      .map(|title| CalendarEvent::new(title, day(10), title));
    EventCalendar::new(bound.clone(), crowd)
      .today(day(10))
      .max_shown(4)
      .more_text(|count| format!("ще {count}"))
  });
  assert!(stage.paints("Lunch") && !stage.paints("Review") && stage.paints("ще 2"));
  let value = Signal::new(None);
  let bound = value.clone();
  let stage = staged(move || {
    let crowd = ["Standup", "Planning", "Lunch", "Review", "Retro"]
      .into_iter()
      .map(|title| CalendarEvent::new(title, day(10), title));
    EventCalendar::new(bound.clone(), crowd)
      .today(day(10))
      .period(EventCalendarPeriod::Day)
  });
  assert!(stage.paints("Retro") && stage.paints("Tuesday, Nov 10, 2026") && stage.paints("Tue 10"));
}

#[test]
fn its_words_are_the_locales() {
  let value = Signal::new(None);
  let bound = value.clone();
  let stage = staged(move || {
    board(&bound)
      .header(true)
      .weekends(true)
      .locale(DateLocale::for_tag("uk"))
  });
  assert!(stage.paints("Чт 12") && stage.paints("Сьогодні"));
  // The week starts on the day the app says.
  let value = Signal::new(None);
  let bound = value.clone();
  let mut stage = staged(move || board(&bound).weekends(true).first_day(Weekday::Sunday));
  let (sunday_x, ..) = stage.bounds("team-2026-11-08");
  assert_eq!(sunday_x, 0.0, "Sunday the 8th is the first column");
}

#[test]
fn a_long_title_is_cut_in_its_pill_and_a_narrow_room_narrows_the_days() {
  const LONG: &str = "Quarterly planning of the renderer team with the design system group";
  let value: Signal<Option<Date>> = Signal::new(None);
  let mut stage = Stage::in_app(light_app(), move |_| {
    let calendar: Element = EventCalendar::new(value.clone(), [CalendarEvent::on(day(10), LONG).id("long")])
      .today(day(12))
      .id("team")
      .into();
    Column::new()
      .size(400.0, 300.0)
      .child(Column::new().width(320.0).child(calendar))
      .into()
  });
  assert_eq!(stage.bounds("team").2, 320.0, "no wider than its room");
  let (x, _, width, height) = stage.bounds("long");
  assert!((height - 21.0).abs() < 0.6, "on one line");
  let title = stage.cut_text_quad(LONG).expect("the title, cut");
  assert!(matches!(&title.content, QuadContent::Text { text, .. } if text.ends_with('…')));
  assert!(title.x + title.width <= x + width - 6.0 + 0.51);
  for quad in stage.quads() {
    if let QuadContent::Text { text, .. } = &quad.content {
      assert!(quad.x + quad.width <= 320.5, "{text:?} runs past the room");
    }
  }
}

#[test]
fn a_disabled_calendar_and_a_disabled_event_take_nothing() {
  let value = Signal::new(None);
  let selected = Arc::new(Mutex::new(0));
  let (bound, select) = (value.clone(), selected.clone());
  let mut stage = staged(move || {
    let select = select.clone();
    EventCalendar::new(
      bound.clone(),
      [
        CalendarEvent::new("held", day(10), "Held").disabled(true).id("held"),
        CalendarEvent::new("open", day(11), "Open").id("open"),
      ],
    )
    .today(day(12))
    .on_select(move |_: &&str| *select.lock().unwrap() += 1)
    .id("team")
  });
  stage.click("held");
  assert_eq!(*selected.lock().unwrap(), 0, "a disabled event is not pressed");
  stage.click("open");
  assert_eq!(*selected.lock().unwrap(), 1);
  let value = Signal::new(None);
  let bound = value.clone();
  let mut stage = staged(move || board(&bound).header(true).disabled(true));
  stage.click("team-2026-11-10");
  stage.click("team-next");
  stage.click("standup");
  assert_eq!(value.get(), None);
  assert!(stage.paints("Thu 12"));
}

#[test]
fn a_theme_draws_it_otherwise_and_the_forced_day_is_the_one_tab_enters_at() {
  let value = Signal::new(None);
  let bound = value.clone();
  let stage = staged(move || {
    board(&bound)
      .style(EventCalendarPart::Label, |s| s.foreground(PaletteColor::Primary))
      .force(Some(Interaction::FocusVisible))
  });
  assert_eq!(stage.text("Mon 9").0, light(PaletteColor::Primary));
  assert!(stage.has_ring(light(PaletteColor::Ring)), "today is drawn focused");
}

#[test]
fn the_first_frame_is_the_settled_one() {
  let value: Signal<Option<Date>> = Signal::new(Some(day(10)));
  let mut stage = Stage::first_frame_in(light_app(), 560.0, 600.0, move |ctx| {
    Column::new()
      .size(560.0, 600.0)
      .child(board(&value).header(true).period(EventCalendarPeriod::Month).mount(ctx))
      .into()
  });
  let first = stage.picture();
  stage.frame();
  assert_eq!(first, stage.picture());
}

#[cfg(any(feature = "mcp", feature = "devtools"))]
#[test]
fn to_tooling_the_days_are_a_grid_of_cells_and_an_event_is_a_button() {
  let value = Signal::new(Some(day(10)));
  let bound = value.clone();
  let mut stage = staged(move || board(&bound).label("Team calendar"));
  let mut said = |id: &str, key: &str| {
    stage
      .tree
      .get_element_by_id_mut(id)
      .and_then(|element| element.description(key))
  };
  assert_eq!(said("team", "a11y_role").as_deref(), Some("group"));
  assert_eq!(said("team", "a11y_name").as_deref(), Some("Team calendar"));
  assert_eq!(said("team-2026-11-10", "a11y_role").as_deref(), Some("gridcell"));
  assert_eq!(
    said("team-2026-11-10", "a11y_name").as_deref(),
    Some("Tuesday, Nov 10, 2026")
  );
  assert_eq!(said("team-2026-11-10", "a11y_selected").as_deref(), Some("true"));
  assert_eq!(said("team-2026-11-12", "a11y_selected").as_deref(), Some("false"));
  assert_eq!(said("team-2026-11-12", "a11y_current").as_deref(), Some("date"));
  assert_eq!(said("release", "a11y_role").as_deref(), Some("button"));
  assert_eq!(said("release", "a11y_name").as_deref(), Some("Release 0.60"));
}
