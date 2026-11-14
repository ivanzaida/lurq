//! A date range picker: the board's field with its range in one line, two presses in its calendar ending a range and
//! closing it, the band of the range, what typing a range sets, its presets, two months, and Escape dropping a range
//! half chosen.

use lurq::{
  app::theme::{PaletteColor, ThemePalette},
  components::Column,
  core::{Date, DateRange, Signal},
  layout::{Constraints, Size},
  node::color::Color,
  ui::DateRangePicker,
};

use super::stage::Stage;

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

fn day(month: u8, day: u8) -> Date {
  Date::new(2026, month, day).unwrap()
}

/// The id of a day of the picker's calendar.
fn cell(month: u8, number: u8) -> String {
  format!("rp-calendar-{}", day(month, number))
}

/// A range picker with the id `rp`, mounted at the top of a window of 640 by 500, with today the 9th of November.
fn picker(
  value: &Signal<Option<DateRange>>,
  make: impl Fn(DateRangePicker) -> DateRangePicker + Send + Sync + 'static,
) -> Stage {
  let value = value.clone();
  let mut stage = Stage::new(move |ctx| {
    Column::new()
      .size(640.0, 500.0)
      .padding(10.0)
      .child(make(DateRangePicker::new(value.clone()).today(day(11, 9)).id("rp")).mount(ctx))
      .into()
  });
  stage
    .tree
    .set_layout_constraints_override(Some(Constraints::loose(Size::new(640.0, 500.0))));
  stage.pass();
  stage
}

fn opened(stage: &mut Stage) -> bool {
  stage.tree.get_element_by_id_mut("rp-panel").is_some()
}

#[test]
fn the_field_holds_the_range_in_one_line() {
  let value = Signal::new(Some(DateRange::new(day(11, 9), day(11, 15))));
  let mut stage = picker(&value, |picker| picker);
  assert_eq!(stage.bounds("rp").2, 260.0);
  assert_eq!(
    stage.text("Nov 9 – Nov 15, 2026"),
    (light(PaletteColor::Foreground), 14.0)
  );
}

#[test]
fn two_presses_choose_a_range_and_close_the_calendar() {
  let value = Signal::new(None);
  let mut stage = picker(&value, |picker| picker);
  stage.click("rp-trigger");
  stage.pass();
  assert!(opened(&mut stage));
  stage.click(&cell(11, 9));
  assert!(opened(&mut stage), "one press is half a range");
  assert_eq!(value.get_untracked(), None);
  stage.hover(&cell(11, 15));
  assert_eq!(stage.fill_color(&cell(11, 12)), Some(light(PaletteColor::Highlight)));
  stage.click(&cell(11, 15));
  stage.pass();
  assert_eq!(value.get_untracked(), Some(DateRange::new(day(11, 9), day(11, 15))));
  assert!(!opened(&mut stage));
  assert!(stage.paints("Nov 9 – Nov 15, 2026"));

  // Opened again, the range is the band of the design: its ends chosen, the days between on the highlight.
  stage.click("rp-trigger");
  stage.pass();
  assert_eq!(stage.fill_color(&cell(11, 9)), Some(light(PaletteColor::Primary)));
  assert_eq!(stage.fill_color(&cell(11, 15)), Some(light(PaletteColor::Primary)));
  assert_eq!(stage.fill_color(&cell(11, 10)), Some(light(PaletteColor::Highlight)));
}

#[test]
fn escape_drops_a_range_half_chosen() {
  let value = Signal::new(Some(DateRange::new(day(11, 9), day(11, 15))));
  let mut stage = picker(&value, |picker| picker);
  stage.click("rp-trigger");
  stage.pass();
  stage.click(&cell(11, 20));
  stage.key("Escape");
  stage.pass();
  assert!(!opened(&mut stage));
  assert_eq!(value.get_untracked(), Some(DateRange::new(day(11, 9), day(11, 15))));
  stage.click("rp-trigger");
  stage.pass();
  assert_eq!(
    stage.fill_color(&cell(11, 20)),
    None,
    "the first press is forgotten as the calendar closes"
  );
}

#[test]
fn a_typed_range_is_the_value() {
  let value = Signal::new(None);
  let mut stage = picker(&value, |picker| picker);
  stage.click("rp");
  stage.tree.text_input_replace(0..0, "Dec 1 – Dec 7, 2026");
  stage.pass();
  assert_eq!(value.get_untracked(), Some(DateRange::new(day(12, 1), day(12, 7))));
}

#[test]
fn a_preset_chooses_its_range_and_closes_the_calendar() {
  let value = Signal::new(None);
  let week = DateRange::new(day(11, 3), day(11, 9));
  let mut stage = picker(&value, move |picker| {
    picker
      .preset("Last 7 days", week)
      .preset("This month", DateRange::new(day(11, 1), day(11, 30)))
  });
  stage.click("rp-trigger");
  stage.pass();
  let (px, ..) = stage.bounds("rp-preset-last-7-days");
  let (cx, ..) = stage.bounds("rp-calendar");
  assert!(px < cx, "the presets are beside the calendar, before it");
  stage.click("rp-preset-last-7-days");
  stage.pass();
  assert_eq!(value.get_untracked(), Some(week));
  assert!(!opened(&mut stage));
}

#[test]
fn two_months_side_by_side() {
  let value = Signal::new(None);
  let mut stage = picker(&value, |picker| picker.months(2));
  stage.click("rp-trigger");
  stage.pass();
  assert!(stage.paints("November 2026") && stage.paints("December 2026"));
  stage.click(&cell(11, 28));
  stage.click(&cell(12, 3));
  stage.pass();
  assert_eq!(value.get_untracked(), Some(DateRange::new(day(11, 28), day(12, 3))));
}
