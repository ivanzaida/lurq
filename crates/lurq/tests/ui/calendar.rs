//! A calendar: the board's panel, heading and days, its states, what a press chooses, the month its buttons and its
//! keys show, a range chosen with two presses and shown under the pointer, two months, week numbers, the locale's
//! first day, what cannot be chosen, what tooling is told, and what a theme's rules and renderer change.

use std::sync::{Arc, Mutex};

use lurq::{
  app::theme::{PaletteColor, ThemePalette},
  components::{Column, Row},
  core::{Date, DateLocale, DateRange, Signal, Weekday, YearMonth},
  layout::{Constraints, Size},
  node::color::Color,
  ui::{Calendar, CalendarPart, CalendarRenderer, CalendarView, Recipe},
};

use super::stage::Stage;

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

fn day(month: u8, day: u8) -> Date {
  Date::new(2026, month, day).unwrap()
}

/// The id of a day of the calendar `cal`.
fn cell(month: u8, number: u8) -> String {
  format!("cal-{}", day(month, number))
}

/// A stage of 640 by 420: room for the six weeks of a calendar, 336 high, and for two months side by side.
fn roomy(stage: Stage) -> Stage {
  let mut stage = stage;
  stage
    .tree
    .set_layout_constraints_override(Some(Constraints::loose(Size::new(640.0, 420.0))));
  stage.pass();
  stage
}

/// A calendar of one date, mounted, with today the 9th of November 2026 as the board has it.
fn single(value: &Signal<Option<Date>>, make: impl Fn(Calendar) -> Calendar + Send + Sync + 'static) -> Stage {
  let value = value.clone();
  roomy(Stage::new(move |ctx| {
    Column::new()
      .padding(10.0)
      .child(make(Calendar::new(value.clone()).today(day(11, 9)).id("cal")).mount(ctx))
      .into()
  }))
}

/// A calendar of a range, mounted.
fn range(value: &Signal<Option<DateRange>>, make: impl Fn(Calendar) -> Calendar + Send + Sync + 'static) -> Stage {
  let value = value.clone();
  roomy(Stage::new(move |ctx| {
    Column::new()
      .padding(10.0)
      .child(make(Calendar::range(value.clone()).today(day(11, 9)).id("cal")).mount(ctx))
      .into()
  }))
}

fn focused(stage: &Stage) -> Option<String> {
  stage
    .tree
    .focused_element()
    .and_then(|element| element.id().map(str::to_owned))
}

fn press_key(stage: &mut Stage, key: &str, shift: bool) {
  stage.tree.key_down(key.to_owned(), key.to_owned(), shift, false, false);
  stage.tree.key_up(key.to_owned(), key.to_owned(), shift, false, false);
  stage.pass();
}

#[test]
fn a_calendar_is_the_panel_and_the_days_of_the_design() {
  let value = Signal::new(Some(day(11, 12)));
  let mut stage = single(&value, |calendar| calendar);
  let (x, y, width, height) = stage.bounds("cal");
  // 12 of padding around seven days of 32; a heading of 32 and seven rows of 32, 8 apart.
  assert_eq!(width, 248.0);
  assert!(
    (height - (24.0 + 32.0 + 8.0 + 7.0 * 32.0 + 6.0 * 8.0)).abs() < 0.5,
    "{height}"
  );
  assert_eq!(stage.fill_color("cal"), Some(light(PaletteColor::Popover)));
  let edge = stage
    .border("cal")
    .and_then(|quad| quad.border)
    .and_then(|border| border.top);
  assert_eq!(
    edge.map(|side| (side.width, side.color)),
    Some((1.0, light(PaletteColor::Border)))
  );

  assert_eq!(stage.text("November 2026"), (light(PaletteColor::Foreground), 14.0));
  assert_eq!(stage.text("Mo"), (light(PaletteColor::MutedForeground), 12.0));
  let (mo, su) = (stage.text_quad("Mo").unwrap(), stage.text_quad("Su").unwrap());
  assert!(mo.x < su.x, "the weeks start on Monday, as the design has them");

  // The first week starts on the 26th of October; the 1st of November ends it.
  let (dx, dy, dwidth, dheight) = stage.bounds(&cell(10, 26));
  assert_eq!((dwidth, dheight), (32.0, 32.0));
  assert_eq!(dx, x + 12.0);
  assert!((dy - (y + 12.0 + 32.0 + 8.0 + 32.0 + 8.0)).abs() < 0.5);
  assert_eq!(stage.bounds(&cell(11, 1)).0, x + 12.0 + 6.0 * 32.0);
  // Six weeks, to the 6th of December.
  stage.bounds(&cell(12, 6));

  // Chosen on the primary fill, today on the highlight, the others on none.
  assert_eq!(stage.fill_color(&cell(11, 12)), Some(light(PaletteColor::Primary)));
  assert_eq!(stage.fill_color(&cell(11, 9)), Some(light(PaletteColor::Highlight)));
  assert_eq!(stage.fill_color(&cell(11, 10)), None);
  assert_eq!(stage.text("12").0, light(PaletteColor::PrimaryForeground));
}

#[test]
fn a_press_chooses_a_day_and_the_pointer_lights_one() {
  let value = Signal::new(Some(day(11, 12)));
  let told = Arc::new(Mutex::new(Vec::new()));
  let heard = told.clone();
  let mut stage = single(&value, move |calendar| {
    let heard = heard.clone();
    calendar.on_change(move |date| heard.lock().unwrap().push(date))
  });
  stage.hover(&cell(11, 18));
  assert_eq!(stage.fill_color(&cell(11, 18)), Some(light(PaletteColor::Highlight)));
  stage.click(&cell(11, 20));
  assert_eq!(value.get_untracked(), Some(day(11, 20)));
  assert_eq!(*told.lock().unwrap(), vec![day(11, 20)]);
  assert_eq!(stage.fill_color(&cell(11, 20)), Some(light(PaletteColor::Primary)));
  assert_eq!(stage.fill_color(&cell(11, 12)), None);

  // A day of the month beside is chosen, and its month shown.
  stage.click(&cell(12, 2));
  assert_eq!(value.get_untracked(), Some(day(12, 2)));
  assert!(stage.paints("December 2026"));
}

#[test]
fn its_buttons_show_the_month_before_and_after_and_not_what_is_chosen() {
  let value = Signal::new(Some(day(11, 12)));
  let mut stage = single(&value, |calendar| calendar);
  stage.click("cal-next");
  assert!(stage.paints("December 2026"));
  assert_eq!(
    value.get_untracked(),
    Some(day(11, 12)),
    "the month shown is not the value"
  );
  stage.click("cal-previous");
  stage.click("cal-previous");
  assert!(stage.paints("October 2026"));

  // The app's own month.
  let month = Signal::new(YearMonth::new(2027, 3).unwrap());
  let shown = month.clone();
  let mut stage = single(&value, move |calendar| calendar.month(shown.clone()));
  assert!(stage.paints("March 2027"));
  stage.click("cal-next");
  assert_eq!(month.get_untracked(), YearMonth::new(2027, 4).unwrap());
  assert!(stage.paints("April 2027"));
}

#[test]
fn a_value_set_from_outside_shows_its_month() {
  let value = Signal::new(Some(day(11, 12)));
  let mut stage = single(&value, |calendar| calendar);
  value.set(Some(Date::new(2027, 2, 3).unwrap()));
  stage.pass();
  assert!(stage.paints("February 2027"));
}

#[test]
fn the_keys_move_by_a_day_a_week_and_a_month_and_choose() {
  let value = Signal::new(Some(day(11, 12)));
  let mut stage = single(&value, |calendar| calendar);
  // One stop of Tab past the buttons: at the chosen day.
  stage.key("Tab");
  stage.key("Tab");
  stage.key("Tab");
  assert_eq!(focused(&stage), Some(cell(11, 12)));
  assert!(stage.tree.focus_visible());
  stage.key("ArrowRight");
  assert_eq!(focused(&stage), Some(cell(11, 13)));
  stage.key("ArrowDown");
  assert_eq!(focused(&stage), Some(cell(11, 20)));
  stage.key("ArrowLeft");
  stage.key("ArrowUp");
  assert_eq!(focused(&stage), Some(cell(11, 12)));
  // Home and End go to the ends of the week, Monday to Sunday.
  stage.key("Home");
  assert_eq!(focused(&stage), Some(cell(11, 9)));
  stage.key("End");
  assert_eq!(focused(&stage), Some(cell(11, 15)));
  // PageDown goes a month on, into the month after, which is then shown.
  stage.key("PageDown");
  assert_eq!(focused(&stage), Some(cell(12, 15)));
  assert!(stage.paints("December 2026"));
  // With Shift, a year.
  press_key(&mut stage, "PageUp", true);
  assert_eq!(
    focused(&stage),
    Some(format!("cal-{}", Date::new(2025, 12, 15).unwrap()))
  );
  assert!(stage.paints("December 2025"));
  // Down from the last week crosses into the month after.
  press_key(&mut stage, "PageDown", true);
  stage.key("ArrowDown");
  stage.key("ArrowDown");
  assert_eq!(focused(&stage), Some(cell(12, 29)));
  stage.key("ArrowDown");
  assert_eq!(focused(&stage), Some(format!("cal-{}", Date::new(2027, 1, 5).unwrap())));
  assert!(stage.paints("January 2027"));
  // Enter chooses the focused day.
  stage.key("Enter");
  assert_eq!(value.get_untracked(), Some(Date::new(2027, 1, 5).unwrap()));
  // Tab leaves the days.
  stage.key("Tab");
  assert_ne!(focused(&stage), Some(format!("cal-{}", Date::new(2027, 1, 5).unwrap())));
}

#[test]
fn tab_enters_at_today_where_nothing_is_chosen() {
  let value = Signal::new(None);
  let mut stage = single(&value, |calendar| calendar);
  for _ in 0..3 {
    stage.key("Tab");
  }
  assert_eq!(focused(&stage), Some(cell(11, 9)));
}

#[test]
fn a_day_that_cannot_be_chosen_takes_the_focus_and_nothing_else() {
  let value = Signal::new(Some(day(11, 12)));
  let mut stage = single(&value, |calendar| {
    calendar
      .min(day(11, 4))
      .disabled_days(|date| date.weekday() == Weekday::Saturday)
  });
  stage.click(&cell(11, 14));
  assert_eq!(value.get_untracked(), Some(day(11, 12)), "a Saturday cannot be chosen");
  stage.click(&cell(11, 3));
  assert_eq!(value.get_untracked(), Some(day(11, 12)), "nor a day before the least");
  // It is muted, and the pointer lights nothing on it.
  assert_eq!(stage.text("14").0, light(PaletteColor::MutedForeground));
  stage.hover(&cell(11, 14));
  assert_eq!(stage.fill_color(&cell(11, 14)), None);

  // The keys reach it, and pass none by; Enter on it chooses nothing. The button before is disabled, at the least
  // month: Tab stops at the one after, then at the days.
  stage.key("Tab");
  stage.key("Tab");
  stage.key("ArrowRight");
  stage.key("ArrowRight");
  assert_eq!(focused(&stage), Some(cell(11, 14)));
  stage.key("Enter");
  assert_eq!(value.get_untracked(), Some(day(11, 12)));
  // The keys stop at the least date, and the month before it is not shown.
  stage.key("PageUp");
  assert_eq!(focused(&stage), Some(cell(11, 4)));
  stage.click("cal-previous");
  assert!(stage.paints("November 2026"));
}

#[test]
fn a_range_is_two_presses_and_the_pointer_shows_where_it_would_end() {
  let value = Signal::new(None);
  let told = Arc::new(Mutex::new(Vec::new()));
  let heard = told.clone();
  let mut stage = range(&value, move |calendar| {
    let heard = heard.clone();
    calendar.on_range_change(move |range| heard.lock().unwrap().push(range))
  });
  stage.click(&cell(11, 15));
  assert_eq!(value.get_untracked(), None, "one press is no range yet");
  assert_eq!(stage.fill_color(&cell(11, 15)), Some(light(PaletteColor::Primary)));
  stage.hover(&cell(11, 9));
  // The days between are one band on the highlight, square; the ends are chosen days.
  let middle = stage.fill(&cell(11, 12)).expect("a day in the range is filled");
  assert!(middle.border_radius.is_none_or(|radius| radius.top_left == 0.0));
  assert_eq!(stage.fill_color(&cell(11, 12)), Some(light(PaletteColor::Highlight)));
  assert_eq!(stage.fill_color(&cell(11, 9)), Some(light(PaletteColor::Primary)));
  assert_eq!(stage.fill_color(&cell(11, 16)), None);
  // The second press ends it, before the first or after it.
  stage.click(&cell(11, 9));
  let week = DateRange::new(day(11, 9), day(11, 15));
  assert_eq!(value.get_untracked(), Some(week));
  assert_eq!(*told.lock().unwrap(), vec![week]);
  assert_eq!(stage.fill_color(&cell(11, 10)), Some(light(PaletteColor::Highlight)));

  // From the keyboard: Enter starts it, the keys show where it would end, Enter ends it.
  for _ in 0..3 {
    stage.key("Tab");
  }
  assert_eq!(focused(&stage), Some(cell(11, 9)));
  stage.key("Enter");
  stage.key("ArrowDown");
  assert_eq!(stage.fill_color(&cell(11, 12)), Some(light(PaletteColor::Highlight)));
  stage.key("Enter");
  assert_eq!(value.get_untracked(), Some(DateRange::new(day(11, 9), day(11, 16))));
}

#[test]
fn two_months_side_by_side_and_the_keys_go_from_one_to_the_other() {
  let value = Signal::new(Some(day(11, 30)));
  let mut stage = single(&value, |calendar| calendar.months(2));
  assert!(stage.paints("November 2026") && stage.paints("December 2026"));
  let (nov, dec) = (stage.bounds(&cell(11, 30)), stage.bounds(&cell(12, 1)));
  assert!(dec.0 > nov.0 + 7.0 * 32.0, "December beside November, 16 apart");
  // No day of the month beside fills a week of November: the 1st of December is once in the calendar, in December.
  assert!(stage.bounds(&cell(12, 1)).1 < stage.bounds(&cell(11, 30)).1);
  for _ in 0..3 {
    stage.key("Tab");
  }
  assert_eq!(focused(&stage), Some(cell(11, 30)));
  stage.key("ArrowRight");
  assert_eq!(focused(&stage), Some(cell(12, 1)));
  assert!(stage.paints("November 2026"), "the months shown do not move");
}

#[test]
fn week_numbers_and_the_first_day_of_a_locale() {
  let value = Signal::new(Some(day(11, 12)));
  let mut stage = single(&value, |calendar| calendar.week_numbers(true));
  assert!(stage.paints("46"));
  assert_eq!(stage.bounds("cal").2, 248.0 + 32.0, "a column of 32 more");

  let stage = single(&value, |calendar| calendar.locale(DateLocale::for_tag("en-US")));
  let (su, mo) = (stage.text_quad("Su").unwrap(), stage.text_quad("Mo").unwrap());
  assert!(su.x < mo.x, "weeks from Sunday");
  let mut stage = single(&value, |calendar| {
    calendar.locale(DateLocale::for_tag("uk")).first_day(Weekday::Sunday)
  });
  assert!(stage.paints("Листопад 2026"));
  let (nd, pn) = (stage.text_quad("Нд").unwrap(), stage.text_quad("Пн").unwrap());
  assert!(nd.x < pn.x, "the app's first day over the locale's");
  stage.bounds(&cell(11, 1));
}

#[test]
fn a_disabled_calendar_takes_nothing() {
  let value = Signal::new(Some(day(11, 12)));
  let mut stage = single(&value, |calendar| calendar.disabled(true));
  stage.click(&cell(11, 20));
  stage.click("cal-next");
  assert_eq!(value.get_untracked(), Some(day(11, 12)));
  assert!(stage.paints("November 2026"));
  stage.key("Tab");
  assert_eq!(focused(&stage), None);
}

#[cfg(any(feature = "devtools", feature = "mcp"))]
#[test]
fn tooling_reads_a_grid_of_days_called_by_its_month() {
  let value = Signal::new(Some(day(11, 12)));
  let mut stage = single(&value, |calendar| calendar.disabled_days(|date| date.day() == 20));
  let said = |stage: &mut Stage, id: &str, name: &str| {
    stage
      .tree
      .get_element_by_id_mut(id)
      .and_then(|element| element.description(name))
  };
  assert_eq!(
    said(&mut stage, &cell(11, 12), "a11y_role").as_deref(),
    Some("gridcell")
  );
  assert_eq!(
    said(&mut stage, &cell(11, 12), "a11y_name").as_deref(),
    Some("Thursday, Nov 12, 2026")
  );
  assert_eq!(
    said(&mut stage, &cell(11, 12), "a11y_selected").as_deref(),
    Some("true")
  );
  assert_eq!(
    said(&mut stage, &cell(11, 13), "a11y_selected").as_deref(),
    Some("false")
  );
  assert_eq!(said(&mut stage, &cell(11, 9), "a11y_current").as_deref(), Some("date"));
  assert_eq!(
    said(&mut stage, &cell(11, 20), "a11y_disabled").as_deref(),
    Some("true")
  );
  assert_eq!(said(&mut stage, "cal-next", "a11y_name").as_deref(), Some("Next month"));
}

#[test]
fn a_themes_rules_and_renderer_change_it() {
  let value = Signal::new(Some(day(11, 12)));
  let rules = Recipe::new().state(lurq::ui::CalendarState::Selected, CalendarPart::Day, |s| {
    s.background(PaletteColor::Destructive)
  });
  let mut stage = single(&value, move |calendar| calendar.rules(&rules));
  assert_eq!(stage.fill_color(&cell(11, 12)), Some(light(PaletteColor::Destructive)));

  let app = lurq::app::App::new();
  let renderer: CalendarRenderer = Arc::new(|view: CalendarView| {
    // The months' headings alone, in a row of their own.
    Row::new().id("own").child(view.headers()).into()
  });
  app.theme().set_renderer::<Calendar>(renderer);
  let shown = value.clone();
  let mut stage = Stage::in_app(app, move |ctx| {
    Column::new()
      .child(Calendar::new(shown.clone()).today(day(11, 9)).mount(ctx))
      .into()
  });
  stage.bounds("own");
  assert!(stage.paints("November 2026"));
  assert!(!stage.paints("Mo"));
}
