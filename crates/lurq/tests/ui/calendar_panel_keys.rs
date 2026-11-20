//! A calendar whose panel reads the keys while the app keeps the focus elsewhere (`CalendarKeys::Panel`), and its
//! heading with the month first and both buttons at the end (`CalendarHeading::MonthThenNav`).

use std::sync::{Arc, Mutex};

use lurq::{
  components::{Column, TextInput},
  core::{Date, Signal},
  layout::{Constraints, Size},
  ui::{Calendar, CalendarHeading, CalendarKeys},
};

use super::stage::Stage;

fn day(month: u8, day: u8) -> Date {
  Date::new(2026, month, day).unwrap()
}

fn cell(month: u8, number: u8) -> String {
  format!("cal-day-{}", day(month, number))
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

/// A field, and under it a mounted calendar whose panel reads the keys, today the 9th of November 2026; what it
/// tells `on_choose` is written in `chosen`.
fn field_and_calendar(value: &Signal<Option<Date>>, chosen: &Arc<Mutex<Vec<Date>>>) -> Stage {
  let (value, chosen) = (value.clone(), chosen.clone());
  let mut stage = Stage::new(move |ctx| {
    let chosen = chosen.clone();
    Column::new()
      .padding(10.0)
      .child(TextInput::new(Signal::new(String::new())).id("field").width(200.0))
      .child(
        Calendar::new(value.clone())
          .today(day(11, 9))
          .id("cal")
          .keys(CalendarKeys::Panel)
          .on_choose(move |date| chosen.lock().unwrap().push(date))
          .mount(ctx),
      )
      .into()
  });
  stage
    .tree
    .set_layout_constraints_override(Some(Constraints::loose(Size::new(640.0, 460.0))));
  stage.pass();
  stage
}

#[test]
fn the_panel_reads_the_keys_while_the_focus_stays_in_the_field() {
  let (value, chosen) = (Signal::new(Some(day(11, 12))), Arc::new(Mutex::new(Vec::new())));
  let mut stage = field_and_calendar(&value, &chosen);
  stage.click("field");
  press_key(&mut stage, "ArrowRight", false);
  press_key(&mut stage, "ArrowDown", false);
  assert_eq!(focused(&stage).as_deref(), Some("field"), "the focus stays in the field");
  assert_eq!(value.get(), Some(day(11, 12)), "the keys only move the day they are on");
  press_key(&mut stage, "Enter", false);
  assert_eq!(value.get(), Some(day(11, 20)), "a day, then a week, and Enter chooses it");
  assert_eq!(*chosen.lock().unwrap(), [day(11, 20)], "and tells on_choose");

  press_key(&mut stage, "PageDown", false);
  assert!(stage.ids().contains(&cell(12, 20)), "a month on, which is shown");
  press_key(&mut stage, "Enter", false);
  assert_eq!(value.get(), Some(day(12, 20)));
  assert_eq!(focused(&stage).as_deref(), Some("field"));
}

#[test]
fn enter_on_the_chosen_day_tells_on_choose_again_and_a_day_takes_no_focus() {
  let (value, chosen) = (Signal::new(Some(day(11, 12))), Arc::new(Mutex::new(Vec::new())));
  let mut stage = field_and_calendar(&value, &chosen);
  stage.click("field");
  press_key(&mut stage, "Enter", false);
  assert_eq!(*chosen.lock().unwrap(), [day(11, 12)], "what is chosen already is told too");

  stage.click(&cell(11, 3));
  assert_eq!(value.get(), Some(day(11, 3)), "a press chooses as before");
  assert_eq!(focused(&stage).as_deref(), Some("field"), "and leaves the focus where it was");
  press_key(&mut stage, "ArrowRight", false);
  press_key(&mut stage, "Enter", false);
  assert_eq!(value.get(), Some(day(11, 4)), "the keys went to what was chosen");
}

#[test]
fn a_key_a_handler_before_took_is_left_alone() {
  let (value, chosen) = (Signal::new(Some(day(11, 12))), Arc::new(Mutex::new(Vec::new())));
  let (held, told) = (value.clone(), chosen.clone());
  let mut stage = Stage::new(move |ctx| {
    let told = told.clone();
    Column::new()
      .on_key_down(|event: lurq::app::events::KeyboardEvent| event.prevent_default())
      .child(
        Calendar::new(held.clone())
          .today(day(11, 9))
          .id("cal")
          .keys(CalendarKeys::Panel)
          .on_choose(move |date| told.lock().unwrap().push(date))
          .mount(ctx),
      )
      .into()
  });
  press_key(&mut stage, "ArrowRight", false);
  press_key(&mut stage, "Enter", false);
  assert_eq!(value.get(), Some(day(11, 12)));
  assert!(chosen.lock().unwrap().is_empty());
}

#[cfg(any(feature = "devtools", feature = "mcp"))]
#[test]
fn tooling_reads_the_day_the_keys_are_on() {
  let (value, chosen) = (Signal::new(Some(day(11, 12))), Arc::new(Mutex::new(Vec::new())));
  let mut stage = field_and_calendar(&value, &chosen);
  let said = |stage: &mut Stage, id: &str, name: &str| {
    stage
      .tree
      .get_element_by_id_mut(id)
      .and_then(|element| element.description(name))
  };
  assert_eq!(said(&mut stage, "cal", "cursor").as_deref(), Some("2026-11-12"), "the chosen day to begin with");
  assert_eq!(said(&mut stage, &cell(11, 12), "cursor").as_deref(), Some("true"));
  press_key(&mut stage, "ArrowLeft", false);
  assert_eq!(said(&mut stage, "cal", "cursor").as_deref(), Some("2026-11-11"));
}

#[test]
fn the_month_first_and_both_buttons_at_the_end() {
  let value = Signal::new(Some(day(11, 12)));
  let held = value.clone();
  let mut stage = Stage::new(move |ctx| {
    Column::new()
      .padding(10.0)
      .child(
        Calendar::new(held.clone())
          .today(day(11, 9))
          .id("cal")
          .heading(CalendarHeading::MonthThenNav)
          .mount(ctx),
      )
      .into()
  });
  stage
    .tree
    .set_layout_constraints_override(Some(Constraints::loose(Size::new(640.0, 460.0))));
  stage.pass();
  let (title_x, _, title_width, _) = stage.bounds("cal-title");
  let (previous_x, _, previous_width, _) = stage.bounds("cal-nav-previous");
  let (next_x, _, next_width, _) = stage.bounds("cal-nav-next");
  let (header_x, _, header_width, _) = stage.bounds("cal-header");
  assert!((title_x - header_x).abs() < 0.5, "the month at the heading's start");
  assert!(previous_x >= title_x + title_width, "the buttons after it");
  assert!(
    (next_x - (previous_x + previous_width) - 4.0).abs() < 0.5,
    "4 apart"
  );
  assert!(
    (next_x + next_width - (header_x + header_width)).abs() < 0.5,
    "at the heading's end"
  );
  stage.click("cal-nav-next");
  assert!(stage.ids().contains(&cell(12, 1)), "they still turn the month");
}

#[test]
fn in_a_popover_that_keeps_the_focus_enter_picks_the_day_and_the_picker_closes() {
  use lurq::{core::ElementRef, ui::Popover};
  let (value, open) = (Signal::new(Some(day(11, 12))), Signal::new(false));
  let (held, shown, field) = (value.clone(), open.clone(), ElementRef::new());
  let mut stage = Stage::new(move |ctx| {
    let close = shown.clone();
    let calendar = Calendar::new(held.clone())
      .today(day(11, 9))
      .id("cal")
      .keys(CalendarKeys::Panel)
      .on_choose(move |_| close.set(false))
      .mount(ctx);
    Column::new()
      .padding(10.0)
      .child(
        TextInput::new(Signal::new(String::new()))
          .id("field")
          .width(200.0)
          .ref_element(field.clone()),
      )
      .child(
        Popover::new(shown.clone(), field.clone())
          .keeps_focus(true)
          .child(calendar),
      )
      .into()
  });
  stage
    .tree
    .set_layout_constraints_override(Some(Constraints::loose(Size::new(640.0, 520.0))));
  stage.pass();
  stage.click("field");
  open.set(true);
  stage.pass();
  assert!(stage.ids().contains(&cell(11, 12)), "the picker is open");
  assert_eq!(focused(&stage).as_deref(), Some("field"), "the popover took no focus");
  press_key(&mut stage, "ArrowUp", false);
  press_key(&mut stage, "Enter", false);
  assert_eq!(value.get(), Some(day(11, 5)));
  assert!(!open.get(), "the picker closed");
  assert_eq!(focused(&stage).as_deref(), Some("field"));
}
