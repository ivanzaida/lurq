//! A time field: the board's box, the hours and the minutes, the step of the minutes, the seconds, a clock of twelve
//! hours with its half of the day, the range, and what tooling is told.

use std::sync::{Arc, Mutex};

use lurq::{
  app::theme::{PaletteColor, ThemePalette},
  components::Column,
  core::{DateLocale, HourCycle, Signal, Time},
  node::color::Color,
  ui::TimeField,
};

use super::stage::Stage;

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

fn at(hour: u8, minute: u8) -> Time {
  Time::new(hour, minute, 0).unwrap()
}

fn focused(stage: &Stage) -> Option<String> {
  stage
    .tree
    .focused_element()
    .and_then(|element| element.id().map(str::to_owned))
}

/// A field with the id `tf`, mounted.
fn field(value: &Signal<Time>, make: impl Fn(TimeField) -> TimeField + Send + Sync + 'static) -> Stage {
  let value = value.clone();
  Stage::new(move |ctx| {
    Column::new()
      .padding(10.0)
      .child(make(TimeField::new(value.clone()).id("tf")).mount(ctx))
      .into()
  })
}

fn press(stage: &mut Stage, keys: &str) {
  for key in keys.chars() {
    stage.key(&key.to_string());
  }
}

#[test]
fn a_time_field_is_the_box_of_the_design_with_its_hours_and_minutes() {
  let value = Signal::new(at(14, 2));
  let mut stage = field(&value, |field| field);
  let (x, _, width, height) = stage.bounds("tf");
  assert_eq!((width, height), (120.0, 36.0));
  assert_eq!(stage.bounds("tf-hour").0, x + 10.0);
  assert!(stage.paints("14") && stage.paints("02") && stage.paints(":"));
  assert_eq!(stage.text(":").0, light(PaletteColor::MutedForeground));
  let (gx, _, gwidth, _) = stage.bounds("tf-icon");
  assert_eq!((gx + gwidth, gwidth), (x + 110.0, 16.0));
  stage.click("tf-minute");
  assert_eq!(stage.fill_color("tf-minute"), Some(light(PaletteColor::Selection)));
}

#[test]
fn digits_and_the_arrows_set_it_and_the_minutes_step() {
  let value = Signal::new(at(14, 2));
  let told = Arc::new(Mutex::new(Vec::new()));
  let heard = told.clone();
  let mut stage = field(&value, move |field| {
    let heard = heard.clone();
    field.step(15).on_change(move |time| heard.lock().unwrap().push(time))
  });
  stage.click("tf-hour");
  // 3 alone is 03: no hour is 30 or more.
  press(&mut stage, "3");
  assert_eq!(focused(&stage).as_deref(), Some("tf-minute"));
  assert_eq!(value.get_untracked(), at(3, 2));
  // A step of 15 goes to the next quarter, and round the hour.
  stage.key("ArrowUp");
  assert_eq!(value.get_untracked(), at(3, 15));
  stage.key("ArrowDown");
  stage.key("ArrowDown");
  assert_eq!(value.get_untracked(), at(3, 45));
  press(&mut stage, "7");
  assert_eq!(value.get_untracked(), at(3, 7), "any minute is typed");
  // Round the clock: 23 is before 0.
  stage.key("ArrowLeft");
  stage.key("Home");
  stage.key("ArrowDown");
  assert_eq!(value.get_untracked(), at(23, 7));
  // 3:02, 3:15, 3:00, 3:45, 3:07, 0:07, 23:07.
  assert_eq!(told.lock().unwrap().len(), 7);
}

#[test]
fn seconds_and_a_clock_of_twelve_hours() {
  let value = Signal::new(at(14, 2));
  let mut stage = field(&value, |field| field.seconds(true));
  assert!(stage.paints("00"));
  stage.click("tf-second");
  press(&mut stage, "45");
  assert_eq!(value.get_untracked(), Time::new(14, 2, 45).unwrap());
  assert!(stage.bounds("tf").2 > 120.0, "wider for its seconds");

  let mut stage = field(&value, |field| field.locale(DateLocale::for_tag("en-US")));
  assert!(stage.paints("02") && stage.paints("PM"));
  stage.click("tf-period");
  press(&mut stage, "a");
  assert_eq!(value.get_untracked(), Time::new(2, 2, 45).unwrap());
  stage.key("ArrowUp");
  assert_eq!(value.get_untracked(), Time::new(14, 2, 45).unwrap());
  let mut stage = field(&value, |field| field.hour_cycle(HourCycle::H12));
  stage.bounds("tf-period");
}

#[test]
fn a_time_out_of_the_range_is_no_value() {
  let value = Signal::new(at(9, 0));
  let mut stage = field(&value, |field| field.min(at(8, 0)).max(at(18, 0)));
  stage.click("tf-hour");
  press(&mut stage, "19");
  assert_eq!(value.get_untracked(), at(9, 0));
  stage.key("ArrowLeft");
  press(&mut stage, "17");
  assert_eq!(value.get_untracked(), at(17, 0));
}

#[test]
fn an_optional_field_is_none_while_a_segment_is_empty() {
  let value = Signal::new(Some(at(9, 30)));
  let shown = value.clone();
  let mut stage = Stage::new(move |ctx| {
    Column::new()
      .padding(10.0)
      .child(TimeField::optional(shown.clone()).id("tf").mount(ctx))
      .into()
  });
  stage.click("tf-minute");
  stage.key("Backspace");
  assert_eq!(value.get_untracked(), None);
  assert!(stage.paints("mm"));
  press(&mut stage, "45");
  assert_eq!(value.get_untracked(), Some(at(9, 45)));
}

#[cfg(any(feature = "devtools", feature = "mcp"))]
#[test]
fn tooling_reads_the_hours_and_the_minutes() {
  let value = Signal::new(at(14, 2));
  let mut stage = field(&value, |field| field.label("Starts at"));
  let said = |stage: &mut Stage, id: &str, name: &str| {
    stage
      .tree
      .get_element_by_id_mut(id)
      .and_then(|element| element.description(name))
  };
  assert_eq!(said(&mut stage, "tf", "a11y_name").as_deref(), Some("Starts at"));
  assert_eq!(said(&mut stage, "tf-hour", "a11y_name").as_deref(), Some("Hour"));
  assert_eq!(said(&mut stage, "tf-minute", "a11y_valuenow").as_deref(), Some("2"));
  assert_eq!(said(&mut stage, "tf-minute", "a11y_valuemax").as_deref(), Some("59"));
}
