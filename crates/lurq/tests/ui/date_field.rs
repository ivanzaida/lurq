//! A date field: the board's box and segments, the segment with the focus, what typing, the arrows, Backspace and
//! the keys between segments do, when the value is a date, the forms bound and not, a field around it, the locale's
//! order and words, and what tooling is told.

use std::sync::{Arc, Mutex};

use lurq::{
  app::theme::{PaletteColor, ThemePalette},
  components::Column,
  core::{Date, DateLocale, Signal},
  node::color::Color,
  ui::{DateField, DateFieldSize, DateMemory, Field},
};

use super::stage::Stage;

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

fn day(year: i32, month: u8, day: u8) -> Date {
  Date::new(year, month, day).unwrap()
}

fn focused(stage: &Stage) -> Option<String> {
  stage
    .tree
    .focused_element()
    .and_then(|element| element.id().map(str::to_owned))
}

/// A field with the id `df`, mounted, in the board's English with the day first (en-GB), today the 9th of November.
fn field(value: &Signal<Date>, make: impl Fn(DateField) -> DateField + Send + Sync + 'static) -> Stage {
  let value = value.clone();
  Stage::new(move |ctx| {
    Column::new()
      .padding(10.0)
      .child(
        make(
          DateField::new(value.clone())
            .locale(DateLocale::for_tag("en-GB"))
            .today(day(2026, 11, 9))
            .id("df"),
        )
        .mount(ctx),
      )
      .into()
  })
}

/// An optional field with the id `df`, mounted.
fn optional(value: &Signal<Option<Date>>) -> Stage {
  let value = value.clone();
  Stage::new(move |ctx| {
    Column::new()
      .padding(10.0)
      .child(
        DateField::optional(value.clone())
          .locale(DateLocale::for_tag("en-GB"))
          .today(day(2026, 11, 9))
          .id("df")
          .mount(ctx),
      )
      .into()
  })
}

fn press(stage: &mut Stage, keys: &str) {
  for key in keys.chars() {
    stage.key(&key.to_string());
  }
}

#[test]
fn a_date_field_is_the_box_and_the_segments_of_the_design() {
  let value = Signal::new(day(2026, 11, 12));
  let mut stage = field(&value, |field| field);
  let (x, _, width, height) = stage.bounds("df");
  assert_eq!((width, height), (180.0, 36.0));
  assert_eq!(stage.fill_color("df"), Some(light(PaletteColor::Background)));
  let edge = stage
    .border("df")
    .and_then(|quad| quad.border)
    .and_then(|border| border.top);
  assert_eq!(
    edge.map(|side| (side.width, side.color)),
    Some((1.0, light(PaletteColor::Input)))
  );
  // The day first, as the board draws it: 12 / 11 / 2026, the separators in the muted foreground.
  let (dx, _, dwidth, dheight) = stage.bounds("df-day");
  assert_eq!((dx, dheight), (x + 10.0, 19.0));
  // 3 of padding either side of two digits of the mono type; 2 to the separator, 2 after it.
  let digits = stage.text_quad("12").unwrap().width;
  assert!((dwidth - (digits + 6.0)).abs() < 0.5);
  let slash = stage.text_quad("/").unwrap();
  assert!((slash.x - (dx + dwidth + 2.0)).abs() < 0.5);
  assert!((stage.bounds("df-month").0 - (slash.x + slash.width + 2.0)).abs() < 0.5);
  assert!(stage.bounds("df-year").2 > dwidth, "four digits");
  // The glyph at the end, 10 from it.
  let (gx, _, gwidth, _) = stage.bounds("df-icon");
  assert_eq!((gx + gwidth, gwidth), (x + 170.0, 16.0));
  assert_eq!(stage.text("12"), (light(PaletteColor::Foreground), 13.0));
  assert_eq!(stage.text("/").0, light(PaletteColor::MutedForeground));
  assert_eq!(stage.font_of("2026"), stage.font_of("/"));
  // No segment is filled while none has the focus.
  assert_eq!(stage.fill_color("df-month"), None);
}

#[test]
fn the_segment_with_the_focus_is_on_the_info_color() {
  let value = Signal::new(day(2026, 11, 12));
  let mut stage = field(&value, |field| field);
  stage.click("df-month");
  assert_eq!(focused(&stage).as_deref(), Some("df-month"));
  assert_eq!(stage.fill_color("df-month"), Some(light(PaletteColor::Info)));
  assert_eq!(stage.text("11").0, light(PaletteColor::DestructiveForeground));
  assert_eq!(stage.fill_color("df-day"), None);
  // Tab goes on to the next segment, and Right too.
  stage.key("Tab");
  assert_eq!(focused(&stage).as_deref(), Some("df-year"));
  stage.key("ArrowLeft");
  stage.key("ArrowLeft");
  assert_eq!(focused(&stage).as_deref(), Some("df-day"));
  assert_eq!(stage.fill_color("df-day"), Some(light(PaletteColor::Info)));
  assert_eq!(stage.fill_color("df-month"), None);
}

#[test]
fn digits_fill_a_segment_and_go_on_to_the_next() {
  let value = Signal::new(day(2026, 11, 12));
  let told = Arc::new(Mutex::new(Vec::new()));
  let heard = told.clone();
  let mut stage = field(&value, move |field| {
    let heard = heard.clone();
    field.on_change(move |date| heard.lock().unwrap().push(date))
  });
  stage.click("df-day");
  // 0 then 3: the 3rd, and on to the month.
  press(&mut stage, "03");
  assert_eq!(focused(&stage).as_deref(), Some("df-month"));
  assert_eq!(value.get_untracked(), day(2026, 11, 3));
  // 4 alone is April: no second digit makes a month.
  press(&mut stage, "4");
  assert_eq!(focused(&stage).as_deref(), Some("df-year"));
  assert_eq!(value.get_untracked(), day(2026, 4, 3));
  press(&mut stage, "202");
  assert!(stage.paints("202"), "the digits as typed");
  assert_eq!(value.get_untracked(), day(2026, 4, 3), "half a year is no date yet");
  press(&mut stage, "7");
  assert_eq!(value.get_untracked(), day(2027, 4, 3));
  assert_eq!(
    *told.lock().unwrap(),
    vec![day(2026, 11, 3), day(2026, 4, 3), day(2027, 4, 3)]
  );
  // The app setting the value tells nothing, and shows in the segments.
  value.set(day(2030, 1, 1));
  stage.pass();
  assert!(stage.paints("2030") && stage.paints("01"));
  assert_eq!(told.lock().unwrap().len(), 3);
}

#[test]
fn the_arrows_step_a_segment_round_and_home_and_end_go_to_its_ends() {
  let value = Signal::new(day(2026, 12, 31));
  let mut stage = field(&value, |field| field);
  stage.click("df-month");
  stage.key("ArrowUp");
  // December goes round to January; the 31st is a day of January too.
  assert_eq!(value.get_untracked(), day(2026, 1, 31));
  stage.key("ArrowDown");
  assert_eq!(value.get_untracked(), day(2026, 12, 31));
  stage.key("Home");
  assert_eq!(value.get_untracked(), day(2026, 1, 31));
  // February has no 31st: no date, and the field keeps the one it had.
  stage.key("ArrowUp");
  assert_eq!(value.get_untracked(), day(2026, 1, 31));
  assert!(stage.paints("02"), "the month is typed");
  stage.key("Tab");
  stage.key("ArrowUp");
  assert!(stage.paints("2027"));
  assert_eq!(
    value.get_untracked(),
    day(2026, 1, 31),
    "February has no 31st in 2027 either"
  );
  stage.key("ArrowLeft");
  stage.key("ArrowLeft");
  press(&mut stage, "28");
  assert_eq!(value.get_untracked(), day(2027, 2, 28));
}

#[test]
fn backspace_empties_a_segment_and_an_optional_field_is_none() {
  let value = Signal::new(Some(day(2026, 11, 12)));
  let mut stage = optional(&value);
  stage.click("df-month");
  stage.key("Backspace");
  assert_eq!(value.get_untracked(), None);
  assert!(stage.paints("mm"), "an empty segment shows what it is for");
  assert_eq!(
    stage.text("mm").0,
    light(PaletteColor::DestructiveForeground),
    "it has the focus"
  );
  stage.key("ArrowRight");
  assert_eq!(stage.text("mm").0, light(PaletteColor::MutedForeground));
  // Up on an empty year starts at this year.
  stage.key("Backspace");
  stage.key("ArrowUp");
  assert!(stage.paints("2026"));
  stage.key("ArrowLeft");
  press(&mut stage, "6");
  assert_eq!(value.get_untracked(), Some(day(2026, 6, 12)));
}

#[test]
fn a_controlled_field_shows_what_the_app_answers() {
  // The app holds the date and takes none in the past of 2000.
  let held = Signal::new(day(2026, 11, 12));
  let app = held.clone();
  let mut stage = Stage::new(move |ctx| {
    let answer = app.clone();
    Column::new()
      .padding(10.0)
      .child(
        DateField::controlled(app.get(), move |date| {
          if date.year() >= 2000 {
            answer.set(date);
          }
        })
        .locale(DateLocale::for_tag("en-GB"))
        .id("df")
        .mount(ctx),
      )
      .into()
  });
  stage.click("df-year");
  press(&mut stage, "2031");
  assert_eq!(held.get_untracked(), day(2031, 11, 12));
  assert!(stage.paints("2031"));
  stage.key("Backspace");
  press(&mut stage, "1999");
  assert_eq!(held.get_untracked(), day(2031, 11, 12), "the app said no");
}

#[test]
fn min_and_max_keep_a_date_out() {
  let value = Signal::new(day(2026, 11, 12));
  let mut stage = field(&value, |field| field.max(day(2026, 12, 31)));
  stage.click("df-year");
  press(&mut stage, "2027");
  assert_eq!(value.get_untracked(), day(2026, 11, 12));
  assert!(stage.paints("2027"), "what is typed stays, and is no date");
}

#[test]
fn the_locale_orders_and_names_the_segments() {
  let value = Signal::new(day(2026, 11, 12));
  let us = value.clone();
  let mut stage = Stage::new(move |ctx| {
    Column::new()
      .padding(10.0)
      .child(DateField::new(us.clone()).id("df").mount(ctx))
      .into()
  });
  // English: the month first.
  assert!(stage.bounds("df-month").0 < stage.bounds("df-day").0);
  let de = value.clone();
  let stage = Stage::new(move |ctx| {
    Column::new()
      .padding(10.0)
      .child(DateField::new(de.clone()).locale(DateLocale::for_tag("de")).mount(ctx))
      .into()
  });
  assert!(stage.paints(".") && stage.paints("12"));
}

#[test]
fn a_field_names_and_judges_it_and_its_sizes_are_an_inputs() {
  let value = Signal::new(Some(day(2026, 11, 12)));
  let (shown, memory) = (value.clone(), DateMemory::new());
  let mut stage = Stage::new(move |_| {
    Column::new()
      .padding(10.0)
      .child(
        Field::new(
          "Due date",
          DateField::optional(shown.clone())
            .locale(DateLocale::for_tag("en-GB"))
            .memory(&memory)
            .id("df"),
        )
        .error("Too late"),
      )
      .into()
  });
  let edge = stage
    .border("df")
    .and_then(|quad| quad.border)
    .and_then(|border| border.top);
  assert_eq!(edge.map(|side| side.color), Some(light(PaletteColor::Destructive)));
  // The memory keeps what is typed while it is no date.
  stage.click("df-day");
  stage.key("Backspace");
  stage.pass();
  assert!(stage.paints("dd"));
  assert_eq!(value.get_untracked(), None);

  let value = Signal::new(day(2026, 11, 12));
  let mut stage = field(&value, |field| field.size(DateFieldSize::Lg));
  assert_eq!(stage.bounds("df").3, 40.0);
  assert_eq!(stage.text("12").1, 14.0);
}

#[test]
fn a_disabled_field_takes_nothing() {
  let value = Signal::new(day(2026, 11, 12));
  let mut stage = field(&value, |field| field.disabled(true));
  stage.key("Tab");
  assert_eq!(focused(&stage), None);
  let mut stage = field(&value, |field| field.read_only(true));
  stage.click("df-day");
  press(&mut stage, "05");
  stage.key("ArrowUp");
  assert_eq!(value.get_untracked(), day(2026, 11, 12));
}

#[cfg(any(feature = "devtools", feature = "mcp"))]
#[test]
fn tooling_reads_a_group_of_spin_buttons() {
  let value = Signal::new(day(2026, 11, 12));
  let mut stage = field(&value, |field| field.label("Due date"));
  let said = |stage: &mut Stage, id: &str, name: &str| {
    stage
      .tree
      .get_element_by_id_mut(id)
      .and_then(|element| element.description(name))
  };
  assert_eq!(said(&mut stage, "df", "a11y_role").as_deref(), Some("group"));
  assert_eq!(said(&mut stage, "df", "a11y_name").as_deref(), Some("Due date"));
  assert_eq!(said(&mut stage, "df-month", "a11y_role").as_deref(), Some("spinbutton"));
  assert_eq!(said(&mut stage, "df-month", "a11y_name").as_deref(), Some("Month"));
  assert_eq!(said(&mut stage, "df-month", "a11y_valuenow").as_deref(), Some("11"));
  assert_eq!(
    said(&mut stage, "df-month", "a11y_valuetext").as_deref(),
    Some("November")
  );
  assert_eq!(said(&mut stage, "df-month", "a11y_valuemax").as_deref(), Some("12"));
}
